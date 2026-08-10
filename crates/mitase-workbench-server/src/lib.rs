#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Extension, Path as AxumPath, Query, Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
};
use mitase_delivery::DeliveryStore;
use mitase_diagnostics::{Severity, ValidationPhase, ValidationResult};
use mitase_planner::{
    SplitWorkRecommendation, TargetSuggestion, TargetSuggestionSet, origin_closure_digest,
    origin_closure_for_slice, plan, split_work_recommendation, suggest_targets,
    validate_work_request,
};
use mitase_project_model::{ChangeBaseline, ReadinessLevel, ValidationPreset};
use mitase_spec_model::format_sha256;
use mitase_spec_model::{
    ArtifactBinding, ArtifactTarget, ArtifactTargetLifecycle, BindingRole, BoundTargetRef,
    Contract, ContractKind, Criterion, CriterionKind, ItemStatus, LocalAnchorKind, LocalId,
    OwnershipScope, Philosophy, Policy, Priority, RepoPath, Requirement, Rule, RuleLevel, Selector,
    SpecAnchor, SpecDocument, SpecId, TargetClaim,
};
use mitase_validation::{ChangeStatus, PlanValidationMode, ValidationContext, validate};
use mitase_work_model::{
    AgentBlocker, AgentEvent, AgentPatch, AgentRun, AgentRunStatus, CompletionAttempt,
    CompletionStatus, ExecutionIdentity, FinalizationPreview, FinalizationReceipt,
    PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus, RequestedTarget, TargetAccessMode,
    TargetTransition, VerificationReceipt, WORK_REQUEST_SCHEMA, WorkConstraints, WorkOperation,
    WorkOrigin, WorkPlan, WorkRequest,
};
use mitase_workspace::{SpecIndex, SpecWorkspace};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkbenchPage {
    #[default]
    Work,
    Readiness,
    Scope,
    Specifications,
    Diagnostics,
    Settings,
}

#[derive(Debug, Clone, Default)]
pub struct WorkbenchSession {
    pub selected_page: WorkbenchPage,
    pub selected_item: Option<String>,
    pub selected_slice: Option<String>,
    pub work_title: Option<String>,
    pub draft_request: Option<WorkRequest>,
    pub plan: Option<WorkPlan>,
    pub context_pack: Option<mitase_work_model::ContextPack>,
    pub verification_receipt: Option<VerificationReceipt>,
    pub agent_run: Option<AgentRun>,
    pub last_validation: Option<ValidationRunView>,
    pub readiness: Option<ReadinessView>,
    /// Rejections are tied to the evidence that was reviewed. A candidate is
    /// eligible again only after its evidence fingerprint changes.
    pub rejected_target_suggestions: BTreeMap<String, String>,
    pub approved_target_suggestions: Vec<ApprovedTargetSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedTargetSuggestion {
    pub criterion: SpecAnchor,
    pub suggestion_id: String,
    pub evidence_fingerprint: String,
}

pub struct WorkbenchEngine;
struct CachedWorkspaceSnapshot {
    signature: String,
    governance_signature: String,
    workspace: Arc<SpecWorkspace>,
    index: Arc<SpecIndex>,
    revision: String,
    projection: Arc<WorkspaceProjection>,
}
pub struct WorkbenchService {
    pub workspace_root: PathBuf,
    pub session: RwLock<WorkbenchSession>,
    pub engine: WorkbenchEngine,
    snapshot: RwLock<Option<Arc<CachedWorkspaceSnapshot>>>,
}
pub struct WorkbenchLaunchConfig {
    pub workspace_root: PathBuf,
    pub bind: IpAddr,
    pub port: u16,
    pub session_token: Option<String>,
    pub no_open: bool,
}
pub struct WorkbenchServer {
    pub service: Arc<WorkbenchService>,
    pub launch: WorkbenchLaunchConfig,
}

#[derive(Clone)]
struct ServerSecurity {
    expected_origin: String,
    csrf_token: String,
    remote_session_token: Option<String>,
}

impl WorkbenchServer {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            service: Arc::new(WorkbenchService {
                workspace_root: workspace_root.clone(),
                session: RwLock::new(WorkbenchSession::default()),
                engine: WorkbenchEngine,
                snapshot: RwLock::new(None),
            }),
            launch: WorkbenchLaunchConfig {
                workspace_root,
                bind: "127.0.0.1".parse().expect("loopback address"),
                port: 7737,
                session_token: None,
                no_open: false,
            },
        }
    }
    pub fn with_launch(mut self, launch: WorkbenchLaunchConfig) -> Self {
        self.launch = launch;
        self
    }
    pub fn with_request(self, request: WorkRequest) -> Result<Self> {
        let workspace = SpecWorkspace::load(&self.launch.workspace_root)?;
        let index = workspace.index()?;
        validate_work_origin(&workspace, &index, &request.origin)
            .context("preloaded Work request requires an exact implemented origin")?;
        mitase_planner::validate_work_request(&index, &request)
            .context("preloaded Work request contains an out-of-scope target")?;
        if let Ok(mut session) = self.service.session.write() {
            session.draft_request = Some(request);
        }
        Ok(self)
    }
    pub fn projection(&self, revision: &str) -> Result<WorkspaceProjection> {
        let snapshot = self.service.snapshot()?;
        if snapshot.revision != revision {
            anyhow::bail!("workspace revision changed while loading the Workbench projection");
        }
        let session = self.service.session.read().expect("workbench session lock");
        project_session(&snapshot, &session)
    }
    pub fn router(&self) -> Router {
        let bind = self.launch.bind;
        let port = self.launch.port;
        let security = ServerSecurity {
            expected_origin: format!("http://{bind}:{port}"),
            csrf_token: security_token(&self.launch.workspace_root),
            remote_session_token: (!bind.is_loopback()).then(|| {
                self.launch
                    .session_token
                    .clone()
                    .expect("remote bind requires a session token")
            }),
        };
        Router::new()
            .route("/api/projection", get(api_projection))
            .route("/", get(api_index))
            .route("/assets/{*asset}", get(api_asset))
            .route("/api/readiness", get(api_readiness))
            .route("/api/readiness/run", post(api_readiness_run))
            .route("/api/diagnostics/run", post(api_diagnostics_run))
            .route("/api/specifications", get(api_specifications))
            .route(
                "/api/specifications/candidates",
                get(api_specification_candidates),
            )
            .route(
                "/api/specifications/candidates/preview",
                post(api_specification_candidate_preview),
            )
            .route(
                "/api/specifications/candidates/apply",
                put(api_specification_candidate_apply),
            )
            .route(
                "/api/specifications/{item_id}/trace",
                get(api_specification_trace),
            )
            .route("/api/specifications/{anchor}", get(api_specification))
            .route(
                "/api/specifications/{anchor}/target-suggestions",
                get(api_target_suggestions),
            )
            .route(
                "/api/specifications/{anchor}/target-suggestions/reject",
                post(api_target_suggestion_reject),
            )
            .route(
                "/api/specifications/{anchor}/target-suggestions/approve",
                post(api_target_suggestions_approve),
            )
            .route(
                "/api/specifications/{anchor}/preview",
                post(api_specification_preview),
            )
            .route(
                "/api/specifications/{anchor}/apply",
                put(api_specification_apply),
            )
            .route("/api/config/preview", post(api_config_preview))
            .route("/api/config/apply", put(api_config_apply))
            .route("/api/config", get(api_config))
            .route("/api/scope/branch", get(api_branch_scope))
            .route("/api/scope/diff", get(api_scope_diff))
            .route("/api/source", get(api_source))
            .route("/api/work/action", post(api_journey_action))
            .route("/api/work/plan", post(api_plan))
            .route("/api/work/validate", post(api_validate))
            .route("/api/work/approve", post(api_approve))
            .route("/api/work/agent/start", post(api_agent_start))
            .route("/api/work/agent/patch", post(api_agent_patch))
            .route("/api/work/agent/verify", post(api_agent_verify))
            .route("/api/work/agent/blocker", post(api_agent_blocker))
            .route(
                "/api/work/agent/scope-expansion",
                post(api_agent_scope_expansion),
            )
            .route("/api/work/context", post(api_context))
            .route("/api/work/verify", post(api_verify))
            .route("/api/work/finalize/preview", post(api_finalize_preview))
            .route("/api/work/finalize/apply", post(api_finalize_apply))
            .route("/api/work/result", post(api_result))
            .route("/api/work/session", get(api_session).delete(api_discard))
            .layer(middleware::from_fn(mutation_guard))
            .layer(Extension(security))
            .with_state(self.service.clone())
    }
    pub fn run(self) -> Result<()> {
        let bind = self.launch.bind;
        let port = self.launch.port;
        let no_open = self.launch.no_open;
        if !bind.is_loopback()
            && self
                .launch
                .session_token
                .as_deref()
                .is_none_or(str::is_empty)
        {
            anyhow::bail!("remote --bind requires --session-token");
        }
        let app = self.router();
        tokio::runtime::Runtime::new()?.block_on(async move {
            let listener = tokio::net::TcpListener::bind((bind, port)).await?;
            println!("Mitase Workbench listening on http://{bind}:{port}");
            if !no_open {
                let url = format!("http://{bind}:{port}");
                if let Err(error) = open_browser(&url) {
                    eprintln!("warning: could not open Workbench browser: {error}");
                }
            }
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            Ok::<(), anyhow::Error>(())
        })
    }
}

impl WorkbenchService {
    fn snapshot(&self) -> Result<Arc<CachedWorkspaceSnapshot>> {
        // Projection construction may initialize a local delivery-store directory on
        // its first access. Retry once more than the usual before/after check so
        // that one-time repository-local setup cannot make the first projection
        // fail as a concurrent source edit.
        for _ in 0..3 {
            let signature = workspace_signature(&self.workspace_root)?;
            if let Some(cached) = self
                .snapshot
                .read()
                .map_err(|_| anyhow::anyhow!("workbench snapshot lock"))?
                .as_ref()
                .filter(|cached| cached.signature == signature)
            {
                return Ok(cached.clone());
            }

            let workspace = Arc::new(SpecWorkspace::load(&self.workspace_root)?);
            let revision = current_revision(&workspace.root)?;
            let index = Arc::new(workspace.index()?);
            let projection = Arc::new(project_with_index(&workspace, &index, None, &revision)?);
            if workspace_signature(&self.workspace_root)? != signature {
                continue;
            }
            let governance_signature = governance_signature(&workspace)?;
            let governance_changed = self
                .snapshot
                .read()
                .map_err(|_| anyhow::anyhow!("workbench snapshot lock"))?
                .as_ref()
                .is_some_and(|cached| cached.governance_signature != governance_signature);
            let cached = Arc::new(CachedWorkspaceSnapshot {
                signature,
                governance_signature,
                workspace,
                index,
                revision,
                projection,
            });
            *self
                .snapshot
                .write()
                .map_err(|_| anyhow::anyhow!("workbench snapshot lock"))? = Some(cached.clone());
            if governance_changed {
                let mut session = self
                    .session
                    .write()
                    .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
                clear_work_execution_state(&mut session);
            }
            return Ok(cached);
        }
        anyhow::bail!("workspace changed while loading the Workbench snapshot")
    }
}

fn governance_signature(workspace: &SpecWorkspace) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(b"mitase/workbench-governance/v1\0");
    hash.update(fs::read(workspace.root.join("mitase.yaml"))?);
    let mut paths = workspace
        .documents
        .iter()
        .map(|document| document.path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        hash.update(b"\0path\0");
        hash.update(path.to_string_lossy().as_bytes());
        hash.update(b"\0content\0");
        hash.update(fs::read(path)?);
    }
    Ok(format_sha256(hash.finalize()))
}

fn workspace_signature(root: &Path) -> Result<String> {
    let revision = current_revision(root)?;
    let tracked = git_output(root, &["diff", "--name-only", "-z", "HEAD", "--"])?;
    let untracked = git_output(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    let mut paths = BTreeSet::new();
    for raw in tracked
        .split(|byte| *byte == 0)
        .chain(untracked.split(|byte| *byte == 0))
    {
        if raw.is_empty() {
            continue;
        }
        let path = PathBuf::from(String::from_utf8(raw.to_vec())?);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            anyhow::bail!("git reported a workspace path outside the repository");
        }
        paths.insert(path);
    }
    // Git's `--exclude-standard` intentionally omits ignored files, but the
    // Workbench's governance boundary is the loaded config and specification
    // documents. Include their bytes explicitly so an ignored spec/config
    // edit cannot survive in a cached snapshot.
    let workspace = SpecWorkspace::load(root)?;
    let governed_fingerprint = workspace.try_fingerprint()?;
    let canonical_root = root.canonicalize()?;
    paths.insert(PathBuf::from("mitase.yaml"));
    for document in &workspace.documents {
        let document_path = document.path.canonicalize()?;
        let relative = document_path
            .strip_prefix(&canonical_root)
            .with_context(|| {
                format!(
                    "specification path {} is outside workspace",
                    document.path.display()
                )
            })?;
        paths.insert(relative.to_path_buf());
    }
    let mut hash = Sha256::new();
    hash.update(b"mitase/workbench-snapshot/v1\0");
    hash.update(revision.as_bytes());
    hash.update(b"\0governed-artifacts\0");
    hash.update(governed_fingerprint.as_bytes());
    for path in paths {
        hash.update(b"\0path\0");
        hash.update(path.to_string_lossy().as_bytes());
        let absolute = root.join(&path);
        match fs::read(&absolute) {
            Ok(content) => {
                hash.update(b"\0content\0");
                hash.update(content);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                hash.update(b"\0deleted\0");
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(format_sha256(hash.finalize()))
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start");
        command
    };
    command
        .arg(url)
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("browser command failed"))
}

fn security_token(root: &Path) -> String {
    let mut hash = Sha256::new();
    hash.update(root.to_string_lossy().as_bytes());
    hash.update(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    format_sha256(hash.finalize())
}

async fn mutation_guard(
    Extension(security): Extension<ServerSecurity>,
    request: Request,
    next: Next,
) -> Response {
    let mutating = matches!(
        request.method(),
        &Method::POST | &Method::PUT | &Method::DELETE
    );
    if !mutating {
        let mut response = next.run(request).await;
        if let Ok(token) = HeaderValue::from_str(&security.csrf_token) {
            response.headers_mut().insert("x-mitase-csrf-token", token);
        }
        return response;
    }
    let headers = request.headers();
    let csrf_valid = headers
        .get("x-mitase-csrf-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == security.csrf_token);
    let origin_valid = headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin == security.expected_origin);
    let session_valid = security.remote_session_token.as_ref().is_none_or(|token| {
        headers
            .get("x-mitase-session-token")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == token)
    });
    if !csrf_valid || !origin_valid || !session_valid {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"Workbench mutation was rejected by origin, CSRF, or session-token protection."})),
        )
            .into_response();
    }
    next.run(request).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationBasis {
    pub expected_revision: String,
    pub expected_workspace_fingerprint: String,
    pub expected_source_hash: String,
}
/// The browser and native WebView use one user-facing action boundary.  The
/// low-level planner and delivery APIs remain server implementation details.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum JourneyAction {
    Create {
        schema: String,
        origin: WorkOrigin,
        title: String,
    },
    SelectSlice {
        schema: String,
        candidate_plan_digest: String,
        slice_id: String,
    },
    Rename {
        title: String,
    },
    Prepare,
    Approve {
        execution: ExecutionIdentity,
    },
    Start {
        execution: ExecutionIdentity,
    },
    Retry {
        execution: ExecutionIdentity,
    },
    Verify {
        execution: ExecutionIdentity,
    },
    Finalize {
        execution: ExecutionIdentity,
        attempt_id: String,
        #[serde(default)]
        preview_token: Option<String>,
    },
    Restart,
    Cancel,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum JourneyActionCommand {
    Create {
        basis: MutationBasis,
        schema: String,
        origin: WorkOrigin,
        title: String,
    },
    SelectSlice {
        basis: MutationBasis,
        schema: String,
        candidate_plan_digest: String,
        slice_id: String,
    },
    Rename {
        basis: MutationBasis,
        title: String,
    },
    Prepare {
        basis: MutationBasis,
    },
    Approve {
        basis: MutationBasis,
        execution: ExecutionIdentity,
    },
    Start {
        basis: MutationBasis,
        execution: ExecutionIdentity,
    },
    Retry {
        basis: MutationBasis,
        execution: ExecutionIdentity,
    },
    Verify {
        basis: MutationBasis,
        execution: ExecutionIdentity,
    },
    Finalize {
        basis: MutationBasis,
        execution: ExecutionIdentity,
        attempt_id: String,
        preview_token: Option<String>,
    },
    Restart {
        basis: MutationBasis,
    },
    Cancel {
        basis: MutationBasis,
    },
}

impl JourneyActionCommand {
    fn into_parts(self) -> (MutationBasis, JourneyAction) {
        match self {
            Self::Create {
                basis,
                schema,
                origin,
                title,
            } => (
                basis,
                JourneyAction::Create {
                    schema,
                    origin,
                    title,
                },
            ),
            Self::SelectSlice {
                basis,
                schema,
                candidate_plan_digest,
                slice_id,
            } => (
                basis,
                JourneyAction::SelectSlice {
                    schema,
                    candidate_plan_digest,
                    slice_id,
                },
            ),
            Self::Rename { basis, title } => (basis, JourneyAction::Rename { title }),
            Self::Prepare { basis } => (basis, JourneyAction::Prepare),
            Self::Approve { basis, execution } => (basis, JourneyAction::Approve { execution }),
            Self::Start { basis, execution } => (basis, JourneyAction::Start { execution }),
            Self::Retry { basis, execution } => (basis, JourneyAction::Retry { execution }),
            Self::Verify { basis, execution } => (basis, JourneyAction::Verify { execution }),
            Self::Finalize {
                basis,
                execution,
                attempt_id,
                preview_token,
            } => (
                basis,
                JourneyAction::Finalize {
                    execution,
                    attempt_id,
                    preview_token,
                },
            ),
            Self::Restart { basis } => (basis, JourneyAction::Restart),
            Self::Cancel { basis } => (basis, JourneyAction::Cancel),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectSliceResponse {
    pub schema: String,
    pub action: String,
    pub candidate_plan_digest: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub projection: WorkspaceProjection,
}

fn journey_action_key(action: &JourneyAction) -> &'static str {
    match action {
        JourneyAction::Create { .. } => "create",
        JourneyAction::SelectSlice { .. } => "select_slice",
        JourneyAction::Rename { .. } => "rename",
        JourneyAction::Prepare => "prepare",
        JourneyAction::Approve { .. } => "approve",
        JourneyAction::Start { .. } => "start",
        JourneyAction::Retry { .. } => "retry",
        JourneyAction::Verify { .. } => "verify",
        JourneyAction::Finalize { .. } => "finalize",
        JourneyAction::Restart => "restart",
        JourneyAction::Cancel => "cancel",
    }
}

fn ensure_journey_transition(
    service: &WorkbenchService,
    basis_command: &MutationBasis,
    action: &JourneyAction,
) -> Result<(), ApiError> {
    let snapshot = match action {
        JourneyAction::Verify { .. } | JourneyAction::Finalize { .. } => {
            execution_basis(service, basis_command).map_err(ApiError::from)?
        }
        _ => basis(service, basis_command).map_err(ApiError::from)?,
    };
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let projection = project_session(&snapshot, &session).map_err(ApiError::from)?;
    let requested = journey_action_key(action);
    let expected = projection.journey.primary_action.action.as_str();
    let recovery = projection
        .journey
        .recovery_action
        .as_ref()
        .map(|action| action.action.as_str());
    if requested != expected && recovery != Some(requested) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "journey action {requested} is not available; the current next action is {expected}"
            ),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
    pub attempt_id: String,
    pub receipt: VerificationReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStartCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPatchCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
    pub run_id: String,
    pub patch: AgentPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBlockerCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
    pub run_id: String,
    pub blocker: AgentBlocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentScopeExpansionCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
    pub run_id: String,
    pub reason: String,
    pub requested_targets: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizeCommand {
    pub basis: MutationBasis,
    pub execution: ExecutionIdentity,
    pub attempt_id: String,
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredEditCommand {
    pub basis: MutationBasis,
    pub patch: EditPatch,
    #[serde(default)]
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditPatch {
    Specification {
        item_id: String,
        fields: SpecificationPatchFields,
    },
    Anchor {
        anchor: String,
        fields: AnchorPatchFields,
    },
    AddCriterion {
        requirement_id: SpecId,
        criterion: NewCriterion,
    },
    /// Typed nested edits are the only supported way to change bindings,
    /// ownership scopes, targets, claims, or contracts. The payload is
    /// intentionally schema-shaped; arbitrary YAML maps are not accepted.
    Nested { item_id: String, edit: NestedEdit },
    CreateRequirement {
        document: String,
        id: SpecId,
        title: String,
        description: String,
        priority: Priority,
        #[serde(default)]
        status: Option<ItemStatus>,
        criteria: Vec<NewCriterion>,
    },
    CreateFeature {
        document: String,
        id: SpecId,
        title: String,
        summary: String,
        #[serde(default)]
        status: Option<ItemStatus>,
        #[serde(default)]
        criterion_anchor: Option<String>,
        #[serde(default)]
        target: Option<FeatureTargetDraft>,
    },
    AddFeatureTarget {
        document: String,
        feature_id: SpecId,
        criterion_anchor: String,
        target: FeatureTargetDraft,
    },
    Config {
        config: Box<mitase_project_model::ProjectConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "entity", rename_all = "snake_case", deny_unknown_fields)]
pub enum NestedEdit {
    Binding {
        operation: NestedEditOperation,
        binding: ArtifactBinding,
        #[serde(default)]
        current_id: Option<String>,
    },
    Ownership {
        operation: NestedEditOperation,
        binding_id: LocalId,
        ownership: OwnershipScope,
        #[serde(default)]
        current_id: Option<String>,
    },
    Target {
        operation: NestedEditOperation,
        binding_id: LocalId,
        target: mitase_spec_model::ArtifactTarget,
        #[serde(default)]
        current_id: Option<String>,
    },
    Claim {
        operation: NestedEditOperation,
        binding_id: LocalId,
        target_id: LocalId,
        claim_index: usize,
        claim: TargetClaim,
    },
    Contract {
        operation: NestedEditOperation,
        contract: Contract,
        #[serde(default)]
        current_id: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NestedEditOperation {
    Upsert,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "anchor_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnchorPatchFields {
    Principle {
        statement: Option<String>,
        applies_to: Option<Vec<String>>,
    },
    Rule {
        statement: Option<String>,
        level: Option<RuleLevel>,
    },
    Criterion {
        statement: Option<String>,
        kind: Option<CriterionKind>,
        governed_by: Option<Vec<SpecAnchor>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCriterion {
    pub id: LocalId,
    pub kind: CriterionKind,
    pub statement: String,
    #[serde(default)]
    pub governed_by: Vec<SpecAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureTargetDraft {
    pub id: LocalId,
    pub adapter: String,
    pub path: String,
    pub selector: Selector,
}

/// Typed edit payloads keep item-kind/schema semantics on the server. The
/// browser sends one of these DTOs and never constructs arbitrary YAML maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpecificationPatchFields {
    Philosophy {
        title: Option<String>,
        summary: Option<String>,
    },
    Policy {
        title: Option<String>,
        summary: Option<String>,
    },
    Requirement {
        title: Option<String>,
        description: Option<String>,
        priority: Option<Priority>,
        status: Option<ItemStatus>,
    },
    Feature {
        title: Option<String>,
        summary: Option<String>,
        status: Option<ItemStatus>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPreview {
    pub path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub valid: bool,
    pub preview_token: String,
    pub candidate_digest: String,
    pub workspace_fingerprint: String,
    pub changed_lines: usize,
    #[serde(default)]
    pub impact: Option<SpecificationImpact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationImpact {
    pub affected_items: Vec<String>,
    pub changed_anchors: Vec<String>,
    pub affected_ownership: Vec<String>,
    pub implementation_targets: Vec<String>,
    pub verification_targets: Vec<String>,
    #[serde(default)]
    pub target_suggestions: Vec<TargetSuggestionSet>,
    pub readiness_before: ReadinessImpact,
    pub readiness_after: ReadinessImpact,
    pub work: WorkImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessImpact {
    pub status: String,
    pub blocking_subjects: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkImpact {
    pub seedable: bool,
    pub requires_replan: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSuggestionRejectCommand {
    pub basis: MutationBasis,
    pub suggestion_token: String,
    pub suggestion_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSuggestionApprovalCommand {
    pub basis: MutationBasis,
    pub suggestion_token: String,
    pub suggestion_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSuggestionApprovalView {
    pub approved_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_recommendation: Option<SplitWorkRecommendation>,
}

struct ApiError(StatusCode, anyhow::Error);
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let message = self.1.to_string();
        if let Some(payload) = message.strip_prefix("__MITASE_STRUCTURED__")
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(payload)
        {
            return (self.0, Json(value)).into_response();
        }
        (self.0, Json(serde_json::json!({"error": message}))).into_response()
    }
}
fn structured_api_error(status: StatusCode, value: serde_json::Value) -> ApiError {
    ApiError(
        status,
        anyhow::anyhow!(format!("__MITASE_STRUCTURED__{}", value)),
    )
}

fn work_action_error(
    status: StatusCode,
    action: &str,
    code: &str,
    message: impl Into<String>,
    candidate_plan_digest: Option<String>,
    nearest: Vec<serde_json::Value>,
) -> ApiError {
    structured_api_error(
        status,
        serde_json::json!({
            "schema": mitase_work_model::WORK_ERROR_SCHEMA,
            "kind": "work-action",
            "action": action,
            "code": code,
            "message": message.into(),
            "candidate_plan_digest": candidate_plan_digest,
            "nearest": nearest,
        }),
    )
}

fn origin_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    origin: Option<&WorkOrigin>,
    nearest: Vec<serde_json::Value>,
) -> ApiError {
    structured_api_error(
        status,
        serde_json::json!({
            "schema": mitase_work_model::WORK_ERROR_SCHEMA,
            "kind": "origin",
            "action": "create",
            "code": code,
            "message": message.into(),
            "origin": origin.and_then(|value| serde_json::to_value(value).ok()),
            "nearest": nearest,
        }),
    )
}
impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        let status = if value.to_string().contains("Workspace changed")
            || value.to_string().contains("source changed")
            || value.to_string().contains("stale")
        {
            StatusCode::CONFLICT
        } else {
            StatusCode::BAD_REQUEST
        };
        Self(status, value)
    }
}
fn current_revision(root: &std::path::Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("resolve workspace revision");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}
fn basis(
    service: &WorkbenchService,
    expected: &MutationBasis,
) -> Result<Arc<CachedWorkspaceSnapshot>> {
    if expected.expected_source_hash.is_empty() {
        anyhow::bail!("mutation requires expected_source_hash");
    }
    let snapshot = service.snapshot()?;
    if snapshot.revision != expected.expected_revision
        || snapshot.projection.snapshot.fingerprint != expected.expected_workspace_fingerprint
        || snapshot.projection.snapshot.source_hash != expected.expected_source_hash
    {
        anyhow::bail!(
            "Workspace changed since this view was loaded. Refresh the projection before applying the operation."
        );
    }
    Ok(snapshot)
}

fn ensure_no_active_agent_run(service: &WorkbenchService) -> Result<()> {
    let session_active = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .as_ref()
        .is_some_and(|run| matches!(run.status, AgentRunStatus::Active | AgentRunStatus::Blocked));
    let persisted_active = !DeliveryStore::for_workspace(&service.workspace_root)?
        .unresolved_agent_runs()?
        .is_empty();
    if session_active || persisted_active {
        anyhow::bail!(
            "specification and config edits are unavailable while an agent run is active"
        );
    }
    Ok(())
}

fn clear_work_execution_state(session: &mut WorkbenchSession) {
    session.work_title = None;
    session.draft_request = None;
    session.plan = None;
    session.selected_slice = None;
    session.context_pack = None;
    session.verification_receipt = None;
    // Keep a durable active/blocked run visible after a governance edit or a
    // process restart. The explicit Cancel action records abandonment before
    // clearing it; silently dropping this handle would strand the run.
    session.last_validation = None;
    session.rejected_target_suggestions.clear();
    session.approved_target_suggestions.clear();
}

fn abandon_active_agent_runs(service: &WorkbenchService, reason: &str) -> Result<()> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let runs = store.unresolved_agent_runs()?;
    for run in runs {
        store.abandon_agent_run_while_locked(&run, reason.to_owned())?;
    }
    Ok(())
}

/// Execution is intentionally based on the plan's immutable revision rather
/// than the pre-edit workspace fingerprint. Editable source changes are the
/// expected transition between validation and verification/result; the
/// canonical plan and readonly basis checks in the validation crate still
/// reject specification, ownership, and guarded-target changes.
fn execution_basis(
    service: &WorkbenchService,
    expected: &MutationBasis,
) -> Result<Arc<CachedWorkspaceSnapshot>> {
    let snapshot = service.snapshot()?;
    if expected.expected_revision.is_empty() || snapshot.revision != expected.expected_revision {
        anyhow::bail!(
            "Workspace revision changed since this work plan was created. Refresh the projection and replan."
        );
    }
    Ok(snapshot)
}

fn workspace_source_hash(workspace: &SpecWorkspace) -> String {
    let mut source = String::new();
    for document in &workspace.documents {
        source.push_str(&document.path.to_string_lossy());
        source.push_str(&workspace.read_to_string(&document.path).unwrap_or_default());
    }
    source.push_str(&serde_yaml::to_string(&workspace.config).unwrap_or_default());
    content_hash(&source)
}

#[derive(Serialize)]
struct BrowserSessionView {
    ready: bool,
}

async fn api_session() -> Json<BrowserSessionView> {
    Json(BrowserSessionView { ready: true })
}

async fn api_projection(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<WorkspaceProjection>, ApiError> {
    let snapshot = service.snapshot()?;
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    Ok(Json(project_session(&snapshot, &session)?))
}

async fn api_index() -> Html<&'static str> {
    Html(include_str!("../../mitase-app-ui/assets/workbench.html"))
}

async fn api_asset(AxumPath(asset): AxumPath<String>) -> Response {
    let (content_type, content): (&str, String) = match asset.as_str() {
        "workbench.css" => (
            "text/css; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/workbench.css").into(),
        ),
        "i18n.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/i18n.js").into(),
        ),
        "catalog.js" => (
            "text/javascript; charset=utf-8",
            format!(
                "window.MITASE_I18N={{en:{},ja:{}}};",
                include_str!("../../mitase-app-ui/assets/locales/en.json"),
                include_str!("../../mitase-app-ui/assets/locales/ja.json")
            ),
        ),
        "js/main.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/main.js").into(),
        ),
        "js/api.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/api.js").into(),
        ),
        "js/state.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/state.js").into(),
        ),
        "js/router.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/router.js").into(),
        ),
        "js/i18n.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/i18n.js").into(),
        ),
        "js/components/action.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/action.js").into(),
        ),
        "js/components/diagnostic.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/diagnostic.js").into(),
        ),
        "js/components/diff.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/diff.js").into(),
        ),
        "js/components/editor.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/editor.js").into(),
        ),
        "js/components/readiness.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/readiness.js").into(),
        ),
        "js/components/target.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/components/target.js").into(),
        ),
        "js/pages/work.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/work.js").into(),
        ),
        "js/pages/readiness.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/readiness.js").into(),
        ),
        "js/pages/scope.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/scope.js").into(),
        ),
        "js/pages/specifications.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/specifications.js").into(),
        ),
        "js/pages/diagnostics.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/diagnostics.js").into(),
        ),
        "js/pages/settings.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../mitase-app-ui/assets/js/pages/settings.js").into(),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([("content-type", content_type)], content).into_response()
}

async fn api_readiness(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<ReadinessView>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let view = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .readiness
        .clone()
        .unwrap_or_else(|| readiness_not_run(&workspace.config));
    Ok(Json(view))
}

async fn api_readiness_run(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<ReadinessView>, ApiError> {
    let snapshot = service.snapshot()?;
    let report = mitase_validation::evaluate_readiness(
        &snapshot.workspace,
        &snapshot.index,
        &snapshot.revision,
        true,
    )?;
    let view = readiness_view(&report);
    service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .readiness = Some(view.clone());
    Ok(Json(view))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticsRunCommand {
    basis: MutationBasis,
    context: String,
    #[serde(default)]
    range: Option<String>,
}

async fn api_diagnostics_run(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<DiagnosticsRunCommand>,
) -> Result<Json<ValidationRunView>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let started = SystemTime::now();
    let context = command.context.as_str();
    let view = match context {
        "workspace" => {
            let result = mitase_validation::validate_without_readiness(&ValidationContext {
                config: &snapshot.workspace.config,
                workspace: &snapshot.workspace,
                index: &snapshot.index,
                changed_files: None,
                reported_changed_files: None,
                work_plan: None,
                selected_slice: None,
                plan_mode: PlanValidationMode::PreState,
                preset: snapshot.workspace.config.validation.preset,
                revision: Some(&snapshot.revision),
                change_base_revision: None,
            });
            ValidationRunView::completed(
                "workspace",
                Some(snapshot.revision.clone()),
                result,
                false,
                false,
                snapshot.workspace.config.validation.preset,
                started,
            )
        }
        "git_range" => {
            let range = command
                .range
                .filter(|range| !range.trim().is_empty())
                .unwrap_or(configured_change_range(&snapshot.workspace)?);
            let changed_files = branch_changed_files(&snapshot.workspace.root, &range)?;
            let result = mitase_validation::validate_without_readiness(&ValidationContext {
                config: &snapshot.workspace.config,
                workspace: &snapshot.workspace,
                index: &snapshot.index,
                changed_files: Some(&changed_files),
                reported_changed_files: None,
                work_plan: None,
                selected_slice: None,
                plan_mode: PlanValidationMode::PreState,
                preset: snapshot.workspace.config.validation.preset,
                revision: Some(&snapshot.revision),
                change_base_revision: None,
            });
            ValidationRunView::completed(
                "git_range",
                Some(range),
                result,
                true,
                false,
                snapshot.workspace.config.validation.preset,
                started,
            )
        }
        "work-plan" | "work_plan" | "slice" => {
            let (plan, selected_slice_id) = {
                let session = service
                    .session
                    .read()
                    .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
                (
                    session.plan.clone().ok_or_else(|| {
                        anyhow::anyhow!("prepare a work plan before diagnosing it")
                    })?,
                    session.selected_slice.clone(),
                )
            };
            let canonical_plan = mitase_validation::canonical_plan_for_execution(
                &snapshot.workspace,
                &snapshot.index,
                &plan,
                &snapshot.revision,
            )
            .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
            let selected_slice = if context == "slice" {
                let selected_slice_id = selected_slice_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("select a work slice before diagnosing it"))?;
                Some(
                    canonical_plan
                        .slices
                        .iter()
                        .find(|slice| slice.id == selected_slice_id)
                        .ok_or_else(|| {
                            anyhow::anyhow!("the selected slice is not in the work plan")
                        })?,
                )
            } else {
                None
            };
            let result = mitase_validation::validate_without_readiness(&ValidationContext {
                config: &snapshot.workspace.config,
                workspace: &snapshot.workspace,
                index: &snapshot.index,
                changed_files: None,
                reported_changed_files: None,
                work_plan: Some(&canonical_plan),
                selected_slice,
                plan_mode: PlanValidationMode::PreState,
                preset: snapshot.workspace.config.validation.preset,
                revision: Some(&snapshot.revision),
                change_base_revision: None,
            });
            ValidationRunView::completed(
                context,
                Some(canonical_plan.canonical_digest.clone()),
                result,
                false,
                true,
                snapshot.workspace.config.validation.preset,
                started,
            )
        }
        _ => {
            return Err(ApiError(
                StatusCode::UNPROCESSABLE_ENTITY,
                anyhow::anyhow!("unknown diagnostics context"),
            ));
        }
    };
    service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .last_validation = Some(view.clone());
    Ok(Json(view))
}

async fn api_specifications(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<SpecificationCatalogView>, ApiError> {
    let snapshot = service.snapshot()?;
    Ok(Json(snapshot.projection.specifications.clone()))
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpecificationCandidateQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationCandidateView {
    pub item: ItemSummary,
    pub matches: Vec<CandidateMatch>,
    pub relevance: Vec<String>,
    /// Ranking is advisory.  It is intentionally separated from the stable
    /// criterion anchors that a person must explicitly select before work can
    /// be created.
    pub score: usize,
    pub evidence: Vec<CandidateEvidence>,
    pub stable_anchors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMatch {
    pub anchor: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateEvidence {
    pub source: String,
    pub detail: String,
}

#[derive(Clone, Copy)]
struct DiscoveryConcept {
    label: &'static str,
    terms: &'static [&'static str],
}

// This intentionally small, deterministic glossary is a discovery aid, not
// an authority on scope.  It keeps multilingual matching inspectable and
// testable without turning an embedding or an LLM result into executable work.
const DISCOVERY_CONCEPTS: &[DiscoveryConcept] = &[
    DiscoveryConcept {
        label: "behavior",
        terms: &[
            "behavior",
            "behaviour",
            "function",
            "functional",
            "動作",
            "振る舞い",
            "挙動",
            "機能",
        ],
    },
    DiscoveryConcept {
        label: "validation",
        terms: &[
            "validate",
            "validation",
            "verify",
            "verification",
            "test",
            "valid",
            "検証",
            "確認",
            "テスト",
            "有効",
        ],
    },
    DiscoveryConcept {
        label: "change",
        terms: &[
            "change",
            "modify",
            "update",
            "implement",
            "implementation",
            "変更",
            "改修",
            "更新",
            "実装",
        ],
    },
    DiscoveryConcept {
        label: "scope",
        terms: &[
            "scope", "boundary", "bounded", "plan", "planning", "範囲", "境界", "計画",
        ],
    },
    DiscoveryConcept {
        label: "security",
        terms: &[
            "security",
            "secure",
            "auth",
            "permission",
            "安全",
            "認証",
            "権限",
        ],
    },
];

fn matching_discovery_concepts(query: &str) -> Vec<&'static DiscoveryConcept> {
    DISCOVERY_CONCEPTS
        .iter()
        .filter(|concept| {
            concept
                .terms
                .iter()
                .any(|term| discovery_term_matches(query, term))
        })
        .collect()
}

fn ascii_tokens(value: &str) -> Vec<&str> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect()
}

/// English discovery terms are whole words. Japanese terms remain explicit
/// glossary entries and use controlled substring matching because the
/// Workbench does not promise a language-specific tokenizer.
fn discovery_term_matches(text: &str, term: &str) -> bool {
    if term.is_ascii() {
        let tokens = ascii_tokens(text);
        return tokens.iter().any(|token| token.eq_ignore_ascii_case(term));
    }
    text.contains(term)
}

fn discovery_query_matches(text: &str, query: &str) -> bool {
    if !query.is_ascii() {
        return text.to_lowercase().contains(&query.to_lowercase());
    }
    text.to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
}

fn discovery_query_exact_matches(text: &str, query: &str) -> bool {
    if !query.is_ascii() {
        return text.to_lowercase() == query.to_lowercase();
    }
    let text_tokens = ascii_tokens(text);
    let query_tokens = ascii_tokens(query);
    if query_tokens.is_empty() {
        return false;
    }
    text_tokens.windows(query_tokens.len()).any(|window| {
        window
            .iter()
            .zip(query_tokens.iter())
            .all(|(text, query)| text.eq_ignore_ascii_case(query))
    })
}

fn candidate_field_weight(kind: &str) -> usize {
    match kind {
        "item" => 7,
        "title" => 6,
        "criterion" => 5,
        "summary" | "description" => 4,
        "rule" | "principle" => 3,
        _ => 1,
    }
}

fn discovery_history(workspace: &SpecWorkspace) -> BTreeMap<String, usize> {
    DeliveryStore::for_workspace(&workspace.root)
        .and_then(|store| store.attempts())
        .map(|attempts| {
            let mut history = BTreeMap::new();
            for attempt in attempts {
                for evidence in attempt.report.demonstrated {
                    *history.entry(evidence.anchor.to_string()).or_insert(0) += 1;
                }
            }
            history
        })
        .unwrap_or_default()
}
#[derive(Debug, Clone, Deserialize)]
pub struct SpecificationTraceQuery {
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub node_budget: Option<usize>,
    #[serde(default)]
    pub edge_budget: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationTraceView {
    pub root_item_id: String,
    pub revision: String,
    pub workspace_fingerprint: String,
    pub source_hash: String,
    pub mode: String,
    pub nodes: Vec<TraceNodeView>,
    pub edges: Vec<TraceEdgeView>,
    /// Server-owned related entries keep the browser from re-joining claims,
    /// items, and targets from a partial projection.
    pub related: TraceRelatedView,
    pub closures: Vec<CriterionClosureView>,
    pub hidden_related_count: usize,
    pub hidden_related_claim_count: usize,
    pub hidden_closure_count: usize,
    pub hidden_closure_target_count: usize,
    pub hidden_reason_count: usize,
    pub hidden_readiness_count: usize,
    pub hidden_diagnostic_count: usize,
    pub truncated: bool,
    pub hidden_node_count: usize,
    pub hidden_edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceRelatedView {
    pub specification: Vec<TraceRelatedSpecificationView>,
    pub implementation: Vec<TraceRelatedTargetView>,
    pub verification: Vec<TraceRelatedTargetView>,
    pub hidden_count: usize,
    pub hidden_claim_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelatedSpecificationView {
    pub item_id: String,
    pub kind: String,
    pub title: String,
    pub presentation_title_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelatedTargetView {
    pub item_id: String,
    pub target: BindingTargetSummary,
    pub hidden_claim_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceNodeView {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub secondary_label: Option<String>,
    pub lane: String,
    pub stable_order: usize,
    pub source_target: Option<String>,
    pub item_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEdgeView {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
    pub display_label: String,
    pub exact_claim: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionClosureView {
    pub criterion: String,
    pub implementation_targets: Vec<String>,
    pub verification_targets: Vec<String>,
    pub state: String,
    pub reasons: Vec<String>,
    /// Definition-time evidence is deliberately separate from runtime facts.
    /// Runtime receipts are populated only by an explicit verification run.
    pub runtime_status: String,
    pub runtime_timestamp: Option<String>,
    pub runtime_revision: Option<String>,
    pub runtime_receipt: Option<String>,
    /// Exact receipt-local executions are kept separately from the aggregate
    /// status so a partial run cannot look like a complete verification.
    pub runtime_executions: Vec<VerificationExecutionView>,
    pub readiness_blockers: Vec<String>,
    pub diagnostics: Vec<TraceDiagnosticView>,
    pub hidden_target_count: usize,
    pub hidden_reason_count: usize,
    pub hidden_readiness_count: usize,
    pub hidden_diagnostic_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationExecutionView {
    /// Stable receipt-local identity; this is deliberately distinct from the
    /// target so two executions of the same target cannot be conflated.
    pub identity: String,
    pub target: String,
    /// The exact target/criterion pair selected for this execution. The
    /// nested exact reference prevents two claims on the same target from
    /// being conflated in the projection while retaining a criterion field
    /// convenient for closure matching.
    pub claim: Option<VerificationClaimView>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationClaimView {
    pub target: String,
    pub criterion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDiagnosticView {
    pub identity: String,
    pub severity: String,
    pub message: String,
    pub reason: Option<String>,
}

async fn api_specification_candidates(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<SpecificationCandidateQuery>,
) -> Result<Json<Vec<SpecificationCandidateView>>, ApiError> {
    let snapshot = service.snapshot()?;
    let entries = snapshot.projection.specifications.specifications.clone();
    let query_text = query.q.unwrap_or_default().trim().to_lowercase();
    let query_concepts = matching_discovery_concepts(&query_text);
    let history = discovery_history(&snapshot.workspace);
    let kind_filter = query.kind.as_deref().filter(|kind| !kind.is_empty());
    let mut candidates = entries
        .into_iter()
        .filter_map(|item| {
            if kind_filter.is_some_and(|kind| kind != item.kind) {
                return None;
            }
            let mut fields = vec![
                (item.id.clone(), "item".to_string(), item.id.clone()),
                (item.id.clone(), "title".to_string(), item.title.clone()),
                (item.id.clone(), "summary".to_string(), item.summary.clone()),
            ];
            if let Some(description) = &item.description {
                fields.push((item.id.clone(), "description".into(), description.clone()));
            }
            for principle in &item.principles {
                fields.push((
                    principle.anchor.clone(),
                    "principle".into(),
                    principle.statement.clone(),
                ));
            }
            for rule in &item.rules {
                fields.push((rule.anchor.clone(), "rule".into(), rule.statement.clone()));
            }
            for criterion in &item.criteria {
                fields.push((
                    criterion.anchor.clone(),
                    "criterion".into(),
                    criterion.statement.clone(),
                ));
            }
            let mut matches = Vec::new();
            let mut evidence = Vec::new();
            let mut score = 0;
            if query_text.is_empty() {
                matches.push(CandidateMatch {
                    anchor: item.id.clone(),
                    kind: "item".into(),
                    text: item.title.clone(),
                });
                evidence.push(CandidateEvidence {
                    source: "available".into(),
                    detail: "available specification".into(),
                });
            } else {
                for (anchor, kind, text) in &fields {
                    if discovery_query_matches(text, &query_text) {
                        let exact = discovery_query_exact_matches(text, &query_text);
                        score += if exact {
                            candidate_field_weight(kind)
                        } else {
                            (candidate_field_weight(kind) / 2).max(1)
                        };
                        matches.push(CandidateMatch {
                            anchor: anchor.clone(),
                            kind: kind.clone(),
                            text: text.clone(),
                        });
                        evidence.push(CandidateEvidence {
                            source: "lexical".into(),
                            detail: if exact {
                                format!("lexical {kind} match")
                            } else {
                                format!("lexical substring {kind} match")
                            },
                        });
                    }
                }
                for concept in &query_concepts {
                    let mut concept_matches = 0;
                    for (anchor, _kind, text) in &fields {
                        if concept
                            .terms
                            .iter()
                            .any(|term| discovery_term_matches(text, term))
                        {
                            concept_matches += 1;
                            if !matches.iter().any(|entry| {
                                entry.anchor == *anchor
                                    && entry.kind == "semantic"
                                    && entry.text == *text
                            }) {
                                matches.push(CandidateMatch {
                                    anchor: anchor.clone(),
                                    kind: "semantic".into(),
                                    text: text.clone(),
                                });
                            }
                        }
                    }
                    if concept_matches > 0 {
                        score += 3;
                        evidence.push(CandidateEvidence {
                            source: "semantic".into(),
                            detail: format!(
                                "semantic concept '{}' links the intent to {concept_matches} specification field(s)",
                                concept.label
                            ),
                        });
                    }
                }
            }
            if matches.is_empty() {
                return None;
            }
            let stable_anchors = item
                .criteria
                .iter()
                .map(|criterion| criterion.anchor.clone())
                .collect::<Vec<_>>();
            let (implementation_targets, verification_targets) = stable_anchors.iter().fold(
                (0, 0),
                |(implementation, verification), anchor| {
                    let anchor = anchor.parse::<SpecAnchor>().ok();
                    let implementation = implementation
                        + anchor
                            .as_ref()
                            .and_then(|anchor| {
                                snapshot
                                    .index
                                    .criteria_to_implementation_targets
                                    .get(anchor)
                            })
                            .map_or(0, Vec::len);
                    let verification = verification
                        + anchor
                            .as_ref()
                            .and_then(|anchor| {
                                snapshot.index.criteria_to_verification_targets.get(anchor)
                            })
                            .map_or(0, Vec::len);
                    (implementation, verification)
                },
            );
            evidence.push(CandidateEvidence {
                source: "graph".into(),
                detail: format!(
                    "{} exact criterion anchor(s), {implementation_targets} implementation target(s), and {verification_targets} verification target(s)",
                    stable_anchors.len(),
                ),
            });
            let completed = stable_anchors
                .iter()
                .map(|anchor| history.get(anchor).copied().unwrap_or_default())
                .sum::<usize>();
            evidence.push(CandidateEvidence {
                source: "history".into(),
                detail: if completed == 0 {
                    "no completed evidence recorded for these criterion anchors".into()
                } else {
                    format!("{completed} completed evidence record(s) reference these criterion anchors")
                },
            });
            let relevance = evidence.iter().map(|entry| entry.detail.clone()).collect();
            Some((
                score,
                SpecificationCandidateView {
                    item,
                    matches,
                    relevance,
                    score,
                    evidence,
                    stable_anchors,
                },
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.item.id.cmp(&right.1.item.id))
    });
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    Ok(Json(
        candidates
            .into_iter()
            .take(limit)
            .map(|(_, candidate)| candidate)
            .collect(),
    ))
}
async fn api_specification(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
) -> Result<Json<ItemSummary>, ApiError> {
    let snapshot = service.snapshot()?;
    let item = snapshot
        .projection
        .specifications
        .specifications
        .iter()
        .find(|item| item.id == anchor)
        .cloned()
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("specification {anchor} not found"),
            )
        })?;
    Ok(Json(item))
}

async fn api_specification_trace(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(item_id): AxumPath<String>,
    Query(query): Query<SpecificationTraceQuery>,
) -> Result<Json<SpecificationTraceView>, ApiError> {
    let snapshot = service.snapshot()?;
    let item = snapshot
        .projection
        .specifications
        .specifications
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("specification {item_id} not found"),
            )
        })?;
    Ok(Json(specification_trace_view(
        &snapshot.projection,
        &snapshot.index,
        item,
        &query,
    )))
}

fn specification_trace_view(
    projection: &WorkspaceProjection,
    index: &SpecIndex,
    root: &ItemSummary,
    query: &SpecificationTraceQuery,
) -> SpecificationTraceView {
    let mode = query
        .mode
        .as_deref()
        .filter(|value| matches!(*value, "readable" | "exact"))
        .unwrap_or("readable")
        .to_string();
    let depth = query.depth.unwrap_or(1).clamp(1, 8);
    let node_budget = query.node_budget.unwrap_or(80).clamp(8, 500);
    let edge_budget = query.edge_budget.unwrap_or(160).clamp(8, 1000);
    let items = &projection.specifications.specifications;
    let mut nodes = BTreeMap::<String, TraceNodeView>::new();
    let mut edges = BTreeMap::<String, TraceEdgeView>::new();

    // Build the canonical graph from every document. The browser receives
    // this bounded neighbourhood and never infers cross-item joins itself.
    for item in items {
        trace_node(
            &mut nodes,
            item.id.clone(),
            TraceNodeSpec {
                kind: "item".into(),
                label: item.title.clone(),
                secondary_label: Some(item.kind.clone()),
                lane: "specification".into(),
                source_target: None,
                item_id: Some(item.id.clone()),
                metadata: BTreeMap::new(),
            },
        );
        for principle in &item.principles {
            add_anchor_trace_node(&mut nodes, items, principle.anchor.clone());
            add_trace_edge(
                &mut edges,
                &item.id,
                &principle.anchor,
                "contains",
                "contains",
                None,
            );
        }
        for rule in &item.rules {
            add_anchor_trace_node(&mut nodes, items, rule.anchor.clone());
            add_trace_edge(
                &mut edges,
                &item.id,
                &rule.anchor,
                "contains",
                "contains",
                None,
            );
            for governed_by in &rule.governed_by {
                let governed_by = governed_by.to_string();
                add_anchor_trace_node(&mut nodes, items, governed_by.clone());
                add_trace_edge(
                    &mut edges,
                    &rule.anchor,
                    &governed_by,
                    "governed_by",
                    "governed by",
                    None,
                );
            }
        }
        for criterion in &item.criteria {
            add_anchor_trace_node(&mut nodes, items, criterion.anchor.clone());
            add_trace_edge(
                &mut edges,
                &item.id,
                &criterion.anchor,
                "contains",
                "contains",
                None,
            );
            for governed_by in &criterion.governed_by {
                let governed_by = governed_by.to_string();
                add_anchor_trace_node(&mut nodes, items, governed_by.clone());
                add_trace_edge(
                    &mut edges,
                    &criterion.anchor,
                    &governed_by,
                    "governed_by",
                    "governed by",
                    None,
                );
            }
            let criterion_anchor = criterion.anchor.parse::<SpecAnchor>().ok();
            for (targets, relation, label) in [
                (
                    criterion_anchor.as_ref().and_then(|anchor| {
                        index.all_criteria_to_implementation_targets.get(anchor)
                    }),
                    "satisfies",
                    "satisfies",
                ),
                (
                    criterion_anchor
                        .as_ref()
                        .and_then(|anchor| index.all_criteria_to_verification_targets.get(anchor)),
                    "verifies",
                    "verifies",
                ),
            ] {
                for target in targets.into_iter().flatten() {
                    let target = target.to_string();
                    add_target_or_anchor_trace_node(&mut nodes, items, index, &target);
                    add_trace_edge(
                        &mut edges,
                        &criterion.anchor,
                        &target,
                        relation,
                        label,
                        None,
                    );
                }
            }
        }
        for binding in &item.bindings {
            add_anchor_trace_node(&mut nodes, items, binding.anchor.clone());
            add_trace_edge(
                &mut edges,
                &item.id,
                &binding.anchor,
                "contains",
                "contains",
                None,
            );
            for ownership in &binding.owns {
                let ownership_id = format!("{}/owns.{}", binding.anchor, ownership.id);
                let metadata = BTreeMap::from([
                    ("adapter".into(), ownership.adapter.clone()),
                    ("path".into(), ownership.path.to_string_lossy().into_owned()),
                    (
                        "selector".into(),
                        serde_json::to_string(&ownership.selector).unwrap_or_default(),
                    ),
                ]);
                trace_node(
                    &mut nodes,
                    ownership_id.clone(),
                    TraceNodeSpec {
                        kind: "ownership".into(),
                        label: ownership.id.to_string(),
                        secondary_label: Some("owned scope".into()),
                        lane: "implementation".into(),
                        source_target: None,
                        item_id: Some(item.id.clone()),
                        metadata,
                    },
                );
                add_trace_edge(
                    &mut edges,
                    &binding.anchor,
                    &ownership_id,
                    "owns",
                    "owns",
                    None,
                );
            }
            for target in &binding.targets {
                add_target_trace_node(&mut nodes, item, binding, target);
                add_trace_edge(
                    &mut edges,
                    &item.id,
                    &target.reference,
                    "contains",
                    "contains",
                    None,
                );
                add_trace_edge(
                    &mut edges,
                    &binding.anchor,
                    &target.reference,
                    "owns",
                    "owns",
                    None,
                );
                for (claim_index, claim) in target.claims.iter().enumerate() {
                    add_claim_trace(
                        &mut nodes,
                        &mut edges,
                        ClaimTraceSpec {
                            items,
                            index,
                            mode: &mode,
                            target_reference: &target.reference,
                            claim_index,
                            claim,
                        },
                    );
                }
            }
        }
        for contract in &item.contracts {
            add_anchor_trace_node(&mut nodes, items, contract.anchor.clone());
            add_trace_edge(
                &mut edges,
                &item.id,
                &contract.anchor,
                "contains",
                "contains",
                None,
            );
            let source = contract.source.to_string();
            add_target_or_anchor_trace_node(&mut nodes, items, index, &source);
            add_trace_edge(
                &mut edges,
                &contract.anchor,
                &source,
                "owns",
                "source",
                None,
            );
            for participant in &contract.participants {
                let participant = participant.binding.clone();
                add_target_or_anchor_trace_node(&mut nodes, items, index, &participant);
                add_trace_edge(
                    &mut edges,
                    &contract.anchor,
                    &participant,
                    "participates",
                    "participant",
                    None,
                );
            }
        }
    }

    let (
        closures,
        hidden_closure_count,
        hidden_closure_target_count,
        hidden_reason_count,
        hidden_readiness_count,
        hidden_diagnostic_count,
    ) = specification_closures(projection, items, index, root, node_budget);
    let (related, hidden_related_count, hidden_related_claim_count) =
        specification_related(items, index, root, node_budget);
    let distances = trace_distances(&root.id, &edges, depth);
    let reachable = distances.keys().cloned().collect::<BTreeSet<_>>();
    let evidence_priority = closures
        .iter()
        .flat_map(|closure| {
            std::iter::once(closure.criterion.clone())
                .chain(closure.implementation_targets.iter().cloned())
                .chain(closure.verification_targets.iter().cloned())
        })
        .collect::<BTreeSet<_>>();
    let mut semantic_ordered = nodes
        .values()
        .filter(|node| reachable.contains(&node.id) && node.kind != "claim")
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    semantic_ordered.sort_by(|left, right| {
        (if left == &root.id {
            0
        } else if evidence_priority.contains(left) {
            1
        } else {
            2
        })
        .cmp(
            &(if right == &root.id {
                0
            } else if evidence_priority.contains(right) {
                1
            } else {
                2
            }),
        )
        .then_with(|| distances[left].cmp(&distances[right]))
        .then_with(|| {
            trace_lane_rank(nodes[left].lane.as_str())
                .cmp(&trace_lane_rank(nodes[right].lane.as_str()))
        })
        .then_with(|| nodes[left].kind.cmp(&nodes[right].kind))
        .then_with(|| left.cmp(right))
    });
    let semantic_candidate_count = semantic_ordered.len();
    let mut visible = semantic_ordered
        .into_iter()
        .take(node_budget)
        .collect::<BTreeSet<_>>();
    if !visible.contains(&root.id) {
        if visible.len() >= node_budget
            && let Some(evicted) = visible.iter().find(|id| id.as_str() != root.id).cloned()
        {
            visible.remove(&evicted);
        }
        visible.insert(root.id.clone());
    }
    let semantic_visible = visible.clone();
    let mut claim_ordered = nodes
        .values()
        .filter(|node| node.kind == "claim" && reachable.contains(&node.id))
        .filter(|node| {
            visible.contains(
                &edges
                    .values()
                    .find_map(|edge| {
                        (edge.to == node.id && edge.relation == "claim")
                            .then_some(edge.from.clone())
                    })
                    .unwrap_or_default(),
            )
        })
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    claim_ordered.sort();
    if mode == "exact" {
        for id in claim_ordered
            .iter()
            .take(node_budget.saturating_sub(visible.len()))
        {
            visible.insert(id.clone());
        }
    }
    let hidden_node_count = semantic_candidate_count.saturating_sub(
        visible
            .iter()
            .filter(|id| nodes[*id].kind != "claim")
            .count(),
    ) + if mode == "exact" {
        claim_ordered.len().saturating_sub(
            visible
                .iter()
                .filter(|id| nodes[*id].kind == "claim")
                .count(),
        )
    } else {
        0
    };
    let node_kinds = nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.kind.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut visible_nodes = visible
        .iter()
        .filter_map(|id| nodes.remove(id))
        .collect::<Vec<_>>();
    visible_nodes.sort_by(|left, right| {
        trace_lane_rank(left.lane.as_str())
            .cmp(&trace_lane_rank(right.lane.as_str()))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    for (stable_order, node) in visible_nodes.iter_mut().enumerate() {
        node.stable_order = stable_order;
    }
    let canonical_edges = edges
        .values()
        .filter(|edge| {
            node_kinds
                .get(&edge.from)
                .is_none_or(|kind| kind != "claim")
                && node_kinds.get(&edge.to).is_none_or(|kind| kind != "claim")
        })
        .filter(|edge| reachable.contains(&edge.from) && reachable.contains(&edge.to))
        .filter(|edge| semantic_visible.contains(&edge.from) && semantic_visible.contains(&edge.to))
        .cloned()
        .collect::<Vec<_>>();
    let claim_edges = edges
        .values()
        .filter(|edge| {
            node_kinds
                .get(&edge.from)
                .is_some_and(|kind| kind == "claim")
                || node_kinds.get(&edge.to).is_some_and(|kind| kind == "claim")
        })
        .filter(|edge| reachable.contains(&edge.from) && reachable.contains(&edge.to))
        .filter(|edge| visible.contains(&edge.from) && visible.contains(&edge.to))
        .cloned()
        .collect::<Vec<_>>();
    let reachable_edge_count = edges
        .values()
        .filter(|edge| reachable.contains(&edge.from) && reachable.contains(&edge.to))
        .filter(|edge| {
            mode == "exact"
                || (node_kinds
                    .get(&edge.from)
                    .is_none_or(|kind| kind != "claim")
                    && node_kinds.get(&edge.to).is_none_or(|kind| kind != "claim"))
        })
        .count();
    let mut visible_edges = canonical_edges;
    visible_edges.sort_by(|left, right| left.id.cmp(&right.id));
    visible_edges.truncate(edge_budget);
    if mode == "exact" && visible_edges.len() < edge_budget {
        let remaining = edge_budget - visible_edges.len();
        let mut presentation_edges = claim_edges;
        presentation_edges.sort_by(|left, right| left.id.cmp(&right.id));
        visible_edges.extend(presentation_edges.into_iter().take(remaining));
    }
    visible_edges.sort_by(|left, right| left.id.cmp(&right.id));
    let hidden_edge_count =
        reachable_edge_count.saturating_sub(edge_budget.min(visible_edges.len()));
    SpecificationTraceView {
        root_item_id: root.id.clone(),
        revision: projection.snapshot.revision.clone(),
        workspace_fingerprint: projection.snapshot.fingerprint.clone(),
        source_hash: root.source_hash.clone(),
        mode,
        nodes: visible_nodes,
        edges: visible_edges,
        related,
        closures,
        hidden_related_count,
        hidden_related_claim_count,
        hidden_closure_count,
        hidden_closure_target_count,
        hidden_reason_count,
        hidden_readiness_count,
        hidden_diagnostic_count,
        truncated: hidden_node_count > 0
            || hidden_edge_count > 0
            || hidden_related_count > 0
            || hidden_related_claim_count > 0
            || hidden_closure_count > 0
            || hidden_closure_target_count > 0
            || hidden_reason_count > 0
            || hidden_readiness_count > 0
            || hidden_diagnostic_count > 0,
        hidden_node_count,
        hidden_edge_count,
    }
}

fn specification_related(
    items: &[ItemSummary],
    index: &SpecIndex,
    root: &ItemSummary,
    budget: usize,
) -> (TraceRelatedView, usize, usize) {
    let root_criteria = closure_criterion_anchors(items, index, root)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut related = TraceRelatedView::default();
    let mut specifications = BTreeSet::new();
    let mut hidden_related_claim_count = 0;
    for item in items {
        for binding in &item.bindings {
            for target in &binding.targets {
                let kind = if binding.role == "verification"
                    || target.claims.iter().any(|claim| {
                        matches!(
                            claim,
                            TargetClaim::Verifies { .. } | TargetClaim::Evidences { .. }
                        )
                    }) {
                    "verification"
                } else {
                    "implementation"
                };
                let matches_root = target.claims.iter().any(|claim| match claim {
                    TargetClaim::Satisfies { criterion }
                    | TargetClaim::Verifies { criterion, .. } => {
                        root_criteria.contains(criterion.to_string().as_str())
                    }
                    TargetClaim::Evidences { anchor } => {
                        root_criteria.contains(anchor.to_string().as_str())
                    }
                    _ => false,
                });
                if !matches_root {
                    continue;
                }
                if item.id != root.id {
                    specifications.insert((
                        item.id.clone(),
                        item.kind.clone(),
                        item.title.clone(),
                        item.presentation_title_key.clone(),
                    ));
                }
                let mut bounded_target = target.clone();
                let hidden_claim_count = bounded_target.claims.len().saturating_sub(budget);
                bounded_target.claims.truncate(budget);
                hidden_related_claim_count += hidden_claim_count;
                let entry = TraceRelatedTargetView {
                    item_id: item.id.clone(),
                    target: bounded_target,
                    hidden_claim_count,
                };
                if kind == "verification" {
                    related.verification.push(entry);
                } else {
                    related.implementation.push(entry);
                }
            }
        }
    }
    related.specification = specifications
        .into_iter()
        .map(
            |(item_id, kind, title, presentation_title_key)| TraceRelatedSpecificationView {
                item_id,
                kind,
                title,
                presentation_title_key,
            },
        )
        .collect();
    related
        .implementation
        .sort_by(|left, right| left.target.reference.cmp(&right.target.reference));
    related
        .verification
        .sort_by(|left, right| left.target.reference.cmp(&right.target.reference));
    let total =
        related.specification.len() + related.implementation.len() + related.verification.len();
    let mut remaining = budget;
    if related.specification.len() > remaining {
        related.specification.truncate(remaining);
        remaining = 0;
    } else {
        remaining -= related.specification.len();
    }
    if related.implementation.len() > remaining {
        related.implementation.truncate(remaining);
        remaining = 0;
    } else {
        remaining -= related.implementation.len();
    }
    if related.verification.len() > remaining {
        related.verification.truncate(remaining);
    }
    let hidden = total.saturating_sub(
        related.specification.len() + related.implementation.len() + related.verification.len(),
    );
    related.hidden_count = hidden;
    related.hidden_claim_count = hidden_related_claim_count;
    (related, hidden, hidden_related_claim_count)
}

struct TraceNodeSpec {
    kind: String,
    label: String,
    secondary_label: Option<String>,
    lane: String,
    source_target: Option<String>,
    item_id: Option<String>,
    metadata: BTreeMap<String, String>,
}

fn trace_node(nodes: &mut BTreeMap<String, TraceNodeView>, id: String, spec: TraceNodeSpec) {
    nodes.entry(id.clone()).or_insert_with(|| TraceNodeView {
        id,
        kind: spec.kind,
        label: spec.label,
        secondary_label: spec.secondary_label,
        lane: spec.lane,
        stable_order: 0,
        source_target: spec.source_target,
        item_id: spec.item_id,
        metadata: spec.metadata,
    });
}

fn add_trace_edge(
    edges: &mut BTreeMap<String, TraceEdgeView>,
    from: &str,
    to: &str,
    relation: &str,
    display_label: &str,
    exact_claim: Option<String>,
) {
    let id = format!("{from}|{relation}|{to}");
    edges.entry(id.clone()).or_insert_with(|| TraceEdgeView {
        id,
        from: from.into(),
        to: to.into(),
        relation: relation.into(),
        display_label: display_label.into(),
        exact_claim,
    });
}

fn add_anchor_trace_node(
    nodes: &mut BTreeMap<String, TraceNodeView>,
    items: &[ItemSummary],
    anchor: String,
) {
    if nodes.contains_key(&anchor) {
        return;
    }
    let (kind, item_id, label, secondary_label, lane, metadata) =
        items
            .iter()
            .flat_map(|item| {
                item.principles
                    .iter()
                    .map(move |value| (&value.anchor, "principle", item, value.statement.clone()))
                    .chain(
                        item.rules.iter().map(move |value| {
                            (&value.anchor, "rule", item, value.statement.clone())
                        }),
                    )
                    .chain(item.criteria.iter().map(move |value| {
                        (&value.anchor, "criterion", item, value.statement.clone())
                    }))
                    .chain(item.bindings.iter().map(move |value| {
                        (&value.anchor, "binding", item, value.responsibility.clone())
                    }))
                    .chain(
                        item.contracts.iter().map(move |value| {
                            (&value.anchor, "contract", item, value.kind.clone())
                        }),
                    )
            })
            .find(|(value, _, _, _)| *value == &anchor)
            .map(|(_, kind, item, statement)| {
                let kind = kind.to_string();
                let lane = if matches!(kind.as_str(), "principle" | "rule") {
                    "governance"
                } else {
                    "specification"
                };
                (
                    kind,
                    Some(item.id.clone()),
                    statement,
                    Some(item.title.clone()),
                    lane,
                    BTreeMap::from([("item_kind".into(), item.kind.clone())]),
                )
            })
            .unwrap_or_else(|| {
                let kind = anchor
                    .clone()
                    .split_once('#')
                    .and_then(|(_, local)| local.split_once('.'))
                    .map(|(kind, _)| kind.to_string())
                    .unwrap_or_else(|| "anchor".into());
                (
                    kind.clone(),
                    anchor.split_once('#').map(|(item, _)| item.to_string()),
                    anchor.clone(),
                    None,
                    if matches!(kind.as_str(), "principle" | "rule") {
                        "governance"
                    } else {
                        "specification"
                    },
                    BTreeMap::new(),
                )
            });
    trace_node(
        nodes,
        anchor,
        TraceNodeSpec {
            kind,
            label,
            secondary_label,
            lane: lane.into(),
            source_target: None,
            item_id,
            metadata,
        },
    );
}

fn add_target_trace_node(
    nodes: &mut BTreeMap<String, TraceNodeView>,
    root: &ItemSummary,
    binding: &BindingSummary,
    target: &BindingTargetSummary,
) {
    let evidence = binding.role == "verification"
        || target.claims.iter().any(|claim| {
            matches!(
                claim,
                TargetClaim::Verifies { .. } | TargetClaim::Evidences { .. }
            )
        });
    let lane = if evidence {
        "evidence"
    } else {
        "implementation"
    };
    let kind = if evidence {
        "verification-target"
    } else {
        "implementation-target"
    };
    let mut metadata = BTreeMap::new();
    metadata.insert("role".into(), binding.role.clone());
    metadata.insert("facet".into(), binding.facet.clone());
    metadata.insert("responsibility".into(), binding.responsibility.clone());
    metadata.insert("adapter".into(), target.adapter.clone());
    metadata.insert("path".into(), target.path.clone());
    metadata.insert(
        "lifecycle".into(),
        serde_json::to_string(&target.lifecycle).unwrap_or_default(),
    );
    metadata.insert(
        "selector".into(),
        serde_json::to_string(&target.selector).unwrap_or_default(),
    );
    trace_node(
        nodes,
        target.reference.clone(),
        TraceNodeSpec {
            kind: kind.into(),
            label: selector_label(&target.selector),
            secondary_label: Some(target.path.clone()),
            lane: lane.into(),
            source_target: Some(target.reference.clone()),
            item_id: Some(root.id.clone()),
            metadata,
        },
    );
}

fn add_target_or_anchor_trace_node(
    nodes: &mut BTreeMap<String, TraceNodeView>,
    items: &[ItemSummary],
    index: &SpecIndex,
    reference: &str,
) {
    if nodes.contains_key(reference) {
        return;
    }
    if let Some((item, binding, target)) = items.iter().find_map(|item| {
        item.bindings.iter().find_map(|binding| {
            binding
                .targets
                .iter()
                .find(|target| target.reference == reference)
                .map(|target| (item, binding, target))
        })
    }) {
        add_target_trace_node(nodes, item, binding, target);
    } else {
        let _known_anchor = reference
            .parse::<SpecAnchor>()
            .ok()
            .is_some_and(|anchor| index.anchors.contains_key(&anchor));
        add_anchor_trace_node(nodes, items, reference.to_string());
    }
}

fn selector_label(selector: &Selector) -> String {
    match selector {
        Selector::File => "file".into(),
        Selector::Symbol { name } => format!("symbol · {name}"),
        Selector::Operation { method, path } => format!("{method} {path}"),
        Selector::Heading { value } => format!("heading · {value}"),
        Selector::JsonPointer { value } => format!("json pointer · {value}"),
        Selector::Marker { value } => format!("marker · {value}"),
    }
}

struct ClaimTraceSpec<'a> {
    items: &'a [ItemSummary],
    index: &'a SpecIndex,
    mode: &'a str,
    target_reference: &'a str,
    claim_index: usize,
    claim: &'a TargetClaim,
}

fn add_claim_trace(
    nodes: &mut BTreeMap<String, TraceNodeView>,
    edges: &mut BTreeMap<String, TraceEdgeView>,
    spec: ClaimTraceSpec<'_>,
) {
    let ClaimTraceSpec {
        items,
        index: spec_index,
        mode,
        target_reference,
        claim_index,
        claim,
    } = spec;
    let exact_claim = serde_json::to_string(claim).ok();
    let claim_node = format!("{target_reference}#claim.{claim_index}");
    let mut destinations = Vec::<(&str, String, &str)>::new();
    match claim {
        TargetClaim::Satisfies { criterion } => {
            destinations.push(("satisfies", criterion.to_string(), "satisfies"));
        }
        TargetClaim::Verifies {
            criterion, covers, ..
        } => {
            destinations.push(("verifies", criterion.to_string(), "verifies"));
            destinations.extend(
                covers
                    .iter()
                    .map(|target| ("covers", target.to_string(), "covers")),
            );
        }
        TargetClaim::Documents { anchor } => {
            destinations.push(("documents", anchor.to_string(), "documents"));
        }
        TargetClaim::Enforces { rule } => {
            destinations.push(("enforces", rule.to_string(), "enforces"));
        }
        TargetClaim::GeneratedFrom { targets } => destinations.extend(
            targets
                .iter()
                .map(|target| ("generated-from", target.to_string(), "generated from")),
        ),
        TargetClaim::Exposes { target } => {
            destinations.push(("exposes", target.to_string(), "exposes"));
        }
        TargetClaim::Evidences { anchor } => {
            destinations.push(("evidences", anchor.to_string(), "evidences"));
        }
    }
    if mode == "exact" {
        let mut metadata = BTreeMap::new();
        metadata.insert("claim".into(), exact_claim.clone().unwrap_or_default());
        trace_node(
            nodes,
            claim_node.clone(),
            TraceNodeSpec {
                kind: "claim".into(),
                label: destinations
                    .first()
                    .map(|(_, _, label)| (*label).to_string())
                    .unwrap_or_else(|| "claim".into()),
                secondary_label: Some("exact canonical claim".into()),
                lane: "specification".into(),
                source_target: None,
                item_id: None,
                metadata,
            },
        );
        add_trace_edge(edges, target_reference, &claim_node, "claim", "claim", None);
    }
    for (relation, destination, display_label) in destinations {
        add_target_or_anchor_trace_node(nodes, items, spec_index, &destination);
        // The readable graph contains the canonical relation. Exact mode adds
        // the claim node without changing neighbourhood reachability.
        add_trace_edge(
            edges,
            target_reference,
            &destination,
            relation,
            display_label,
            exact_claim.clone(),
        );
        if mode == "exact" {
            add_trace_edge(
                edges,
                &claim_node,
                &destination,
                relation,
                display_label,
                exact_claim.clone(),
            );
        }
    }
}

fn closure_criterion_anchors(
    items: &[ItemSummary],
    index: &SpecIndex,
    root: &ItemSummary,
) -> Vec<String> {
    let mut anchors = BTreeSet::new();
    for criterion in &root.criteria {
        anchors.insert(criterion.anchor.clone());
    }
    match root.kind.as_str() {
        "feature" => {
            for binding in &root.bindings {
                for target in &binding.targets {
                    for claim in &target.claims {
                        match claim {
                            TargetClaim::Satisfies { criterion }
                            | TargetClaim::Verifies { criterion, .. } => {
                                anchors.insert(criterion.to_string());
                            }
                            TargetClaim::Evidences { anchor }
                                if anchor.kind == LocalAnchorKind::Criterion =>
                            {
                                anchors.insert(anchor.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        "policy" => {
            let rules = root
                .rules
                .iter()
                .filter_map(|rule| rule.anchor.parse::<SpecAnchor>().ok())
                .collect::<BTreeSet<_>>();
            for (criterion, governed_rules) in &index.criteria_to_rules {
                if governed_rules.iter().any(|rule| rules.contains(rule)) {
                    anchors.insert(criterion.to_string());
                }
            }
        }
        "philosophy" => {
            let principles = root
                .principles
                .iter()
                .filter_map(|principle| principle.anchor.parse::<SpecAnchor>().ok())
                .collect::<BTreeSet<_>>();
            let governing_rules = index
                .rules_to_principles
                .iter()
                .filter(|(_, governed_by)| governed_by.iter().any(|p| principles.contains(p)))
                .map(|(rule, _)| rule)
                .collect::<BTreeSet<_>>();
            for (criterion, governed_rules) in &index.criteria_to_rules {
                if governed_rules
                    .iter()
                    .any(|rule| governing_rules.contains(rule))
                {
                    anchors.insert(criterion.to_string());
                }
            }
        }
        _ => {}
    }
    // Keep only criteria that are present in the canonical projection. This
    // prevents a stale claim from creating an evidence card with no anchor.
    anchors
        .into_iter()
        .filter(|anchor| {
            items.iter().any(|item| {
                item.criteria
                    .iter()
                    .any(|criterion| criterion.anchor == *anchor)
            })
        })
        .collect()
}

fn readiness_identity_matches(value: &str, expected: &str) -> bool {
    value == expected
        || value
            .strip_prefix("criterion:")
            .is_some_and(|candidate| candidate == expected)
        || value
            .strip_prefix("item:")
            .is_some_and(|candidate| candidate == expected)
        || value
            .split(['/', '|', ','])
            .any(|segment| segment == expected)
}

fn readiness_subject_matches(
    subject: &mitase_validation::ReadinessSubject,
    root_id: &str,
    criterion: &str,
) -> bool {
    [subject.id.as_str(), subject.scope_id.as_str()]
        .into_iter()
        .any(|value| {
            readiness_identity_matches(value, root_id)
                || readiness_identity_matches(value, criterion)
        })
}

fn specification_closures(
    projection: &WorkspaceProjection,
    items: &[ItemSummary],
    index: &SpecIndex,
    root: &ItemSummary,
    budget: usize,
) -> (Vec<CriterionClosureView>, usize, usize, usize, usize, usize) {
    let mut hidden_closure_target_count = 0;
    let mut closures = closure_criterion_anchors(items, index, root)
        .into_iter()
        .map(|criterion_anchor_text| {
            let criterion_anchor = criterion_anchor_text.parse::<SpecAnchor>().ok();
            let mut implementation_targets = criterion_anchor
                .as_ref()
                .and_then(|anchor| index.all_criteria_to_implementation_targets.get(anchor))
                .into_iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>();
            let mut verification_targets = criterion_anchor
                .as_ref()
                .and_then(|anchor| index.all_criteria_to_verification_targets.get(anchor))
                .into_iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>();
            for item in items {
                for binding in &item.bindings {
                    for target in &binding.targets {
                        for claim in &target.claims {
                            match claim {
                                TargetClaim::Satisfies { criterion: claimed }
                                    if claimed.to_string() == criterion_anchor_text =>
                                {
                                    implementation_targets.insert(target.reference.clone());
                                }
                                TargetClaim::Verifies {
                                    criterion: claimed,
                                    covers,
                                    ..
                                } if claimed.to_string() == criterion_anchor_text => {
                                    verification_targets.insert(target.reference.clone());
                                    for covered in covers {
                                        implementation_targets.insert(covered.to_string());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            let mut reasons = Vec::new();
            if implementation_targets.is_empty() {
                reasons.push("No exact implementation target satisfies this criterion.".into());
            }
            if verification_targets.is_empty() {
                reasons.push("No exact verification target covers this criterion.".into());
            }
            let known_targets = items
                .iter()
                .flat_map(|item| item.bindings.iter())
                .flat_map(|binding| binding.targets.iter())
                .map(|target| target.reference.as_str())
                .collect::<BTreeSet<_>>();
            let unresolved = implementation_targets
                .iter()
                .chain(verification_targets.iter())
                .any(|target| !known_targets.contains(target.as_str()));
            let state = if implementation_targets.is_empty() {
                "implementation-missing"
            } else if verification_targets.is_empty() {
                "verification-missing"
            } else if unresolved {
                "target-unresolved"
            } else {
                "declaration-only"
            };
            if unresolved {
                reasons.push(
                    "A claim points at a target that is not in the canonical projection.".into(),
                );
            }
            let readiness_blockers = projection
                .readiness
                .axes
                .values()
                .flat_map(|axis| axis.subjects.iter())
                .filter(|subject| {
                    readiness_subject_matches(subject, &root.id, &criterion_anchor_text)
                })
                .flat_map(|subject| subject.blockers.clone())
                .collect::<Vec<_>>();
            reasons.extend(readiness_blockers.iter().cloned());
            let diagnostics = projection
                .diagnostics
                .validation
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.diagnostic.anchor.as_ref().is_some_and(|anchor| {
                        anchor.to_string() == criterion_anchor_text
                            || anchor.item.to_string() == root.id
                    })
                })
                .map(|diagnostic| TraceDiagnosticView {
                    identity: diagnostic.diagnostic.rule_id.clone(),
                    severity: format!("{:?}", diagnostic.diagnostic.severity).to_ascii_lowercase(),
                    message: diagnostic.diagnostic.message.clone(),
                    reason: diagnostic.diagnostic.help.clone(),
                })
                .collect::<Vec<_>>();
            reasons.extend(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.clone()),
            );
            let matching_executions = projection
                .work
                .verification_receipt
                .as_ref()
                .map(|receipt| {
                    receipt
                        .executions
                        .iter()
                        .filter(|execution| {
                            execution.claim.as_ref().is_some_and(|claim| {
                                claim.criterion == criterion_anchor_text
                                    && claim.target == execution.target
                                    && verification_targets.contains(&execution.target)
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let missing_declared_target = verification_targets.iter().any(|target| {
                !matching_executions
                    .iter()
                    .any(|execution| execution.target == *target)
            });
            let runtime_status = if matching_executions.is_empty() {
                "unavailable".to_string()
            } else if matching_executions
                .iter()
                .any(|execution| execution.status == "failed")
            {
                "failed".into()
            } else if verification_targets.is_empty() || missing_declared_target {
                "partial".into()
            } else {
                "passed".into()
            };
            let runtime_details =
                projection
                    .work
                    .verification_receipt
                    .as_ref()
                    .and_then(|receipt| {
                        (!matching_executions.is_empty()).then_some((
                            receipt.completed_at.clone(),
                            receipt.revision.clone(),
                            format!(
                                "{}@{}@{}",
                                receipt.slice_id, receipt.revision, receipt.completed_at
                            ),
                        ))
                    });
            CriterionClosureView {
                criterion: criterion_anchor_text,
                implementation_targets: implementation_targets.into_iter().collect(),
                verification_targets: verification_targets.into_iter().collect(),
                state: state.into(),
                reasons,
                runtime_status,
                runtime_timestamp: runtime_details.as_ref().map(|details| details.0.clone()),
                runtime_revision: runtime_details.as_ref().map(|details| details.1.clone()),
                runtime_receipt: runtime_details.map(|details| details.2),
                runtime_executions: matching_executions,
                readiness_blockers,
                diagnostics,
                hidden_target_count: 0,
                hidden_reason_count: 0,
                hidden_readiness_count: 0,
                hidden_diagnostic_count: 0,
            }
        })
        .collect::<Vec<_>>();
    closures.sort_by(|left, right| left.criterion.cmp(&right.criterion));
    let hidden_closure_count = closures.len().saturating_sub(budget);
    let mut hidden_reason_count = 0;
    let mut hidden_readiness_count = 0;
    let mut hidden_diagnostic_count = 0;
    for closure in &mut closures {
        let total_targets =
            closure.implementation_targets.len() + closure.verification_targets.len();
        let mut remaining = budget;
        if closure.implementation_targets.len() > remaining {
            closure.implementation_targets.truncate(remaining);
            remaining = 0;
        } else {
            remaining -= closure.implementation_targets.len();
        }
        if closure.verification_targets.len() > remaining {
            closure.verification_targets.truncate(remaining);
        }
        closure.hidden_target_count = total_targets.saturating_sub(
            closure.implementation_targets.len() + closure.verification_targets.len(),
        );
        let reason_count = closure.reasons.len();
        let readiness_count = closure.readiness_blockers.len();
        let diagnostic_count = closure.diagnostics.len();
        closure.reasons.truncate(budget);
        closure.readiness_blockers.truncate(budget);
        closure.diagnostics.truncate(budget);
        closure.hidden_reason_count = reason_count.saturating_sub(closure.reasons.len());
        closure.hidden_readiness_count =
            readiness_count.saturating_sub(closure.readiness_blockers.len());
        closure.hidden_diagnostic_count =
            diagnostic_count.saturating_sub(closure.diagnostics.len());
        hidden_closure_target_count += closure.hidden_target_count;
        hidden_reason_count += closure.hidden_reason_count;
        hidden_readiness_count += closure.hidden_readiness_count;
        hidden_diagnostic_count += closure.hidden_diagnostic_count;
    }
    if closures.len() > budget {
        for closure in closures.iter().skip(budget) {
            hidden_closure_target_count +=
                closure.implementation_targets.len() + closure.verification_targets.len();
            hidden_reason_count += closure.reasons.len();
            hidden_readiness_count += closure.readiness_blockers.len();
            hidden_diagnostic_count += closure.diagnostics.len();
        }
        closures.truncate(budget);
    }
    (
        closures,
        hidden_closure_count,
        hidden_closure_target_count,
        hidden_reason_count,
        hidden_readiness_count,
        hidden_diagnostic_count,
    )
}

fn trace_distances(
    root: &str,
    edges: &BTreeMap<String, TraceEdgeView>,
    depth: usize,
) -> BTreeMap<String, usize> {
    let mut distances = BTreeMap::from([(root.to_string(), 0usize)]);
    let mut frontier = vec![root.to_string()];
    while let Some(current) = frontier.pop() {
        let current_depth = distances[&current];
        if current_depth >= depth {
            continue;
        }
        for neighbour in edges.values().filter_map(|edge| {
            if edge.from == current {
                Some(edge.to.clone())
            } else if edge.to == current {
                Some(edge.from.clone())
            } else {
                None
            }
        }) {
            if distances.contains_key(&neighbour) {
                continue;
            }
            distances.insert(neighbour.clone(), current_depth + 1);
            frontier.push(neighbour);
        }
    }
    distances
}

fn trace_lane_rank(lane: &str) -> usize {
    match lane {
        "governance" => 0,
        "specification" => 1,
        "implementation" => 2,
        "evidence" => 3,
        _ => 4,
    }
}

fn filtered_target_suggestions(
    workspace: &SpecWorkspace,
    index: &mitase_workspace::SpecIndex,
    criterion: &SpecAnchor,
    rejected: &BTreeMap<String, String>,
) -> Result<TargetSuggestionSet> {
    let mut set = suggest_targets(criterion, workspace, index)?;
    set.suggestions.retain(|candidate| {
        rejected
            .get(&candidate.id)
            .is_none_or(|fingerprint| fingerprint != &candidate.evidence_fingerprint)
    });
    set.split_recommendation = split_work_recommendation(&set.suggestions, workspace, index);
    for (offset, candidate) in set.suggestions.iter_mut().enumerate() {
        candidate.rank = offset + 1;
    }
    Ok(set)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TargetSuggestionsView {
    #[serde(flatten)]
    set: TargetSuggestionSet,
    approved_ids: Vec<String>,
}

fn approved_suggestion_ids(
    set: &TargetSuggestionSet,
    approvals: &[ApprovedTargetSuggestion],
) -> Vec<String> {
    set.suggestions
        .iter()
        .filter(|candidate| {
            approvals.iter().any(|approval| {
                approval.criterion == set.criterion
                    && approval.suggestion_id == candidate.id
                    && approval.evidence_fingerprint == candidate.evidence_fingerprint
            })
        })
        .map(|candidate| candidate.id.clone())
        .collect()
}

fn target_suggestions_view(
    set: TargetSuggestionSet,
    approvals: &[ApprovedTargetSuggestion],
) -> TargetSuggestionsView {
    let approved_ids = approved_suggestion_ids(&set, approvals);
    TargetSuggestionsView { set, approved_ids }
}

async fn api_target_suggestions(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
) -> Result<Json<TargetSuggestionsView>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let snapshot = service.snapshot()?;
    let (rejected, approvals) = {
        let session = service
            .session
            .read()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        (
            session.rejected_target_suggestions.clone(),
            session.approved_target_suggestions.clone(),
        )
    };
    Ok(Json(target_suggestions_view(
        filtered_target_suggestions(&snapshot.workspace, &snapshot.index, &criterion, &rejected)?,
        &approvals,
    )))
}

async fn api_target_suggestion_reject(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<TargetSuggestionRejectCommand>,
) -> Result<Json<TargetSuggestionsView>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let index = &snapshot.index;
    let current = suggest_targets(&criterion, workspace, index)?;
    if current.suggestion_token != command.suggestion_token {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("target suggestions changed; review the refreshed evidence"),
        ));
    }
    let candidate = current
        .suggestions
        .iter()
        .find(|candidate| candidate.id == command.suggestion_id)
        .ok_or_else(|| anyhow::anyhow!("suggestion is not part of the reviewed candidate set"))?;
    let (rejected, approvals) = {
        let mut session = service
            .session
            .write()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        session
            .rejected_target_suggestions
            .insert(candidate.id.clone(), candidate.evidence_fingerprint.clone());
        session.approved_target_suggestions.retain(|approval| {
            !(approval.criterion == criterion && approval.suggestion_id == candidate.id)
        });
        (
            session.rejected_target_suggestions.clone(),
            session.approved_target_suggestions.clone(),
        )
    };
    Ok(Json(target_suggestions_view(
        filtered_target_suggestions(workspace, index, &criterion, &rejected)?,
        &approvals,
    )))
}

async fn api_target_suggestions_approve(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<TargetSuggestionApprovalCommand>,
) -> Result<Json<TargetSuggestionApprovalView>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let index = &snapshot.index;
    let rejected = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .rejected_target_suggestions
        .clone();
    let current = filtered_target_suggestions(workspace, index, &criterion, &rejected)?;
    if current.suggestion_token != command.suggestion_token {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("target suggestions changed; review the refreshed evidence"),
        ));
    }
    let requested_ids = command
        .suggestion_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if requested_ids.is_empty() || requested_ids.len() != command.suggestion_ids.len() {
        return Err(anyhow::anyhow!("approval requires one or more unique suggestion ids").into());
    }
    let approved = current
        .suggestions
        .iter()
        .filter(|candidate| requested_ids.contains(&candidate.id))
        .cloned()
        .collect::<Vec<_>>();
    if approved.len() != requested_ids.len() {
        return Err(
            anyhow::anyhow!("approval includes a stale, rejected, or unknown suggestion").into(),
        );
    }
    let approved_ids = approved
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    if let Some(split_recommendation) = split_work_recommendation(&approved, workspace, index)
        && approved
            .iter()
            .any(|candidate| !matches!(candidate.transition, TargetTransition::Modify))
    {
        return Ok(Json(TargetSuggestionApprovalView {
            approved_ids: vec![],
            split_recommendation: Some(split_recommendation),
        }));
    }
    let mut transition_groups = BTreeMap::<String, Vec<String>>::new();
    for candidate in &approved {
        transition_groups
            .entry(format!("{:?}", candidate.transition))
            .or_default()
            .push(candidate.id.clone());
    }
    if transition_groups.len() > 1
        && approved.len() > 2
        && approved
            .iter()
            .any(|candidate| !matches!(candidate.transition, TargetTransition::Modify))
    {
        return Ok(Json(TargetSuggestionApprovalView {
            approved_ids: vec![],
            split_recommendation: Some(SplitWorkRecommendation {
                reason: "Selected targets use incompatible transitions; approve one transition group at a time.".into(),
                suggested_groups: transition_groups.into_values().collect(),
            }),
        }));
    }
    {
        let mut session = service
            .session
            .write()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        for candidate in &approved {
            session.approved_target_suggestions.retain(|approval| {
                !(approval.criterion == criterion && approval.suggestion_id == candidate.id)
            });
            session
                .approved_target_suggestions
                .push(ApprovedTargetSuggestion {
                    criterion: criterion.clone(),
                    suggestion_id: candidate.id.clone(),
                    evidence_fingerprint: candidate.evidence_fingerprint.clone(),
                });
        }
    }
    Ok(Json(TargetSuggestionApprovalView {
        approved_ids,
        split_recommendation: None,
    }))
}

fn specification_path(workspace: &SpecWorkspace, anchor: &str) -> Result<PathBuf> {
    let item_id = anchor.split('#').next().unwrap_or(anchor);
    workspace
        .documents
        .iter()
        .find(|loaded| match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => philosophies
                .iter()
                .any(|item| item.id.to_string() == item_id),
            SpecDocument::Policies { policies, .. } => {
                policies.iter().any(|item| item.id.to_string() == item_id)
            }
            SpecDocument::Requirements { requirements, .. } => requirements
                .iter()
                .any(|item| item.id.to_string() == item_id),
            SpecDocument::Features { features, .. } => {
                features.iter().any(|item| item.id.to_string() == item_id)
            }
        })
        .map(|loaded| loaded.path.clone())
        .ok_or_else(|| anyhow::anyhow!("specification {item_id} not found"))
}

fn specification_document_path(workspace: &SpecWorkspace, document: &str) -> Result<PathBuf> {
    let requested = PathBuf::from(document);
    if requested.is_absolute()
        || requested
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        anyhow::bail!("candidate document must be a workspace-relative specification path");
    }
    let requested_path = workspace.root.join(&requested);
    let canonical = requested_path
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("candidate document does not exist"))?;
    workspace
        .documents
        .iter()
        .find(|loaded| loaded.path.canonicalize().ok().as_ref() == Some(&canonical))
        .map(|loaded| loaded.path.clone())
        .ok_or_else(|| anyhow::anyhow!("candidate document is not a loaded specification"))
}

fn patch_path(workspace: &SpecWorkspace, patch: &EditPatch) -> Result<PathBuf> {
    match patch {
        EditPatch::Specification { item_id, .. } => specification_path(workspace, item_id),
        EditPatch::Nested { item_id, .. } => specification_path(workspace, item_id),
        EditPatch::Anchor { anchor, .. } => {
            let item = anchor
                .split('#')
                .next()
                .ok_or_else(|| anyhow::anyhow!("anchor is missing an item id"))?;
            specification_path(workspace, item)
        }
        EditPatch::AddCriterion { requirement_id, .. } => {
            specification_path(workspace, &requirement_id.to_string())
        }
        EditPatch::CreateRequirement { document, .. }
        | EditPatch::CreateFeature { document, .. }
        | EditPatch::AddFeatureTarget { document, .. } => {
            specification_document_path(workspace, document)
        }
        EditPatch::Config { .. } => anyhow::bail!("configuration is not a candidate patch"),
    }
}

fn validate_feature_criterion_link(
    snapshot: &CachedWorkspaceSnapshot,
    patch: &EditPatch,
) -> Result<()> {
    let (anchor, feature_id) = match patch {
        EditPatch::CreateFeature {
            criterion_anchor,
            target,
            status,
            ..
        } => {
            if criterion_anchor.is_some() != target.is_some() {
                anyhow::bail!(
                    "a Feature must declare an exact Requirement Criterion and planned target together"
                );
            }
            if criterion_anchor.is_none() && target.is_none() {
                if status.is_some_and(|status| status != ItemStatus::Planned) {
                    anyhow::bail!(
                        "a Feature target must remain planned until its WorkRequest is approved and finalized"
                    );
                }
                return Ok(());
            }
            let Some(anchor) = criterion_anchor else {
                anyhow::bail!(
                    "a Feature must declare an exact Requirement Criterion and planned target together"
                );
            };
            if status.is_some_and(|status| status != ItemStatus::Planned) {
                anyhow::bail!(
                    "a Feature target must remain planned until its WorkRequest is approved and finalized"
                );
            }
            (anchor, None)
        }
        EditPatch::AddFeatureTarget {
            criterion_anchor,
            feature_id,
            ..
        } => (criterion_anchor, Some(feature_id)),
        _ => return Ok(()),
    };
    let parsed = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!("invalid feature criterion anchor: {error}"))?;
    if parsed.kind != LocalAnchorKind::Criterion {
        anyhow::bail!("feature journey link must point to an exact Requirement Criterion");
    }
    let item = snapshot
        .projection
        .specifications
        .specifications
        .iter()
        .find(|item| item.id == parsed.item.to_string())
        .ok_or_else(|| anyhow::anyhow!("feature journey link references an unknown Requirement"))?;
    if item.kind != "requirement" || item.status.as_deref() == Some("deprecated") {
        anyhow::bail!("feature journey link must reference an active Requirement");
    }
    if !item
        .criteria
        .iter()
        .any(|criterion| criterion.anchor == *anchor)
    {
        anyhow::bail!("feature journey link must reference an exact existing Criterion");
    }
    if let Some(feature_id) = feature_id {
        let feature = snapshot
            .projection
            .specifications
            .specifications
            .iter()
            .find(|item| item.id == feature_id.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("Feature target addition references an unknown Feature")
            })?;
        if feature.kind != "feature" || feature.status.as_deref() == Some("deprecated") {
            anyhow::bail!("Feature target addition must reference an active Feature");
        }
        if feature.status.as_deref() != Some("planned") {
            anyhow::bail!(
                "Feature target addition requires a planned Feature; implemented Features need an explicit lifecycle transition"
            );
        }
    }
    Ok(())
}

fn edit_preview(
    base: &SpecWorkspace,
    _candidate: &SpecWorkspace,
    path: &Path,
    content: &str,
) -> Result<EditPreview> {
    let old = base.read_to_string(path)?;
    let old_hash = content_hash(&old);
    let new_hash = content_hash(content);
    let changed_lines = old
        .lines()
        .zip(content.lines())
        .filter(|(left, right)| left != right)
        .count()
        + old.lines().count().abs_diff(content.lines().count());
    let workspace_fingerprint = base.try_fingerprint()?;
    let preview_token = content_hash(&format!(
        "{}\n{}\n{}",
        path.to_string_lossy(),
        new_hash.clone(),
        workspace_fingerprint
    ));
    Ok(EditPreview {
        path: path.to_string_lossy().into_owned(),
        old_hash,
        new_hash: new_hash.clone(),
        valid: true,
        preview_token,
        candidate_digest: new_hash.clone(),
        workspace_fingerprint,
        changed_lines,
        impact: None,
    })
}

fn edit_preview_for_patch(
    base: &SpecWorkspace,
    candidate: &SpecWorkspace,
    path: &Path,
    content: &str,
    patch: &EditPatch,
    requires_replan: bool,
) -> Result<EditPreview> {
    let mut preview = edit_preview(base, candidate, path, content)?;
    preview.impact = Some(specification_impact(
        base,
        candidate,
        Some(patch),
        requires_replan,
    )?);
    Ok(preview)
}

fn specification_impact(
    base: &SpecWorkspace,
    candidate: &SpecWorkspace,
    patch: Option<&EditPatch>,
    requires_replan: bool,
) -> Result<SpecificationImpact> {
    let base_index = base.index()?;
    let candidate_index = candidate.index()?;
    let changed_anchors = changed_specification_anchors(&candidate_index, patch);
    let mut affected_items = changed_anchors
        .iter()
        .filter_map(|anchor| anchor.split('#').next())
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if let Some(EditPatch::Specification { item_id, .. }) = patch {
        affected_items.insert(item_id.clone());
    }
    if let Some(EditPatch::Nested { item_id, .. }) = patch {
        affected_items.insert(item_id.clone());
    }
    if let Some(EditPatch::AddCriterion { requirement_id, .. }) = patch {
        affected_items.insert(requirement_id.to_string());
    }
    if let Some(EditPatch::CreateRequirement { id, .. } | EditPatch::CreateFeature { id, .. }) =
        patch
    {
        affected_items.insert(id.to_string());
    }
    if let Some(EditPatch::AddFeatureTarget { feature_id, .. }) = patch {
        affected_items.insert(feature_id.to_string());
    }
    let mut implementation_targets: BTreeSet<BoundTargetRef> = BTreeSet::new();
    let mut verification_targets: BTreeSet<BoundTargetRef> = BTreeSet::new();
    for anchor_text in &changed_anchors {
        if let Ok(anchor) = anchor_text.parse::<SpecAnchor>() {
            for target in candidate_index
                .criteria_to_implementation_targets
                .get(&anchor)
                .into_iter()
                .flatten()
            {
                implementation_targets.insert(target.clone());
            }
            for target in candidate_index
                .criteria_to_verification_targets
                .get(&anchor)
                .into_iter()
                .flatten()
            {
                verification_targets.insert(target.clone());
            }
        }
    }
    let affected_targets = implementation_targets
        .iter()
        .chain(verification_targets.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let affected_ownership = ownership_for_targets(&candidate_index, &affected_targets);
    let target_suggestions = changed_anchors
        .iter()
        .filter_map(|anchor| anchor.parse::<SpecAnchor>().ok())
        .filter(|anchor| {
            matches!(
                candidate_index.anchor(anchor),
                Some(mitase_workspace::AnchorValue::Criterion(_))
            )
        })
        .map(|anchor| suggest_targets(&anchor, candidate, &candidate_index))
        .collect::<Result<Vec<_>>>()?;
    let revision = current_revision(&base.root)?;
    let before = mitase_validation::evaluate_readiness(base, &base_index, &revision, false)?;
    let after =
        mitase_validation::evaluate_readiness(candidate, &candidate_index, &revision, false)?;
    let has_implementation_targets = !implementation_targets.is_empty();
    Ok(SpecificationImpact {
        affected_items: affected_items.into_iter().collect(),
        changed_anchors,
        affected_ownership,
        implementation_targets: implementation_targets
            .into_iter()
            .map(|target| target.to_string())
            .collect(),
        verification_targets: verification_targets
            .into_iter()
            .map(|target| target.to_string())
            .collect(),
        target_suggestions,
        readiness_before: readiness_impact(&before),
        readiness_after: readiness_impact(&after),
        work: WorkImpact {
            seedable: has_implementation_targets,
            requires_replan,
            reason: if requires_replan {
                "Existing work plan must be reviewed after a specification change.".into()
            } else if !has_implementation_targets {
                "No explicit implementation target is attached to this candidate yet.".into()
            } else {
                "Explicit implementation targets are available for review.".into()
            },
        },
    })
}

fn ownership_for_targets(
    index: &mitase_workspace::SpecIndex,
    targets: &BTreeSet<mitase_spec_model::BoundTargetRef>,
) -> Vec<String> {
    let mut ownership = BTreeSet::new();
    for target in targets {
        let Some(artifact) = index.target_to_artifact.get(target) else {
            continue;
        };
        for owner in index.artifact_owners.get(artifact).into_iter().flatten() {
            ownership.insert(format!("{}#{}", owner.binding, owner.scope_id));
        }
    }
    ownership.into_iter().collect()
}

fn readiness_impact(report: &mitase_validation::ReadinessReport) -> ReadinessImpact {
    let blockers = [
        &report.inventory,
        &report.ownership,
        &report.seedability,
        &report.workability,
        &report.verification,
        &report.closed_loop,
    ]
    .iter()
    .map(|axis| axis.blockers.len())
    .sum();
    ReadinessImpact {
        status: if blockers == 0 { "ready" } else { "blocked" }.into(),
        blocking_subjects: blockers,
    }
}

fn changed_specification_anchors(
    index: &mitase_workspace::SpecIndex,
    patch: Option<&EditPatch>,
) -> Vec<String> {
    let mut anchors = BTreeSet::new();
    match patch {
        Some(EditPatch::Anchor { anchor, .. }) => {
            anchors.insert(anchor.clone());
        }
        Some(EditPatch::Specification { item_id, .. }) => {
            let _ = (index, item_id);
        }
        Some(EditPatch::Nested { item_id, edit }) => {
            anchors.insert(item_id.clone());
            match edit {
                NestedEdit::Binding { binding, .. } => {
                    anchors.insert(format!("{item_id}#binding.{}", binding.id));
                }
                NestedEdit::Ownership {
                    binding_id,
                    ownership,
                    ..
                } => {
                    anchors.insert(format!("{item_id}#binding.{binding_id}"));
                    anchors.insert(format!(
                        "{item_id}#binding.{binding_id}/owns.{}",
                        ownership.id
                    ));
                }
                NestedEdit::Target {
                    binding_id, target, ..
                } => {
                    anchors.insert(format!("{item_id}#binding.{binding_id}"));
                    anchors.insert(format!(
                        "{item_id}#binding.{binding_id}/target.{}",
                        target.id
                    ));
                }
                NestedEdit::Claim {
                    binding_id,
                    target_id,
                    ..
                } => {
                    anchors.insert(format!("{item_id}#binding.{binding_id}/target.{target_id}"));
                }
                NestedEdit::Contract { contract, .. } => {
                    anchors.insert(format!("{item_id}#contract.{}", contract.id));
                }
            }
        }
        Some(EditPatch::AddCriterion {
            requirement_id,
            criterion,
        }) => {
            let anchor = anchor_string(requirement_id, LocalAnchorKind::Criterion, &criterion.id);
            if let Ok(parsed) = anchor.parse::<SpecAnchor>()
                && index.anchor(&parsed).is_some()
            {
                anchors.insert(anchor);
            }
        }
        Some(EditPatch::CreateRequirement { id, .. })
        | Some(EditPatch::CreateFeature { id, .. }) => {
            if let Some(item) = index
                .item_anchors
                .keys()
                .find(|item| item.to_string() == id.to_string())
            {
                anchors.extend(
                    index
                        .item_anchors
                        .get(item)
                        .into_iter()
                        .flatten()
                        .map(ToString::to_string),
                );
            }
        }
        Some(EditPatch::AddFeatureTarget {
            criterion_anchor, ..
        }) => {
            anchors.insert(criterion_anchor.clone());
        }
        _ => {}
    }
    anchors.into_iter().collect()
}

fn specification_patch_content(
    workspace: &SpecWorkspace,
    path: &Path,
    patch: &EditPatch,
) -> Result<String> {
    let old = workspace.read_to_string(path)?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&old)?;
    match patch {
        EditPatch::Specification { item_id, fields } => {
            let collection = collection_for_value(&value)?;
            let sequence = specification_sequence(&mut value, collection)?;
            let item = sequence
                .iter_mut()
                .find(|item| item.get("id").and_then(serde_yaml::Value::as_str) == Some(item_id))
                .ok_or_else(|| anyhow::anyhow!("specification item {item_id} not found"))?;
            let mapping = item
                .as_mapping_mut()
                .ok_or_else(|| anyhow::anyhow!("specification item is not a mapping"))?;
            for (key, field) in patch_fields(fields)? {
                let key = serde_yaml::Value::String(key);
                if !matches!(key.as_str(), Some("id" | "bindings" | "contracts")) {
                    mapping.insert(key, field);
                }
            }
        }
        EditPatch::Nested { item_id, edit } => {
            nested_patch_content(&mut value, item_id, edit)?;
        }
        EditPatch::Anchor { anchor, fields } => {
            let parsed = anchor
                .parse::<SpecAnchor>()
                .map_err(|error| anyhow::anyhow!("invalid specification anchor: {error}"))?;
            let collection = collection_for_value(&value)?;
            let sequence = specification_sequence(&mut value, collection)?;
            let item = sequence
                .iter_mut()
                .find(|item| {
                    item.get("id")
                        .and_then(serde_yaml::Value::as_str)
                        .is_some_and(|id| id == parsed.item.to_string())
                })
                .ok_or_else(|| anyhow::anyhow!("specification item {} not found", parsed.item))?;
            let mapping = item
                .as_mapping_mut()
                .ok_or_else(|| anyhow::anyhow!("specification item is not a mapping"))?;
            let collection = match parsed.kind {
                LocalAnchorKind::Principle => "principles",
                LocalAnchorKind::Rule => "rules",
                LocalAnchorKind::Criterion => "criteria",
                LocalAnchorKind::Binding | LocalAnchorKind::Contract => {
                    anyhow::bail!("anchor kind is not human-editable")
                }
            };
            let nested = mapping
                .get_mut(serde_yaml::Value::String(collection.into()))
                .and_then(serde_yaml::Value::as_sequence_mut)
                .ok_or_else(|| anyhow::anyhow!("anchor collection is missing"))?;
            let entry = nested
                .iter_mut()
                .find(|entry| {
                    entry
                        .get("id")
                        .and_then(serde_yaml::Value::as_str)
                        .is_some_and(|id| id == parsed.local_id.to_string())
                })
                .ok_or_else(|| anyhow::anyhow!("anchor {} not found", anchor))?;
            let entry = entry
                .as_mapping_mut()
                .ok_or_else(|| anyhow::anyhow!("anchor is not a mapping"))?;
            for (key, field) in anchor_patch_fields(fields)? {
                entry.insert(serde_yaml::Value::String(key), field);
            }
        }
        EditPatch::AddCriterion {
            requirement_id,
            criterion,
        } => {
            if collection_for_value(&value)? != "requirements" {
                anyhow::bail!("a criterion can only be added to a requirement");
            }
            let requirements = specification_sequence(&mut value, "requirements")?;
            let requirement = requirements
                .iter_mut()
                .find(|item| {
                    item.get("id")
                        .and_then(serde_yaml::Value::as_str)
                        .is_some_and(|id| id == requirement_id.to_string())
                })
                .ok_or_else(|| anyhow::anyhow!("requirement {requirement_id} not found"))?;
            let mapping = requirement
                .as_mapping_mut()
                .ok_or_else(|| anyhow::anyhow!("requirement is not a mapping"))?;
            let criteria = mapping
                .get_mut(serde_yaml::Value::String("criteria".into()))
                .and_then(serde_yaml::Value::as_sequence_mut)
                .ok_or_else(|| anyhow::anyhow!("requirement criteria are missing"))?;
            if criteria.iter().any(|entry| {
                entry
                    .get("id")
                    .and_then(serde_yaml::Value::as_str)
                    .is_some_and(|id| id == criterion.id.to_string())
            }) {
                anyhow::bail!(
                    "criterion {} already exists in requirement {requirement_id}",
                    criterion.id
                );
            }
            criteria.push(serde_yaml::to_value(Criterion {
                id: criterion.id.clone(),
                kind: criterion.kind,
                statement: criterion.statement.clone(),
                governed_by: criterion.governed_by.clone(),
            })?);
        }
        EditPatch::CreateRequirement {
            id,
            title,
            description,
            priority,
            status,
            criteria,
            ..
        } => {
            if collection_for_value(&value)? != "requirements" {
                anyhow::bail!("candidate destination is not a requirements document");
            }
            if criteria.is_empty() {
                anyhow::bail!("a requirement requires at least one criterion");
            }
            let requirement = Requirement {
                id: id.clone(),
                title: title.clone(),
                description: description.clone(),
                priority: *priority,
                status: status.unwrap_or(ItemStatus::Planned),
                criteria: criteria
                    .iter()
                    .map(|criterion| Criterion {
                        id: criterion.id.clone(),
                        kind: criterion.kind,
                        statement: criterion.statement.clone(),
                        governed_by: criterion.governed_by.clone(),
                    })
                    .collect(),
                bindings: vec![],
            };
            specification_sequence(&mut value, "requirements")?
                .push(serde_yaml::to_value(requirement)?);
        }
        EditPatch::CreateFeature {
            id,
            title,
            summary,
            status,
            criterion_anchor,
            target,
            ..
        } => {
            if collection_for_value(&value)? != "features" {
                anyhow::bail!("candidate destination is not a features document");
            }
            let bindings = target
                .as_ref()
                .map(|target| {
                    let path = RepoPath::new(target.path.clone())
                        .map_err(|error| anyhow::anyhow!("invalid Feature target path: {error}"))?;
                    let claims = criterion_anchor
                        .as_deref()
                        .map(|anchor| {
                            anchor
                                .parse::<SpecAnchor>()
                                .map(|criterion| vec![TargetClaim::Satisfies { criterion }])
                                .map_err(|error| {
                                    anyhow::anyhow!("invalid Feature target claim: {error}")
                                })
                        })
                        .transpose()?
                        .unwrap_or_default();
                    Ok::<_, anyhow::Error>(vec![ArtifactBinding {
                        id: "implementation".into(),
                        role: BindingRole::Implementation,
                        facet: "work".into(),
                        responsibility: "Implement the planned Feature target.".into(),
                        owns: vec![],
                        targets: vec![ArtifactTarget {
                            id: target.id.clone(),
                            adapter: target.adapter.clone(),
                            path,
                            selector: target.selector.clone(),
                            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
                            claims,
                        }],
                    }])
                })
                .transpose()?
                .unwrap_or_default();
            let feature = mitase_spec_model::Feature {
                id: id.clone(),
                title: title.clone(),
                summary: summary.clone(),
                status: status.unwrap_or(ItemStatus::Planned),
                bindings,
                contracts: vec![],
            };
            specification_sequence(&mut value, "features")?.push(serde_yaml::to_value(feature)?);
        }
        EditPatch::AddFeatureTarget {
            feature_id,
            criterion_anchor,
            target,
            ..
        } => {
            if collection_for_value(&value)? != "features" {
                anyhow::bail!("candidate destination is not a features document");
            }
            let parsed_criterion = criterion_anchor
                .parse::<SpecAnchor>()
                .map_err(|error| anyhow::anyhow!("invalid Feature target claim: {error}"))?;
            let sequence = specification_sequence(&mut value, "features")?;
            let item = sequence
                .iter_mut()
                .find(|item| {
                    item.get("id")
                        .and_then(serde_yaml::Value::as_str)
                        .is_some_and(|id| id == feature_id.to_string())
                })
                .ok_or_else(|| anyhow::anyhow!("Feature {feature_id} not found"))?;
            let mut feature: mitase_spec_model::Feature = serde_yaml::from_value(item.clone())?;
            let path = RepoPath::new(target.path.clone())
                .map_err(|error| anyhow::anyhow!("invalid Feature target path: {error}"))?;
            let binding = feature
                .bindings
                .iter_mut()
                .find(|binding| binding.role == BindingRole::Implementation)
                .ok_or_else(|| anyhow::anyhow!("Feature has no implementation binding"))?;
            if binding
                .targets
                .iter()
                .any(|candidate| candidate.id == target.id)
            {
                anyhow::bail!("Feature target {} already exists", target.id);
            }
            binding.targets.push(ArtifactTarget {
                id: target.id.clone(),
                adapter: target.adapter.clone(),
                path,
                selector: target.selector.clone(),
                lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
                claims: vec![TargetClaim::Satisfies {
                    criterion: parsed_criterion,
                }],
            });
            *item = serde_yaml::to_value(feature)?;
        }
        EditPatch::Config { .. } => anyhow::bail!("configuration is not a specification patch"),
    }
    let content = serde_yaml::to_string(&value)?;
    let candidate: SpecDocument = serde_yaml::from_str(&content)?;
    if candidate.schema() != mitase_spec_model::SPEC_SCHEMA {
        anyhow::bail!("specification schema must be mitase/spec/v1");
    }
    Ok(content)
}

fn nested_patch_content(
    value: &mut serde_yaml::Value,
    item_id: &str,
    edit: &NestedEdit,
) -> Result<()> {
    let collection = collection_for_value(value)?;
    let sequence = specification_sequence(value, collection)?;
    let item = sequence
        .iter_mut()
        .find(|item| item.get("id").and_then(serde_yaml::Value::as_str) == Some(item_id))
        .ok_or_else(|| anyhow::anyhow!("specification item {item_id} not found"))?;
    let mapping = item
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("specification item is not a mapping"))?;
    match edit {
        NestedEdit::Binding {
            operation,
            binding,
            current_id,
        } => {
            let bindings = mapping_sequence(mapping, "bindings")?;
            upsert_or_delete(
                bindings,
                "id",
                binding.id.to_string(),
                serde_yaml::to_value(binding)?,
                *operation,
                current_id.as_deref(),
            )?;
        }
        NestedEdit::Ownership {
            operation,
            binding_id,
            ownership,
            current_id,
        } => {
            let binding = find_mapping(
                mapping_sequence(mapping, "bindings")?,
                binding_id.to_string().as_str(),
            )?;
            let owns = mapping_sequence(binding, "owns")?;
            upsert_or_delete(
                owns,
                "id",
                ownership.id.to_string(),
                serde_yaml::to_value(ownership)?,
                *operation,
                current_id.as_deref(),
            )?;
        }
        NestedEdit::Target {
            operation,
            binding_id,
            target,
            current_id,
        } => {
            let binding = find_mapping(
                mapping_sequence(mapping, "bindings")?,
                binding_id.to_string().as_str(),
            )?;
            let targets = mapping_sequence(binding, "targets")?;
            upsert_or_delete(
                targets,
                "id",
                target.id.to_string(),
                serde_yaml::to_value(target)?,
                *operation,
                current_id.as_deref(),
            )?;
        }
        NestedEdit::Claim {
            operation,
            binding_id,
            target_id,
            claim_index,
            claim,
        } => {
            let binding = find_mapping(
                mapping_sequence(mapping, "bindings")?,
                binding_id.to_string().as_str(),
            )?;
            let target = find_mapping(
                mapping_sequence(binding, "targets")?,
                target_id.to_string().as_str(),
            )?;
            let claims = mapping_sequence(target, "claims")?;
            match operation {
                NestedEditOperation::Upsert => {
                    let value = serde_yaml::to_value(claim)?;
                    if *claim_index > claims.len() {
                        anyhow::bail!(
                            "claim index {} is outside the target claim list",
                            claim_index
                        );
                    }
                    if *claim_index == claims.len() {
                        claims.push(value);
                    } else {
                        claims[*claim_index] = value;
                    }
                }
                NestedEditOperation::Delete => {
                    if *claim_index >= claims.len() {
                        anyhow::bail!(
                            "claim index {} is outside the target claim list",
                            claim_index
                        );
                    }
                    claims.remove(*claim_index);
                }
            }
        }
        NestedEdit::Contract {
            operation,
            contract,
            current_id,
        } => {
            let contracts = mapping_sequence(mapping, "contracts")?;
            upsert_or_delete(
                contracts,
                "id",
                contract.id.to_string(),
                serde_yaml::to_value(contract)?,
                *operation,
                current_id.as_deref(),
            )?;
        }
    }
    Ok(())
}

fn mapping_sequence<'a>(
    mapping: &'a mut serde_yaml::Mapping,
    key: &str,
) -> Result<&'a mut Vec<serde_yaml::Value>> {
    let key_value = serde_yaml::Value::String(key.into());
    if !mapping.contains_key(&key_value) {
        mapping.insert(key_value.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(&key_value)
        .and_then(serde_yaml::Value::as_sequence_mut)
        .ok_or_else(|| anyhow::anyhow!("{key} must be a sequence"))
}

fn find_mapping<'a>(
    sequence: &'a mut [serde_yaml::Value],
    id: &str,
) -> Result<&'a mut serde_yaml::Mapping> {
    sequence
        .iter_mut()
        .find(|entry| entry.get("id").and_then(serde_yaml::Value::as_str) == Some(id))
        .and_then(serde_yaml::Value::as_mapping_mut)
        .ok_or_else(|| anyhow::anyhow!("nested entity {id} not found"))
}

fn upsert_or_delete(
    sequence: &mut Vec<serde_yaml::Value>,
    id_key: &str,
    id: String,
    value: serde_yaml::Value,
    operation: NestedEditOperation,
    current_id: Option<&str>,
) -> Result<()> {
    if let Some(current_id) = current_id
        && current_id != id
    {
        anyhow::bail!(
            "nested entity ids are immutable; create a new entity instead of renaming {current_id}"
        );
    }
    let position = sequence.iter().position(|entry| {
        entry.get(id_key).and_then(serde_yaml::Value::as_str) == Some(id.as_str())
    });
    match (operation, position) {
        (NestedEditOperation::Upsert, Some(position)) => sequence[position] = value,
        (NestedEditOperation::Upsert, None) if current_id.is_some() => {
            anyhow::bail!("nested entity {id} not found for immutable update")
        }
        (NestedEditOperation::Upsert, None) => sequence.push(value),
        (NestedEditOperation::Delete, Some(position)) => {
            sequence.remove(position);
        }
        (NestedEditOperation::Delete, None) => anyhow::bail!("nested entity {id} not found"),
    }
    Ok(())
}

fn collection_for_value(value: &serde_yaml::Value) -> Result<&'static str> {
    let kind = value
        .get("kind")
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("specification kind is missing"))?;
    match kind {
        "philosophies" => Ok("philosophies"),
        "policies" => Ok("policies"),
        "requirements" => Ok("requirements"),
        "features" => Ok("features"),
        _ => anyhow::bail!("unsupported specification kind {kind}"),
    }
}

fn specification_sequence<'a>(
    value: &'a mut serde_yaml::Value,
    collection: &str,
) -> Result<&'a mut Vec<serde_yaml::Value>> {
    value
        .get_mut(collection)
        .and_then(serde_yaml::Value::as_sequence_mut)
        .ok_or_else(|| anyhow::anyhow!("specification collection {collection} is missing"))
}

fn anchor_patch_fields(fields: &AnchorPatchFields) -> Result<BTreeMap<String, serde_yaml::Value>> {
    let mut output = BTreeMap::new();
    match fields {
        AnchorPatchFields::Principle {
            statement,
            applies_to,
        } => {
            insert_optional(&mut output, "statement", statement)?;
            insert_optional(&mut output, "applies_to", applies_to)?;
        }
        AnchorPatchFields::Rule { statement, level } => {
            insert_optional(&mut output, "statement", statement)?;
            insert_optional(&mut output, "level", level)?;
        }
        AnchorPatchFields::Criterion {
            statement,
            kind,
            governed_by,
        } => {
            insert_optional(&mut output, "statement", statement)?;
            insert_optional(&mut output, "kind", kind)?;
            insert_optional(&mut output, "governed_by", governed_by)?;
        }
    }
    Ok(output)
}

fn patch_fields(fields: &SpecificationPatchFields) -> Result<BTreeMap<String, serde_yaml::Value>> {
    let mut output = BTreeMap::new();
    match fields {
        SpecificationPatchFields::Philosophy { title, summary }
        | SpecificationPatchFields::Policy { title, summary } => {
            insert_optional(&mut output, "title", title)?;
            insert_optional(&mut output, "summary", summary)?;
        }
        SpecificationPatchFields::Requirement {
            title,
            description,
            priority,
            status,
        } => {
            insert_optional(&mut output, "title", title)?;
            insert_optional(&mut output, "description", description)?;
            insert_optional(&mut output, "priority", priority)?;
            insert_optional(&mut output, "status", status)?;
        }
        SpecificationPatchFields::Feature {
            title,
            summary,
            status,
        } => {
            insert_optional(&mut output, "title", title)?;
            insert_optional(&mut output, "summary", summary)?;
            insert_optional(&mut output, "status", status)?;
        }
    }
    Ok(output)
}

fn insert_optional<T: serde::Serialize>(
    output: &mut BTreeMap<String, serde_yaml::Value>,
    name: &str,
    value: &Option<T>,
) -> Result<()> {
    if let Some(value) = value {
        output.insert(name.into(), serde_yaml::to_value(value)?);
    }
    Ok(())
}

fn edit_content(workspace: &SpecWorkspace, path: &Path, patch: &EditPatch) -> Result<String> {
    match patch {
        EditPatch::Specification { .. }
        | EditPatch::Anchor { .. }
        | EditPatch::AddCriterion { .. }
        | EditPatch::Nested { .. }
        | EditPatch::CreateRequirement { .. }
        | EditPatch::CreateFeature { .. }
        | EditPatch::AddFeatureTarget { .. } => specification_patch_content(workspace, path, patch),
        EditPatch::Config { config } => Ok(serde_yaml::to_string(config)?),
    }
}

fn validate_overlay(workspace: &SpecWorkspace, index: &mitase_workspace::SpecIndex) -> Result<()> {
    let revision = current_revision(&workspace.root)?;
    // Preview is a structural operation.  It must never execute a candidate
    // runner (the candidate config may contain an arbitrary executable).  The
    // explicit POST /api/readiness/run and /api/work/verify paths are the only
    // execution entry points.
    let result =
        mitase_validation::validate_without_readiness(&mitase_validation::ValidationContext {
            config: &workspace.config,
            workspace,
            index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: mitase_validation::PlanValidationMode::PreState,
            preset: workspace.config.validation.preset,
            revision: Some(&revision),
            change_base_revision: None,
        });
    if result
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, mitase_diagnostics::Severity::Error))
    {
        anyhow::bail!(
            "candidate overlay failed validation: {}",
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    Ok(())
}

fn atomic_replace(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("edit path has no parent"))?;
    let temporary = parent.join(format!(
        ".mitase-edit-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&temporary, content)?;
    #[cfg(unix)]
    fs::File::open(&temporary)?.sync_all()?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

async fn api_specification_preview(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let path = specification_path(workspace, &anchor)?;
    let content = edit_content(workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document.clone())?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    Ok(Json(edit_preview(workspace, &overlay, &path, &content)?))
}

async fn api_specification_apply(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = basis(&service, &command.basis)?;
    ensure_no_active_agent_run(&service)?;
    let workspace = &snapshot.workspace;
    let path = specification_path(workspace, &anchor)?;
    let content = edit_content(workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document.clone())?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let old = workspace.read_to_string(&path)?;
    let preview = edit_preview(workspace, &overlay, &path, &content)?;
    if command.preview_token.as_deref() != Some(preview.preview_token.as_str()) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "apply requires the exact preview token for this candidate; preview the patch again"
            ),
        ));
    }
    store.write_mutation_journal(
        "governance-edit",
        &preview.preview_token,
        vec![mitase_delivery::MutationJournalFile {
            path: path.to_string_lossy().into_owned(),
            original: Some(old.as_bytes().to_vec()),
        }],
        Vec::new(),
    )?;
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        store.clear_mutation_journal()?;
        return Err(error.into());
    }
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    clear_work_execution_state(&mut session);
    store.clear_mutation_journal()?;
    Ok(Json(preview))
}

async fn api_specification_candidate_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    validate_feature_criterion_link(&snapshot, &command.patch)?;
    let path = patch_path(workspace, &command.patch)?;
    let content = edit_content(workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let requires_replan = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .plan
        .is_some();
    Ok(Json(edit_preview_for_patch(
        workspace,
        &overlay,
        &path,
        &content,
        &command.patch,
        requires_replan,
    )?))
}

async fn api_specification_candidate_apply(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = basis(&service, &command.basis)?;
    ensure_no_active_agent_run(&service)?;
    let workspace = &snapshot.workspace;
    validate_feature_criterion_link(&snapshot, &command.patch)?;
    let path = patch_path(workspace, &command.patch)?;
    let content = edit_content(workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let old = workspace.read_to_string(&path)?;
    let requires_replan = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .plan
        .is_some();
    let preview = edit_preview_for_patch(
        workspace,
        &overlay,
        &path,
        &content,
        &command.patch,
        requires_replan,
    )?;
    if command.preview_token.as_deref() != Some(preview.preview_token.as_str()) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "apply requires the exact preview token for this candidate; preview the patch again"
            ),
        ));
    }
    store.write_mutation_journal(
        "governance-edit",
        &preview.preview_token,
        vec![mitase_delivery::MutationJournalFile {
            path: path.to_string_lossy().into_owned(),
            original: Some(old.as_bytes().to_vec()),
        }],
        Vec::new(),
    )?;
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        store.clear_mutation_journal()?;
        return Err(error.into());
    }
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    clear_work_execution_state(&mut session);
    store.clear_mutation_journal()?;
    Ok(Json(preview))
}

async fn api_config_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let content = edit_content(
        workspace,
        &workspace.root.join("mitase.yaml"),
        &command.patch,
    )?;
    let config: mitase_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    Ok(Json(edit_preview(
        workspace,
        &overlay,
        &workspace.root.join("mitase.yaml"),
        &content,
    )?))
}

async fn api_config(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<mitase_project_model::ProjectConfig>, ApiError> {
    Ok(Json(SpecWorkspace::load(&service.workspace_root)?.config))
}

async fn api_config_apply(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = basis(&service, &command.basis)?;
    ensure_no_active_agent_run(&service)?;
    let workspace = &snapshot.workspace;
    let content = edit_content(
        workspace,
        &workspace.root.join("mitase.yaml"),
        &command.patch,
    )?;
    let config: mitase_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let path = workspace.root.join("mitase.yaml");
    let old = workspace.read_to_string(&path)?;
    let preview = edit_preview(workspace, &overlay, &path, &content)?;
    if command.preview_token.as_deref() != Some(preview.preview_token.as_str()) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "apply requires the exact preview token for this candidate; preview the patch again"
            ),
        ));
    }
    store.write_mutation_journal(
        "governance-edit",
        &preview.preview_token,
        vec![mitase_delivery::MutationJournalFile {
            path: path.to_string_lossy().into_owned(),
            original: Some(old.as_bytes().to_vec()),
        }],
        Vec::new(),
    )?;
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        store.clear_mutation_journal()?;
        return Err(error.into());
    }
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    clear_work_execution_state(&mut session);
    store.clear_mutation_journal()?;
    Ok(Json(preview))
}

async fn api_branch_scope(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<BranchScopeQuery>,
) -> Result<Json<ScopeView>, ApiError> {
    let snapshot = service.snapshot()?;
    let workspace = &snapshot.workspace;
    let specifications = snapshot.projection.specifications.specifications.clone();
    let range = match query.range {
        Some(range) => range,
        None => configured_change_range(workspace)?,
    };
    validate_diff_range(&range)?;
    let changed = branch_changed_files(&workspace.root, &range)?;
    Ok(Json(ScopeView {
        branch: Some(branch_scope_view(
            &snapshot.index,
            &specifications,
            range,
            &changed,
        )),
    }))
}

#[derive(Debug, Deserialize)]
struct BranchScopeQuery {
    range: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopeDiffView {
    pub range: String,
    pub state: String,
    pub additions: usize,
    pub deletions: usize,
    pub files: Vec<ScopeDiffFileView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopeDiffFileView {
    pub path: String,
    pub status: ChangeStatus,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
}

async fn api_scope_diff(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<BranchScopeQuery>,
) -> Result<Json<ScopeDiffView>, ApiError> {
    let snapshot = service.snapshot()?;
    let range = match query.range {
        Some(range) => range,
        None => configured_change_range(&snapshot.workspace)?,
    };
    validate_diff_range(&range)?;
    let changed = branch_changed_files(&snapshot.workspace.root, &range)?;
    Ok(Json(scope_diff_view(
        &snapshot.workspace.root,
        range,
        &changed,
    )?))
}

fn validate_diff_range(range: &str) -> Result<()> {
    if range.trim().is_empty()
        || range.starts_with('-')
        || range.chars().any(char::is_control)
        || range.chars().any(char::is_whitespace)
    {
        anyhow::bail!("invalid Git range");
    }
    Ok(())
}

fn diff_base_revision(root: &Path, range: &str) -> Result<String> {
    if let Some((left, right)) = range.split_once("...") {
        validate_diff_range(left)?;
        validate_diff_range(right)?;
        return git_merge_base(root, left, right);
    }
    if let Some((left, _)) = range.split_once("..") {
        validate_diff_range(left)?;
        return Ok(left.to_owned());
    }
    Ok(range.to_owned())
}

fn scope_diff_view(
    root: &Path,
    range: String,
    changed: &[mitase_validation::ChangedFile],
) -> Result<ScopeDiffView> {
    let base = diff_base_revision(root, &range)?;
    let mut files = Vec::new();
    for changed_file in changed {
        let path = changed_file
            .new_path
            .as_ref()
            .or(changed_file.old_path.as_ref())
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        let mut patch = git_output(
            root,
            &[
                "diff",
                "--no-ext-diff",
                "--unified=3",
                "--no-color",
                &base,
                "--",
                &path,
            ],
        )
        .map(String::from_utf8)??;
        if patch.is_empty()
            && matches!(
                changed_file.status,
                mitase_validation::ChangeStatus::Untracked
            )
        {
            let content = fs::read_to_string(root.join(&path)).unwrap_or_default();
            patch = format!(
                "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{}",
                content.lines().count(),
                content
                    .lines()
                    .map(|line| format!("+{line}\n"))
                    .collect::<String>()
            );
        }
        let additions = patch
            .lines()
            .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
            .count();
        let deletions = patch
            .lines()
            .filter(|line| line.starts_with('-') && !line.starts_with("---"))
            .count();
        files.push(ScopeDiffFileView {
            path,
            status: changed_file.status,
            additions,
            deletions,
            patch,
        });
    }
    let additions = files.iter().map(|file| file.additions).sum();
    let deletions = files.iter().map(|file| file.deletions).sum();
    Ok(ScopeDiffView {
        range,
        state: if files.is_empty() {
            "empty".into()
        } else {
            "ready".into()
        },
        additions,
        deletions,
        files,
    })
}

fn branch_changed_files(root: &Path, range: &str) -> Result<Vec<mitase_validation::ChangedFile>> {
    let mut files = Vec::new();
    collect_branch_status(
        root,
        &["diff", "--name-status", "-z", "-M", "--relative", range],
        &mut files,
    )?;
    collect_branch_patch(
        root,
        &[
            "diff",
            "-M",
            "--relative",
            "--unified=0",
            "--no-color",
            range,
        ],
        &mut files,
    )?;
    collect_branch_status(
        root,
        &[
            "diff",
            "--name-status",
            "-z",
            "-M",
            "--relative",
            "--cached",
        ],
        &mut files,
    )?;
    collect_branch_patch(
        root,
        &[
            "diff",
            "-M",
            "--relative",
            "--unified=0",
            "--no-color",
            "--cached",
        ],
        &mut files,
    )?;
    collect_branch_status(
        root,
        &["diff", "--name-status", "-z", "-M", "--relative"],
        &mut files,
    )?;
    collect_branch_patch(
        root,
        &["diff", "-M", "--relative", "--unified=0", "--no-color"],
        &mut files,
    )?;
    let output = git_output(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    for path in output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let Ok(path) = mitase_spec_model::RepoPath::new(String::from_utf8_lossy(path).as_ref())
        else {
            continue;
        };
        if files
            .iter()
            .all(|file| file.new_path.as_ref() != Some(&path))
        {
            files.push(mitase_validation::ChangedFile {
                status: mitase_validation::ChangeStatus::Untracked,
                old_path: None,
                new_path: Some(path),
                hunks: vec![],
            });
        }
    }
    Ok(files)
}

fn configured_change_range(workspace: &SpecWorkspace) -> Result<String> {
    match workspace.config.validation.changed.baseline.as_ref() {
        Some(ChangeBaseline::MergeBase { against }) => {
            git_merge_base(&workspace.root, "HEAD", &against.0)
                .or_else(|_| parent_or_current(&workspace.root))
        }
        Some(ChangeBaseline::Revision { revision }) => Ok(revision.0.clone()),
        Some(ChangeBaseline::Parent) => parent_or_current(&workspace.root),
        None => git_merge_base(&workspace.root, "HEAD", "origin/main")
            .or_else(|_| parent_or_current(&workspace.root)),
    }
}

fn parent_or_current(root: &Path) -> Result<String> {
    git_output(root, &["rev-parse", "HEAD^"])
        .map(|output| String::from_utf8_lossy(&output).trim().to_owned())
        .or_else(|_| {
            git_output(root, &["rev-parse", "HEAD"])
                .map(|output| String::from_utf8_lossy(&output).trim().to_owned())
        })
}

fn git_merge_base(root: &Path, left: &str, right: &str) -> Result<String> {
    let output = git_output(root, &["merge-base", left, right])?;
    let revision = String::from_utf8(output)?.trim().to_owned();
    if revision.is_empty() {
        anyhow::bail!("git merge-base {left} {right} returned no revision");
    }
    Ok(revision)
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(args)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git command failed: git {}", args.join(" "));
    }
    Ok(output.stdout)
}

fn collect_branch_status(
    root: &Path,
    args: &[&str],
    files: &mut Vec<mitase_validation::ChangedFile>,
) -> Result<()> {
    let output = git_output(root, args)?;
    let mut entries = output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    while let Some(status_bytes) = entries.next() {
        let status_text = String::from_utf8_lossy(status_bytes);
        let kind = status_text.chars().next().unwrap_or('M');
        let first = entries
            .next()
            .map(String::from_utf8_lossy)
            .map(|path| path.into_owned());
        let second = (kind == 'R' || kind == 'C')
            .then(|| {
                entries
                    .next()
                    .map(String::from_utf8_lossy)
                    .map(|path| path.into_owned())
            })
            .flatten();
        let (old_text, new_text) = match kind {
            'A' => (None, first),
            'D' => (first, None),
            'R' | 'C' => (first, second),
            _ => (first.clone(), first),
        };
        let old_path = old_text
            .as_deref()
            .map(mitase_spec_model::RepoPath::new)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let new_path = new_text
            .as_deref()
            .map(mitase_spec_model::RepoPath::new)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let status = match kind {
            'A' => mitase_validation::ChangeStatus::Added,
            'D' => mitase_validation::ChangeStatus::Deleted,
            'R' => mitase_validation::ChangeStatus::Renamed,
            _ => mitase_validation::ChangeStatus::Modified,
        };
        if let Some(file) = files
            .iter_mut()
            .find(|file| file.new_path == new_path && file.old_path == old_path)
        {
            file.status = status;
        } else {
            files.push(mitase_validation::ChangedFile {
                status,
                old_path,
                new_path,
                hunks: vec![],
            });
        }
    }
    Ok(())
}

fn collect_branch_patch(
    root: &Path,
    args: &[&str],
    files: &mut [mitase_validation::ChangedFile],
) -> Result<()> {
    let output = git_output(root, args)?;
    let mut current_old_path: Option<String> = None;
    let mut current_new_path: Option<String> = None;
    for line in String::from_utf8(output)?.lines() {
        if let Some(path) = line.strip_prefix("--- a/") {
            current_old_path = Some(path.to_owned());
            current_new_path = None;
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_new_path = Some(path.to_owned());
            continue;
        }
        if line == "+++ /dev/null" {
            current_new_path = None;
            continue;
        }
        let Some(hunk) = line.strip_prefix("@@ ") else {
            continue;
        };
        let mut parts = hunk.split_whitespace();
        let Some(old) = parts.next() else { continue };
        let Some(new) = parts.next() else { continue };
        let parse_range = |value: &str| -> Option<(usize, usize)> {
            let value = value.get(1..)?;
            let (start, count) = value.split_once(',').map_or((value, "1"), |parts| parts);
            Some((start.parse().ok()?, count.parse().ok()?))
        };
        let Some((old_start, old_count)) = parse_range(old) else {
            continue;
        };
        let Some((new_start, new_count)) = parse_range(new) else {
            continue;
        };
        if let Some(file) = files.iter_mut().find(|file| {
            current_new_path.as_deref().is_some_and(|path| {
                file.new_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            }) || current_old_path.as_deref().is_some_and(|path| {
                file.old_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            })
        }) {
            file.hunks.push(mitase_validation::ChangedRange {
                old_start,
                old_end: old_start.saturating_add(old_count),
                new_start,
                new_end: new_start.saturating_add(new_count),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct SourceQuery {
    path: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Serialize)]
struct SourceView {
    path: String,
    content: String,
    hash: String,
    line_start: usize,
    line_end: usize,
    is_excerpt: bool,
}

async fn api_source(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<SourceQuery>,
) -> Result<Json<SourceView>, ApiError> {
    if query.path.is_some() == query.target.is_some() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("source requests require exactly one of path or target"),
        ));
    }
    if let Some(reference) = query.target {
        let target = reference
            .parse::<BoundTargetRef>()
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, anyhow::anyhow!(error)))?;
        let snapshot = service.snapshot()?;
        let artifact = snapshot.index.target(&target).ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                anyhow::anyhow!("target {target} was not found"),
            )
        })?;
        let resolved =
            mitase_workspace::resolve_target_in_workspace(&snapshot.workspace, artifact)?;
        return Ok(Json(SourceView {
            path: artifact.path.to_string_lossy().into_owned(),
            content: resolved.excerpt,
            hash: resolved.content_hash,
            line_start: resolved.line_start,
            line_end: resolved.line_end,
            is_excerpt: true,
        }));
    }
    let source_path = query.path.expect("validated source path");
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let relative = mitase_spec_model::RepoPath::new(&source_path)
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, anyhow::anyhow!(error)))?;
    let root = fs::canonicalize(&workspace.root).map_err(anyhow::Error::from)?;
    let path = fs::canonicalize(root.join(relative.as_path()))
        .map_err(|error| ApiError(StatusCode::NOT_FOUND, error.into()))?;
    if !path.starts_with(&root) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            anyhow::anyhow!("source path escapes the workspace"),
        ));
    }
    let content = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
    let line_end = content.lines().count().max(1);
    Ok(Json(SourceView {
        path: relative.to_string_lossy().into_owned(),
        hash: content_hash(&content),
        content,
        line_start: 1,
        line_end,
        is_excerpt: false,
    }))
}
fn validate_requirement_origin(index: &SpecIndex, origin: &WorkOrigin) -> Result<()> {
    mitase_planner::validate_work_origin(index, origin)
}

fn validate_work_origin(
    _workspace: &SpecWorkspace,
    index: &SpecIndex,
    origin: &WorkOrigin,
) -> Result<()> {
    if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
        validate_requirement_origin(index, origin)
    } else {
        mitase_planner::validate_work_origin(index, origin)
    }
}

fn ensure_homogeneous_approved_transitions(candidates: &[TargetSuggestion]) -> Result<()> {
    let transitions = candidates
        .iter()
        .map(|candidate| format!("{:?}", candidate.transition))
        .collect::<BTreeSet<_>>();
    if transitions.len() > 1 {
        anyhow::bail!(
            "approved target suggestions use multiple transitions; create separate WorkRequests"
        );
    }
    Ok(())
}

fn resolve_feature_origin_targets(
    index: &SpecIndex,
    origin: &WorkOrigin,
) -> Result<Vec<RequestedTarget>> {
    let targets = match origin {
        WorkOrigin::FeatureImplementationBinding { targets, .. } => targets.clone(),
        WorkOrigin::FeatureImplementationTarget { target, .. } => vec![target.clone()],
        WorkOrigin::RequirementCriterion { .. } => {
            anyhow::bail!("feature origin targets require a Feature implementation origin")
        }
    };
    targets
        .into_iter()
        .map(|reference| {
            let artifact = index
                .target(&reference)
                .ok_or_else(|| anyhow::anyhow!("feature origin target {reference} is unknown"))?;
            if matches!(
                artifact.lifecycle,
                mitase_spec_model::ArtifactTargetLifecycle::Absent
            ) {
                anyhow::bail!("feature origin target {reference} is absent");
            }
            Ok(RequestedTarget {
                reference,
                criterion: Some(origin.criterion().clone()),
                transition: TargetTransition::Modify,
            })
        })
        .collect()
}

fn resolve_approved_target_candidates(
    anchor: &SpecAnchor,
    suggestions: Result<Vec<TargetSuggestion>>,
    approvals: &[ApprovedTargetSuggestion],
) -> Result<Vec<TargetSuggestion>> {
    let suggestions = suggestions?;
    Ok(approvals
        .iter()
        .filter(|approval| approval.criterion == *anchor)
        .filter_map(|approval| {
            suggestions
                .iter()
                .find(|candidate| {
                    candidate.id == approval.suggestion_id
                        && candidate.evidence_fingerprint == approval.evidence_fingerprint
                })
                .cloned()
        })
        .collect())
}

fn resolve_requested_targets(
    anchor: &SpecAnchor,
    suggestions: Result<Vec<TargetSuggestion>>,
    approvals: &[ApprovedTargetSuggestion],
) -> Result<Vec<RequestedTarget>> {
    let suggestions = suggestions?;
    Ok(approvals
        .iter()
        .filter(|approval| approval.criterion == *anchor)
        .filter_map(|approval| {
            suggestions.iter().find(|candidate| {
                candidate.id == approval.suggestion_id
                    && candidate.evidence_fingerprint == approval.evidence_fingerprint
            })
        })
        .map(|candidate| RequestedTarget {
            reference: candidate.reference.clone(),
            criterion: Some(anchor.clone()),
            transition: candidate.transition,
        })
        .collect())
}

async fn api_journey_action(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<JourneyActionCommand>,
) -> Result<Response, ApiError> {
    let (basis_command, action) = command.into_parts();
    let selection_candidate_digest = match &action {
        JourneyAction::SelectSlice {
            candidate_plan_digest,
            ..
        } => Some(candidate_plan_digest.clone()),
        _ => None,
    };
    if !matches!(
        &action,
        JourneyAction::Create { .. }
            | JourneyAction::SelectSlice { .. }
            | JourneyAction::Rename { .. }
    ) {
        ensure_journey_transition(&service, &basis_command, &action)?;
    }
    match action {
        JourneyAction::Create {
            schema,
            origin,
            title,
        } => {
            let snapshot = basis(&service, &basis_command).map_err(|error| {
                origin_error(
                    StatusCode::CONFLICT,
                    "stale-basis",
                    error.to_string(),
                    Some(&origin),
                    vec![],
                )
            })?;
            if schema != mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA {
                return Err(origin_error(
                    StatusCode::BAD_REQUEST,
                    "unsupported-origin-shape",
                    "origin capability schema is invalid",
                    Some(&origin),
                    vec![],
                ));
            }
            let anchor = origin.criterion().clone();
            validate_work_origin(&snapshot.workspace, &snapshot.index, &origin).map_err(
                |error| {
                    origin_error(
                        origin_error_status(&error),
                        &origin_error_code(&error),
                        error.to_string(),
                        Some(&origin),
                        vec![],
                    )
                },
            )?;
            if title.trim().is_empty() {
                return Err(origin_error(
                    StatusCode::BAD_REQUEST,
                    "unsupported-origin-shape",
                    "provide a work title before continuing",
                    Some(&origin),
                    vec![],
                ));
            }
            let approvals = service
                .session
                .read()
                .map_err(|_| anyhow::anyhow!("workbench session lock"))?
                .approved_target_suggestions
                .clone();
            let (
                requested_targets,
                operation,
                max_added_bytes_per_target,
                max_added_lines_per_target,
            ) = match &origin {
                WorkOrigin::RequirementCriterion { .. } => {
                    let suggestions =
                        suggest_targets(&anchor, &snapshot.workspace, &snapshot.index)
                            .map(|set| set.suggestions)
                            .map_err(|error| ApiError(StatusCode::INTERNAL_SERVER_ERROR, error))?;
                    let approved_candidates = resolve_approved_target_candidates(
                        &anchor,
                        Ok(suggestions.clone()),
                        &approvals,
                    )
                    .map_err(|error| ApiError(StatusCode::INTERNAL_SERVER_ERROR, error))?;
                    ensure_homogeneous_approved_transitions(&approved_candidates).map_err(
                        |error| {
                            origin_error(
                                StatusCode::UNPROCESSABLE_ENTITY,
                                "mixed-transition",
                                error.to_string(),
                                Some(&origin),
                                vec![],
                            )
                        },
                    )?;
                    let requested_targets =
                        resolve_requested_targets(&anchor, Ok(suggestions), &approvals)
                            .map_err(|error| ApiError(StatusCode::INTERNAL_SERVER_ERROR, error))?;
                    let operation = match approved_candidates
                        .first()
                        .map(|candidate| candidate.transition)
                    {
                        Some(TargetTransition::Add) => WorkOperation::Add,
                        Some(TargetTransition::Remove) => WorkOperation::Remove,
                        _ => WorkOperation::Modify,
                    };
                    let max_added_bytes_per_target = approved_candidates
                        .iter()
                        .filter_map(|candidate| candidate.budget_bytes)
                        .max();
                    let max_added_lines_per_target = approved_candidates
                        .iter()
                        .filter_map(|candidate| candidate.budget_lines)
                        .max();
                    (
                        requested_targets,
                        operation,
                        max_added_bytes_per_target,
                        max_added_lines_per_target,
                    )
                }
                WorkOrigin::FeatureImplementationBinding { .. }
                | WorkOrigin::FeatureImplementationTarget { .. } => (
                    resolve_feature_origin_targets(&snapshot.index, &origin).map_err(|error| {
                        origin_error(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            &origin_error_code(&error),
                            error.to_string(),
                            Some(&origin),
                            vec![],
                        )
                    })?,
                    WorkOperation::Modify,
                    None,
                    None,
                ),
            };
            let requirement_origin = matches!(&origin, WorkOrigin::RequirementCriterion { .. });
            let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
            let title = title.trim().to_owned();
            let request = WorkRequest {
                schema: WORK_REQUEST_SCHEMA.into(),
                id: store.new_id("work"),
                title,
                operation,
                origin,
                constraints: WorkConstraints {
                    max_slices: None,
                    max_added_bytes_per_target,
                    max_added_lines_per_target,
                    ..WorkConstraints::default()
                },
                requested_targets,
            };
            let mut session = service
                .session
                .write()
                .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
            if session.approved_target_suggestions != approvals {
                return Err(ApiError(
                    StatusCode::CONFLICT,
                    anyhow::anyhow!("target suggestions changed; review the refreshed evidence"),
                ));
            }
            abandon_active_agent_runs(&service, "work request was replaced")?;
            if requirement_origin {
                session
                    .approved_target_suggestions
                    .retain(|approval| approval.criterion != anchor);
            }
            session.work_title = Some(request.title.clone());
            session.draft_request = Some(request);
            session.plan = None;
            session.selected_slice = None;
            session.context_pack = None;
            session.verification_receipt = None;
            session.agent_run = None;
            session.last_validation = None;
        }
        JourneyAction::SelectSlice {
            schema,
            candidate_plan_digest,
            slice_id,
        } => {
            if schema != mitase_work_model::WORK_SELECT_SLICE_SCHEMA {
                return Err(ApiError(
                    StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("invalid select_slice schema"),
                ));
            }
            select_slice(&service, &basis_command, &candidate_plan_digest, &slice_id)?;
        }
        JourneyAction::Rename { title } => {
            let _ = basis(&service, &basis_command)?;
            let title = title.trim();
            if title.is_empty() || title.chars().count() > 120 {
                return Err(ApiError(
                    StatusCode::BAD_REQUEST,
                    anyhow::anyhow!("work title must contain 1 to 120 characters"),
                ));
            }
            let mut session = service
                .session
                .write()
                .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
            if session.draft_request.is_none() {
                return Err(ApiError(
                    StatusCode::CONFLICT,
                    anyhow::anyhow!("start work before changing its title"),
                ));
            }
            session.work_title = Some(title.to_owned());
        }
        JourneyAction::Prepare => {
            let Json(plan) = api_plan(State(service.clone()), Json(basis_command.clone())).await?;
            // A guided journey owns one focused change boundary. Requests
            // supplied by a CLI caller can legitimately plan multiple ready
            // slices, so do not quietly pick the first one here.
            if matches!(plan.status, PlanStatus::Ready) && plan.slices.len() == 1 {
                let slice_id = plan
                    .slices
                    .first()
                    .map(|slice| slice.id.clone())
                    .ok_or_else(|| anyhow::anyhow!("the plan has no executable slice"))?;
                let _ = api_context(
                    State(service.clone()),
                    Json(ContextCommand {
                        basis: basis_command.clone(),
                        execution: ExecutionIdentity {
                            plan_digest: plan.canonical_digest.clone(),
                            slice_id,
                        },
                    }),
                )
                .await?;
                let selected_slice = service
                    .session
                    .read()
                    .map_err(|_| anyhow::anyhow!("workbench session lock"))?
                    .selected_slice
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("context selection was not stored"))?;
                let _ = api_validate(
                    State(service.clone()),
                    Json(ValidateCommand {
                        basis: basis_command.clone(),
                        execution: ExecutionIdentity {
                            plan_digest: plan.canonical_digest.clone(),
                            slice_id: selected_slice,
                        },
                    }),
                )
                .await?;
            }
        }
        JourneyAction::Approve { execution } => {
            let _ = api_approve(
                State(service.clone()),
                Json(ApproveCommand {
                    basis: basis_command.clone(),
                    execution,
                }),
            )
            .await?;
        }
        JourneyAction::Start { execution } => {
            let _ = api_agent_start(
                State(service.clone()),
                Json(AgentStartCommand {
                    basis: basis_command.clone(),
                    execution,
                }),
            )
            .await?;
        }
        JourneyAction::Retry { execution } => {
            let _ = api_agent_start_with_retry(
                State(service.clone()),
                Json(AgentStartCommand {
                    basis: basis_command.clone(),
                    execution,
                }),
                true,
            )
            .await?;
        }
        JourneyAction::Verify { execution } => {
            let _ = api_agent_verify(
                State(service.clone()),
                Json(SliceCommand {
                    basis: basis_command.clone(),
                    execution,
                }),
            )
            .await?;
        }
        JourneyAction::Finalize {
            execution,
            attempt_id,
            preview_token,
        } => {
            let Json(preview) = api_finalize_preview(
                State(service.clone()),
                Json(FinalizeCommand {
                    basis: basis_command.clone(),
                    execution: execution.clone(),
                    attempt_id: attempt_id.clone(),
                    preview_token: preview_token.clone(),
                }),
            )
            .await?;
            let _ = api_finalize_apply(
                State(service.clone()),
                Json(FinalizeCommand {
                    basis: basis_command.clone(),
                    execution,
                    attempt_id,
                    preview_token: preview_token.or(Some(preview.preview_token)),
                }),
            )
            .await?;
        }
        JourneyAction::Restart | JourneyAction::Cancel => {
            let _ = api_discard(State(service.clone())).await?;
        }
    }
    // Verification and finalization are allowed to update editable source.
    // The action's precondition has already been checked by its lower-level
    // endpoint, so build the response from the fresh snapshot rather than
    // rejecting a side effect that just succeeded against the old basis.
    let snapshot = service.snapshot()?;
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let projection = project_session(&snapshot, &session)?;
    if let Some(candidate_plan_digest) = selection_candidate_digest {
        let plan_digest = projection
            .work
            .plan
            .as_ref()
            .map(|plan| plan.digest.clone())
            .ok_or_else(|| anyhow::anyhow!("selected work plan is missing"))?;
        let slice_id = projection
            .work
            .selected_slice
            .clone()
            .ok_or_else(|| anyhow::anyhow!("selected work slice is missing"))?;
        return Ok(Json(SelectSliceResponse {
            schema: mitase_work_model::WORK_SELECT_SLICE_RESPONSE_SCHEMA.into(),
            action: "select_slice".into(),
            candidate_plan_digest,
            plan_digest,
            slice_id,
            projection,
        })
        .into_response());
    }
    Ok(Json(projection).into_response())
}

fn select_slice(
    service: &WorkbenchService,
    basis_command: &MutationBasis,
    candidate_plan_digest: &str,
    slice_id: &str,
) -> Result<(), ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root).map_err(ApiError::from)?;
    let _workspace_lock = store.lock_workspace().map_err(ApiError::from)?;
    let snapshot = basis(service, basis_command).map_err(|error| {
        work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "stale-basis",
            error.to_string(),
            Some(candidate_plan_digest.to_owned()),
            vec![],
        )
    })?;
    if store
        .has_current_approval_for_plan(&snapshot.workspace, candidate_plan_digest)
        .map_err(ApiError::from)?
    {
        return Err(work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "selection-locked",
            "slice selection is locked after approval",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    let (request, candidate_plan) = {
        let session = service
            .session
            .read()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        let request = session
            .draft_request
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no work request is available"))?;
        let plan = session
            .plan
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no candidate work plan is available"))?;
        if plan.canonical_digest != candidate_plan_digest {
            return Err(work_action_error(
                StatusCode::CONFLICT,
                "select_slice",
                "stale-candidate-plan",
                "candidate plan digest is stale",
                Some(candidate_plan_digest.to_owned()),
                vec![],
            ));
        }
        (request, plan)
    };
    let candidate = candidate_plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| {
            work_action_error(
                StatusCode::NOT_FOUND,
                "select_slice",
                "unknown-slice",
                "the requested execution slice is not in the candidate plan",
                Some(candidate_plan_digest.to_owned()),
                slice_nearest(&candidate_plan, candidate_plan_digest),
            )
        })?;
    let canonical_candidate_plan = mitase_planner::plan(
        &request,
        &snapshot.workspace,
        &snapshot.index,
        &snapshot.revision,
    )
    .map_err(|error| {
        work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "replan-failed",
            error.to_string(),
            Some(candidate_plan_digest.to_owned()),
            vec![],
        )
    })?;
    let canonical_candidate = canonical_candidate_plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| {
            work_action_error(
                StatusCode::CONFLICT,
                "select_slice",
                "non-canonical-slice",
                "the candidate slice is not present in the canonical planner output",
                Some(candidate_plan_digest.to_owned()),
                vec![],
            )
        })?;
    if canonical_candidate != candidate {
        return Err(work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "non-canonical-slice",
            "the candidate slice differs from the canonical planner output",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    let editable_transitions = candidate
        .editable_targets
        .iter()
        .map(|target| format!("{:?}", target.transition))
        .collect::<BTreeSet<_>>();
    if editable_transitions.len() > 1 {
        return Err(work_action_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "select_slice",
            "mixed-transition",
            "the selected slice contains multiple editable transitions",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    let candidate_blockers = split_candidate_blockers(
        &candidate_plan,
        candidate,
        &request,
        &snapshot.workspace.config,
        &snapshot.index,
    );
    if !split_candidate_selectable(
        &candidate_plan,
        candidate,
        &request,
        &snapshot.workspace.config,
        &snapshot.index,
        &candidate_blockers,
    ) {
        return Err(work_action_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "select_slice",
            "non-selectable-slice",
            "the selected slice is not selectable",
            Some(candidate_plan_digest.to_owned()),
            vec![slice_nearest_entry(candidate, candidate_plan_digest)],
        ));
    }
    let mut requested_targets = candidate
        .editable_targets
        .iter()
        .chain(candidate.verification_targets.iter())
        .chain(candidate.readonly_context.iter())
        .map(|target| RequestedTarget {
            reference: target.reference.clone(),
            criterion: Some(request.origin.criterion().clone()),
            transition: target.transition,
        })
        .collect::<Vec<_>>();
    requested_targets.sort_by(|left, right| {
        left.reference.cmp(&right.reference).then_with(|| {
            requested_transition_rank(left.transition)
                .cmp(&requested_transition_rank(right.transition))
        })
    });
    let mut deduplicated: Vec<RequestedTarget> = Vec::with_capacity(requested_targets.len());
    for requested in requested_targets {
        if let Some(previous) = deduplicated.last()
            && previous.reference == requested.reference
        {
            let same_editable_boundary = is_editable_transition(previous.transition)
                && is_editable_transition(requested.transition)
                && previous.transition == requested.transition;
            let editable_with_derived_verification = is_editable_transition(previous.transition)
                && requested.transition == TargetTransition::RunOnly;
            if same_editable_boundary || editable_with_derived_verification {
                continue;
            }
            return Err(work_action_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "select_slice",
                "mixed-transition",
                "the selected slice derives conflicting transitions for one exact target",
                Some(candidate_plan_digest.to_owned()),
                vec![],
            ));
        }
        deduplicated.push(requested);
    }
    let requested_targets = deduplicated;
    if requested_targets.is_empty() && request.operation != WorkOperation::Investigate {
        return Err(work_action_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "select_slice",
            "non-selectable-slice",
            "the selected slice has no exact target boundary",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    let mut selected_request = request;
    selected_request.requested_targets = requested_targets;
    selected_request.constraints.max_slices = Some(1);
    selected_request.constraints.exact_scope = true;
    selected_request.constraints.exact_generated_targets = candidate
        .readonly_context
        .iter()
        .filter(|target| target.access == TargetAccessMode::Generated)
        .map(|target| target.reference.clone())
        .collect();
    selected_request.constraints.exact_contracts = candidate.contracts.clone();
    let replanned = mitase_planner::plan(
        &selected_request,
        &snapshot.workspace,
        &snapshot.index,
        &snapshot.revision,
    )
    .map_err(|error| {
        work_action_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "select_slice",
            "replan-failed",
            error.to_string(),
            Some(candidate_plan_digest.to_owned()),
            vec![],
        )
    })?;
    if replanned.status != PlanStatus::Ready || replanned.slices.len() != 1 {
        return Err(work_action_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "select_slice",
            "replan-failed",
            "selected slice could not be replanned as one ready slice",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    let returned_slice_id = replanned.slices[0].id.clone();
    let selected = &replanned.slices[0];
    if !target_boundaries_match(&selected.editable_targets, &candidate.editable_targets)
        || !target_boundaries_match(
            &selected.verification_targets,
            &candidate.verification_targets,
        )
        || !target_boundaries_match(&selected.readonly_context, &candidate.readonly_context)
        || selected.contracts != candidate.contracts
    {
        return Err(work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "scope-drift",
            "the selected execution boundary changed during canonical replan",
            Some(candidate_plan_digest.to_owned()),
            vec![slice_nearest_entry(candidate, candidate_plan_digest)],
        ));
    }
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    if session
        .plan
        .as_ref()
        .is_none_or(|plan| plan.canonical_digest != candidate_plan_digest)
    {
        return Err(work_action_error(
            StatusCode::CONFLICT,
            "select_slice",
            "stale-candidate-plan",
            "candidate plan changed during selection",
            Some(candidate_plan_digest.to_owned()),
            vec![],
        ));
    }
    session.draft_request = Some(selected_request);
    session.plan = Some(replanned);
    session.selected_slice = Some(returned_slice_id);
    session.context_pack = None;
    session.last_validation = None;
    Ok(())
}

fn target_boundaries_match(
    selected: &[mitase_work_model::PlannedTarget],
    candidate: &[mitase_work_model::PlannedTarget],
) -> bool {
    if selected.len() != candidate.len() {
        return false;
    }
    selected.iter().zip(candidate).all(|(selected, candidate)| {
        let mut selected = selected.clone();
        let mut candidate = candidate.clone();
        // The canonical planner may describe the same readonly target from
        // its explicit request or from criterion context. Provenance text is
        // explanatory; the execution boundary is the exact target identity,
        // mode, transition, selector, and content basis.
        selected.reason.clear();
        candidate.reason.clear();
        selected == candidate
    })
}

fn slice_nearest(plan: &WorkPlan, candidate_plan_digest: &str) -> Vec<serde_json::Value> {
    plan.slices
        .iter()
        .map(|slice| slice_nearest_entry(slice, candidate_plan_digest))
        .collect()
}

fn is_editable_transition(transition: TargetTransition) -> bool {
    matches!(
        transition,
        TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove
    )
}

fn requested_transition_rank(transition: TargetTransition) -> u8 {
    match transition {
        TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove => 0,
        TargetTransition::RunOnly => 1,
        TargetTransition::Readonly => 2,
    }
}

fn slice_nearest_entry(
    slice: &mitase_work_model::ExecutionSlice,
    candidate_plan_digest: &str,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "slice",
        "id": slice.id,
        "candidate_plan_digest": candidate_plan_digest,
    })
}

async fn api_plan(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<MutationBasis>,
) -> Result<Json<WorkPlan>, ApiError> {
    let snapshot = basis(&service, &command)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let request = session
        .draft_request
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no work request selected"))?;
    validate_work_origin(&snapshot.workspace, &snapshot.index, &request.origin)
        .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    mitase_planner::validate_work_request(&snapshot.index, &request)
        .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    let plan = plan(
        &request,
        &snapshot.workspace,
        &snapshot.index,
        &snapshot.revision,
    )?;
    session.plan = Some(plan.clone());
    session.selected_slice = None;
    session.context_pack = None;
    session.verification_receipt = None;
    session.agent_run = None;
    session.last_validation = None;
    Ok(Json(plan))
}
async fn api_context(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ContextCommand>,
) -> Result<Json<mitase_work_model::ContextPack>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    if plan.canonical_digest != command.execution.plan_digest {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("execution plan digest does not match the session plan"),
        ));
    }
    if let Some(selected) = &session.selected_slice
        && selected != &command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("execution slice does not match the selected session slice"),
        ));
    }
    let context = mitase_planner::export_context(
        plan,
        &command.execution.slice_id,
        &snapshot.workspace,
        &snapshot.index,
        &snapshot.revision,
    )?;
    session.context_pack = Some(context.clone());
    session.selected_slice = Some(command.execution.slice_id.clone());
    Ok(Json(context))
}
async fn api_approve(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ApproveCommand>,
) -> Result<Json<PlanApproval>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = basis(&service, &command.basis)?;
    let session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    if plan.canonical_digest != command.execution.plan_digest
        || session.selected_slice.as_deref() != Some(command.execution.slice_id.as_str())
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("approval execution identity does not match the selected plan slice"),
        ));
    }
    let canonical = mitase_validation::canonical_plan_for_execution(
        &snapshot.workspace,
        &snapshot.index,
        plan,
        &snapshot.revision,
    )
    .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    if !matches!(canonical.status, PlanStatus::Ready) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("only a ready plan can be approved"),
        ));
    }
    if canonical.slices.len() != 1 || canonical.slices[0].id != command.execution.slice_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("approval requires exactly one canonical execution slice"),
        ));
    }
    let approval = PlanApproval {
        schema: PLAN_APPROVAL_SCHEMA.into(),
        approval_id: store.new_id("approval"),
        plan_digest: canonical.canonical_digest.clone(),
        slice_id: command.execution.slice_id.clone(),
        workspace_fingerprint: snapshot.workspace.try_fingerprint()?,
        revision: snapshot.revision.clone(),
        reviewed_at: timestamp(),
        plan: canonical,
    };
    Ok(Json(store.approve_while_locked(&approval)?))
}

async fn api_agent_start(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentStartCommand>,
) -> Result<Json<AgentRun>, ApiError> {
    api_agent_start_with_retry(State(service), Json(command), false).await
}

async fn api_agent_start_with_retry(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentStartCommand>,
    retry: bool,
) -> Result<Json<AgentRun>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let (plan, selected_slice) = {
        let session = service
            .session
            .read()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        (
            session
                .plan
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no work plan"))?,
            session.selected_slice.clone(),
        )
    };
    if plan.canonical_digest != command.execution.plan_digest
        || selected_slice.as_deref() != Some(command.execution.slice_id.as_str())
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("select the requested slice before starting the agent"),
        ));
    }
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
    let approval = store.approval(&command.execution).map_err(|error| {
        ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent start requires an approved plan: {error}"),
        )
    })?;
    if approval.plan != plan {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("session plan differs from the approved plan"),
        ));
    }
    let run = mitase_agent::start_run(
        &snapshot.workspace,
        &approval,
        &command.execution.slice_id,
        retry,
    )?;
    service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run = Some(run.clone());
    Ok(Json(run))
}

async fn api_agent_patch(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentPatchCommand>,
) -> Result<Json<mitase_work_model::AgentPatchRecord>, ApiError> {
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = mitase_agent::current_run(&snapshot.workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    if run.plan_digest != command.execution.plan_digest
        || run.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the requested execution identity"),
        ));
    }
    match mitase_agent::apply_scoped_patch(&snapshot.workspace, &run, &command.patch) {
        Ok(record) => Ok(Json(record)),
        Err(error) => Err(ApiError(StatusCode::CONFLICT, error)),
    }
}

async fn api_agent_blocker(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentBlockerCommand>,
) -> Result<Json<AgentEvent>, ApiError> {
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = mitase_agent::current_run(&snapshot.workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    if run.plan_digest != command.execution.plan_digest
        || run.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the requested execution identity"),
        ));
    }
    let event = mitase_agent::record_blocker(&snapshot.workspace, &run, command.blocker)?;
    Ok(Json(event))
}

async fn api_agent_scope_expansion(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentScopeExpansionCommand>,
) -> Result<Json<AgentEvent>, ApiError> {
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = mitase_agent::current_run(&snapshot.workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    if run.plan_digest != command.execution.plan_digest
        || run.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the requested execution identity"),
        ));
    }
    Ok(Json(mitase_agent::request_scope_expansion(
        &snapshot.workspace,
        &run,
        command.reason,
        command.requested_targets,
    )?))
}

async fn api_agent_verify(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<SliceCommand>,
) -> Result<Json<CompletionAttempt>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = mitase_agent::current_run(&snapshot.workspace, &run)?;
    if run.plan_digest != command.execution.plan_digest
        || run.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("verification slice does not match the active agent run"),
        ));
    }
    if !matches!(run.status, AgentRunStatus::Active) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run is not active; resolve its blocker or start a new run"),
        ));
    }
    let approval = store
        .approval(&ExecutionIdentity {
            plan_digest: run.plan_digest.clone(),
            slice_id: run.slice_id.clone(),
        })
        .map_err(|error| {
            ApiError(
                StatusCode::CONFLICT,
                anyhow::anyhow!("agent verification requires an approved plan: {error}"),
            )
        })?;
    let attempt = store.execute_and_record_agent_verification_while_locked(
        &snapshot.workspace,
        &run,
        &approval.plan,
        &command.execution.slice_id,
    )?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    session.verification_receipt = attempt.receipt.clone();
    Ok(Json(attempt))
}

async fn api_validate(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ValidateCommand>,
) -> Result<Json<ValidationRunView>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let started = SystemTime::now();
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    if plan.canonical_digest != command.execution.plan_digest
        || session.selected_slice.as_deref() != Some(command.execution.slice_id.as_str())
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("validation execution identity does not match the selected plan slice"),
        ));
    }
    let canonical_plan = mitase_validation::canonical_plan_for_execution(
        &snapshot.workspace,
        &snapshot.index,
        plan,
        &snapshot.revision,
    )
    .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    let result = mitase_validation::validate_without_readiness(&ValidationContext {
        config: &snapshot.workspace.config,
        workspace: &snapshot.workspace,
        index: &snapshot.index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(&canonical_plan),
        selected_slice: session
            .selected_slice
            .as_ref()
            .and_then(|id| canonical_plan.slices.iter().find(|slice| &slice.id == id)),
        plan_mode: PlanValidationMode::PreState,
        preset: snapshot.workspace.config.validation.preset,
        revision: Some(&snapshot.revision),
        change_base_revision: None,
    });
    let view = ValidationRunView::completed(
        "work-plan",
        Some(canonical_plan.canonical_digest.clone()),
        result,
        false,
        true,
        snapshot.workspace.config.validation.preset,
        started,
    );
    session.last_validation = Some(view.clone());
    Ok(Json(view))
}
async fn api_verify(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<SliceCommand>,
) -> Result<Json<CompletionAttempt>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = execution_basis(&service, &command.basis)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let approval = store.approval(&command.execution).map_err(|error| {
        ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("plan approval required before verification: {error}"),
        )
    })?;
    if approval.plan != *plan {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("session plan differs from approved plan"),
        ));
    }
    if plan.canonical_digest != command.execution.plan_digest
        || session
            .selected_slice
            .as_deref()
            .is_none_or(|selected| selected != command.execution.slice_id)
        || !session
            .last_validation
            .as_ref()
            .is_some_and(|validation| matches!(validation.state, ValidationRunState::Passed))
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("verification requires a validated selected slice"),
        ));
    }
    let attempt = store.execute_and_append_attempt_while_locked(
        &snapshot.workspace,
        plan,
        &command.execution.slice_id,
    )?;
    session.verification_receipt = attempt.receipt.clone();
    Ok(Json(attempt))
}

async fn api_finalize_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<FinalizeCommand>,
) -> Result<Json<FinalizationPreview>, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = execution_basis(&service, &command.basis)?;
    let attempt = store.attempt(&command.execution, &command.attempt_id)?;
    if attempt.plan_digest != command.execution.plan_digest
        || attempt.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("attempt does not match execution identity"),
        ));
    }
    Ok(Json(store.finalization_preview_while_locked(
        &snapshot.workspace,
        &attempt,
    )?))
}

async fn api_finalize_apply(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<FinalizeCommand>,
) -> Result<Json<FinalizationReceipt>, ApiError> {
    let token = command.preview_token.as_deref().ok_or_else(|| {
        ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("preview_token is required"),
        )
    })?;
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = execution_basis(&service, &command.basis)?;
    let attempt = store.attempt(&command.execution, &command.attempt_id)?;
    if attempt.plan_digest != command.execution.plan_digest
        || attempt.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("attempt does not match execution identity"),
        ));
    }
    let preview = store.finalization_preview_while_locked(&snapshot.workspace, &attempt)?;
    Ok(Json(store.apply_finalization_while_locked(
        &snapshot.workspace,
        &attempt,
        &preview,
        token,
    )?))
}

async fn api_result(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ResultCommand>,
) -> Result<StatusCode, ApiError> {
    let store = DeliveryStore::for_workspace(&service.workspace_root)?;
    let _workspace_lock = store.lock_workspace()?;
    let snapshot = execution_basis(&service, &command.basis)?;
    let attempt = store.attempt(&command.execution, &command.attempt_id)?;
    let plan = store.approval(&command.execution)?.plan;
    let canonical = attempt.receipt.clone().ok_or_else(|| {
        anyhow::anyhow!("completion attempt has no successful verification receipt")
    })?;
    if attempt.attempt_id != command.attempt_id
        || attempt.report.attempt_id != command.attempt_id
        || attempt.report.status != CompletionStatus::Complete
        || attempt.report.receipt_digest.as_deref()
            != Some(DeliveryStore::verification_digest(&canonical)?.as_str())
        || command.execution.plan_digest != plan.canonical_digest
        || command.execution.slice_id != canonical.slice_id
        || command.receipt != canonical
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "result must reference the exact complete durable verification attempt"
            ),
        ));
    }
    validate_verification_receipt(
        &snapshot.workspace,
        &snapshot.index,
        &plan,
        &canonical.slice_id,
        &canonical,
        &plan.basis.revision,
    )?;
    let changed_files = mitase_validation::changed_files_against_revision(
        &snapshot.workspace.root,
        &plan.basis.revision,
    )?;
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == canonical.slice_id);
    if slice.is_none() {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("receipt slice is not present in the canonical plan"),
        ));
    }
    let result = validate(&ValidationContext {
        config: &snapshot.workspace.config,
        workspace: &snapshot.workspace,
        index: &snapshot.index,
        // Result validation must inspect the real diff from the plan basis.
        // This is what rejects unrelated files and readonly changes even when
        // verification itself succeeded.
        changed_files: Some(&changed_files),
        reported_changed_files: None,
        work_plan: Some(&plan),
        selected_slice: slice,
        plan_mode: PlanValidationMode::PostState,
        preset: snapshot.workspace.config.validation.preset,
        revision: Some(&snapshot.revision),
        change_base_revision: Some(&plan.basis.revision),
    });
    let view = ValidationRunView::completed(
        "work-result",
        Some(plan.canonical_digest.clone()),
        result.clone(),
        false,
        true,
        snapshot.workspace.config.validation.preset,
        SystemTime::now(),
    );
    service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .last_validation = Some(view);
    if !result.is_valid() {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("post-state result validation failed"),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn api_discard(State(service): State<Arc<WorkbenchService>>) -> Result<StatusCode, ApiError> {
    abandon_active_agent_runs(&service, "work request was cancelled or restarted")?;
    *service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))? = WorkbenchSession::default();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjection {
    pub snapshot: WorkspaceSummary,
    pub navigation: NavigationView,
    pub journey: WorkJourneyView,
    pub capabilities: Vec<ActionCapabilityView>,
    pub work: WorkSessionView,
    pub readiness: ReadinessView,
    pub scope: ScopeView,
    pub specifications: SpecificationCatalogView,
    pub diagnostics: DiagnosticsView,
}
pub type WorkbenchProjection = WorkspaceProjection;

/// A presentation-safe view of the work lifecycle.  It deliberately contains
/// no repository paths, selectors, opaque IDs, or commands; those belong in
/// `advanced` and are revealed only by an explicit user choice.
#[derive(Debug, Clone, Serialize)]
pub struct WorkJourneyView {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_key: Option<String>,
    pub current_step: String,
    pub steps: Vec<JourneyStepView>,
    pub primary_action: JourneyActionView,
    pub recovery_action: Option<JourneyActionView>,
    pub approved_scope: Option<JourneyScopeView>,
    pub evidence: JourneyEvidenceView,
    pub related_specification: Option<JourneySpecificationView>,
    pub advanced: JourneyAdvancedView,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneySpecificationView {
    pub title: String,
    pub overview: String,
    pub status: Option<String>,
    pub criterion_statement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneyStepView {
    pub id: String,
    pub status: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneyActionView {
    pub action: String,
    pub label: String,
    pub label_key: String,
    pub explanation: String,
    pub explanation_key: String,
    pub confirmation_required: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneyScopeView {
    pub summary: String,
    pub status: String,
    pub editable_target_count: usize,
    pub slice_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneyEvidenceView {
    pub status: String,
    pub summary: String,
    pub blockers: Vec<JourneyBlockerView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JourneyBlockerView {
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JourneyAdvancedView {
    pub request_id: Option<String>,
    pub plan_id: Option<String>,
    pub selected_slice_id: Option<String>,
    pub attempt_id: Option<String>,
    pub specification_anchor: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct NavigationView {
    pub selected_page: WorkbenchPage,
    pub pages: Vec<WorkbenchPage>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ActionCapabilityView {
    pub id: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkSessionView {
    pub request: Option<WorkRequestView>,
    pub plan: Option<PlanView>,
    pub verification_receipt: Option<VerificationReceiptView>,
    pub completion: CompletionHistoryView,
    pub agent: Option<AgentRun>,
    pub agent_events: Vec<AgentEvent>,
    pub context_pack: Option<ContextPackView>,
    pub selected_slice: Option<String>,
    pub validation: ValidationRunView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_recovery: Option<SplitRecoveryView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitRecoveryView {
    pub schema: String,
    pub candidate_plan_digest: String,
    pub criterion: SplitCriterionView,
    pub reason: SplitReasonView,
    pub candidates: Vec<SplitRecoveryCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitCriterionView {
    pub anchor: SpecAnchor,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitReasonView {
    pub code: String,
    pub message: String,
    pub planner_basis_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitRecoveryCandidate {
    pub id: String,
    pub selectable: bool,
    pub goal: String,
    pub anchors: Vec<SpecAnchor>,
    pub editable_targets: Vec<PlannedTargetView>,
    pub verification_targets: Vec<PlannedTargetView>,
    pub readonly_context: Vec<PlannedTargetView>,
    pub acceptance: Vec<mitase_work_model::AcceptanceRef>,
    pub contracts: Vec<ContractRefView>,
    pub origin_closure: mitase_work_model::OriginClosure,
    pub origin_closure_digest: String,
    pub budget: mitase_work_model::SliceBudgetUsage,
    pub confidence: mitase_work_model::PlanConfidence,
    pub blockers: Vec<DiagnosticView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedTargetView {
    pub reference: BoundTargetRef,
    pub access: TargetAccessMode,
    pub transition: TargetTransition,
    pub lifecycle: mitase_work_model::TargetLifecycle,
    pub path: String,
    pub selector: mitase_work_model::ResolvedSelector,
    pub artifact_identity: Option<String>,
    pub adapter: String,
    pub facet: String,
    pub role: String,
    pub verification_claim: Option<mitase_work_model::VerificationClaimRef>,
    pub content_hash: String,
    pub excerpt_hash: String,
    pub line_start: usize,
    pub line_end: usize,
    pub budget_bytes: usize,
    pub budget_lines: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractRefView {
    pub anchor: SpecAnchor,
    pub kind: String,
    pub source: BoundTargetRef,
    pub participants: Vec<BoundTargetRef>,
    pub guarantees: Vec<SpecAnchor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticView {
    pub rule_id: String,
    pub phase: String,
    pub severity: String,
    pub message: String,
    pub primary: mitase_diagnostics::Location,
    pub related: Vec<mitase_diagnostics::RelatedLocation>,
    pub anchor: Option<SpecAnchor>,
    pub target: Option<BoundTargetRef>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CompletionHistoryView {
    pub current: Option<CompletionAttemptView>,
    pub previous: Vec<CompletionAttemptView>,
}

impl CompletionHistoryView {
    fn current_for(&self, plan_digest: &str, slice_id: &str) -> Option<&CompletionAttemptView> {
        self.current
            .iter()
            .chain(self.previous.iter())
            .find(|attempt| attempt.plan_digest == plan_digest && attempt.slice_id == slice_id)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CompletionAttemptView {
    pub attempt_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub status: CompletionStatus,
    pub demonstrated: Vec<String>,
    pub blockers: Vec<mitase_work_model::CompletionBlocker>,
    pub next_action: Option<String>,
    pub finalized: bool,
    pub stale: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkRequestView {
    pub title: String,
    pub origin: WorkOrigin,
    pub operation: WorkOperation,
    pub requested_target_count: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanView {
    pub id: String,
    pub digest: String,
    pub status: PlanStatus,
    pub slices: Vec<SliceView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SliceView {
    pub id: String,
    pub editable_targets: Vec<TargetView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TargetView {
    pub reference: String,
    pub access: TargetAccessMode,
    pub transition: TargetTransition,
    pub path: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct VerificationReceiptView {
    pub slice_id: String,
    pub status: String,
    pub revision: String,
    pub workspace_fingerprint: String,
    pub started_at: String,
    pub completed_at: String,
    pub executions: Vec<VerificationExecutionView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ContextPackView {
    pub plan_digest: String,
    pub slice_id: String,
    pub entry_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessView {
    pub target: ReadinessLevel,
    pub status: String,
    pub blocking_subjects: usize,
    pub axes: BTreeMap<String, mitase_validation::ReadinessAxis>,
    pub blockers: Vec<String>,
    pub execution_state: String,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScopeView {
    pub branch: Option<BranchScopeView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SpecificationCatalogView {
    pub specifications: Vec<ItemSummary>,
    pub documents: Vec<SpecificationDocumentView>,
    pub origin_capabilities: Vec<OriginCapabilityView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginCapabilityView {
    pub schema: String,
    pub origin: Option<WorkOrigin>,
    pub label: String,
    pub enabled: bool,
    pub disabled_code: Option<String>,
    pub disabled_message: Option<String>,
    pub nearest: Vec<WorkOrigin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecificationDocumentView {
    pub kind: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsView {
    pub validation: ValidationRunView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRunState {
    NotRun,
    Running,
    Passed,
    Issues,
    Failed,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRunView {
    pub state: ValidationRunState,
    pub context: String,
    pub basis: Option<String>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub duration_ms: Option<u64>,
    pub evaluated_rule_count: usize,
    pub issue_counts: IssueCounts,
    pub applicable_phase_count: usize,
    pub skipped_phase_count: usize,
    pub phases: Vec<ValidationPhaseView>,
    pub diagnostics: Vec<ValidationDiagnosticView>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationPhaseView {
    pub id: String,
    pub state: ValidationRunState,
    pub issue_count: usize,
    pub evaluated_rules: usize,
    pub issue_counts: IssueCounts,
    pub not_applicable_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueCounts {
    pub error: usize,
    pub warning: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDiagnosticView {
    #[serde(flatten)]
    pub diagnostic: mitase_diagnostics::Diagnostic,
}

impl ValidationRunView {
    pub fn not_run() -> Self {
        Self {
            state: ValidationRunState::NotRun,
            context: "workspace".into(),
            basis: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            evaluated_rule_count: 0,
            issue_counts: IssueCounts::default(),
            applicable_phase_count: 0,
            skipped_phase_count: 5,
            phases: ["config", "graph", "targets", "scope", "plan"]
                .into_iter()
                .map(|id| ValidationPhaseView {
                    id: id.into(),
                    state: ValidationRunState::NotRun,
                    issue_count: 0,
                    evaluated_rules: 0,
                    issue_counts: IssueCounts::default(),
                    not_applicable_reason: None,
                })
                .collect(),
            diagnostics: vec![],
            reason: None,
        }
    }

    pub fn not_applicable(context: impl Into<String>, reason: impl Into<String>) -> Self {
        let context = context.into();
        let reason = reason.into();
        Self {
            state: ValidationRunState::NotApplicable,
            context,
            basis: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            evaluated_rule_count: 0,
            issue_counts: IssueCounts::default(),
            applicable_phase_count: 0,
            skipped_phase_count: 5,
            phases: ["config", "graph", "targets", "scope", "plan"]
                .into_iter()
                .map(|id| ValidationPhaseView {
                    id: id.into(),
                    state: ValidationRunState::NotApplicable,
                    issue_count: 0,
                    evaluated_rules: 0,
                    issue_counts: IssueCounts::default(),
                    not_applicable_reason: Some(reason.clone()),
                })
                .collect(),
            diagnostics: vec![],
            reason: Some(reason),
        }
    }

    pub fn failed(
        context: impl Into<String>,
        reason: impl Into<String>,
        started_at: SystemTime,
    ) -> Self {
        let mut run = Self::not_applicable(context, reason);
        run.state = ValidationRunState::Failed;
        run.started_at = epoch_ms(started_at);
        run.completed_at = epoch_ms(SystemTime::now());
        for phase in &mut run.phases {
            phase.state = ValidationRunState::Failed;
        }
        run
    }

    pub fn completed(
        context: impl Into<String>,
        basis: Option<String>,
        result: ValidationResult,
        has_changes: bool,
        has_plan: bool,
        preset: ValidationPreset,
        started_at: SystemTime,
    ) -> Self {
        let context = context.into();
        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|diagnostic| ValidationDiagnosticView { diagnostic })
            .collect::<Vec<_>>();
        let phases = phase_views(&diagnostics, has_changes, has_plan, preset);
        let applicable_phase_count = phases
            .iter()
            .filter(|p| !matches!(p.state, ValidationRunState::NotApplicable))
            .count();
        let completed_at = SystemTime::now();
        Self {
            state: if diagnostics.is_empty() {
                ValidationRunState::Passed
            } else {
                ValidationRunState::Issues
            },
            context,
            basis,
            started_at: epoch_ms(started_at),
            completed_at: epoch_ms(completed_at),
            duration_ms: completed_at
                .duration_since(started_at)
                .ok()
                .map(|d| d.as_millis() as u64),
            evaluated_rule_count: phases.iter().map(|phase| phase.evaluated_rules).sum(),
            issue_counts: issue_counts(&diagnostics),
            applicable_phase_count,
            skipped_phase_count: phases.len() - applicable_phase_count,
            phases,
            diagnostics,
            reason: None,
        }
    }
}

fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn phase_views(
    diagnostics: &[ValidationDiagnosticView],
    has_changes: bool,
    has_plan: bool,
    preset: ValidationPreset,
) -> Vec<ValidationPhaseView> {
    [
        ("config", true, None),
        ("graph", true, None),
        ("targets", true, None),
        (
            "scope",
            has_changes,
            Some("No changed-file range was selected"),
        ),
        ("plan", has_plan, Some("No work plan or slice is selected")),
    ]
    .into_iter()
    .map(|(id, applicable, reason)| {
        let phase_diagnostics = diagnostics
            .iter()
            .filter(|d| validation_phase_id(d.diagnostic.phase) == id)
            .cloned()
            .collect::<Vec<_>>();
        let issue_count = phase_diagnostics.len();
        ValidationPhaseView {
            id: id.into(),
            state: if !applicable {
                ValidationRunState::NotApplicable
            } else if issue_count == 0 {
                ValidationRunState::Passed
            } else {
                ValidationRunState::Issues
            },
            issue_count,
            evaluated_rules: if applicable {
                rules_in_phase(id, preset)
            } else {
                0
            },
            issue_counts: issue_counts(&phase_diagnostics),
            not_applicable_reason: (!applicable).then(|| reason.unwrap().into()),
        }
    })
    .collect()
}

fn issue_counts(diagnostics: &[ValidationDiagnosticView]) -> IssueCounts {
    let mut counts = IssueCounts::default();
    for diagnostic in diagnostics {
        match diagnostic.diagnostic.severity {
            Severity::Error => counts.error += 1,
            Severity::Warning => counts.warning += 1,
            Severity::Info => counts.info += 1,
        }
    }
    counts
}

fn rules_in_phase(phase: &str, preset: ValidationPreset) -> usize {
    mitase_validation::RULES
        .iter()
        .filter(|rule| {
            rule.presets.contains(&preset)
                && mitase_validation::phase_for_rule(rule.id) == validation_phase_from_id(phase)
        })
        .count()
}

fn validation_phase_id(phase: ValidationPhase) -> &'static str {
    match phase {
        ValidationPhase::Config => "config",
        ValidationPhase::Graph => "graph",
        ValidationPhase::Targets => "targets",
        ValidationPhase::Scope => "scope",
        ValidationPhase::Plan => "plan",
    }
}

fn validation_phase_from_id(id: &str) -> ValidationPhase {
    match id {
        "config" => ValidationPhase::Config,
        "graph" => ValidationPhase::Graph,
        "targets" => ValidationPhase::Targets,
        "scope" => ValidationPhase::Scope,
        _ => ValidationPhase::Plan,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub root: String,
    pub revision: String,
    pub fingerprint: String,
    pub config_schema: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSummary {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub source_hash: String,
    pub title: String,
    #[serde(default)]
    pub presentation_title_key: Option<String>,
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub principles: Vec<PrincipleSummary>,
    pub rules: Vec<RuleSummary>,
    pub criteria: Vec<CriterionSummary>,
    pub bindings: Vec<BindingSummary>,
    pub contracts: Vec<ContractSummary>,
    /// Exact origin anchors exposed by the server. Item ids alone are
    /// intentionally not accepted by the browser create-work flow.
    pub anchors: Vec<String>,
    #[serde(default)]
    pub origin_capabilities: Vec<OriginCapabilityView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchScopeView {
    pub range: String,
    pub state: String,
    pub reason: Option<String>,
    pub changed: Vec<BranchChangedTargetView>,
    pub owned: Vec<BranchChangedTargetView>,
    pub unowned: Vec<BranchChangedTargetView>,
    pub affected_items: Vec<ItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchChangedTargetView {
    pub path: String,
    pub status: ChangeStatus,
    pub owners: Vec<String>,
    pub anchors: Vec<String>,
    pub artifact_identities: Vec<String>,
    pub unresolved_reason: Option<String>,
    pub plan_inclusion_reason: Option<String>,
}

impl BranchScopeView {
    pub fn not_applicable(reason: impl Into<String>) -> Self {
        Self {
            range: String::new(),
            state: "not_applicable".into(),
            reason: Some(reason.into()),
            changed: Vec::new(),
            owned: Vec::new(),
            unowned: Vec::new(),
            affected_items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleSummary {
    pub anchor: String,
    pub statement: String,
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    pub anchor: String,
    pub level: String,
    pub statement: String,
    pub governed_by: Vec<String>,
    #[serde(default)]
    pub applies_to_roles: Vec<String>,
    #[serde(default)]
    pub enforcement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionSummary {
    pub anchor: String,
    pub kind: CriterionKind,
    pub statement: String,
    pub governed_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingSummary {
    pub anchor: String,
    pub role: String,
    pub facet: String,
    pub responsibility: String,
    pub owns: Vec<OwnershipScope>,
    pub targets: Vec<BindingTargetSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingTargetSummary {
    pub reference: String,
    pub path: String,
    pub selector: Selector,
    pub adapter: String,
    pub lifecycle: ArtifactTargetLifecycle,
    pub claims: Vec<TargetClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSummary {
    pub anchor: String,
    pub kind: String,
    pub source: String,
    pub participants: Vec<ContractParticipantSummary>,
    pub guarantees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractParticipantSummary {
    pub binding: String,
    pub role: String,
}

pub fn project(
    workspace: &SpecWorkspace,
    request: Option<&WorkRequest>,
    revision: &str,
) -> Result<WorkspaceProjection> {
    let index = workspace.index()?;
    if let Some(request) = request {
        validate_work_request(&index, request)
            .context("workbench projection request is outside its exact origin")?;
    }
    project_with_index(workspace, &index, request, revision)
}

fn project_with_index(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    request: Option<&WorkRequest>,
    revision: &str,
) -> Result<WorkspaceProjection> {
    // Projection loading is intentionally side-effect free and lightweight.
    // In particular, a checked-in `work.yaml` is not an implicit Workbench
    // session and must not cause planning during the first GET.
    let requested_work = request.cloned();
    let mut specifications = Vec::new();
    for loaded in &workspace.documents {
        let path = relative_display(&workspace.root, &loaded.path);
        let source_hash = content_hash(&workspace.read_to_string(&loaded.path).unwrap_or_default());
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                for item in philosophies {
                    specifications.push(item_summary_from_philosophy(
                        item,
                        &path,
                        &source_hash,
                        index,
                        &workspace.root,
                    ));
                }
            }
            SpecDocument::Policies { policies, .. } => {
                for item in policies {
                    specifications.push(item_summary_from_policy(
                        item,
                        &path,
                        &source_hash,
                        index,
                        &workspace.root,
                    ));
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    specifications.push(item_summary_from_requirement(
                        item,
                        &path,
                        &source_hash,
                        index,
                        &workspace.root,
                    ));
                }
            }
            SpecDocument::Features { features, .. } => {
                for item in features {
                    specifications.push(item_summary_from_feature(
                        item,
                        &path,
                        &source_hash,
                        index,
                        &workspace.root,
                    ));
                }
            }
        }
    }
    // A WorkRequest is a draft until the explicit Plan action. Keeping the
    // projection side-effect free also makes the browser flow observable:
    // request creation and canonical planning are separate transitions.
    let plan: Option<WorkPlan> = None;
    let validation = ValidationRunView::not_run();
    let readiness = readiness_not_run(&workspace.config);
    let origin_capabilities = origin_capabilities(workspace, index, &specifications);
    for item in &mut specifications {
        item.origin_capabilities = origin_capabilities
            .iter()
            .filter(|capability| {
                capability
                    .origin
                    .as_ref()
                    .or_else(|| capability.nearest.first())
                    .is_some_and(|origin| origin_owner_item(origin).to_string() == item.id)
            })
            .cloned()
            .collect();
    }
    Ok(WorkspaceProjection {
        snapshot: WorkspaceSummary {
            root: workspace
                .root
                .canonicalize()
                .unwrap_or_else(|_| workspace.root.clone())
                .display()
                .to_string(),
            revision: revision.to_string(),
            fingerprint: workspace.try_fingerprint()?,
            config_schema: workspace.config.schema.clone(),
            source_hash: workspace_source_hash(workspace),
        },
        navigation: NavigationView {
            selected_page: WorkbenchPage::Work,
            pages: vec![
                WorkbenchPage::Work,
                WorkbenchPage::Readiness,
                WorkbenchPage::Scope,
                WorkbenchPage::Specifications,
                WorkbenchPage::Diagnostics,
                WorkbenchPage::Settings,
            ],
        },
        journey: empty_journey(),
        capabilities: vec![
            ActionCapabilityView {
                id: "work.plan".into(),
                enabled: requested_work.is_some(),
                disabled_reason: requested_work
                    .is_none()
                    .then(|| "Select a WorkRequest before planning.".into()),
            },
            ActionCapabilityView {
                id: "work.verify".into(),
                enabled: plan.as_ref().is_some_and(|plan| {
                    matches!(plan.status, mitase_work_model::PlanStatus::Ready)
                        && !plan.slices.is_empty()
                }),
                disabled_reason: plan
                    .as_ref()
                    .is_none_or(|plan| {
                        !matches!(plan.status, mitase_work_model::PlanStatus::Ready)
                            || plan.slices.is_empty()
                    })
                    .then(|| "Validate the selected plan before verification.".into()),
            },
        ],
        work: WorkSessionView {
            request: requested_work.as_ref().map(request_view),
            plan: plan.as_ref().map(plan_view),
            verification_receipt: None,
            completion: completion_history(workspace)?,
            agent: None,
            agent_events: vec![],
            context_pack: None,
            selected_slice: None,
            validation: validation.clone(),
            split_recovery: None,
        },
        readiness,
        scope: ScopeView::default(),
        specifications: SpecificationCatalogView {
            specifications,
            documents: workspace
                .documents
                .iter()
                .map(|document| SpecificationDocumentView {
                    kind: match &document.document {
                        mitase_spec_model::SpecDocument::Philosophies { .. } => "philosophy",
                        mitase_spec_model::SpecDocument::Policies { .. } => "policy",
                        mitase_spec_model::SpecDocument::Requirements { .. } => "requirement",
                        mitase_spec_model::SpecDocument::Features { .. } => "feature",
                    }
                    .into(),
                    path: document
                        .path
                        .strip_prefix(&workspace.root)
                        .unwrap_or(&document.path)
                        .to_string_lossy()
                        .into_owned(),
                })
                .collect(),
            origin_capabilities,
        },
        diagnostics: DiagnosticsView { validation },
    })
}

fn empty_journey() -> WorkJourneyView {
    WorkJourneyView {
        title: "Choose a specification to create Work".into(),
        title_key: Some("journey.title.select_specification".into()),
        current_step: "select_specification".into(),
        steps: journey_steps("select_specification"),
        primary_action: JourneyActionView {
            action: "choose_specification".into(),
            label: "Open Specifications".into(),
            label_key: "journey.action.choose_specification".into(),
            explanation: "Choose a target from Specifications before creating Work.".into(),
            explanation_key: "journey.explanation.choose_specification".into(),
            confirmation_required: false,
            enabled: true,
        },
        recovery_action: None,
        approved_scope: None,
        evidence: JourneyEvidenceView {
            status: "not_started".into(),
            summary: "No work has been started yet.".into(),
            blockers: vec![],
        },
        related_specification: None,
        advanced: JourneyAdvancedView::default(),
    }
}

fn journey_steps(current: &str) -> Vec<JourneyStepView> {
    let ids = [
        ("select_specification", "Select specification"),
        ("review", "Review"),
        ("approve", "Approve"),
        ("implement", "Implement"),
        ("verify", "Check"),
        ("complete", "Complete"),
    ];
    let current_index = ids.iter().position(|(id, _)| *id == current).unwrap_or(0);
    ids.iter()
        .enumerate()
        .map(|(index, (id, title))| JourneyStepView {
            id: (*id).into(),
            title: (*title).into(),
            status: if index < current_index {
                "complete"
            } else if index == current_index {
                "current"
            } else {
                "upcoming"
            }
            .into(),
        })
        .collect()
}

fn request_view(request: &WorkRequest) -> WorkRequestView {
    WorkRequestView {
        title: request.title.clone(),
        origin: request.origin.clone(),
        operation: request.operation,
        requested_target_count: request.requested_targets.len(),
    }
}

fn target_view(target: &mitase_work_model::PlannedTarget) -> TargetView {
    TargetView {
        reference: target.reference.to_string(),
        access: target.access,
        transition: target.transition,
        path: target.resolved_path.clone(),
    }
}

fn plan_view(plan: &WorkPlan) -> PlanView {
    PlanView {
        id: plan.id.clone(),
        digest: plan.canonical_digest.clone(),
        status: plan.status,
        slices: plan
            .slices
            .iter()
            .map(|slice| SliceView {
                id: slice.id.clone(),
                editable_targets: slice.editable_targets.iter().map(target_view).collect(),
            })
            .collect(),
    }
}

fn split_recovery_view(
    plan: &WorkPlan,
    request: &WorkRequest,
    config: &mitase_project_model::ProjectConfig,
    index: &SpecIndex,
) -> SplitRecoveryView {
    let criterion = request.origin.criterion().clone();
    let statement = index
        .anchors
        .get(&criterion)
        .and_then(|value| match value {
            mitase_workspace::AnchorValue::Criterion(value) => Some(value.statement.clone()),
            _ => None,
        })
        .unwrap_or_else(|| criterion.to_string());
    let reason = split_reason(plan, config);
    SplitRecoveryView {
        schema: mitase_work_model::WORK_SPLIT_RECOVERY_SCHEMA.into(),
        candidate_plan_digest: plan.canonical_digest.clone(),
        criterion: SplitCriterionView {
            anchor: criterion,
            statement,
        },
        reason,
        candidates: plan
            .slices
            .iter()
            .map(|slice| {
                let blockers = split_candidate_blockers(plan, slice, request, config, index);
                let origin_closure = origin_closure_for_slice(index, slice);
                SplitRecoveryCandidate {
                    id: slice.id.clone(),
                    selectable: split_candidate_selectable(
                        plan, slice, request, config, index, &blockers,
                    ),
                    goal: slice.goal.clone(),
                    anchors: slice.anchors.clone(),
                    editable_targets: slice
                        .editable_targets
                        .iter()
                        .map(planned_target_view)
                        .collect(),
                    verification_targets: slice
                        .verification_targets
                        .iter()
                        .map(planned_target_view)
                        .collect(),
                    readonly_context: slice
                        .readonly_context
                        .iter()
                        .map(planned_target_view)
                        .collect(),
                    acceptance: slice.acceptance.clone(),
                    contracts: slice
                        .contracts
                        .iter()
                        .filter_map(|anchor| {
                            index
                                .contracts
                                .get(anchor)
                                .map(|contract| contract_view(anchor, contract))
                        })
                        .collect(),
                    origin_closure_digest: origin_closure_digest(&origin_closure),
                    origin_closure,
                    budget: slice.budget.clone(),
                    confidence: slice.confidence,
                    blockers: blockers.iter().map(diagnostic_view).collect(),
                }
            })
            .collect(),
    }
}

fn split_candidate_selectable(
    plan: &WorkPlan,
    slice: &mitase_work_model::ExecutionSlice,
    request: &WorkRequest,
    config: &mitase_project_model::ProjectConfig,
    index: &SpecIndex,
    blockers: &[mitase_diagnostics::Diagnostic],
) -> bool {
    let recoverable_plan_block = plan_can_replan_candidate(plan, slice);
    let intrinsic_blockers = blockers
        .iter()
        .filter(|blocker| blocker.rule_id != "plan-needs-review")
        .count();
    (plan.status == PlanStatus::Ready || recoverable_plan_block)
        && intrinsic_blockers == 0
        && slice.confidence != mitase_work_model::PlanConfidence::Low
        && (request.operation == WorkOperation::Investigate || !slice.editable_targets.is_empty())
        && slice_budget_within_limits(slice, config)
        && slice_targets_are_active(index, slice)
        && origin_closure_is_complete(plan, request, slice, index)
        && (request.operation == WorkOperation::Investigate
            || verification_covers_editable_targets(
                index,
                slice,
                request,
                request.origin.criterion(),
            ))
}

fn split_candidate_blockers(
    plan: &WorkPlan,
    slice: &mitase_work_model::ExecutionSlice,
    request: &WorkRequest,
    config: &mitase_project_model::ProjectConfig,
    index: &SpecIndex,
) -> Vec<mitase_diagnostics::Diagnostic> {
    let mut blockers = slice.blockers.clone();
    if plan.status != PlanStatus::Ready
        && !plan_can_replan_candidate(plan, slice)
        && !blockers
            .iter()
            .any(|blocker| blocker.rule_id == "plan-needs-review")
    {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "plan-needs-review",
            "the canonical candidate plan is blocked and cannot be selected",
            "work-plan",
        ));
    }
    if slice.confidence == mitase_work_model::PlanConfidence::Low {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "low-confidence-slice",
            "the candidate requires exact server review before it can be selected",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate && slice.editable_targets.is_empty() {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "missing-editable-target",
            "the candidate has no exact editable target",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate && slice.verification_targets.is_empty() {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "missing-verification-coverage",
            "the candidate has no exact verification target for its editable boundary",
            "work-plan",
        ));
    }
    for target in slice
        .editable_targets
        .iter()
        .chain(slice.verification_targets.iter())
        .chain(slice.readonly_context.iter())
    {
        if !candidate_target_is_valid(index, target) {
            blockers.push(mitase_diagnostics::Diagnostic::error(
                "target-lifecycle",
                format!(
                    "candidate target {} has no valid exact pre-state or planned lifecycle artifact",
                    target.reference
                ),
                target.resolved_path.clone(),
            ));
        }
    }
    if !origin_closure_is_complete(plan, request, slice, index) {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "missing-origin-closure",
            "the candidate does not retain a complete active implementation, verification, readonly, and contract closure",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate
        && !verification_covers_editable_targets(index, slice, request, request.origin.criterion())
    {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "missing-verification-coverage",
            "the candidate verification targets do not cover the exact origin criterion",
            "work-plan",
        ));
    }
    let closure = &plan.origin_closure;
    if request
        .origin
        .targets()
        .iter()
        .any(|target| !closure.implementation_targets.contains(target))
    {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "missing-origin-closure",
            "the candidate does not retain the complete implementation origin closure",
            "work-plan",
        ));
    }
    if !slice_budget_within_limits(slice, config) {
        blockers.push(mitase_diagnostics::Diagnostic::error(
            "budget-exceeded",
            "the candidate exceeds a configured execution limit",
            "work-plan",
        ));
    }
    blockers
}

fn plan_can_replan_candidate(plan: &WorkPlan, slice: &mitase_work_model::ExecutionSlice) -> bool {
    plan.status != PlanStatus::Ready
        && slice.blockers.is_empty()
        && plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-WORK-003"
                && diagnostic.message.to_ascii_lowercase().contains("slices")
        })
}

fn active_exact_target(index: &SpecIndex, reference: &BoundTargetRef) -> bool {
    index.target(reference).is_some_and(|target| {
        !matches!(
            target.lifecycle,
            mitase_spec_model::ArtifactTargetLifecycle::Absent
        )
    })
}

fn slice_targets_are_active(index: &SpecIndex, slice: &mitase_work_model::ExecutionSlice) -> bool {
    slice
        .editable_targets
        .iter()
        .chain(slice.verification_targets.iter())
        .chain(slice.readonly_context.iter())
        .all(|target| candidate_target_is_valid(index, target))
}

fn candidate_target_is_valid(index: &SpecIndex, target: &mitase_work_model::PlannedTarget) -> bool {
    match target.lifecycle {
        mitase_work_model::TargetLifecycle::Stable => active_exact_target(index, &target.reference),
        mitase_work_model::TargetLifecycle::EnsurePresent => {
            active_exact_target(index, &target.reference)
                || (target.transition == TargetTransition::Add
                    && index.target(&target.reference).is_some())
        }
        mitase_work_model::TargetLifecycle::EnsureAbsent => {
            target.transition == TargetTransition::Remove
                && index.all_target_to_artifact.contains_key(&target.reference)
        }
    }
}

fn verification_covers_editable_targets(
    index: &SpecIndex,
    slice: &mitase_work_model::ExecutionSlice,
    request: &WorkRequest,
    criterion: &SpecAnchor,
) -> bool {
    let valid_verifications = slice
        .verification_targets
        .iter()
        .filter(|target| verification_target_is_valid(index, target, request, criterion))
        .collect::<Vec<_>>();
    !slice.editable_targets.is_empty()
        && slice.editable_targets.iter().all(|editable| {
            if explicitly_requested_verification_add(index, request, &editable.reference) {
                return true;
            }
            valid_verifications.iter().any(|verification| {
                let covered = if explicitly_requested_verification_add(
                    index,
                    request,
                    &verification.reference,
                ) {
                    index.all_verification_by_target.get(&editable.reference)
                } else {
                    index.verification_by_target.get(&editable.reference)
                };
                covered.is_some_and(|covered| covered.contains(&verification.reference))
            })
        })
}

fn explicitly_requested_verification_add(
    index: &SpecIndex,
    request: &WorkRequest,
    reference: &BoundTargetRef,
) -> bool {
    request.requested_targets.iter().any(|requested| {
        requested.reference() == reference
            && requested.transition(work_default_transition(request.operation))
                == TargetTransition::Add
            && index
                .bindings
                .get(&reference.binding)
                .is_some_and(|binding| binding.role == BindingRole::Verification)
    })
}

fn work_default_transition(operation: WorkOperation) -> TargetTransition {
    match operation {
        WorkOperation::Add => TargetTransition::Add,
        WorkOperation::Remove => TargetTransition::Remove,
        WorkOperation::Modify
        | WorkOperation::Refactor
        | WorkOperation::Document
        | WorkOperation::Investigate => TargetTransition::Modify,
    }
}

fn verification_target_is_valid(
    index: &SpecIndex,
    target: &mitase_work_model::PlannedTarget,
    request: &WorkRequest,
    criterion: &SpecAnchor,
) -> bool {
    let Some(claim) = target.verification_claim.as_ref() else {
        return false;
    };
    if claim.criterion != *criterion || claim.target != target.reference {
        return false;
    }
    let Some(binding) = index.bindings.get(&target.reference.binding) else {
        return false;
    };
    if binding.role != BindingRole::Verification {
        return false;
    }
    let planned_add = explicitly_requested_verification_add(index, request, &target.reference);
    if !planned_add
        && index.item_status.get(&target.reference.binding.item) != Some(&ItemStatus::Implemented)
    {
        return false;
    }
    let Some(artifact) = index.target(&target.reference) else {
        return false;
    };
    if !planned_add
        && matches!(
            artifact.lifecycle,
            mitase_spec_model::ArtifactTargetLifecycle::Absent
        )
    {
        return false;
    }
    let matching = artifact
        .claims
        .iter()
        .filter_map(|entry| match entry {
            TargetClaim::Verifies {
                criterion: actual,
                covers,
                ..
            } if actual == criterion && !covers.is_empty() => Some(covers),
            _ => None,
        })
        .count();
    matching == 1
}

fn origin_closure_is_complete(
    plan: &WorkPlan,
    request: &WorkRequest,
    slice: &mitase_work_model::ExecutionSlice,
    index: &SpecIndex,
) -> bool {
    let candidate_closure = mitase_planner::origin_closure_for_slice(index, slice);
    let slice_implementation = slice
        .editable_targets
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let slice_verification = slice
        .verification_targets
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let slice_readonly = slice
        .readonly_context
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let slice_contracts = slice.contracts.iter().cloned().collect::<BTreeSet<_>>();
    if candidate_closure
        .implementation_targets
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        != slice_implementation
        || candidate_closure
            .verification_targets
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != slice_verification
        || candidate_closure
            .readonly_targets
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != slice_readonly
        || candidate_closure
            .contracts
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != slice_contracts
    {
        return false;
    }
    let closure = &plan.origin_closure;
    if request
        .origin
        .targets()
        .iter()
        .any(|target| !closure.implementation_targets.contains(target))
    {
        return false;
    }
    let planned_targets = slice
        .editable_targets
        .iter()
        .chain(slice.verification_targets.iter())
        .chain(slice.readonly_context.iter())
        .map(|target| (&target.reference, target))
        .collect::<BTreeMap<_, _>>();
    if closure
        .implementation_targets
        .iter()
        .chain(closure.verification_targets.iter())
        .chain(closure.readonly_targets.iter())
        .any(|target| {
            !planned_targets
                .get(target)
                .is_some_and(|planned| candidate_target_is_valid(index, planned))
                && !active_exact_target(index, target)
        })
    {
        return false;
    }
    if slice
        .editable_targets
        .iter()
        .any(|target| !closure.implementation_targets.contains(&target.reference))
        || slice
            .verification_targets
            .iter()
            .any(|target| !closure.verification_targets.contains(&target.reference))
        || slice
            .readonly_context
            .iter()
            .any(|target| !closure.readonly_targets.contains(&target.reference))
    {
        return false;
    }
    let slice_targets = slice
        .editable_targets
        .iter()
        .chain(slice.verification_targets.iter())
        .chain(slice.readonly_context.iter())
        .map(|target| &target.reference)
        .collect::<BTreeSet<_>>();
    closure
        .contracts
        .iter()
        .filter(|anchor| {
            index.contracts.get(*anchor).is_some_and(|contract| {
                contract.guarantees.contains(request.origin.criterion())
                    && std::iter::once(&contract.source)
                        .chain(
                            contract
                                .participants
                                .iter()
                                .map(|participant| &participant.target),
                        )
                        .any(|target| slice_targets.contains(target))
            })
        })
        .all(|anchor| {
            let Some(contract) = index.contracts.get(anchor) else {
                return false;
            };
            if contract.guarantees.is_empty()
                || !contract.guarantees.contains(request.origin.criterion())
            {
                return false;
            }
            let mut related = std::iter::once(&contract.source).chain(
                contract
                    .participants
                    .iter()
                    .map(|participant| &participant.target),
            );
            related.all(|target| {
                closure.readonly_targets.contains(target) && active_exact_target(index, target)
            })
        })
}

fn split_reason(plan: &WorkPlan, config: &mitase_project_model::ProjectConfig) -> SplitReasonView {
    let budget = plan.slices.iter().fold(
        mitase_work_model::SliceBudgetUsage::default(),
        |mut total, slice| {
            total.editable_files += slice.budget.editable_files;
            total.editable_symbols += slice.budget.editable_symbols;
            total.verification_targets += slice.budget.verification_targets;
            total.readonly_targets += slice.budget.readonly_targets;
            total.total_bytes += slice.budget.total_bytes;
            total
        },
    );
    let limits = &config.work.slicing;
    let (code, message) = if plan.slices.len() > 1 {
        (
            "independent-target-components",
            "The Requirement criterion expands into independent executable target components.",
        )
    } else if budget.editable_files > limits.max_editable_files
        || budget.editable_symbols > limits.max_editable_symbols
    {
        (
            "editable-budget",
            "The candidate exceeds the configured editable target budget.",
        )
    } else if budget.verification_targets > limits.max_verification_targets {
        (
            "verification-budget",
            "The candidate exceeds the configured verification target budget.",
        )
    } else if budget.readonly_targets > limits.max_readonly_targets {
        (
            "readonly-budget",
            "The candidate exceeds the configured readonly context budget.",
        )
    } else if budget.total_bytes > limits.max_total_bytes {
        (
            "total-byte-budget",
            "The candidate exceeds the configured total byte budget.",
        )
    } else {
        let message = plan
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.as_str())
            .or_else(|| {
                plan.slices
                    .iter()
                    .flat_map(|slice| slice.blockers.iter())
                    .next()
                    .map(|diagnostic| diagnostic.message.as_str())
            })
            .unwrap_or("The candidate exceeds a configured execution limit.");
        let lower = message.to_ascii_lowercase();
        let code = if lower.contains("verification") {
            "verification-coverage"
        } else if lower.contains("readonly") {
            "readonly-budget"
        } else if lower.contains("byte") {
            "total-byte-budget"
        } else {
            "editable-budget"
        };
        (code, message)
    };
    SplitReasonView {
        code: code.into(),
        message: message.into(),
        planner_basis_digest: plan.canonical_digest.clone(),
    }
}

fn slice_budget_within_limits(
    slice: &mitase_work_model::ExecutionSlice,
    config: &mitase_project_model::ProjectConfig,
) -> bool {
    let limits = &config.work.slicing;
    slice.budget.editable_files <= limits.max_editable_files
        && slice.budget.editable_symbols <= limits.max_editable_symbols
        && slice.budget.verification_targets <= limits.max_verification_targets
        && slice.budget.readonly_targets <= limits.max_readonly_targets
        && slice.budget.total_bytes <= limits.max_total_bytes
}

fn planned_target_view(target: &mitase_work_model::PlannedTarget) -> PlannedTargetView {
    PlannedTargetView {
        reference: target.reference.clone(),
        access: target.access,
        transition: target.transition,
        lifecycle: target.lifecycle,
        path: target.resolved_path.clone(),
        selector: target.resolved_selector.clone(),
        artifact_identity: target.artifact_identity.clone(),
        adapter: target.adapter.clone(),
        facet: target.facet.clone(),
        role: format!("{:?}", target.role).to_ascii_lowercase(),
        verification_claim: target.verification_claim.clone(),
        content_hash: target.content_hash.clone(),
        excerpt_hash: target.excerpt_hash.clone(),
        line_start: target.line_start,
        line_end: target.line_end,
        budget_bytes: target.budget_bytes,
        budget_lines: target.budget_lines,
    }
}

fn contract_view(anchor: &SpecAnchor, contract: &Contract) -> ContractRefView {
    ContractRefView {
        anchor: anchor.clone(),
        kind: format!("{:?}", contract.kind).to_ascii_lowercase(),
        source: contract.source.clone(),
        participants: contract
            .participants
            .iter()
            .map(|participant| participant.target.clone())
            .collect(),
        guarantees: contract.guarantees.clone(),
    }
}

fn diagnostic_view(diagnostic: &mitase_diagnostics::Diagnostic) -> DiagnosticView {
    DiagnosticView {
        rule_id: diagnostic.rule_id.clone(),
        phase: format!("{:?}", diagnostic.phase).to_ascii_lowercase(),
        severity: format!("{:?}", diagnostic.severity).to_ascii_lowercase(),
        message: diagnostic.message.clone(),
        primary: diagnostic.primary.clone(),
        related: diagnostic.related.clone(),
        anchor: diagnostic.anchor.clone(),
        target: diagnostic.target.clone(),
    }
}

fn verification_receipt_view(receipt: &VerificationReceipt) -> VerificationReceiptView {
    VerificationReceiptView {
        slice_id: receipt.slice_id.clone(),
        status: if receipt
            .executions
            .iter()
            .all(|execution| execution.exit_code == 0)
        {
            "passed".into()
        } else {
            "failed".into()
        },
        revision: receipt.revision.clone(),
        workspace_fingerprint: receipt.workspace_fingerprint.clone(),
        started_at: receipt.started_at.clone(),
        completed_at: receipt.completed_at.clone(),
        executions: receipt
            .executions
            .iter()
            .enumerate()
            .map(|(index, execution)| VerificationExecutionView {
                identity: format!("{}#execution-{index}", receipt.slice_id),
                target: execution.target.to_string(),
                claim: execution.claim.as_ref().map(|claim| VerificationClaimView {
                    target: claim.target.to_string(),
                    criterion: claim.criterion.to_string(),
                }),
                status: if execution.exit_code == 0 {
                    "passed".into()
                } else {
                    "failed".into()
                },
            })
            .collect(),
    }
}

fn completion_history(workspace: &SpecWorkspace) -> Result<CompletionHistoryView> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let attempts = store.attempts()?;
    let current_fingerprint = workspace.try_fingerprint()?;
    let current_revision = current_revision(&workspace.root)?;
    let mut views = Vec::with_capacity(attempts.len());
    for attempt in attempts {
        let next_action = attempt
            .report
            .blockers
            .first()
            .map(|blocker| blocker.next_action.clone());
        // Invalid or unreadable finalization evidence is deliberately treated
        // as not finalized in the projection. The durable store still returns
        // the error to mutation/verification callers; the UI must fail closed
        // instead of turning forged evidence into a server error.
        let finalized = store
            .finalization(
                &ExecutionIdentity {
                    plan_digest: attempt.plan_digest.clone(),
                    slice_id: attempt.slice_id.clone(),
                },
                &attempt.attempt_id,
            )
            .map(|receipt| receipt.is_some())
            .unwrap_or(false);
        let stale = store
            .approval(&ExecutionIdentity {
                plan_digest: attempt.plan_digest.clone(),
                slice_id: attempt.slice_id.clone(),
            })
            .map(|approval| {
                approval.workspace_fingerprint != current_fingerprint
                    || approval.revision != current_revision
            })
            .unwrap_or(true);
        views.push(CompletionAttemptView {
            attempt_id: attempt.attempt_id,
            plan_digest: attempt.plan_digest,
            slice_id: attempt.slice_id,
            status: attempt.report.status,
            demonstrated: attempt
                .report
                .demonstrated
                .into_iter()
                .map(|value| value.anchor.to_string())
                .collect(),
            blockers: attempt.report.blockers,
            next_action,
            finalized,
            stale,
        });
    }
    let mut iter = views.into_iter();
    Ok(CompletionHistoryView {
        current: iter.next(),
        previous: iter.collect(),
    })
}

fn context_pack_view(context: &mitase_work_model::ContextPack) -> ContextPackView {
    ContextPackView {
        plan_digest: context.plan_digest.clone(),
        slice_id: context.slice_id.clone(),
        entry_count: context.artifact_context.len(),
    }
}

fn readiness_view(report: &mitase_validation::ReadinessReport) -> ReadinessView {
    let axes = BTreeMap::from([
        ("inventory".into(), report.inventory.clone()),
        ("ownership".into(), report.ownership.clone()),
        ("seedability".into(), report.seedability.clone()),
        ("workability".into(), report.workability.clone()),
        ("verification".into(), report.verification.clone()),
        ("closed_loop".into(), report.closed_loop.clone()),
    ]);
    const TRACEABLE_AXES: &[&str] = &["inventory", "ownership"];
    const SEEDABLE_AXES: &[&str] = &["inventory", "ownership", "seedability"];
    const WORK_READY_AXES: &[&str] = &["inventory", "ownership", "seedability", "workability"];
    const VERIFIABLE_AXES: &[&str] = &[
        "inventory",
        "ownership",
        "seedability",
        "workability",
        "verification",
    ];
    const CLOSED_LOOP_AXES: &[&str] = &[
        "inventory",
        "ownership",
        "seedability",
        "workability",
        "verification",
        "closed_loop",
    ];
    let required_axes = match report.target {
        ReadinessLevel::Traceable => TRACEABLE_AXES,
        ReadinessLevel::Seedable => SEEDABLE_AXES,
        ReadinessLevel::WorkReady => WORK_READY_AXES,
        ReadinessLevel::Verifiable => VERIFIABLE_AXES,
        ReadinessLevel::ClosedLoop => CLOSED_LOOP_AXES,
        _ => &[],
    };
    let blocker_details = required_axes
        .iter()
        .filter_map(|axis| axes.get(*axis))
        .flat_map(|axis| axis.blockers.clone())
        .collect::<Vec<_>>();
    let has_subjects = required_axes
        .iter()
        .filter_map(|axis| axes.get(*axis))
        .any(|axis| axis.required > 0);
    ReadinessView {
        target: report.target,
        status: if !has_subjects {
            "Blocked".into()
        } else if blocker_details.is_empty() {
            "Ready".into()
        } else {
            "Blocked".into()
        },
        blocking_subjects: blocker_details.len(),
        axes,
        blockers: blocker_details,
        execution_state: report.execution_state.clone(),
    }
}

fn readiness_not_run(config: &mitase_project_model::ProjectConfig) -> ReadinessView {
    ReadinessView {
        target: config.validation.readiness.target,
        status: "Not run".into(),
        blocking_subjects: 0,
        axes: BTreeMap::new(),
        blockers: vec![],
        execution_state: "not-run".into(),
    }
}

fn project_session(
    snapshot: &CachedWorkspaceSnapshot,
    session: &WorkbenchSession,
) -> Result<WorkspaceProjection> {
    let workspace = &snapshot.workspace;
    let mut projection = (*snapshot.projection).clone();
    if let Some(readiness) = &session.readiness {
        projection.readiness = readiness.clone();
    }
    projection.navigation.selected_page = session.selected_page;
    projection.work.request = session.draft_request.as_ref().map(request_view);
    if let Some(capability) = projection
        .capabilities
        .iter_mut()
        .find(|capability| capability.id == "work.plan")
    {
        capability.enabled = session.draft_request.is_some();
        capability.disabled_reason = session
            .draft_request
            .is_none()
            .then(|| "Select a WorkRequest before planning.".into());
    }
    projection.work.plan = session
        .plan
        .as_ref()
        .map(plan_view)
        .or(projection.work.plan);
    projection.work.split_recovery = session
        .plan
        .as_ref()
        .zip(session.draft_request.as_ref())
        .filter(|(plan, _)| plan.slices.len() > 1 || plan.status != PlanStatus::Ready)
        .map(|(plan, request)| {
            split_recovery_view(plan, request, &workspace.config, &snapshot.index)
        });
    projection.work.verification_receipt = session
        .verification_receipt
        .as_ref()
        .map(verification_receipt_view);
    projection.work.completion = completion_history(workspace)?;
    // A session-bound run controls Workbench actions. When no work request is
    // loaded (including after a server restart), retain the latest run for evidence
    // inspection only; it must not be mistaken for the current plan's agent.
    projection.work.agent = match session
        .agent_run
        .as_ref()
        .map(|run| mitase_agent::current_run(workspace, run))
        .transpose()?
    {
        Some(run) => Some(run),
        None if session.draft_request.is_none() => {
            DeliveryStore::for_workspace(&workspace.root)?.latest_agent_run()?
        }
        None => None,
    };
    projection.work.agent_events = projection
        .work
        .agent
        .as_ref()
        .map(|run| mitase_agent::events(workspace, run))
        .transpose()?
        .unwrap_or_default();
    projection.work.context_pack = session.context_pack.as_ref().map(context_pack_view);
    projection.work.selected_slice = session.selected_slice.clone();
    projection.work.validation = session
        .last_validation
        .clone()
        .unwrap_or_else(ValidationRunView::not_run);
    projection.diagnostics.validation = projection.work.validation.clone();
    let plan_validated = session
        .last_validation
        .as_ref()
        .is_some_and(|validation| matches!(validation.state, ValidationRunState::Passed));
    let slice_selected = session.selected_slice.as_ref().is_some_and(|id| {
        session
            .plan
            .as_ref()
            .is_some_and(|plan| plan.slices.iter().any(|slice| &slice.id == id))
    });
    let verifiable = projection
        .work
        .plan
        .as_ref()
        .is_some_and(|plan| plan.status == PlanStatus::Ready)
        && plan_validated
        && slice_selected
        && session.plan.as_ref().is_some_and(|plan| {
            plan.slices
                .iter()
                .find(|slice| Some(&slice.id) == session.selected_slice.as_ref())
                .is_some_and(|slice| !slice.verification_targets.is_empty())
        });
    if let Some(capability) = projection
        .capabilities
        .iter_mut()
        .find(|capability| capability.id == "work.verify")
    {
        capability.enabled = verifiable;
        capability.disabled_reason = (!verifiable)
            .then(|| "Validate a selected verifiable slice before verification.".into());
    }
    projection.journey = journey_view(
        workspace,
        &projection.work,
        &projection.specifications.specifications,
        session,
    )?;
    Ok(projection)
}

fn journey_specification_context(
    items: &[ItemSummary],
    request: Option<&WorkRequest>,
) -> Option<(JourneySpecificationView, String)> {
    let request = request?;
    let mut criteria = std::iter::once(request.origin.criterion().clone())
        .chain(
            request
                .requested_targets
                .iter()
                .filter_map(|target| target.criterion.clone()),
        )
        .collect::<BTreeSet<_>>();
    if criteria.len() != 1 {
        return None;
    }
    let anchor = criteria.pop_first()?;
    let anchor_text = anchor.to_string();
    let item_id = anchor.item.to_string();
    let item = items.iter().find(|item| item.id == item_id)?;
    let criterion = item
        .criteria
        .iter()
        .find(|criterion| criterion.anchor == anchor_text)?;
    let overview = if item.summary.trim().is_empty() {
        item.description.clone().unwrap_or_default()
    } else {
        item.summary.clone()
    };
    Some((
        JourneySpecificationView {
            title: item.title.clone(),
            overview,
            status: item.status.clone(),
            criterion_statement: criterion.statement.clone(),
        },
        anchor_text,
    ))
}

fn journey_view(
    workspace: &SpecWorkspace,
    work: &WorkSessionView,
    items: &[ItemSummary],
    session: &WorkbenchSession,
) -> Result<WorkJourneyView> {
    if work.request.is_none() {
        if work.agent.as_ref().is_some_and(|agent| {
            matches!(
                agent.status,
                AgentRunStatus::Active | AgentRunStatus::Blocked
            )
        }) {
            return Ok(WorkJourneyView {
                title: "Interrupted implementation".into(),
                title_key: None,
                current_step: "implement".into(),
                steps: journey_steps("implement"),
                primary_action: cancel_action(),
                recovery_action: None,
                approved_scope: None,
                evidence: JourneyEvidenceView {
                    status: "recovery_required".into(),
                    summary: "A durable implementation run is still present. Cancel it to record abandonment before starting new work.".into(),
                    blockers: vec![],
                },
                related_specification: None,
                advanced: JourneyAdvancedView {
                    request_id: None,
                    plan_id: None,
                    selected_slice_id: None,
                    attempt_id: None,
                    specification_anchor: None,
                },
            });
        }
        return Ok(empty_journey());
    }
    let title = session
        .work_title
        .clone()
        .or_else(|| work.request.as_ref().map(|request| request.title.clone()))
        .expect("work title is set when a WorkRequest exists");
    let specification = journey_specification_context(items, session.draft_request.as_ref());
    let related_specification = specification.as_ref().map(|(view, _)| view.clone());
    let mut advanced = JourneyAdvancedView {
        request_id: session
            .draft_request
            .as_ref()
            .map(|request| request.id.clone()),
        plan_id: work.plan.as_ref().map(|plan| plan.id.clone()),
        selected_slice_id: work.selected_slice.clone(),
        attempt_id: None,
        specification_anchor: specification.as_ref().map(|(_, anchor)| anchor.clone()),
    };
    let Some(plan) = work.plan.as_ref() else {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "prepare".into(),
                label: "Prepare a safe plan".into(),
                label_key: "journey.action.prepare".into(),
                explanation:
                    "We will explain the proposed change and check it before asking for approval."
                        .into(),
                explanation_key: "journey.explanation.prepare".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: None,
            evidence: JourneyEvidenceView {
                status: "draft".into(),
                summary: "An exact origin has been selected; its scope has not been approved."
                    .into(),
                blockers: vec![],
            },
            related_specification,
            advanced,
        });
    };
    if plan.status == PlanStatus::Ready && plan.slices.len() > 1 {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "select_slice".into(),
                label: "Select a focused step".into(),
                label_key: "journey.action.select_slice".into(),
                explanation: "This Requirement criterion expands into independent execution slices. Select one exact slice to continue.".into(),
                explanation_key: "journey.explanation.select_slice".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(JourneyScopeView {
                summary: format!("{} focused slices are available.", plan.slices.len()),
                status: "split-required".into(),
                editable_target_count: plan.slices.iter().map(|slice| slice.editable_targets.len()).sum(),
                slice_count: plan.slices.len(),
            }),
            evidence: JourneyEvidenceView {
                status: "split_required".into(),
                summary: "Select one exact execution slice for the linked Requirement criterion.".into(),
                blockers: vec![],
            },
            related_specification,
            advanced,
        });
    }
    if plan.status != PlanStatus::Ready
        && work.split_recovery.as_ref().is_some_and(|recovery| {
            recovery
                .candidates
                .iter()
                .any(|candidate| candidate.selectable)
        })
    {
        let recovery = work
            .split_recovery
            .as_ref()
            .expect("selectable recovery has a split view");
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "select_slice".into(),
                label: "Select a focused step".into(),
                label_key: "journey.action.select_slice".into(),
                explanation:
                    "The broad candidate is blocked only by its slice limit. Select one exact step to replan it safely."
                        .into(),
                explanation_key: "journey.explanation.select_slice".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(JourneyScopeView {
                summary: format!("{} focused slices can be replanned.", recovery.candidates.len()),
                status: "split-required".into(),
                editable_target_count: recovery
                    .candidates
                    .iter()
                    .map(|candidate| candidate.editable_targets.len())
                    .sum(),
                slice_count: recovery.candidates.len(),
            }),
            evidence: JourneyEvidenceView {
                status: "split_required".into(),
                summary: recovery.reason.message.clone(),
                blockers: vec![],
            },
            related_specification,
            advanced,
        });
    }
    if plan.status != PlanStatus::Ready && work.split_recovery.is_some() {
        let reason = work
            .split_recovery
            .as_ref()
            .map(|recovery| recovery.reason.message.clone())
            .unwrap_or_else(|| "The candidate plan needs review before execution.".into());
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "choose_specification".into(),
                label: "Choose another exact criterion".into(),
                label_key: "journey.action.choose_specification".into(),
                explanation:
                    "This candidate is blocked; choose another Requirement criterion or return to its specification."
                        .into(),
                explanation_key: "journey.explanation.choose_specification".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: None,
            evidence: JourneyEvidenceView {
                status: "blocked".into(),
                summary: reason.clone(),
                blockers: vec![JourneyBlockerView {
                    message: reason,
                    next_action: "Choose another exact Requirement criterion or return to its specification."
                        .into(),
                }],
            },
            related_specification,
            advanced,
        });
    }
    if plan.status != PlanStatus::Ready || plan.slices.len() != 1 {
        let (summary, message, next_action) = if plan.slices.len() > 1 {
            (
                "The proposed change needs separate focused steps.",
                "This change needs separate focused steps.",
                "Choose one focused execution slice to change first.",
            )
        } else if plan.slices.is_empty() {
            (
                "We could not find a safe executable change for this exact origin.",
                "This Requirement criterion does not yet have a bounded implementation path.",
                "Choose a related Requirement criterion or add its implementation guidance.",
            )
        } else {
            (
                "The proposed change cannot be prepared safely yet.",
                "The change boundary needs attention before implementation can begin.",
                "Choose a smaller related Requirement criterion and review it again.",
            )
        };
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "choose_specification".into(),
                label: "Choose another exact origin".into(),
                label_key: "journey.action.choose_specification".into(),
                explanation:
                    "This candidate cannot continue safely. Choose another exact origin or return to its specification."
                        .into(),
                explanation_key: "journey.explanation.choose_specification".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: None,
            evidence: JourneyEvidenceView {
                status: "blocked".into(),
                summary: summary.into(),
                blockers: vec![JourneyBlockerView {
                    message: message.into(),
                    next_action: next_action.into(),
                }],
            },
            related_specification,
            advanced,
        });
    }
    let approved = DeliveryStore::for_workspace(&workspace.root)
        .ok()
        .and_then(|store| {
            plan.slices.first().and_then(|slice| {
                store
                    .approval(&ExecutionIdentity {
                        plan_digest: plan.digest.clone(),
                        slice_id: slice.id.clone(),
                    })
                    .ok()
            })
        })
        .is_some();
    let scope = JourneyScopeView {
        summary: format!(
            "{} focused change{} are proposed.",
            plan.slices.len(),
            if plan.slices.len() == 1 { "" } else { "s" }
        ),
        status: if approved { "approved" } else { "proposed" }.into(),
        editable_target_count: plan
            .slices
            .iter()
            .map(|slice| slice.editable_targets.len())
            .sum(),
        slice_count: plan.slices.len(),
    };
    let validation_passed = matches!(work.validation.state, ValidationRunState::Passed);
    let completed = work
        .selected_slice
        .as_deref()
        .and_then(|slice_id| work.completion.current_for(&plan.digest, slice_id));
    advanced.attempt_id = completed.map(|attempt| attempt.attempt_id.clone());
    let blockers = completed
        .map(|attempt| {
            attempt
                .blockers
                .iter()
                .map(|blocker| JourneyBlockerView {
                    message: blocker.message.clone(),
                    next_action: blocker.next_action.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    if completed.is_some_and(|attempt| attempt.finalized) {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "complete".into(),
            steps: journey_steps("complete"),
            primary_action: JourneyActionView {
                action: "cancel".into(),
                label: "Start another change".into(),
                label_key: "journey.action.start_another".into(),
                explanation: "This work is complete. Start a new change when you are ready.".into(),
                explanation_key: "journey.explanation.start_another".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: None,
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "complete".into(),
                summary: "The approved change and its completion evidence were recorded.".into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    if completed.is_some_and(|attempt| attempt.status == CompletionStatus::Complete) {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "complete".into(),
            steps: journey_steps("complete"),
            primary_action: JourneyActionView {
                action: "finalize".into(),
                label: "Confirm completion".into(),
                label_key: "journey.action.finalize".into(),
                explanation: "Confirm the checked change after reviewing the completion evidence."
                    .into(),
                explanation_key: "journey.explanation.finalize".into(),
                confirmation_required: true,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "ready".into(),
                summary: "Verification evidence is ready for confirmation.".into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    if work
        .agent
        .as_ref()
        .is_some_and(|agent| matches!(agent.status, AgentRunStatus::Blocked))
    {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "implement".into(),
            steps: journey_steps("implement"),
            primary_action: JourneyActionView {
                action: "retry".into(),
                label: "Start a new implementation run".into(),
                label_key: "journey.action.retry".into(),
                explanation:
                    "Resolve the reported blocker, then start a new run in the approved scope."
                        .into(),
                explanation_key: "journey.explanation.retry".into(),
                confirmation_required: true,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "implementation_blocked".into(),
                summary:
                    "Implementation could not be verified; resolve the blocker before retrying."
                        .into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    if !validation_passed {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "review".into(),
            steps: journey_steps("review"),
            primary_action: JourneyActionView {
                action: "prepare".into(),
                label: "Review the plan again".into(),
                label_key: "journey.action.prepare".into(),
                explanation: "The plan needs a successful safety check before it can be approved."
                    .into(),
                explanation_key: "journey.explanation.prepare".into(),
                confirmation_required: false,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "needs_attention".into(),
                summary: "Resolve the plan review findings before continuing.".into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    if !approved {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "approve".into(),
            steps: journey_steps("approve"),
            primary_action: JourneyActionView {
                action: "approve".into(),
                label: "Approve this plan".into(),
                label_key: "journey.action.approve".into(),
                explanation: "Approval fixes the bounded change before implementation can begin."
                    .into(),
                explanation_key: "journey.explanation.approve".into(),
                confirmation_required: true,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "reviewed".into(),
                summary: "The proposed scope passed its safety check.".into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    if work.agent.is_none() {
        return Ok(WorkJourneyView {
            title,
            title_key: None,
            current_step: "implement".into(),
            steps: journey_steps("implement"),
            primary_action: JourneyActionView {
                action: "start".into(),
                label: "Start implementation".into(),
                label_key: "journey.action.start".into(),
                explanation: "Implementation is limited to the approved scope.".into(),
                explanation_key: "journey.explanation.start".into(),
                confirmation_required: true,
                enabled: true,
            },
            recovery_action: Some(cancel_action()),
            approved_scope: Some(scope),
            evidence: JourneyEvidenceView {
                status: "approved".into(),
                summary: "The scope is approved and ready for implementation.".into(),
                blockers,
            },
            related_specification,
            advanced,
        });
    }
    Ok(WorkJourneyView {
        title,
        title_key: None,
        current_step: "verify".into(),
        steps: journey_steps("verify"),
        primary_action: JourneyActionView {
            action: "verify".into(),
            label: "Check the completed change".into(),
            label_key: "journey.action.verify".into(),
            explanation: "Run the approved completion checks and show the evidence.".into(),
            explanation_key: "journey.explanation.verify".into(),
            confirmation_required: false,
            enabled: true,
        },
        recovery_action: Some(cancel_action()),
        approved_scope: Some(scope),
        evidence: JourneyEvidenceView {
            status: "in_progress".into(),
            summary: "Implementation is in progress inside the approved scope.".into(),
            blockers,
        },
        related_specification,
        advanced,
    })
}

fn cancel_action() -> JourneyActionView {
    JourneyActionView {
        action: "cancel".into(),
        label: "Cancel this work".into(),
        label_key: "journey.action.cancel".into(),
        explanation:
            "This stops the current journey. Changes already written to files are not undone."
                .into(),
        explanation_key: "journey.explanation.cancel".into(),
        confirmation_required: true,
        enabled: true,
    }
}

/// Compatibility entrypoint for CLI and HTTP callers. The implementation lives
/// in validation so readiness, Workbench, and CLI verification share one
/// canonical plan/receipt path.
pub fn execute_verification(
    workspace: &SpecWorkspace,
    index: &mitase_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    revision: &str,
) -> Result<VerificationReceipt> {
    mitase_validation::execute_verification(workspace, index, plan, slice_id, revision)
}

pub fn validate_verification_receipt(
    workspace: &SpecWorkspace,
    index: &mitase_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    receipt: &VerificationReceipt,
    revision: &str,
) -> Result<()> {
    mitase_validation::validate_verification_receipt(
        workspace, index, plan, slice_id, receipt, revision,
    )
}

pub fn branch_scope_view(
    index: &mitase_workspace::SpecIndex,
    items: &[ItemSummary],
    range: String,
    files: &[mitase_validation::ChangedFile],
) -> BranchScopeView {
    let mut changed = Vec::new();
    let mut affected_ids = BTreeSet::new();
    for file in files {
        let path = file
            .new_path
            .as_ref()
            .or(file.old_path.as_ref())
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let path_refs = index
            .path_to_targets
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let artifact_identities = index
            .artifact_units
            .iter()
            .filter(|unit| unit.path.to_string_lossy() == path)
            .map(|unit| unit.identity.clone())
            .collect::<Vec<_>>();
        let mut target_refs = path_refs;
        target_refs.extend(
            index
                .target_to_artifact
                .iter()
                .filter(|(_, identity)| artifact_identities.contains(identity))
                .map(|(reference, _)| reference.clone()),
        );
        target_refs.sort();
        target_refs.dedup();
        let owner_bindings = artifact_identities
            .iter()
            .filter_map(|identity| index.artifact_owners.get(identity))
            .flatten()
            .map(|owner| owner.binding.clone())
            .collect::<BTreeSet<_>>();
        let owners = owner_bindings
            .iter()
            .map(|binding| binding.item.to_string())
            .collect::<BTreeSet<_>>();
        let anchors = target_refs
            .iter()
            .map(ToString::to_string)
            .chain(owner_bindings.iter().map(ToString::to_string))
            .collect::<BTreeSet<_>>();
        affected_ids.extend(owners.iter().cloned());
        changed.push(BranchChangedTargetView {
            path,
            status: file.status,
            owners: owners.into_iter().collect(),
            anchors: anchors.into_iter().collect(),
            unresolved_reason: (artifact_identities.is_empty() && target_refs.is_empty())
                .then(|| "no active artifact identity or exact target".into()),
            plan_inclusion_reason: (!target_refs.is_empty() || !owner_bindings.is_empty())
                .then(|| "canonical artifact identity and ownership relation is present".into()),
            artifact_identities,
        });
    }
    let owned = changed
        .iter()
        .filter(|entry| !entry.owners.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let unowned = changed
        .iter()
        .filter(|entry| entry.owners.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let affected_items = items
        .iter()
        .filter(|item| affected_ids.contains(&item.id))
        .cloned()
        .collect::<Vec<_>>();
    BranchScopeView {
        range,
        state: if unowned.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .into(),
        reason: (!unowned.is_empty())
            .then(|| "changed artifacts include unowned or unresolved units".into()),
        changed,
        owned,
        unowned,
        affected_items,
    }
}

fn item_summary_from_philosophy(
    item: &Philosophy,
    path: &str,
    source_hash: &str,
    index: &mitase_workspace::SpecIndex,
    workspace_root: &Path,
) -> ItemSummary {
    let item_id = item.id.clone();
    ItemSummary {
        id: item.id.to_string(),
        kind: "philosophy".into(),
        path: path.into(),
        source_hash: source_hash.into(),
        title: item.title.clone(),
        presentation_title_key: builtin_presentation_title_key(&item.id, &item.title),
        summary: item.summary.clone(),
        description: None,
        status: None,
        priority: None,
        principles: item
            .principles
            .iter()
            .map(|principle| PrincipleSummary {
                anchor: anchor_string(&item_id, LocalAnchorKind::Principle, &principle.id),
                statement: principle.statement.clone(),
                applies_to: principle.applies_to.clone(),
            })
            .collect(),
        rules: vec![],
        criteria: vec![],
        bindings: bindings_for(&item_id, &item.bindings, workspace_root),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
        origin_capabilities: vec![],
    }
}

fn item_summary_from_policy(
    item: &Policy,
    path: &str,
    source_hash: &str,
    index: &mitase_workspace::SpecIndex,
    workspace_root: &Path,
) -> ItemSummary {
    let item_id = item.id.clone();
    ItemSummary {
        id: item.id.to_string(),
        kind: "policy".into(),
        path: path.into(),
        source_hash: source_hash.into(),
        title: item.title.clone(),
        presentation_title_key: builtin_presentation_title_key(&item.id, &item.title),
        summary: item.summary.clone(),
        description: (!item.description.is_empty()).then(|| item.description.clone()),
        status: None,
        priority: None,
        principles: vec![],
        rules: item
            .rules
            .iter()
            .map(|rule| rule_summary(&item_id, rule))
            .collect(),
        criteria: vec![],
        bindings: bindings_for(&item_id, &item.bindings, workspace_root),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
        origin_capabilities: vec![],
    }
}

fn item_summary_from_requirement(
    item: &Requirement,
    path: &str,
    source_hash: &str,
    index: &mitase_workspace::SpecIndex,
    workspace_root: &Path,
) -> ItemSummary {
    let item_id = item.id.clone();
    ItemSummary {
        id: item.id.to_string(),
        kind: "requirement".into(),
        path: path.into(),
        source_hash: source_hash.into(),
        title: item.title.clone(),
        presentation_title_key: builtin_presentation_title_key(&item.id, &item.title),
        summary: item.description.clone(),
        description: Some(item.description.clone()),
        status: Some(status_label(item.status).into()),
        priority: Some(priority_label(item.priority).into()),
        principles: vec![],
        rules: vec![],
        criteria: item
            .criteria
            .iter()
            .map(|criterion| criterion_summary(&item_id, criterion))
            .collect(),
        bindings: bindings_for(&item_id, &item.bindings, workspace_root),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
        origin_capabilities: vec![],
    }
}

fn item_summary_from_feature(
    item: &mitase_spec_model::Feature,
    path: &str,
    source_hash: &str,
    index: &mitase_workspace::SpecIndex,
    workspace_root: &Path,
) -> ItemSummary {
    let item_id = item.id.clone();
    ItemSummary {
        id: item.id.to_string(),
        kind: "feature".into(),
        path: path.into(),
        source_hash: source_hash.into(),
        title: item.title.clone(),
        presentation_title_key: builtin_presentation_title_key(&item.id, &item.title),
        summary: item.summary.clone(),
        description: None,
        status: Some(status_label(item.status).into()),
        priority: None,
        principles: vec![],
        rules: vec![],
        criteria: vec![],
        bindings: bindings_for(&item_id, &item.bindings, workspace_root),
        contracts: item
            .contracts
            .iter()
            .map(|contract| contract_summary(&item_id, contract))
            .collect(),
        anchors: anchors_for(index, &item.id),
        origin_capabilities: vec![],
    }
}

fn origin_capabilities(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    items: &[ItemSummary],
) -> Vec<OriginCapabilityView> {
    let mut capabilities = Vec::new();
    for item in items {
        if item.kind == "requirement" {
            for criterion in &item.criteria {
                if let Ok(anchor) = criterion.anchor.parse::<SpecAnchor>() {
                    capabilities.push(origin_capability(
                        workspace,
                        index,
                        WorkOrigin::RequirementCriterion { criterion: anchor },
                        "Requirement criterion",
                    ));
                }
            }
        } else if item.kind == "feature" {
            for binding in &item.bindings {
                if binding.role != "implementation" {
                    continue;
                }
                let Ok(binding_anchor) = binding.anchor.parse::<SpecAnchor>() else {
                    continue;
                };
                let mut target_criteria = binding
                    .targets
                    .iter()
                    .filter_map(|target| {
                        let reference = target.reference.parse::<BoundTargetRef>().ok()?;
                        let artifact = index.target(&reference)?;
                        if matches!(
                            artifact.lifecycle,
                            mitase_spec_model::ArtifactTargetLifecycle::Absent
                        ) {
                            return None;
                        }
                        let criteria = artifact
                            .claims
                            .iter()
                            .filter_map(|claim| match claim {
                                TargetClaim::Satisfies { criterion } => Some(criterion.clone()),
                                _ => None,
                            })
                            .collect::<BTreeSet<_>>();
                        Some((reference, criteria))
                    })
                    .collect::<Vec<_>>();
                target_criteria.sort_by(|(left, _), (right, _)| left.cmp(right));
                target_criteria.dedup_by(|(left, _), (right, _)| left == right);
                let targets = target_criteria
                    .iter()
                    .map(|(reference, _)| reference.clone())
                    .collect::<Vec<_>>();
                let criteria = target_criteria
                    .iter()
                    .flat_map(|(_, criteria)| criteria.iter().cloned())
                    .collect::<BTreeSet<_>>();
                if criteria.len() == 1 {
                    let criterion = criteria.into_iter().next().expect("one criterion");
                    capabilities.push(origin_capability(
                        workspace,
                        index,
                        WorkOrigin::FeatureImplementationBinding {
                            binding: binding_anchor.clone(),
                            criterion: criterion.clone(),
                            targets: targets.clone(),
                        },
                        "Feature implementation",
                    ));
                    for (target, target_criteria) in &target_criteria {
                        if !target_criteria.contains(&criterion) {
                            continue;
                        }
                        capabilities.push(origin_capability(
                            workspace,
                            index,
                            WorkOrigin::FeatureImplementationTarget {
                                target: target.clone(),
                                binding: binding_anchor.clone(),
                                criterion: criterion.clone(),
                            },
                            "Implementation target",
                        ));
                    }
                } else {
                    for (target, target_criteria) in &target_criteria {
                        for criterion in target_criteria {
                            capabilities.push(origin_capability(
                                workspace,
                                index,
                                WorkOrigin::FeatureImplementationTarget {
                                    target: target.clone(),
                                    binding: binding_anchor.clone(),
                                    criterion: criterion.clone(),
                                },
                                "Implementation target",
                            ));
                        }
                    }
                    let nearest = criteria
                        .iter()
                        .cloned()
                        .map(|criterion| WorkOrigin::FeatureImplementationBinding {
                            binding: binding_anchor.clone(),
                            criterion,
                            targets: targets.clone(),
                        })
                        .collect();
                    capabilities.push(OriginCapabilityView {
                        schema: mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
                        origin: None,
                        label: "Feature implementation".into(),
                        enabled: false,
                        disabled_code: Some("ambiguous-origin".into()),
                        disabled_message: Some(
                            "choose a Feature implementation binding with one exact Requirement criterion"
                                .into(),
                        ),
                        nearest,
                    });
                }
            }
        }
    }
    for capability in &mut capabilities {
        capability.nearest.sort_by(|left, right| {
            origin_kind_rank(left)
                .cmp(&origin_kind_rank(right))
                .then_with(|| origin_bytes(left).cmp(&origin_bytes(right)))
        });
        capability
            .nearest
            .dedup_by(|left, right| origin_bytes(left) == origin_bytes(right));
    }
    capabilities.sort_by(|left, right| {
        let left_key = left
            .origin
            .as_ref()
            .or_else(|| left.nearest.first())
            .map(origin_bytes)
            .unwrap_or_default();
        let right_key = right
            .origin
            .as_ref()
            .or_else(|| right.nearest.first())
            .map(origin_bytes)
            .unwrap_or_default();
        left_key
            .cmp(&right_key)
            .then_with(|| left.label.cmp(&right.label))
    });
    capabilities
}

fn origin_bytes(origin: &WorkOrigin) -> Vec<u8> {
    serde_json::to_vec(origin).expect("serialize exact origin")
}

fn origin_kind_rank(origin: &WorkOrigin) -> u8 {
    match origin {
        WorkOrigin::RequirementCriterion { .. } => 0,
        WorkOrigin::FeatureImplementationBinding { .. } => 1,
        WorkOrigin::FeatureImplementationTarget { .. } => 2,
    }
}

fn origin_owner_item(origin: &WorkOrigin) -> &SpecId {
    match origin {
        WorkOrigin::RequirementCriterion { criterion }
        | WorkOrigin::FeatureImplementationBinding {
            binding: criterion, ..
        }
        | WorkOrigin::FeatureImplementationTarget {
            binding: criterion, ..
        } => &criterion.item,
    }
}

fn origin_capability(
    snapshot_workspace: &SpecWorkspace,
    index: &SpecIndex,
    origin: WorkOrigin,
    label: &str,
) -> OriginCapabilityView {
    match validate_work_origin(snapshot_workspace, index, &origin) {
        Ok(()) => OriginCapabilityView {
            schema: mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
            origin: Some(origin),
            label: label.into(),
            enabled: true,
            disabled_code: None,
            disabled_message: None,
            nearest: vec![],
        },
        Err(error) => OriginCapabilityView {
            schema: mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
            origin: None,
            label: label.into(),
            enabled: false,
            disabled_code: Some(origin_error_code(&error)),
            disabled_message: Some(error.to_string()),
            nearest: vec![origin],
        },
    }
}

fn origin_error_code(error: &anyhow::Error) -> String {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("must start from an exact requirement criterion")
        || message.contains("does not resolve to an exact requirement criterion")
        || message.contains(" unknown")
        || message.contains("unknown ")
        || message.contains("does not resolve")
    {
        "unknown-origin"
    } else if message.contains("not implemented") || message.contains("planned") {
        "planned-origin"
    } else if message.contains("not an implementation binding") {
        "non-implementation-binding"
    } else if message.contains("canonically sorted") || message.contains("must be exact") {
        "unsupported-origin-shape"
    } else if message.contains("ambiguous") {
        "ambiguous-criterion"
    } else if message.contains("absent") {
        "target-lifecycle"
    } else if message.contains("no active") || message.contains("no exact") {
        if message.contains("verification") {
            "missing-verification-coverage"
        } else {
            "missing-satisfies"
        }
    } else if message.contains("satisfies") || message.contains("criterion") {
        "missing-satisfies"
    } else if message.contains("verification") {
        "missing-verification-coverage"
    } else if message.contains("contract") {
        "missing-contract-closure"
    } else {
        "unknown-origin"
    }
    .into()
}

fn origin_error_status(error: &anyhow::Error) -> StatusCode {
    match origin_error_code(error).as_str() {
        "unknown-origin" => StatusCode::NOT_FOUND,
        "unsupported-origin-shape" => StatusCode::BAD_REQUEST,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format_sha256(hasher.finalize())
}

fn builtin_presentation_title_key(id: &SpecId, title: &str) -> Option<String> {
    (id.to_string() == "REQ-CAPABILITY-001" && title == "Canonical capability behavior")
        .then(|| "specification.title.REQ-CAPABILITY-001".into())
}

fn anchors_for(
    index: &mitase_workspace::SpecIndex,
    item: &mitase_spec_model::SpecId,
) -> Vec<String> {
    index
        .item_anchors
        .get(item)
        .into_iter()
        .flatten()
        .map(ToString::to_string)
        .collect()
}

fn rule_summary(item: &mitase_spec_model::SpecId, rule: &Rule) -> RuleSummary {
    RuleSummary {
        anchor: anchor_string(item, LocalAnchorKind::Rule, &rule.id),
        level: match rule.level {
            RuleLevel::Must => "must",
            RuleLevel::Should => "should",
            RuleLevel::May => "may",
        }
        .into(),
        statement: rule.statement.clone(),
        governed_by: rule.governed_by.iter().map(ToString::to_string).collect(),
        applies_to_roles: rule
            .applies_to
            .roles
            .iter()
            .map(|role| binding_role_label(*role).to_string())
            .collect(),
        enforcement: rule.enforcement.as_ref().map(|value| match value {
            mitase_spec_model::RuleEnforcement::External(text) => text.clone(),
        }),
    }
}

fn criterion_summary(item: &mitase_spec_model::SpecId, criterion: &Criterion) -> CriterionSummary {
    CriterionSummary {
        anchor: anchor_string(item, LocalAnchorKind::Criterion, &criterion.id),
        kind: criterion.kind,
        statement: criterion.statement.clone(),
        governed_by: criterion
            .governed_by
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn bindings_for(
    item: &mitase_spec_model::SpecId,
    bindings: &[ArtifactBinding],
    workspace_root: &Path,
) -> Vec<BindingSummary> {
    bindings
        .iter()
        .map(|binding| {
            let binding_anchor = SpecAnchor {
                item: item.clone(),
                kind: LocalAnchorKind::Binding,
                local_id: binding.id.clone(),
            };
            BindingSummary {
                anchor: binding_anchor.to_string(),
                role: binding_role_label(binding.role).into(),
                facet: binding.facet.clone(),
                responsibility: binding.responsibility.clone(),
                // Deleted retirement markers remain in the self-hosting spec
                // so change validation can account for their deletion, but
                // they are not live UI capabilities and must not reintroduce
                // retired browser assets into the projection.
                owns: binding
                    .owns
                    .iter()
                    .filter(|scope| workspace_root.join(scope.path.as_path()).exists())
                    .cloned()
                    .collect(),
                targets: binding
                    .targets
                    .iter()
                    .map(|target| BindingTargetSummary {
                        reference: BoundTargetRef {
                            binding: binding_anchor.clone(),
                            target_id: target.id.clone(),
                        }
                        .to_string(),
                        path: target.path.to_string_lossy().into_owned(),
                        selector: target.selector.clone(),
                        adapter: target.adapter.clone(),
                        lifecycle: target.lifecycle,
                        claims: target.claims.clone(),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn contract_summary(item: &mitase_spec_model::SpecId, contract: &Contract) -> ContractSummary {
    ContractSummary {
        anchor: anchor_string(item, LocalAnchorKind::Contract, &contract.id),
        kind: contract_kind_label(contract.kind).into(),
        source: contract.source.to_string(),
        participants: contract
            .participants
            .iter()
            .map(|participant| ContractParticipantSummary {
                binding: participant.target.to_string(),
                role: participant.role.clone(),
            })
            .collect(),
        guarantees: contract
            .guarantees
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn anchor_string(
    item: &mitase_spec_model::SpecId,
    kind: LocalAnchorKind,
    local_id: &mitase_spec_model::LocalId,
) -> String {
    SpecAnchor {
        item: item.clone(),
        kind,
        local_id: local_id.clone(),
    }
    .to_string()
}

fn binding_role_label(role: BindingRole) -> &'static str {
    match role {
        BindingRole::Implementation => "implementation",
        BindingRole::Verification => "verification",
        BindingRole::Documentation => "documentation",
        BindingRole::Enforcement => "enforcement",
        BindingRole::ContractSource => "contract_source",
        BindingRole::Configuration => "configuration",
        BindingRole::Generated => "generated",
        BindingRole::Migration => "migration",
        BindingRole::Operation => "operation",
        BindingRole::Evidence => "evidence",
    }
}

fn contract_kind_label(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Http => "http",
        ContractKind::Event => "event",
        ContractKind::Function => "function",
        ContractKind::Schema => "schema",
        ContractKind::Cli => "cli",
        ContractKind::File => "file",
        ContractKind::Custom => "custom",
    }
}

fn status_label(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Planned => "planned",
        ItemStatus::Implemented => "implemented",
        ItemStatus::Deprecated => "deprecated",
    }
}

fn priority_label(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
        Priority::Critical => "critical",
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .or_else(|_| {
            let root = root.canonicalize()?;
            let path = path.canonicalize()?;
            path.strip_prefix(root)
                .map(Path::to_path_buf)
                .map_err(std::io::Error::other)
        })
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use mitase_work_model::TargetTransition;
    use std::sync::OnceLock;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, MutexGuard};
    use tower::ServiceExt;

    static WORKSPACE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    async fn workspace_test_lock() -> MutexGuard<'static, ()> {
        WORKSPACE_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .await
    }

    #[test]
    fn split_candidate_accepts_a_planned_remove_with_an_exact_pre_state_artifact() {
        let reference: BoundTargetRef = "FEAT-TEST-001#binding.implementation/target.removed"
            .parse()
            .unwrap();
        let mut index = SpecIndex::default();
        index
            .all_target_to_artifact
            .insert(reference.clone(), "artifact-before-remove".into());
        let target: mitase_work_model::PlannedTarget = serde_json::from_value(serde_json::json!({
            "ref": reference,
            "transition": "remove",
            "lifecycle": "ensure-absent",
            "access": "editable",
            "resolved_path": "src/removed.rs",
            "resolved_selector": { "description": "removed", "symbols": ["removed"] },
            "content_hash": "sha256:before",
            "excerpt_hash": "sha256:excerpt",
            "adapter": "rust",
            "facet": "work",
            "role": "implementation",
            "byte_start": 0,
            "byte_end": 20,
            "line_start": 1,
            "line_end": 1,
            "budget_bytes": 20,
            "reason": "remove the exact planned target"
        }))
        .expect("planned remove target");

        assert!(candidate_target_is_valid(&index, &target));
        index.all_target_to_artifact.clear();
        assert!(!candidate_target_is_valid(&index, &target));
    }

    #[test]
    fn scope_diff_combines_status_and_patch_for_the_working_tree() {
        let temp = tempfile::tempdir().expect("diff fixture");
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(temp.path())
                .args(args)
                .status()
                .expect("run git");
            assert!(status.success(), "git {}", args.join(" "));
        };
        git(&["init", "--quiet"]);
        git(&["config", "user.email", "workbench@example.invalid"]);
        git(&["config", "user.name", "Workbench Test"]);
        fs::write(temp.path().join("sample.txt"), "before\n").expect("write fixture");
        git(&["add", "sample.txt"]);
        git(&["commit", "--quiet", "-m", "fixture"]);
        fs::write(temp.path().join("sample.txt"), "after\nadded\n").expect("modify fixture");

        let changed = branch_changed_files(temp.path(), "HEAD").expect("changed files");
        let view = scope_diff_view(temp.path(), "HEAD".into(), &changed).expect("diff view");

        assert_eq!(view.state, "ready");
        assert_eq!(view.files.len(), 1);
        assert_eq!(view.files[0].path, "sample.txt");
        assert_eq!(view.files[0].additions, 2);
        assert_eq!(view.files[0].deletions, 1);
        assert!(view.files[0].patch.contains("+added"));
    }

    #[test]
    fn semantic_projection_uses_serde_enum_labels_and_exact_builtin_title_identity() {
        let target = TargetView {
            reference: "FEAT-X#binding.impl/target.code".into(),
            access: TargetAccessMode::RunOnly,
            transition: TargetTransition::RunOnly,
            path: "src/lib.rs".into(),
        };
        let target_json = serde_json::to_value(target).expect("target view JSON");
        assert_eq!(target_json["access"], "run-only");
        assert_eq!(target_json["transition"], "run-only");

        let readiness = serde_json::to_value(ReadinessView {
            target: ReadinessLevel::WorkReady,
            status: "Ready".into(),
            blocking_subjects: 0,
            axes: BTreeMap::new(),
            blockers: vec![],
            execution_state: "not-run".into(),
        })
        .expect("readiness view JSON");
        assert_eq!(readiness["target"], "work-ready");

        let id = SpecId::from("REQ-CAPABILITY-001");
        assert_eq!(
            builtin_presentation_title_key(&id, "Canonical capability behavior"),
            Some("specification.title.REQ-CAPABILITY-001".into())
        );
        assert_eq!(
            builtin_presentation_title_key(&id, "User-edited capability title"),
            None
        );
    }

    #[test]
    fn specification_trace_is_deterministic_and_preserves_canonical_claims() {
        let fixture = workspace_root().join("fixtures/v1/valid-web-app");
        let workspace = SpecWorkspace::load(fixture).expect("trace fixture loads");
        let index = workspace.index().expect("trace index loads");
        let projection = project(&workspace, None, "test-revision").expect("projection loads");
        let root = projection
            .specifications
            .specifications
            .iter()
            .find(|item| item.id == "REQ-AUTH-001")
            .expect("fixture requirement");
        let query = SpecificationTraceQuery {
            depth: Some(4),
            mode: Some("exact".into()),
            node_budget: Some(80),
            edge_budget: Some(160),
        };
        let first = specification_trace_view(&projection, &index, root, &query);
        let second = specification_trace_view(&projection, &index, root, &query);
        assert_eq!(
            serde_json::to_string(&first).expect("serialize trace"),
            serde_json::to_string(&second).expect("serialize trace")
        );
        assert_eq!(first.root_item_id, "REQ-AUTH-001");
        assert_eq!(first.mode, "exact");
        assert!(first.nodes.iter().any(|node| node.kind == "claim"));
        assert!(first.edges.iter().any(|edge| edge.exact_claim.is_some()));
        assert_eq!(first.closures.len(), 1);
        assert_eq!(first.closures[0].state, "declaration-only");
        for mode in ["readable", "exact"] {
            let bounded = specification_trace_view(
                &projection,
                &index,
                root,
                &SpecificationTraceQuery {
                    depth: Some(8),
                    mode: Some(mode.into()),
                    node_budget: Some(8),
                    edge_budget: Some(8),
                },
            );
            assert!(bounded.nodes.len() <= 8, "{mode} node budget exceeded");
            assert!(bounded.edges.len() <= 8, "{mode} edge budget exceeded");
        }
        let readable = specification_trace_view(
            &projection,
            &index,
            root,
            &SpecificationTraceQuery {
                depth: Some(8),
                mode: Some("readable".into()),
                node_budget: Some(20),
                edge_budget: Some(20),
            },
        );
        let exact = specification_trace_view(
            &projection,
            &index,
            root,
            &SpecificationTraceQuery {
                depth: Some(8),
                mode: Some("exact".into()),
                node_budget: Some(20),
                edge_budget: Some(20),
            },
        );
        let readable_nodes = readable
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        let exact_semantic_nodes = exact
            .nodes
            .iter()
            .filter(|node| node.kind != "claim")
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(readable_nodes, exact_semantic_nodes);
        let readable_edges = readable
            .edges
            .iter()
            .filter(|edge| {
                readable
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.from)
                    .is_none_or(|node| node.kind != "claim")
                    && readable
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.to)
                        .is_none_or(|node| node.kind != "claim")
            })
            .map(|edge| edge.id.clone())
            .collect::<BTreeSet<_>>();
        let exact_edges = exact
            .edges
            .iter()
            .filter(|edge| {
                exact
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.from)
                    .is_none_or(|node| node.kind != "claim")
                    && exact
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.to)
                        .is_none_or(|node| node.kind != "claim")
            })
            .map(|edge| edge.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(readable_edges, exact_edges);
        let exact_verification_target = readable.closures[0]
            .verification_targets
            .first()
            .cloned()
            .expect("fixture declares a verification target");
        let mut with_receipt = projection.clone();
        with_receipt.work.verification_receipt = Some(VerificationReceiptView {
            slice_id: "slice-fixture".into(),
            status: "failed".into(),
            revision: "receipt-revision".into(),
            workspace_fingerprint: "receipt-fingerprint".into(),
            started_at: "2026-08-01T00:00:00Z".into(),
            completed_at: "2026-08-01T00:01:00Z".into(),
            executions: vec![VerificationExecutionView {
                identity: "slice-fixture#execution-0".into(),
                target: exact_verification_target.clone(),
                claim: Some(VerificationClaimView {
                    target: exact_verification_target.clone(),
                    criterion: root.criteria[0].anchor.clone(),
                }),
                status: "failed".into(),
            }],
        });
        let evidence = specification_trace_view(
            &with_receipt,
            &index,
            root,
            &SpecificationTraceQuery {
                depth: Some(4),
                mode: Some("readable".into()),
                node_budget: Some(80),
                edge_budget: Some(160),
            },
        );
        assert_eq!(
            with_receipt
                .work
                .verification_receipt
                .as_ref()
                .expect("fixture receipt")
                .executions[0]
                .claim
                .as_ref()
                .expect("exact claim")
                .target,
            exact_verification_target
        );
        assert_eq!(evidence.closures[0].runtime_status, "failed");
        assert_eq!(
            evidence.closures[0].runtime_revision.as_deref(),
            Some("receipt-revision")
        );
        assert_eq!(evidence.closures[0].runtime_executions.len(), 1);
        assert_eq!(
            evidence.closures[0].runtime_executions[0].identity,
            "slice-fixture#execution-0"
        );
        assert_eq!(
            evidence.closures[0].runtime_executions[0].target,
            exact_verification_target
        );
        assert_eq!(
            evidence.closures[0].runtime_receipt.as_deref(),
            Some("slice-fixture@receipt-revision@2026-08-01T00:01:00Z")
        );
        let mut unrelated_receipt = with_receipt.clone();
        unrelated_receipt
            .work
            .verification_receipt
            .as_mut()
            .expect("fixture receipt")
            .executions[0]
            .claim = Some(VerificationClaimView {
            target: "other-target".into(),
            criterion: "REQ-UNRELATED-001#criterion.other".into(),
        });
        let unrelated = specification_trace_view(
            &unrelated_receipt,
            &index,
            root,
            &SpecificationTraceQuery {
                depth: Some(4),
                mode: Some("readable".into()),
                node_budget: Some(80),
                edge_budget: Some(160),
            },
        );
        assert!(
            unrelated
                .closures
                .iter()
                .all(|closure| closure.runtime_status == "unavailable")
        );
        let mut mismatched_target = with_receipt.clone();
        let mismatched_execution = mismatched_target
            .work
            .verification_receipt
            .as_mut()
            .expect("fixture receipt")
            .executions
            .first_mut()
            .expect("fixture execution");
        mismatched_execution.target = "other-target".into();
        let mismatched = specification_trace_view(
            &mismatched_target,
            &index,
            root,
            &SpecificationTraceQuery {
                depth: Some(4),
                mode: Some("readable".into()),
                node_budget: Some(80),
                edge_budget: Some(160),
            },
        );
        assert!(
            mismatched
                .closures
                .iter()
                .all(|closure| closure.runtime_status == "unavailable")
        );
    }

    #[test]
    fn specification_trace_reaches_external_workbench_targets_from_spec_index() {
        let workspace = SpecWorkspace::load(workspace_root()).expect("repository loads");
        let index = workspace.index().expect("repository index loads");
        let projection = project(&workspace, None, "test-revision").expect("projection loads");
        let root = projection
            .specifications
            .specifications
            .iter()
            .find(|item| item.id == "REQ-WORKBENCH-003")
            .expect("workbench requirement");
        let query = SpecificationTraceQuery {
            depth: Some(4),
            mode: Some("readable".into()),
            node_budget: Some(200),
            edge_budget: Some(400),
        };
        let trace = specification_trace_view(&projection, &index, root, &query);
        assert!(trace.nodes.iter().any(|node| {
            node.id == "FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-apply"
        }));
        assert!(
            trace
                .related
                .specification
                .iter()
                .any(|item| item.item_id == "FEAT-WORKBENCH-SPEC-EDITOR-001")
        );
        assert!(
            trace
                .nodes
                .iter()
                .any(|node| node.id == "POL-DELIVERY-001#rule.exact-ownership")
        );
        let philosophy = projection
            .specifications
            .specifications
            .iter()
            .find(|item| item.id == "PHIL-001")
            .expect("workbench philosophy");
        let philosophy_trace = specification_trace_view(
            &projection,
            &index,
            philosophy,
            &SpecificationTraceQuery {
                depth: Some(4),
                mode: Some("readable".into()),
                node_budget: Some(240),
                edge_budget: Some(480),
            },
        );
        assert!(
            philosophy_trace
                .nodes
                .iter()
                .any(|node| node.id == "PHIL-001#principle.exact-intent")
        );
        assert!(
            philosophy_trace
                .nodes
                .iter()
                .any(|node| node.id == "POL-DELIVERY-001#rule.exact-ownership")
        );
        assert!(
            philosophy_trace
                .nodes
                .iter()
                .any(|node| node.id == "REQ-WORKBENCH-003#criterion.transactional-spec-edit")
        );
        assert!(philosophy_trace.nodes.iter().any(|node| node.id
            == "FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-apply"));
        assert!(philosophy_trace.nodes.iter().any(
            |node| node.id == "REQ-WORKBENCH-003#binding.spec-edit-check/target.spec-edit-test"
        ));
        for kind in ["policy", "feature"] {
            let item = projection
                .specifications
                .specifications
                .iter()
                .find(|item| item.kind == kind)
                .expect("item kind exists");
            let evidence = specification_trace_view(
                &projection,
                &index,
                item,
                &SpecificationTraceQuery {
                    depth: Some(4),
                    mode: Some("readable".into()),
                    node_budget: Some(240),
                    edge_budget: Some(480),
                },
            );
            assert!(
                !evidence.closures.is_empty(),
                "{kind} must expose related evidence"
            );
        }
    }

    #[test]
    fn workbench_detail_context_budget_is_explicitly_bounded() {
        let workspace = SpecWorkspace::load(workspace_root()).expect("repository loads");
        assert_eq!(
            workspace.config.work.slicing.max_total_bytes, 160_000,
            "the detail workspace budget is measured and explicitly bounded"
        );
    }

    #[test]
    fn typed_nested_binding_and_target_edits_round_trip_without_yaml_maps() {
        let fixture = workspace_root().join("fixtures/v1/valid-web-app");
        let workspace = SpecWorkspace::load(fixture).expect("trace fixture loads");
        let loaded = workspace
            .documents
            .iter()
            .find(|loaded| matches!(loaded.document, SpecDocument::Requirements { .. }))
            .expect("requirement document");
        let requirement = match &loaded.document {
            SpecDocument::Requirements { requirements, .. } => requirements
                .iter()
                .find(|item| item.id.to_string() == "REQ-AUTH-001")
                .expect("fixture requirement"),
            _ => unreachable!(),
        };
        let binding = requirement.bindings.first().expect("fixture binding");
        let binding_patch = ArtifactBinding {
            facet: format!("{}-edited", binding.facet),
            ..binding.clone()
        };
        let binding_edit = EditPatch::Nested {
            item_id: requirement.id.to_string(),
            edit: NestedEdit::Binding {
                operation: NestedEditOperation::Upsert,
                binding: binding_patch.clone(),
                current_id: Some(binding.id.to_string()),
            },
        };
        let path = specification_path(&workspace, &requirement.id.to_string()).expect("path");
        let binding_content = specification_patch_content(&workspace, &path, &binding_edit)
            .expect("typed binding edit");
        let edited: SpecDocument = serde_yaml::from_str(&binding_content).expect("edited document");
        let edited_binding = match edited {
            SpecDocument::Requirements { requirements, .. } => requirements
                .into_iter()
                .find(|item| item.id == requirement.id)
                .and_then(|item| {
                    item.bindings
                        .into_iter()
                        .find(|binding| binding.id == binding_patch.id)
                })
                .expect("edited binding"),
            _ => unreachable!(),
        };
        assert_eq!(edited_binding.facet, format!("{}-edited", binding.facet));

        let target = binding.targets.first().expect("fixture target");
        let target_edit = EditPatch::Nested {
            item_id: requirement.id.to_string(),
            edit: NestedEdit::Target {
                operation: NestedEditOperation::Delete,
                binding_id: binding.id.clone(),
                target: target.clone(),
                current_id: Some(target.id.to_string()),
            },
        };
        let target_content = specification_patch_content(&workspace, &path, &target_edit)
            .expect("typed target delete");
        let deleted: SpecDocument =
            serde_yaml::from_str(&target_content).expect("deleted document");
        let remaining = match deleted {
            SpecDocument::Requirements { requirements, .. } => requirements
                .into_iter()
                .find(|item| item.id == requirement.id)
                .and_then(|item| {
                    item.bindings
                        .into_iter()
                        .find(|binding| binding.id == binding_edit_binding_id(&binding_edit))
                })
                .map(|binding| binding.targets)
                .expect("remaining binding"),
            _ => unreachable!(),
        };
        assert!(remaining.iter().all(|candidate| candidate.id != target.id));
    }

    #[test]
    fn module_ownership_round_trip_preserves_name_and_renames_are_rejected() {
        let workspace = SpecWorkspace::load(workspace_root()).expect("repository loads");
        let projection = project(&workspace, None, "test-revision").expect("projection loads");
        let (item, binding, ownership) = projection
            .specifications
            .specifications
            .iter()
            .find_map(|item| {
                item.bindings.iter().find_map(|binding| {
                    binding.owns.iter().find_map(|ownership| {
                        matches!(
                            &ownership.selector,
                            mitase_spec_model::OwnershipSelector::Module { .. }
                        )
                        .then_some((item, binding, ownership))
                    })
                })
            })
            .expect("self-hosting module ownership");
        let path = specification_path(&workspace, &item.id).expect("module ownership path");
        let edit = EditPatch::Nested {
            item_id: item.id.clone(),
            edit: NestedEdit::Ownership {
                operation: NestedEditOperation::Upsert,
                binding_id: LocalId::from(binding.anchor.split("#binding.").last().unwrap()),
                ownership: ownership.clone(),
                current_id: Some(ownership.id.to_string()),
            },
        };
        let content = specification_patch_content(&workspace, &path, &edit).expect("round trip");
        assert!(content.contains("kind: module"));
        assert!(content.contains("name:"));
        let renamed = EditPatch::Nested {
            item_id: item.id.clone(),
            edit: NestedEdit::Ownership {
                operation: NestedEditOperation::Upsert,
                binding_id: LocalId::from(binding.anchor.split("#binding.").last().unwrap()),
                ownership: ownership.clone(),
                current_id: Some("different-id".into()),
            },
        };
        assert!(specification_patch_content(&workspace, &path, &renamed).is_err());
    }

    fn binding_edit_binding_id(patch: &EditPatch) -> LocalId {
        match patch {
            EditPatch::Nested {
                edit: NestedEdit::Binding { binding, .. },
                ..
            } => binding.id.clone(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn typed_nested_edit_round_trip_covers_all_entity_variants() {
        let fixture = workspace_root().join("fixtures/v1/valid-web-app");
        let workspace = SpecWorkspace::load(fixture).expect("trace fixture loads");

        let feature_loaded = workspace
            .documents
            .iter()
            .find(|loaded| matches!(loaded.document, SpecDocument::Features { .. }))
            .expect("feature document");
        let (feature_id, binding, target, claim_binding, claim_target, ownership, contract) =
            match &feature_loaded.document {
                SpecDocument::Features { features, .. } => {
                    let feature = features.first().expect("feature");
                    let binding = feature
                        .bindings
                        .iter()
                        .find(|binding| binding.id.to_string() == "schema")
                        .expect("schema binding");
                    let claim_target = feature
                        .bindings
                        .iter()
                        .find(|binding| binding.id.to_string() == "ui")
                        .and_then(|binding| binding.targets.first())
                        .expect("claim target");
                    let claim_binding = feature
                        .bindings
                        .iter()
                        .find(|binding| binding.id.to_string() == "ui")
                        .expect("claim binding");
                    (
                        feature.id.clone(),
                        binding.clone(),
                        binding.targets.first().expect("contract target").clone(),
                        claim_binding.clone(),
                        claim_target.clone(),
                        binding.owns.first().expect("ownership scope").clone(),
                        feature.contracts.first().expect("contract").clone(),
                    )
                }
                _ => unreachable!(),
            };
        let feature_path =
            specification_path(&workspace, &feature_id.to_string()).expect("feature path");

        let ownership_edit = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Ownership {
                operation: NestedEditOperation::Upsert,
                binding_id: binding.id.clone(),
                ownership: OwnershipScope {
                    adapter: "openapi-edited".into(),
                    ..ownership.clone()
                },
                current_id: Some(ownership.id.to_string()),
            },
        };
        let ownership_content =
            specification_patch_content(&workspace, &feature_path, &ownership_edit)
                .expect("ownership upsert");
        assert!(ownership_content.contains("openapi-edited"));
        let ownership_delete = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Ownership {
                operation: NestedEditOperation::Delete,
                binding_id: binding.id.clone(),
                ownership,
                current_id: None,
            },
        };
        let ownership_deleted =
            specification_patch_content(&workspace, &feature_path, &ownership_delete)
                .expect("ownership delete");
        assert!(!ownership_deleted.contains("openapi-source"));

        let target_edit = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Target {
                operation: NestedEditOperation::Upsert,
                binding_id: binding.id.clone(),
                target: mitase_spec_model::ArtifactTarget {
                    path: mitase_spec_model::RepoPath::new("openapi-v2.yaml")
                        .expect("repository path"),
                    ..target.clone()
                },
                current_id: Some(target.id.to_string()),
            },
        };
        assert!(
            specification_patch_content(&workspace, &feature_path, &target_edit)
                .expect("target selector/path update")
                .contains("openapi-v2.yaml")
        );
        let absent_target_edit = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Target {
                operation: NestedEditOperation::Upsert,
                binding_id: binding.id.clone(),
                target: mitase_spec_model::ArtifactTarget {
                    path: mitase_spec_model::RepoPath::new("openapi-removed.yaml")
                        .expect("repository path"),
                    lifecycle: ArtifactTargetLifecycle::Absent,
                    ..target.clone()
                },
                current_id: Some(target.id.to_string()),
            },
        };
        let absent_target_content =
            specification_patch_content(&workspace, &feature_path, &absent_target_edit)
                .expect("absent target edit");
        let absent_document: SpecDocument =
            serde_yaml::from_str(&absent_target_content).expect("absent target document");
        let absent_lifecycle = match absent_document {
            SpecDocument::Features { features, .. } => features
                .into_iter()
                .flat_map(|feature| feature.bindings)
                .find(|candidate| candidate.id == binding.id)
                .and_then(|candidate| {
                    candidate
                        .targets
                        .into_iter()
                        .find(|candidate| candidate.id == target.id)
                })
                .map(|candidate| candidate.lifecycle),
            _ => None,
        };
        assert_eq!(absent_lifecycle, Some(ArtifactTargetLifecycle::Absent));

        let claim = TargetClaim::Documents {
            anchor: "REQ-AUTH-001#criterion.invalid-credentials"
                .parse()
                .expect("claim anchor"),
        };
        let claim_edit = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Claim {
                operation: NestedEditOperation::Upsert,
                binding_id: claim_binding.id.clone(),
                target_id: claim_target.id.clone(),
                claim_index: claim_target.claims.len(),
                claim: claim.clone(),
            },
        };
        let claim_content = specification_patch_content(&workspace, &feature_path, &claim_edit)
            .expect("claim create");
        assert!(claim_content.contains("documents"));
        let claim_delete = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Claim {
                operation: NestedEditOperation::Delete,
                binding_id: claim_binding.id.clone(),
                target_id: claim_target.id.clone(),
                claim_index: 0,
                claim: claim_target.claims.first().expect("existing claim").clone(),
            },
        };
        let claim_deleted = specification_patch_content(&workspace, &feature_path, &claim_delete)
            .expect("claim delete");
        let claim_deleted_doc: SpecDocument =
            serde_yaml::from_str(&claim_deleted).expect("claim delete document");
        let remaining_claims = match claim_deleted_doc {
            SpecDocument::Features { features, .. } => features
                .into_iter()
                .flat_map(|feature| feature.bindings)
                .find(|candidate| candidate.id == claim_binding.id)
                .and_then(|candidate| {
                    candidate
                        .targets
                        .into_iter()
                        .find(|candidate| candidate.id == claim_target.id)
                })
                .map(|candidate| candidate.claims)
                .expect("claim target after delete"),
            _ => unreachable!(),
        };
        assert!(remaining_claims.is_empty());

        let contract_edit = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Contract {
                operation: NestedEditOperation::Upsert,
                contract: Contract {
                    guarantees: vec![
                        "REQ-AUTH-001#criterion.invalid-credentials"
                            .parse()
                            .expect("guarantee"),
                    ],
                    ..contract.clone()
                },
                current_id: Some(contract.id.to_string()),
            },
        };
        assert!(
            specification_patch_content(&workspace, &feature_path, &contract_edit)
                .expect("contract update")
                .contains("invalid-credentials")
        );
        let contract_delete = EditPatch::Nested {
            item_id: feature_id.to_string(),
            edit: NestedEdit::Contract {
                operation: NestedEditOperation::Delete,
                contract,
                current_id: None,
            },
        };
        let contract_deleted =
            specification_patch_content(&workspace, &feature_path, &contract_delete)
                .expect("contract delete");
        assert!(!contract_deleted.contains("login-http"));

        let malformed = serde_json::from_value::<NestedEdit>(serde_json::json!({
            "entity": "claim",
            "operation": "upsert",
            "binding_id": "schema",
            "target_id": "operation",
            "claim_index": 0,
            "claim": { "kind": "satisfies", "criterion": "REQ-AUTH-001#criterion.invalid-credentials", "unexpected": true }
        }));
        assert!(malformed.is_err(), "unknown claim fields must be rejected");
    }

    #[tokio::test]
    async fn specification_trace_endpoint_returns_server_owned_view() {
        let _workspace_lock = workspace_test_lock().await;
        let app = WorkbenchServer::new(workspace_root().join("fixtures/v1/valid-web-app")).router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/REQ-AUTH-001/trace?depth=4&mode=readable")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let trace: SpecificationTraceView = serde_json::from_slice(&body).expect("trace JSON");
        assert_eq!(trace.root_item_id, "REQ-AUTH-001");
        assert_eq!(trace.mode, "readable");
        assert!(
            trace
                .nodes
                .iter()
                .any(|node| node.kind == "verification-target")
        );
    }

    #[tokio::test]
    async fn workbench_http_projection_readiness_and_esm_flow() {
        let _workspace_lock = workspace_test_lock().await;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf();
        let server = WorkbenchServer::new(root);
        let app = server.router();

        let html = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(html.status(), StatusCode::OK);
        let html = html.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8(html.to_vec()).unwrap();
        assert!(html.contains("type=\"module\" src=\"/assets/js/main.js\""));
        assert!(!html.contains("/assets/projection.js"));

        let readiness = app
            .oneshot(
                Request::builder()
                    .uri("/api/readiness")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(readiness.status(), StatusCode::OK);
        let body = readiness.into_body().collect().await.unwrap().to_bytes();
        let view: ReadinessView = serde_json::from_slice(&body).unwrap();
        assert_eq!(view.status, "Not run");
    }

    #[tokio::test]
    async fn diagnostics_endpoint_runs_the_selected_context_and_persists_the_result() {
        let _workspace_lock = workspace_test_lock().await;
        let app = WorkbenchServer::new(workspace_root()).router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/diagnostics/run",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "context": "workspace"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let validation: ValidationRunView = serde_json::from_slice(&body).unwrap();
        assert_eq!(validation.context, "workspace");
        assert!(!matches!(validation.state, ValidationRunState::NotRun));
        assert_eq!(validation.phases.len(), 5);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(
            projection["diagnostics"]["validation"]["context"],
            "workspace"
        );
        assert_ne!(projection["diagnostics"]["validation"]["state"], "not_run");
    }

    #[tokio::test]
    async fn source_endpoint_returns_an_exact_target_excerpt() {
        let _workspace_lock = workspace_test_lock().await;
        let app = WorkbenchServer::new(workspace_root()).router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/source?target=FEAT-WORKBENCH-GUIDED-JOURNEY-001%23binding.journey%2Ftarget.journey-action")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let source: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(source["path"], "crates/mitase-workbench-server/src/lib.rs");
        assert_eq!(source["is_excerpt"], true);
        assert!(
            source["content"]
                .as_str()
                .unwrap()
                .contains("api_journey_action")
        );
        assert!(source["line_start"].as_u64().unwrap() > 0);
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf()
    }

    async fn projection_and_basis(app: &Router) -> (MutationBasis, String, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response_status = response.status();
        let csrf = response
            .headers()
            .get("x-mitase-csrf-token")
            .and_then(|value| value.to_str().ok())
            .expect("projection csrf token")
            .to_owned();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "projection response: {}",
            String::from_utf8_lossy(&response_body)
        );
        let projection: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
        let basis = MutationBasis {
            expected_revision: projection["snapshot"]["revision"].as_str().unwrap().into(),
            expected_workspace_fingerprint: projection["snapshot"]["fingerprint"]
                .as_str()
                .unwrap()
                .into(),
            expected_source_hash: projection["snapshot"]["source_hash"]
                .as_str()
                .unwrap()
                .into(),
        };
        (basis, csrf, projection)
    }

    fn basis_from_projection(projection: &serde_json::Value) -> MutationBasis {
        serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("projection mutation basis")
    }

    fn execution_from_projection(projection: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "plan_digest": projection["work"]["plan"]["digest"],
            "slice_id": projection["work"]["selected_slice"],
        })
    }

    fn json_files_recursive(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = fs::read_dir(root) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(json_files_recursive(&path));
            } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
                files.push(path);
            }
        }
        files
    }

    async fn json_mutation<T: Serialize>(
        app: &Router,
        method: Method,
        uri: &str,
        csrf: &str,
        value: &T,
    ) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .header("origin", "http://127.0.0.1:7737")
                    .header("x-mitase-csrf-token", csrf)
                    .body(Body::from(serde_json::to_vec(value).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    fn copy_fixture_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).expect("fixture destination");
        for entry in fs::read_dir(source).expect("fixture directory") {
            let entry = entry.expect("fixture entry");
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            if source_path.is_dir() {
                copy_fixture_tree(&source_path, &destination_path);
            } else {
                fs::copy(source_path, destination_path).expect("copy fixture file");
            }
        }
    }

    fn copy_workspace_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).expect("workspace destination");
        for entry in fs::read_dir(source).expect("workspace source") {
            let entry = entry.expect("workspace entry");
            let name = entry.file_name();
            if matches!(name.to_str(), Some(".git" | "target" | "node_modules")) {
                continue;
            }
            let source_path = entry.path();
            let destination_path = destination.join(&name);
            if source_path.is_dir() {
                copy_workspace_tree(&source_path, &destination_path);
            } else {
                fs::copy(source_path, destination_path).expect("copy workspace file");
            }
        }
    }

    /// The specification and config transaction tests must exercise real
    /// workspace writes, but they must never mutate the repository that the
    /// rest of `cargo test --workspace` is concurrently inventorying. Copy the
    /// tracked workspace surface and create a fresh git baseline so candidate
    /// validation sees the same exact targets as production.
    fn isolated_workspace_for_transactions() -> tempfile::TempDir {
        let root = workspace_root();
        let temp = tempfile::tempdir().expect("transaction workspace");
        copy_workspace_tree(&root, temp.path());
        initialize_fixture_git(temp.path());
        temp
    }

    fn initialize_fixture_git(root: &Path) {
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "mitase-tests@example.invalid"],
            vec!["config", "user.name", "Mitase Tests"],
            vec!["add", "."],
            vec!["commit", "-qm", "fixture baseline"],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(root)
                    .status()
                    .expect("run fixture git command")
                    .success()
            );
        }
    }

    #[test]
    fn workbench_snapshot_reuses_unchanged_state_and_invalidates_content_changes() {
        let fixture = workspace_root().join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("snapshot fixture");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());

        let first = server.service.snapshot().expect("initial snapshot");
        let second = server.service.snapshot().expect("reused snapshot");
        assert!(Arc::ptr_eq(&first, &second));

        let source = temp.path().join("src/lib.rs");
        let original = fs::read_to_string(&source).expect("fixture source");
        fs::write(&source, format!("{original}\n// snapshot-one\n")).expect("first edit");
        let third = server.service.snapshot().expect("changed snapshot");
        assert!(!Arc::ptr_eq(&second, &third));
        assert_ne!(
            second.projection.snapshot.fingerprint,
            third.projection.snapshot.fingerprint
        );

        fs::write(&source, format!("{original}\n// snapshot-two\n")).expect("second edit");
        let fourth = server.service.snapshot().expect("same-path content change");
        assert!(!Arc::ptr_eq(&third, &fourth));
        assert_ne!(third.signature, fourth.signature);

        fs::write(temp.path().join("src/new.rs"), "pub fn added() {}\n").expect("untracked edit");
        let fifth = server.service.snapshot().expect("untracked snapshot");
        assert!(!Arc::ptr_eq(&fourth, &fifth));

        {
            let mut session = server.service.session.write().expect("session lock");
            session.work_title = Some("stale work".into());
            session.draft_request = Some(WorkRequest {
                schema: WORK_REQUEST_SCHEMA.into(),
                id: "WORK-SNAPSHOT-GOVERNANCE".into(),
                title: "stale work".into(),
                operation: WorkOperation::Modify,
                origin: WorkOrigin::RequirementCriterion {
                    criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                },
                constraints: WorkConstraints::default(),
                requested_targets: vec![],
            });
        }
        let requirement = temp.path().join("spec/requirement.yaml");
        let requirement_source = fs::read_to_string(&requirement).expect("requirement source");
        fs::write(
            &requirement,
            requirement_source.replace(
                "Keep the fixture behavior valid",
                "Keep the edited fixture behavior valid",
            ),
        )
        .expect("governance edit");
        let sixth = server.service.snapshot().expect("governance snapshot");
        assert!(!Arc::ptr_eq(&fifth, &sixth));
        let session = server.service.session.read().expect("session lock");
        assert!(session.draft_request.is_none());
        assert!(session.plan.is_none());
        assert!(session.work_title.is_none());
    }

    async fn run_fixture_post_state_flow(out_of_scope: bool) -> StatusCode {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());

        let request = WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-POST-STATE".into(),
            title: "modify the fixture behavior".into(),
            operation: mitase_work_model::WorkOperation::Modify,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            },
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let app = WorkbenchServer::new(temp.path().to_path_buf())
            .with_request(request)
            .expect("preloaded fixture request")
            .router();
        let (basis, csrf, request_projection) = projection_and_basis(&app).await;
        assert!(request_projection["work"]["request"].is_object());
        assert!(request_projection["work"]["plan"].is_null());

        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("fixture plan");
        assert_eq!(
            plan.status,
            mitase_work_model::PlanStatus::Ready,
            "{plan:?}"
        );
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("fixture verification slice")
            .id
            .clone();
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/context",
            &csrf,
            &SliceCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/validate",
            &csrf,
            &ValidateCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let validation: ValidationRunView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("fixture pre-state validation");
        assert!(
            matches!(validation.state, ValidationRunState::Passed),
            "{validation:?}"
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/approve",
            &csrf,
            &ApproveCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let source = temp.path().join("src/lib.rs");
        if out_of_scope {
            fs::write(
                temp.path().join("src/unrelated.rs"),
                "pub const UNRELATED: bool = true;\n",
            )
            .expect("modify unrelated fixture source");
        } else {
            fs::write(
                &source,
                "mod removable;\n\npub fn behavior() -> bool {\n    1 == 1\n}\n",
            )
            .expect("modify editable fixture source");
        }

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/verify",
            &csrf,
            &SliceCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let attempt: CompletionAttempt =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("fixture completion attempt");
        let receipt = attempt.receipt.clone().expect("successful receipt");
        assert!(
            receipt
                .executions
                .iter()
                .all(|execution| execution.exit_code == 0)
        );

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/result",
            &csrf,
            &ResultCommand {
                basis,
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: receipt.slice_id.clone(),
                },
                attempt_id: attempt.attempt_id.clone(),
                receipt,
            },
        )
        .await;
        let status = response.status();
        let _ = response.into_body().collect().await.unwrap().to_bytes();
        status
    }

    #[tokio::test]
    async fn workbench_http_post_state_allows_editable_change() {
        let _workspace_lock = workspace_test_lock().await;
        assert_eq!(
            run_fixture_post_state_flow(false).await,
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn workbench_http_result_rejects_out_of_scope_change() {
        let _workspace_lock = workspace_test_lock().await;
        assert_eq!(
            run_fixture_post_state_flow(true).await,
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn workbench_agent_rejects_unrelated_write_before_application() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let request = WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-AGENT".into(),
            title: "scoped agent fixture change".into(),
            operation: mitase_work_model::WorkOperation::Modify,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            },
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let app = WorkbenchServer::new(temp.path().to_path_buf())
            .with_request(request)
            .expect("preloaded fixture request")
            .router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("agent plan");
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("agent slice")
            .id
            .clone();
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/context",
            &csrf,
            &SliceCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/validate",
            &csrf,
            &ValidateCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/approve",
            &csrf,
            &ApproveCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/start",
            &csrf,
            &AgentStartCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let run: AgentRun =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("agent run");
        let target = run
            .context
            .editable_targets
            .first()
            .expect("editable target");
        let source = temp.path().join("src/lib.rs");
        let original_source = fs::read_to_string(&source).unwrap();
        for malicious in [
            "pub fn behavior() -> bool {\n    false\n}\npub fn unapproved() {}\n",
            "pub fn behavior(\n",
        ] {
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/patch",
                &csrf,
                &AgentPatchCommand {
                    basis: basis.clone(),
                    execution: ExecutionIdentity {
                        plan_digest: run.plan_digest.clone(),
                        slice_id: run.slice_id.clone(),
                    },
                    run_id: run.run_id.clone(),
                    patch: AgentPatch {
                        schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id.clone(),
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![mitase_work_model::AgentTargetWrite::Replace {
                            target: target.reference.clone(),
                            expected_excerpt_hash: target.excerpt_hash.clone(),
                            content: malicious.into(),
                        }],
                    },
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::CONFLICT);
            assert_eq!(fs::read_to_string(&source).unwrap(), original_source);
        }
        let replacement = "pub fn behavior() -> bool {\n    false\n}\n".to_owned();
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/patch",
            &csrf,
            &AgentPatchCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id.clone(),
                patch: AgentPatch {
                    schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![mitase_work_model::AgentTargetWrite::Replace {
                        target: target.reference.clone(),
                        expected_excerpt_hash: target.excerpt_hash.clone(),
                        content: replacement,
                    }],
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(fs::read_to_string(&source).unwrap().contains("false"));
        let before_rejected = fs::read_to_string(&source).unwrap();
        let (fresh_basis, _, _) = projection_and_basis(&app).await;
        let unrelated = BoundTargetRef {
            binding: target.reference.binding.clone(),
            target_id: mitase_spec_model::LocalId("unrelated".into()),
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/patch",
            &csrf,
            &AgentPatchCommand {
                basis: fresh_basis,
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id.clone(),
                patch: AgentPatch {
                    schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![mitase_work_model::AgentTargetWrite::Replace {
                        target: unrelated,
                        expected_excerpt_hash: target.excerpt_hash.clone(),
                        content: "unrelated".into(),
                    }],
                },
            },
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::CONFLICT,
            "{}",
            String::from_utf8_lossy(&response_body)
        );
        assert_eq!(fs::read_to_string(source).unwrap(), before_rejected);

        let (basis, _, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/blocker",
            &csrf,
            &AgentBlockerCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id.clone(),
                blocker: AgentBlocker {
                    code: "MITASE-AGENT-TEST".into(),
                    message: "test blocker".into(),
                    next_action: "request review".into(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/scope-expansion",
            &csrf,
            &AgentScopeExpansionCommand {
                basis,
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id,
                reason: "the test needs a second target".into(),
                requested_targets: vec![target.reference.clone()],
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let projection = app
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let projection: serde_json::Value =
            serde_json::from_slice(&projection.into_body().collect().await.unwrap().to_bytes())
                .expect("agent projection");
        assert!(projection["work"]["agent"].is_object());
        assert_eq!(projection["work"]["agent"]["status"], "blocked");
        assert!(projection["work"]["agent_events"].as_array().unwrap().len() >= 5);

        let restarted_server = WorkbenchServer::new(temp.path().to_path_buf());
        let restarted = restarted_server.router();
        let projection = restarted
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let projection: serde_json::Value =
            serde_json::from_slice(&projection.into_body().collect().await.unwrap().to_bytes())
                .expect("restarted agent projection");
        assert_eq!(projection["work"]["agent"]["status"], "blocked");

        {
            let mut session = restarted_server
                .service
                .session
                .write()
                .expect("restarted session lock");
            session.work_title = Some("a subsequent work request".into());
            session.draft_request = Some(WorkRequest {
                schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
                id: "WORK-FIXTURE-NEXT".into(),
                title: "a subsequent work request".into(),
                operation: mitase_work_model::WorkOperation::Modify,
                origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                    criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                },
                constraints: Default::default(),
                requested_targets: vec![],
            });
        }
        let projection: serde_json::Value = serde_json::from_slice(
            &restarted
                .oneshot(
                    Request::builder()
                        .uri("/api/projection")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .expect("subsequent request projection");
        assert!(projection["work"]["agent"].is_null());
    }

    fn activate_lifecycle_feature_fixture(root: &Path, target_id: &str) {
        let feature_id = match target_id {
            "behavior" => "FEAT-FIXTURE-001",
            "added-symbol" => "FEAT-LIFECYCLE-ADD-SYMBOL-001",
            "added-file" => "FEAT-LIFECYCLE-ADD-FILE-001",
            "removed-symbol" => "FEAT-LIFECYCLE-REMOVE-SYMBOL-001",
            "removed-file" => "FEAT-LIFECYCLE-REMOVE-FILE-001",
            _ => unreachable!("declared lifecycle fixture target"),
        };
        let feature_path = root.join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path).expect("lifecycle feature fixture");
        let marker = format!("  - id: {feature_id}");
        let start = feature
            .find(&marker)
            .expect("lifecycle feature fixture entry");
        let end = feature[start + marker.len()..]
            .find("\n  - id:")
            .map(|offset| start + marker.len() + offset)
            .unwrap_or(feature.len());
        let mut section = feature[start..end].to_owned();
        section = section.replace("status: planned", "status: implemented");
        if matches!(target_id, "removed-symbol" | "removed-file") {
            section = section.replace("lifecycle: absent", "lifecycle: present");
        }
        let mut updated = feature;
        updated.replace_range(start..end, &section);
        fs::write(feature_path, updated).expect("activate lifecycle feature fixture");
        if target_id == "added-file" {
            let source_path = root.join("src/lib.rs");
            let mut source = fs::read_to_string(&source_path).expect("lifecycle source fixture");
            source.push_str(
                "\n// Keep the planned file target visible to inventory once it is created.\n#[cfg(any())]\n#[path = \"added.rs\"]\nmod added;\n",
            );
            fs::write(source_path, source).expect("activate added-file inventory fixture");
        }
    }

    async fn start_lifecycle_agent(
        server: &WorkbenchServer,
        app: &Router,
        target_id: &str,
        transition: mitase_work_model::TargetTransition,
    ) -> (
        MutationBasis,
        String,
        String,
        AgentRun,
        mitase_work_model::AgentTargetDigest,
    ) {
        let (feature, binding, criterion) = match target_id {
            "behavior" => ("FEAT-FIXTURE-001", "implementation", "behavior"),
            "added-symbol" => ("FEAT-LIFECYCLE-ADD-SYMBOL-001", "lifecycle", "add-symbol"),
            "added-file" => ("FEAT-LIFECYCLE-ADD-FILE-001", "lifecycle", "add-file"),
            "removed-symbol" => (
                "FEAT-LIFECYCLE-REMOVE-SYMBOL-001",
                "lifecycle",
                "remove-symbol",
            ),
            "removed-file" => ("FEAT-LIFECYCLE-REMOVE-FILE-001", "lifecycle", "remove-file"),
            _ => unreachable!("declared lifecycle case"),
        };
        let target: BoundTargetRef = format!("{feature}#binding.{binding}/target.{target_id}")
            .parse()
            .expect("lifecycle target");
        let operation = match transition {
            mitase_work_model::TargetTransition::Add => mitase_work_model::WorkOperation::Add,
            mitase_work_model::TargetTransition::Modify => mitase_work_model::WorkOperation::Modify,
            mitase_work_model::TargetTransition::Remove => mitase_work_model::WorkOperation::Remove,
            mitase_work_model::TargetTransition::RunOnly
            | mitase_work_model::TargetTransition::Readonly => unreachable!("editable lifecycle"),
        };
        let constraints = if transition == mitase_work_model::TargetTransition::Add {
            mitase_work_model::WorkConstraints {
                max_added_bytes_per_target: Some(512),
                max_added_lines_per_target: Some(32),
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let request = WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: format!("WORK-FIXTURE-LIFECYCLE-{target_id}"),
            title: format!("apply {transition:?} to {target_id}"),
            operation,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: format!("REQ-FIXTURE-001#criterion.{criterion}")
                    .parse()
                    .unwrap(),
            },
            constraints,
            requested_targets: vec![mitase_work_model::RequestedTarget {
                reference: target.clone(),
                criterion: Some(
                    format!("REQ-FIXTURE-001#criterion.{criterion}")
                        .parse()
                        .unwrap(),
                ),
                transition,
            }],
        };
        start_lifecycle_agent_with_request(server, app, target, request).await
    }

    async fn start_lifecycle_agent_with_request(
        server: &WorkbenchServer,
        app: &Router,
        target: BoundTargetRef,
        request: WorkRequest,
    ) -> (
        MutationBasis,
        String,
        String,
        AgentRun,
        mitase_work_model::AgentTargetDigest,
    ) {
        if let Ok(mut session) = server.service.session.write() {
            session.draft_request = Some(request);
        }
        let (basis, csrf, _) = projection_and_basis(app).await;
        let response = json_mutation(app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let plan: WorkPlan = serde_json::from_slice(&body).expect("lifecycle plan");
        assert_eq!(
            plan.status,
            mitase_work_model::PlanStatus::Ready,
            "{plan:?}"
        );
        let slice = plan
            .slices
            .iter()
            .find(|slice| {
                slice
                    .editable_targets
                    .iter()
                    .any(|planned| planned.reference == target)
            })
            .expect("lifecycle slice")
            .id
            .clone();
        let response = json_mutation(
            app,
            Method::POST,
            "/api/work/context",
            &csrf,
            &SliceCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            app,
            Method::POST,
            "/api/work/validate",
            &csrf,
            &ValidateCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            app,
            Method::POST,
            "/api/work/approve",
            &csrf,
            &ApproveCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            app,
            Method::POST,
            "/api/work/agent/start",
            &csrf,
            &AgentStartCommand {
                basis: basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let run: AgentRun =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("lifecycle agent run");
        let digest = run
            .context
            .editable_targets
            .iter()
            .find(|planned| planned.reference == target)
            .expect("lifecycle target digest")
            .clone();
        (basis, csrf, slice, run, digest)
    }

    async fn create_approved_work_request(
        server: &WorkbenchServer,
        anchor: &str,
        summary: &str,
    ) -> WorkRequest {
        let app = server.router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": {
                    "kind": "requirement-criterion",
                    "criterion": anchor,
                },
                "title": summary,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        server
            .service
            .session
            .read()
            .expect("workbench session lock")
            .draft_request
            .clone()
            .expect("created work request")
    }

    #[tokio::test]
    #[ignore = "pre-v1 cutover: planned origins are rejected before Work creation"]
    async fn planned_remove_target_flows_from_suggestion_to_finalization() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("lifecycle suggestion tempdir");
        copy_fixture_tree(&fixture, temp.path());
        let config_path = temp.path().join("mitase.yaml");
        let config = fs::read_to_string(&config_path).expect("fixture config");
        fs::write(
            config_path,
            config.replace("target: off", "target: traceable"),
        )
        .expect("enable readiness");
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let app = server.router();
        let suggestion_path =
            "/api/specifications/REQ-FIXTURE-001%23criterion.remove-symbol/target-suggestions";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("remove target suggestions");
        let candidate = suggestions
            .suggestions
            .iter()
            .find(|candidate| {
                candidate
                    .reference
                    .to_string()
                    .contains("FEAT-LIFECYCLE-REMOVE-SYMBOL-001")
            })
            .expect("planned remove suggestion");
        assert_eq!(
            candidate.transition,
            mitase_work_model::TargetTransition::Remove
        );
        assert_eq!(
            candidate.lifecycle,
            mitase_work_model::TargetLifecycle::EnsureAbsent
        );
        let (basis, csrf, _) = projection_and_basis(&app).await;
        assert!(
            server
                .service
                .session
                .read()
                .unwrap()
                .draft_request
                .is_none()
        );
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis,
                suggestion_token: suggestions.suggestion_token,
                suggestion_ids: vec![candidate.id.clone()],
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("remove suggestion approval");
        assert_eq!(approval.approved_ids, vec![candidate.id.clone()]);
        let request = create_approved_work_request(
            &server,
            "REQ-FIXTURE-001#criterion.remove-symbol",
            "remove the approved symbol",
        )
        .await;
        assert_eq!(request.operation, mitase_work_model::WorkOperation::Remove);
        let target = request.requested_targets[0].reference.clone();
        let (basis, csrf, slice, run, target_digest) =
            start_lifecycle_agent_with_request(&server, &app, target.clone(), request).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/patch",
            &csrf,
            &AgentPatchCommand {
                basis,
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id.clone(),
                patch: AgentPatch {
                    schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![mitase_work_model::AgentTargetWrite::Remove {
                        target: target_digest.reference.clone(),
                        expected_excerpt_hash: target_digest.excerpt_hash.clone(),
                    }],
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let (post_basis, post_csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/verify",
            &post_csrf,
            &SliceCommand {
                basis: post_basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let attempt: CompletionAttempt =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("remove completion attempt");
        assert_eq!(
            attempt.report.status,
            mitase_work_model::CompletionStatus::Complete
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/finalize/preview",
            &post_csrf,
            &FinalizeCommand {
                basis: post_basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: attempt.plan_digest.clone(),
                    slice_id: attempt.slice_id.clone(),
                },
                attempt_id: attempt.attempt_id.clone(),
                preview_token: None,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let preview: FinalizationPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("remove finalization preview");
        assert_eq!(
            preview.status,
            mitase_work_model::CompletionStatus::Complete
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/finalize/apply",
            &post_csrf,
            &FinalizeCommand {
                basis: post_basis,
                execution: ExecutionIdentity {
                    plan_digest: attempt.plan_digest.clone(),
                    slice_id: attempt.slice_id.clone(),
                },
                attempt_id: attempt.attempt_id,
                preview_token: Some(preview.preview_token),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn planned_verification_add_runs_its_exact_runner_before_finalization() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("verification add tempdir");
        copy_fixture_tree(&fixture, temp.path());
        let feature_path = temp.path().join("spec/feature.yaml");
        let mut feature = fs::read_to_string(&feature_path).expect("feature spec");
        feature = feature.replacen(
            "  - id: FEAT-LIFECYCLE-REMOVE-SYMBOL-001",
            concat!(
                "  - id: FEAT-LIFECYCLE-ADD-VERIFICATION-001\n",
                "    title: Add an exact verification test\n",
                "    summary: A planned verification target is written before it runs.\n",
                "    status: planned\n",
                "    bindings:\n",
                "      - id: implementation\n",
                "        role: implementation\n",
                "        facet: work\n",
                "        responsibility: Keep the planned implementation connected to its verification proof.\n",
                "        targets:\n",
                "          - id: implementation\n",
                "            adapter: rust\n",
                "            path: src/lib.rs\n",
                "            selector: { kind: symbol, name: behavior }\n",
                "            claims:\n",
                "              - kind: satisfies\n",
                "                criterion: REQ-FIXTURE-001#criterion.behavior\n",
                "      - id: verification\n",
                "        role: verification\n",
                "        facet: verification\n",
                "        responsibility: Prove the add transition with its exact runner.\n",
                "        targets:\n",
                "          - id: added-verification\n",
                "            adapter: rust\n",
                "            path: tests/behavior.rs\n",
                "            selector: { kind: symbol, name: added_verification_lifecycle_stays_valid }\n",
                "            claims:\n",
                "              - kind: verifies\n",
                "                criterion: REQ-FIXTURE-001#criterion.behavior\n",
                "                covers: [FEAT-FIXTURE-001#binding.implementation/target.behavior, FEAT-LIFECYCLE-ADD-VERIFICATION-001#binding.implementation/target.implementation]\n",
                "                runner: { runner: cargo-test, arguments: { package: workbench-flow-fixture, test: added_verification_lifecycle_stays_valid } }\n",
                "  - id: FEAT-LIFECYCLE-REMOVE-SYMBOL-001"
            ),
            1,
        );
        fs::write(feature_path, feature).expect("verification feature");
        let requirement_path = temp.path().join("spec/requirement.yaml");
        let mut requirement = fs::read_to_string(&requirement_path).expect("requirement spec");
        requirement = requirement.replacen(
            concat!(
                "criterion: REQ-FIXTURE-001#criterion.add-symbol\n",
                "                covers: [FEAT-LIFECYCLE-ADD-SYMBOL-001#binding.lifecycle/target.added-symbol]",
            ),
            concat!(
                "criterion: REQ-FIXTURE-001#criterion.behavior\n",
                "                covers: [FEAT-FIXTURE-001#binding.implementation/target.behavior]",
            ),
            1,
        );
        fs::write(requirement_path, requirement).expect("verification requirement");
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let app = server.router();
        let suggestion_path =
            "/api/specifications/REQ-FIXTURE-001%23criterion.behavior/target-suggestions";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verification add suggestions");
        let candidate = suggestions
            .suggestions
            .iter()
            .find(|candidate| {
                candidate
                    .reference
                    .to_string()
                    .contains("FEAT-LIFECYCLE-ADD-VERIFICATION-001")
            })
            .expect("planned verification add suggestion");
        assert_eq!(
            candidate.transition,
            mitase_work_model::TargetTransition::Add
        );
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis,
                suggestion_token: suggestions.suggestion_token,
                suggestion_ids: vec![candidate.id.clone()],
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verification add approval");
        assert_eq!(approval.approved_ids, vec![candidate.id.clone()]);
        let request = create_approved_work_request(
            &server,
            "REQ-FIXTURE-001#criterion.behavior",
            "add the approved verification target",
        )
        .await;
        assert_eq!(request.operation, mitase_work_model::WorkOperation::Add);
        let target = request.requested_targets[0].reference.clone();
        let (basis, csrf, slice, run, target_digest) =
            start_lifecycle_agent_with_request(&server, &app, target.clone(), request).await;
        assert_eq!(
            target_digest.transition,
            mitase_work_model::TargetTransition::Add
        );
        assert_eq!(
            target_digest.access,
            mitase_work_model::TargetAccessMode::Editable
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/patch",
            &csrf,
            &AgentPatchCommand {
                basis,
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: run.slice_id.clone(),
                },
                run_id: run.run_id.clone(),
                patch: AgentPatch {
                    schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![mitase_work_model::AgentTargetWrite::AddToFile {
                        target: target_digest.reference.clone(),
                        expected_path_hash: target_digest
                            .container_content_hash
                            .clone()
                            .expect("verification container digest"),
                        content: "\n#[test]\nfn added_verification_lifecycle_stays_valid() {\n    assert!(workbench_flow_fixture::behavior());\n}\n".into(),
                    }],
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let (post_basis, post_csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/verify",
            &post_csrf,
            &SliceCommand {
                basis: post_basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: run.plan_digest.clone(),
                    slice_id: slice.clone(),
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let attempt: CompletionAttempt =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verification add attempt");
        assert_eq!(
            attempt.report.status,
            mitase_work_model::CompletionStatus::Complete
        );
        let receipt = attempt.receipt.as_ref().expect("verification add receipt");
        assert!(receipt.executions.iter().any(|execution| {
            execution.target == target
                && execution.proof.identity == "added_verification_lifecycle_stays_valid"
        }));
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/finalize/preview",
            &post_csrf,
            &FinalizeCommand {
                basis: post_basis.clone(),
                execution: ExecutionIdentity {
                    plan_digest: attempt.plan_digest.clone(),
                    slice_id: attempt.slice_id.clone(),
                },
                attempt_id: attempt.attempt_id.clone(),
                preview_token: None,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let preview: FinalizationPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verification add preview");
        assert_eq!(
            preview.status,
            mitase_work_model::CompletionStatus::Complete
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/finalize/apply",
            &post_csrf,
            &FinalizeCommand {
                basis: post_basis,
                execution: ExecutionIdentity {
                    plan_digest: attempt.plan_digest.clone(),
                    slice_id: attempt.slice_id.clone(),
                },
                attempt_id: attempt.attempt_id,
                preview_token: Some(preview.preview_token),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn workbench_agent_applies_all_approved_lifecycle_writes() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let cases = [
            (
                "behavior",
                mitase_work_model::TargetTransition::Modify,
                "modify existing symbol",
            ),
            (
                "added-symbol",
                mitase_work_model::TargetTransition::Add,
                "add symbol to existing file",
            ),
            (
                "added-file",
                mitase_work_model::TargetTransition::Add,
                "add new file",
            ),
            (
                "removed-symbol",
                mitase_work_model::TargetTransition::Remove,
                "remove symbol",
            ),
            (
                "removed-file",
                mitase_work_model::TargetTransition::Remove,
                "remove file",
            ),
        ];
        for (target_id, transition, description) in cases {
            let temp = tempfile::tempdir().expect("lifecycle fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            // Exercise the approved lifecycle write path after admission.
            activate_lifecycle_feature_fixture(temp.path(), target_id);
            if target_id == "removed-symbol" {
                let config_path = temp.path().join("mitase.yaml");
                let config = fs::read_to_string(&config_path).expect("fixture config");
                fs::write(
                    config_path,
                    config.replace("target: off", "target: traceable"),
                )
                .expect("enable readiness for remove-symbol lifecycle");
            }
            initialize_fixture_git(temp.path());
            let server = WorkbenchServer::new(temp.path().to_path_buf());
            let app = server.router();
            let (basis, csrf, slice, run, target) =
                start_lifecycle_agent(&server, &app, target_id, transition).await;
            assert_eq!(target.transition, transition, "{description}");
            let write = match target_id {
                "behavior" => mitase_work_model::AgentTargetWrite::Replace {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                    content: "pub fn behavior() -> bool {\n    1 == 1\n}".into(),
                },
                "added-symbol" => mitase_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool {\n    true\n}\n".into(),
                },
                "added-file" => mitase_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn added_file() {}\n".into(),
                },
                "removed-symbol" => mitase_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => mitase_work_model::AgentTargetWrite::RemoveFile {
                    target: target.reference.clone(),
                    expected_content_hash: target.content_hash.clone(),
                },
                _ => unreachable!("declared lifecycle case"),
            };
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/patch",
                &csrf,
                &AgentPatchCommand {
                    basis,
                    execution: ExecutionIdentity {
                        plan_digest: run.plan_digest.clone(),
                        slice_id: run.slice_id.clone(),
                    },
                    run_id: run.run_id.clone(),
                    patch: AgentPatch {
                        schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id.clone(),
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![write],
                    },
                },
            )
            .await;
            let status = response.status();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(
                status,
                StatusCode::OK,
                "{description}: {}",
                String::from_utf8_lossy(&body)
            );
            let patch: mitase_work_model::AgentPatchRecord =
                serde_json::from_slice(&body).expect("lifecycle patch record");
            assert_eq!(patch.changes.len(), 1, "{description}");
            assert_eq!(patch.changes[0].transition, transition, "{description}");
            assert_eq!(
                patch.changes[0].lifecycle, target.lifecycle,
                "{description}"
            );
            let (post_basis, post_csrf, _) = projection_and_basis(&app).await;
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/verify",
                &post_csrf,
                &SliceCommand {
                    basis: post_basis.clone(),
                    execution: ExecutionIdentity {
                        plan_digest: run.plan_digest.clone(),
                        slice_id: slice.clone(),
                    },
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{description}");
            let attempt: CompletionAttempt =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("lifecycle completion attempt");
            assert_eq!(
                attempt.report.status,
                mitase_work_model::CompletionStatus::Complete,
                "{description}: {attempt:?}"
            );
            let receipt = attempt.receipt.expect("lifecycle receipt");
            assert_eq!(receipt.lifecycle_proofs.len(), 1, "{description}");
            assert_eq!(receipt.lifecycle_proofs[0].reference, target.reference);
            assert_eq!(receipt.lifecycle_proofs[0].transition, transition);

            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/finalize/preview",
                &post_csrf,
                &FinalizeCommand {
                    basis: post_basis.clone(),
                    execution: ExecutionIdentity {
                        plan_digest: attempt.plan_digest.clone(),
                        slice_id: attempt.slice_id.clone(),
                    },
                    attempt_id: attempt.attempt_id.clone(),
                    preview_token: None,
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{description}");
            let preview: FinalizationPreview =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("lifecycle finalization preview");
            assert_eq!(
                preview.status,
                mitase_work_model::CompletionStatus::Complete
            );
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/finalize/apply",
                &post_csrf,
                &FinalizeCommand {
                    basis: post_basis,
                    execution: ExecutionIdentity {
                        plan_digest: attempt.plan_digest.clone(),
                        slice_id: attempt.slice_id.clone(),
                    },
                    attempt_id: attempt.attempt_id,
                    preview_token: Some(preview.preview_token),
                },
            )
            .await;
            let status = response.status();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(
                status,
                StatusCode::OK,
                "{description}: {}",
                String::from_utf8_lossy(&body)
            );
            let finalization: mitase_work_model::FinalizationReceipt =
                serde_json::from_slice(&body).expect("lifecycle finalization receipt");
            assert_eq!(finalization.lifecycle_proofs.len(), 1, "{description}");
            assert_eq!(finalization.lifecycle_proofs[0].reference, target.reference);
            if target_id == "removed-symbol" {
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(readiness.status(), StatusCode::OK, "{description}");
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("post-finalization readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");
                let removed_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("finalized absent target readiness subject");
                assert!(removed_subject.ready, "{removed_subject:?}");

                // Absence evidence is durable across the commit that records
                // finalization. It must not be tied to the exact HEAD or
                // workspace fingerprint that existed immediately after the
                // lifecycle write.
                for args in [
                    vec!["add", "-A"],
                    vec!["commit", "-qm", "record finalized absence"],
                ] {
                    assert!(
                        Command::new("git")
                            .args(args)
                            .current_dir(temp.path())
                            .status()
                            .expect("record finalized absence commit")
                            .success()
                    );
                }
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("post-commit readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");

                // An unrelated, valid commit must also leave the historical
                // lifecycle proof usable. Only the current exact absence
                // obligation is relevant to this readiness decision.
                let unrelated = temp.path().join("src/unrelated.rs");
                let unrelated_content =
                    fs::read_to_string(&unrelated).expect("unrelated fixture source");
                fs::write(&unrelated, unrelated_content.replace("false", "true"))
                    .expect("unrelated approved change");
                for args in [
                    vec!["add", "src/unrelated.rs"],
                    vec!["commit", "-qm", "record unrelated approved change"],
                ] {
                    assert!(
                        Command::new("git")
                            .args(args)
                            .current_dir(temp.path())
                            .status()
                            .expect("record unrelated change commit")
                            .success()
                    );
                }
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("unrelated-change readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");

                // Readiness must reject a forged finalization record even
                // when its post-state fingerprint still matches. The
                // attempt, approval, exact Remove slice, and receipt proof
                // are the durable closure, not this top-level JSON alone.
                let store = DeliveryStore::for_workspace(temp.path()).expect("evidence store");
                let finalization_path =
                    json_files_recursive(&store.root().join("completion/v1/finalizations"))
                        .into_iter()
                        .next()
                        .expect("finalization evidence");
                let mut forged: serde_json::Value = serde_json::from_slice(
                    &fs::read(&finalization_path).expect("read finalization evidence"),
                )
                .expect("parse finalization evidence");
                forged["lifecycle_proofs"][0]["transition"] = serde_json::json!("add");
                fs::write(
                    &finalization_path,
                    serde_json::to_vec_pretty(&forged).expect("serialize forged evidence"),
                )
                .expect("forge finalization evidence");
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("forged-evidence readiness");
                let forged_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("forged finalized absent target subject");
                assert!(!forged_subject.ready, "{forged_subject:?}");

                let source = temp.path().join("src/removable.rs");
                let mut restored = fs::read_to_string(&source).expect("removed source");
                restored.push_str("\npub fn remove_me() {}\n");
                fs::write(&source, restored).expect("restore removed target");
                let readiness = app
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness_status = readiness.status();
                let readiness_body = readiness.into_body().collect().await.unwrap().to_bytes();
                assert_eq!(
                    readiness_status,
                    StatusCode::OK,
                    "{description}: {}",
                    String::from_utf8_lossy(&readiness_body)
                );
                let readiness: ReadinessView =
                    serde_json::from_slice(&readiness_body).expect("restored-target readiness");
                let restored_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("restored target readiness subject");
                assert!(!restored_subject.ready, "{restored_subject:?}");
                assert!(
                    restored_subject
                        .blockers
                        .iter()
                        .any(|blocker| blocker.contains("finalized lifecycle proof"))
                );
            }
        }
    }

    #[tokio::test]
    async fn workbench_agent_rejects_stale_or_newly_existing_lifecycle_targets() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let cases = [
            (
                "added-symbol",
                mitase_work_model::TargetTransition::Add,
                "src/lib.rs",
                "\npub fn added_behavior() -> bool { true }\n",
                "now exists",
            ),
            (
                "added-file",
                mitase_work_model::TargetTransition::Add,
                "src/added.rs",
                "pub fn preexisting_file() {}\n",
                "now exists",
            ),
            (
                "removed-symbol",
                mitase_work_model::TargetTransition::Remove,
                "src/removable.rs",
                "pub fn remove_me() { panic!(\"changed\") }\n",
                "is stale",
            ),
            (
                "removed-file",
                mitase_work_model::TargetTransition::Remove,
                "remove-file.txt",
                "changed before approved removal\n",
                "is stale",
            ),
        ];
        for (target_id, transition, path, changed_content, expected_blocker) in cases {
            let temp = tempfile::tempdir().expect("lifecycle precondition fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            // Exercise agent-time stale-state rejection after admission.
            activate_lifecycle_feature_fixture(temp.path(), target_id);
            initialize_fixture_git(temp.path());
            let server = WorkbenchServer::new(temp.path().to_path_buf());
            let app = server.router();
            let (_, _, _, run, target) =
                start_lifecycle_agent(&server, &app, target_id, transition).await;

            let changed_path = temp.path().join(path);
            if target_id == "added-symbol" {
                let mut content = fs::read_to_string(&changed_path).expect("existing source");
                content.push_str(changed_content);
                fs::write(&changed_path, content).expect("create approved target early");
            } else {
                fs::write(&changed_path, changed_content).expect("change lifecycle target");
            }
            let (basis, csrf, _) = projection_and_basis(&app).await;
            let write = match target_id {
                "added-symbol" => mitase_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool { true }\n".into(),
                },
                "added-file" => mitase_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn approved_file() {}\n".into(),
                },
                "removed-symbol" => mitase_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => mitase_work_model::AgentTargetWrite::RemoveFile {
                    target: target.reference.clone(),
                    expected_content_hash: target.content_hash.clone(),
                },
                _ => unreachable!("declared lifecycle precondition case"),
            };
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/patch",
                &csrf,
                &AgentPatchCommand {
                    basis,
                    execution: ExecutionIdentity {
                        plan_digest: run.plan_digest.clone(),
                        slice_id: run.slice_id.clone(),
                    },
                    run_id: run.run_id.clone(),
                    patch: AgentPatch {
                        schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id,
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![write],
                    },
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::CONFLICT, "{target_id}");
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert!(
                String::from_utf8_lossy(&body).contains(expected_blocker),
                "{target_id}: {}",
                String::from_utf8_lossy(&body)
            );
            assert_eq!(
                fs::read_to_string(&changed_path).expect("rejected write preserves target"),
                if target_id == "added-symbol" {
                    let mut expected =
                        fs::read_to_string(fixture.join(path)).expect("fixture source");
                    expected.push_str(changed_content);
                    expected
                } else {
                    changed_content.to_string()
                },
                "{target_id}"
            );
        }
    }

    #[tokio::test]
    async fn advisory_specification_discovery_is_multilingual_and_never_creates_scope() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();

        let (_, _, projection) = projection_and_basis(&app).await;
        let feature = projection["specifications"]["specifications"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["id"] == "FEAT-FIXTURE-001"))
            .expect("fixture feature projection");
        let feature_capabilities = feature["origin_capabilities"]
            .as_array()
            .expect("feature origin capabilities");
        assert!(feature_capabilities.iter().any(|capability| {
            capability["label"] == "Feature implementation"
                && capability["enabled"] == true
                && capability["origin"]["kind"] == "feature-implementation-binding"
        }));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/candidates?q=%E6%8C%AF%E3%82%8B%E8%88%9E%E3%81%84%E3%82%92%E6%A4%9C%E8%A8%BC&kind=requirement")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let candidates: Vec<SpecificationCandidateView> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("multilingual candidate response");
        let fixture_candidate = candidates
            .iter()
            .find(|candidate| candidate.item.id == "REQ-FIXTURE-001")
            .expect("Japanese intent finds the English fixture requirement");
        assert!(
            fixture_candidate
                .evidence
                .iter()
                .any(|entry| entry.source == "semantic")
        );
        assert!(
            fixture_candidate
                .evidence
                .iter()
                .any(|entry| entry.source == "graph")
        );
        assert!(
            fixture_candidate
                .evidence
                .iter()
                .any(|entry| entry.source == "history")
        );
        assert_eq!(
            fixture_candidate.stable_anchors,
            vec![
                "REQ-FIXTURE-001#criterion.behavior",
                "REQ-FIXTURE-001#criterion.add-symbol",
                "REQ-FIXTURE-001#criterion.add-file",
                "REQ-FIXTURE-001#criterion.remove-symbol",
                "REQ-FIXTURE-001#criterion.remove-file",
            ]
        );
        assert!(
            service.session.read().unwrap().draft_request.is_none(),
            "discovery results must remain advisory"
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/candidates?q=function&kind=requirement")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let synonyms: Vec<SpecificationCandidateView> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("synonym candidate response");
        assert!(synonyms.iter().any(|candidate| {
            candidate.item.id == "REQ-FIXTURE-001"
                && candidate
                    .evidence
                    .iter()
                    .any(|entry| entry.source == "semantic")
        }));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/candidates?q=orchard&kind=requirement")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let no_match: Vec<SpecificationCandidateView> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("no-match response");
        assert!(
            no_match.is_empty(),
            "unrelated intent must not become a false positive"
        );

        for query in ["contest", "invalid", "authorization"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!(
                            "/api/specifications/candidates?q={query}&kind=requirement"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let embedded_token: Vec<SpecificationCandidateView> =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("embedded-token response");
            assert!(
                embedded_token.iter().all(|candidate| {
                    candidate
                        .evidence
                        .iter()
                        .all(|entry| entry.source != "semantic")
                }),
                "{query} must not produce semantic evidence from an embedded token"
            );
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/candidates?q=behav&kind=requirement")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let lexical: Vec<SpecificationCandidateView> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("substring candidate response");
        let lexical_candidate = lexical
            .iter()
            .find(|candidate| candidate.item.id == "REQ-FIXTURE-001")
            .expect("substring query finds behavior");
        assert!(
            lexical_candidate
                .evidence
                .iter()
                .any(|entry| { entry.source == "lexical" && entry.detail.contains("substring") })
        );
        assert!(discovery_query_exact_matches("Behavior", "behavior"));
        assert!(discovery_query_exact_matches("BEHAVIOR", "behavior"));
        assert!(!discovery_query_exact_matches("Behavior", "behav"));
        assert!(
            matching_discovery_concepts("test")
                .iter()
                .any(|concept| concept.label == "validation")
        );
        assert!(
            !matching_discovery_concepts("contest")
                .iter()
                .any(|concept| concept.label == "validation")
        );
        assert!(
            !matching_discovery_concepts("invalid")
                .iter()
                .any(|concept| concept.label == "validation")
        );

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#binding.verification" },
                "title": "Do not accept a non-criterion anchor"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            service.session.read().unwrap().draft_request.is_none(),
            "an unapproved or non-criterion discovery result cannot create scope"
        );
    }
    #[tokio::test]
    async fn feature_origin_capability_creates_work_with_exact_binding() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let (basis, csrf, projection) = projection_and_basis(&app).await;
        let feature = projection["specifications"]["specifications"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["id"] == "FEAT-FIXTURE-001"))
            .expect("fixture feature projection");
        let origin = feature["origin_capabilities"]
            .as_array()
            .and_then(|capabilities| {
                capabilities.iter().find_map(|capability| {
                    (capability["label"] == "Feature implementation"
                        && capability["enabled"] == true)
                        .then(|| capability["origin"].clone())
                })
            })
            .expect("enabled Feature binding origin");
        let selected_origin: WorkOrigin =
            serde_json::from_value(origin.clone()).expect("server-projected Feature origin");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": origin,
                "title": "Focus the fixture implementation"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let created: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("Feature-origin Work projection");
        assert_eq!(
            created["work"]["request"]["origin"]["kind"],
            "feature-implementation-binding"
        );
        assert_eq!(created["work"]["request"]["requested_target_count"], 1);
        let request = service
            .session
            .read()
            .expect("Feature-origin session")
            .draft_request
            .clone()
            .expect("created Feature-origin request");
        assert_eq!(request.origin, selected_origin);
        let expected_targets = match &selected_origin {
            WorkOrigin::FeatureImplementationBinding { targets, .. } => targets.clone(),
            _ => panic!("expected a Feature implementation binding origin"),
        };
        assert_eq!(
            request
                .requested_targets
                .iter()
                .map(|target| target.reference.clone())
                .collect::<Vec<_>>(),
            expected_targets,
            "Work must retain the exact server-projected target boundary"
        );

        let mut broadened = request.clone();
        if let WorkOrigin::FeatureImplementationBinding { targets, .. } = &mut broadened.origin {
            targets.clear();
        }
        let workspace = SpecWorkspace::load(temp.path()).expect("fixture workspace");
        let index = workspace.index().expect("fixture index");
        let revision = service
            .snapshot()
            .expect("fixture snapshot")
            .revision
            .clone();
        let plan = mitase_planner::plan(&broadened, &workspace, &index, &revision)
            .expect("invalid exact origin becomes a blocked plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("exact Work origin is invalid") })
        );
    }

    #[tokio::test]
    async fn feature_origin_target_capabilities_keep_each_criterion_selectable() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("multi-criterion fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());

        let feature_path = temp.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path).expect("feature spec");
        fs::write(
            &feature_path,
            feature.replacen(
                concat!(
                    "              - kind: satisfies\n",
                    "                criterion: REQ-FIXTURE-001#criterion.behavior\n",
                ),
                concat!(
                    "              - kind: satisfies\n",
                    "                criterion: REQ-FIXTURE-001#criterion.behavior\n",
                    "          - id: journey-source\n",
                    "            adapter: rust\n",
                    "            path: src/lib.rs\n",
                    "            selector: { kind: symbol, name: behavior }\n",
                    "            claims:\n",
                    "              - kind: satisfies\n",
                    "                criterion: REQ-FIXTURE-001#criterion.add-symbol\n",
                ),
                1,
            ),
        )
        .expect("multi-criterion feature spec");

        let requirement_path = temp.path().join("spec/requirement.yaml");
        let requirement = fs::read_to_string(&requirement_path).expect("requirement spec");
        fs::write(
            &requirement_path,
            requirement.replacen(
                "FEAT-LIFECYCLE-ADD-SYMBOL-001#binding.lifecycle/target.added-symbol",
                "FEAT-FIXTURE-001#binding.implementation/target.journey-source",
                1,
            ),
        )
        .expect("multi-criterion verification coverage");

        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let (basis, csrf, projection) = projection_and_basis(&app).await;
        let feature = projection["specifications"]["specifications"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["id"] == "FEAT-FIXTURE-001"))
            .expect("multi-criterion Feature projection");
        let capabilities = feature["origin_capabilities"]
            .as_array()
            .expect("Feature origin capabilities");
        assert!(capabilities.iter().any(|capability| {
            capability["label"] == "Feature implementation"
                && capability["enabled"] == false
                && capability["disabled_code"] == "ambiguous-origin"
        }));
        let target_origin = capabilities
            .iter()
            .find_map(|capability| {
                (capability["label"] == "Implementation target"
                    && capability["enabled"] == true
                    && capability["origin"]["target"]
                        == "FEAT-FIXTURE-001#binding.implementation/target.journey-source"
                    && capability["origin"]["criterion"] == "REQ-FIXTURE-001#criterion.add-symbol")
                    .then(|| capability["origin"].clone())
            })
            .expect("the exact multi-criterion target remains selectable");

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": target_origin,
                "title": "Focus the journey source"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let request = service
            .session
            .read()
            .expect("multi-criterion session")
            .draft_request
            .clone()
            .expect("created target-origin request");
        assert!(matches!(
            request.origin,
            WorkOrigin::FeatureImplementationTarget { ref target, ref criterion, .. }
                if target.target_id.to_string() == "journey-source"
                    && criterion.to_string() == "REQ-FIXTURE-001#criterion.add-symbol"
        ));
        assert_eq!(request.requested_targets.len(), 1);
    }

    #[tokio::test]
    async fn workbench_specification_candidates_support_search_edit_and_create() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let app = WorkbenchServer::new(temp.path().to_path_buf()).router();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/specifications/candidates?q=behavior&kind=requirement")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let candidates: Vec<SpecificationCandidateView> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("candidate response");
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0]
                .matches
                .iter()
                .any(|candidate| candidate.kind == "criterion")
        );

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let item_update = StructuredEditCommand {
            basis: basis.clone(),
            patch: EditPatch::Specification {
                item_id: "REQ-FIXTURE-001".into(),
                fields: SpecificationPatchFields::Requirement {
                    title: Some("Renamed fixture requirement".into()),
                    description: None,
                    priority: None,
                    status: None,
                },
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &item_update,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let item_preview: EditPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("item preview");
        let impact = item_preview.impact.expect("item impact");
        assert!(impact.changed_anchors.is_empty());
        assert_eq!(impact.affected_items, vec!["REQ-FIXTURE-001"]);

        let update = StructuredEditCommand {
            basis: basis.clone(),
            patch: EditPatch::Anchor {
                anchor: "REQ-FIXTURE-001#criterion.behavior".into(),
                fields: AnchorPatchFields::Criterion {
                    statement: Some("The fixture behavior remains true.".into()),
                    kind: None,
                    governed_by: None,
                },
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &update,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let preview: EditPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("candidate preview");
        assert!(preview.impact.as_ref().is_some_and(|impact| {
            impact
                .changed_anchors
                .iter()
                .any(|anchor| anchor == "REQ-FIXTURE-001#criterion.behavior")
        }));
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/candidates/apply",
            &csrf,
            &update,
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::CONFLICT,
            "{}",
            String::from_utf8_lossy(&response_body)
        );
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/candidates/apply",
            &csrf,
            &StructuredEditCommand {
                preview_token: Some(preview.preview_token),
                ..update
            },
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "specification apply response: {}",
            String::from_utf8_lossy(&response_body)
        );
        let updated = fs::read_to_string(temp.path().join("spec/requirement.yaml"))
            .expect("updated requirement");
        assert!(updated.contains("The fixture behavior remains true."));

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let create = StructuredEditCommand {
            basis,
            patch: EditPatch::CreateRequirement {
                document: "spec/requirement.yaml".into(),
                id: "REQ-FIXTURE-002".into(),
                title: "A guided requirement".into(),
                description: "Created through the typed Workbench wizard.".into(),
                priority: Priority::Medium,
                status: None,
                criteria: vec![NewCriterion {
                    id: "guided".into(),
                    kind: CriterionKind::Behavior,
                    statement: "The wizard preserves exact specification structure.".into(),
                    governed_by: vec!["POL-FIXTURE-001#rule.behavior".parse().unwrap()],
                }],
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &create,
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&response_body)
        );
        let preview: EditPreview = serde_json::from_slice(&response_body).expect("create preview");
        assert_eq!(
            preview.old_hash,
            content_hash(&fs::read_to_string(temp.path().join("spec/requirement.yaml")).unwrap())
        );
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/candidates/apply",
            &csrf,
            &StructuredEditCommand {
                preview_token: Some(preview.preview_token),
                ..create
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let created = fs::read_to_string(temp.path().join("spec/requirement.yaml"))
            .expect("created requirement");
        assert!(created.contains("REQ-FIXTURE-002"));
        assert!(created.contains("guided"));

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let feature = StructuredEditCommand {
            basis,
            patch: EditPatch::CreateFeature {
                document: "spec/feature.yaml".into(),
                id: "FEAT-FIXTURE-002".into(),
                title: "A guided feature".into(),
                summary: "Created through the same typed wizard.".into(),
                status: None,
                criterion_anchor: None,
                target: None,
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &feature,
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&response_body)
        );
        let preview: EditPreview = serde_json::from_slice(&response_body).expect("feature preview");
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/candidates/apply",
            &csrf,
            &StructuredEditCommand {
                preview_token: Some(preview.preview_token),
                ..feature
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            fs::read_to_string(temp.path().join("spec/feature.yaml"))
                .expect("created feature")
                .contains("FEAT-FIXTURE-002")
        );
    }

    #[test]
    fn approved_target_resolution_fails_closed_and_filters_stale_evidence() {
        let anchor: SpecAnchor = "REQ-FIXTURE-001#criterion.behavior".parse().unwrap();
        let target_reference: BoundTargetRef =
            "FEAT-FIXTURE-001#binding.implementation/target.behavior"
                .parse()
                .unwrap();
        let suggestions = vec![
            TargetSuggestion {
                id: "target-current".into(),
                rank: 1,
                reference: target_reference.clone(),
                role: BindingRole::Implementation,
                transition: TargetTransition::Modify,
                lifecycle: mitase_work_model::TargetLifecycle::Stable,
                path: "src/lib.rs".into(),
                selector: "behavior".into(),
                existing_file: true,
                budget_bytes: None,
                budget_lines: None,
                confidence: mitase_planner::SuggestionConfidence::High,
                evidence: vec!["current evidence".into()],
                evidence_fingerprint: "current-fingerprint".into(),
            },
            TargetSuggestion {
                id: "target-stale".into(),
                rank: 2,
                reference: target_reference,
                role: BindingRole::Implementation,
                transition: TargetTransition::Modify,
                lifecycle: mitase_work_model::TargetLifecycle::Stable,
                path: "src/lib.rs".into(),
                selector: "behavior".into(),
                existing_file: true,
                budget_bytes: None,
                budget_lines: None,
                confidence: mitase_planner::SuggestionConfidence::High,
                evidence: vec!["new evidence".into()],
                evidence_fingerprint: "new-fingerprint".into(),
            },
        ];
        let approvals = vec![
            ApprovedTargetSuggestion {
                criterion: anchor.clone(),
                suggestion_id: "target-current".into(),
                evidence_fingerprint: "current-fingerprint".into(),
            },
            ApprovedTargetSuggestion {
                criterion: anchor.clone(),
                suggestion_id: "target-stale".into(),
                evidence_fingerprint: "old-fingerprint".into(),
            },
        ];
        let requested = resolve_requested_targets(&anchor, Ok(suggestions), &approvals)
            .expect("matching target resolution");
        assert_eq!(requested.len(), 1);
        assert_eq!(
            requested[0].reference.to_string(),
            "FEAT-FIXTURE-001#binding.implementation/target.behavior"
        );
        let error = resolve_requested_targets(
            &anchor,
            Err(anyhow::anyhow!("target suggestion recalculation failed")),
            &approvals,
        )
        .expect_err("suggestion errors must not become empty targets");
        assert_eq!(error.to_string(), "target suggestion recalculation failed");
    }

    #[tokio::test]
    #[ignore = "pre-v1 cutover: evidence changes invalidate the advisory candidate set"]
    async fn target_suggestions_remain_advisory_until_create_work() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let suggestion_path =
            "/api/specifications/REQ-FIXTURE-001%23criterion.behavior/target-suggestions";

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("suggestion response");
        assert_eq!(suggestions.suggestions.len(), 2);
        assert!(suggestions.suggestions.iter().all(|candidate| {
            !candidate.evidence.is_empty() && !candidate.evidence_fingerprint.is_empty()
        }));
        assert!(
            service.session.read().unwrap().draft_request.is_none(),
            "reading advisory suggestions must not create executable scope"
        );

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let rejected_candidate = suggestions.suggestions[0].clone();
        let rejected_id = rejected_candidate.id.clone();
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/reject"),
            &csrf,
            &TargetSuggestionRejectCommand {
                basis: basis.clone(),
                suggestion_token: suggestions.suggestion_token.clone(),
                suggestion_id: rejected_id.clone(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let after_reject: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("rejected suggestion response");
        assert!(
            after_reject
                .suggestions
                .iter()
                .all(|candidate| candidate.id != rejected_id)
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let refreshed: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("refreshed suggestion response");
        assert!(
            refreshed
                .suggestions
                .iter()
                .all(|candidate| candidate.id != rejected_id)
        );

        let workspace = SpecWorkspace::load(temp.path()).expect("fixture workspace");
        let index = workspace.index().expect("fixture index");
        let target = index
            .target(&rejected_candidate.reference)
            .expect("rejected target");
        let identity = index
            .target_to_artifact
            .get(&rejected_candidate.reference)
            .expect("rejected artifact identity");
        let unit = index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)
            .expect("rejected artifact unit");
        let target_path = temp.path().join(target.path.as_path());
        let mut changed_source = fs::read_to_string(&target_path).expect("target source");
        let body_offset = changed_source[unit.span.byte_start..unit.span.byte_end]
            .find('{')
            .map(|offset| unit.span.byte_start + offset + 1)
            .expect("Rust target body");
        changed_source.insert_str(body_offset, "\n// new target evidence\n");
        fs::write(&target_path, changed_source).expect("changed target source");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let refreshed: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("new evidence suggestion response");
        assert!(
            refreshed
                .suggestions
                .iter()
                .any(|candidate| candidate.id == rejected_id),
            "a rejected candidate must reappear when its artifact evidence changes"
        );
        let (basis, csrf, _) = projection_and_basis(&app).await;

        let approved_ids = refreshed
            .suggestions
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>();
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis: basis.clone(),
                suggestion_token: refreshed.suggestion_token,
                suggestion_ids: approved_ids.clone(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approval response");
        assert_eq!(approval.approved_ids, approved_ids);
        assert!(approval.split_recommendation.is_none());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let persisted: TargetSuggestionsView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("persisted suggestion response");
        assert_eq!(persisted.approved_ids, approved_ids);

        let mut changed_again = fs::read_to_string(&target_path).expect("changed target source");
        let next_body_offset = changed_again[unit.span.byte_start..unit.span.byte_end]
            .find('{')
            .map(|offset| unit.span.byte_start + offset + 1)
            .expect("Rust target body after approval");
        changed_again.insert_str(next_body_offset, "\n// approved evidence changed\n");
        fs::write(&target_path, changed_again).expect("changed approved target source");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let invalidated: TargetSuggestionsView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("invalidated suggestion response");
        assert!(!invalidated.approved_ids.contains(&rejected_id));
        assert!(
            service
                .session
                .read()
                .expect("target suggestion session")
                .draft_request
                .is_none()
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approved target projection");
        assert!(projection["work"]["request"].is_null());
        assert_eq!(
            projection["journey"]["current_step"],
            "select_specification"
        );

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.behavior" },
                "title": "Start the approved fixture work"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let request = service
            .session
            .read()
            .expect("target suggestion session")
            .draft_request
            .clone()
            .expect("created work request");
        assert_eq!(request.requested_targets.len(), 0);
        assert!(request.requested_targets.iter().all(|target| {
            target.criterion.as_ref()
                == Some(&"REQ-FIXTURE-001#criterion.behavior".parse().unwrap())
        }));
        assert!(
            service
                .session
                .read()
                .expect("target suggestion session")
                .approved_target_suggestions
                .is_empty()
        );
    }

    #[tokio::test]
    async fn create_work_requires_an_exact_implemented_requirement_criterion() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        for (anchor, status) in [
            ("POL-FIXTURE-001#rule.behavior", None),
            ("PHIL-FIXTURE-001#principle.bounded-evidence", None),
            ("REQ-FIXTURE-001#criterion.missing", None),
            ("REQ-FIXTURE-001", None),
            ("REQ-FIXTURE-001#criterion.behavior", Some("planned")),
            ("REQ-FIXTURE-001#criterion.behavior", Some("deprecated")),
        ] {
            let temp = tempfile::tempdir().expect("fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            if let Some(status) = status {
                let requirement = temp.path().join("spec/requirement.yaml");
                let source = fs::read_to_string(&requirement).expect("requirement fixture");
                fs::write(
                    &requirement,
                    source.replacen("status: implemented", &format!("status: {status}"), 1),
                )
                .expect("updated requirement fixture");
            }
            initialize_fixture_git(temp.path());
            let server = WorkbenchServer::new(temp.path().to_path_buf());
            let app = server.router();
            let (basis, csrf, _) = projection_and_basis(&app).await;
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/action",
                &csrf,
                &serde_json::json!({
                    "basis": basis,
                    "action": "create",
                    "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                    "origin": { "kind": "requirement-criterion", "criterion": anchor },
                    "title": "Must not start from an inactive specification"
                }),
            )
            .await;
            let expected_status = match (anchor, status) {
                ("REQ-FIXTURE-001", None) => StatusCode::UNPROCESSABLE_ENTITY,
                (_, Some(_)) => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::NOT_FOUND,
            };
            let response_status = response.status();
            let response_body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(
                response_status,
                expected_status,
                "anchor={anchor} status={status:?}: {}",
                String::from_utf8_lossy(&response_body)
            );
            let (_, _, projection) = projection_and_basis(&app).await;
            assert!(
                projection["work"]["request"].is_null(),
                "anchor={anchor} status={status:?}"
            );
            assert_eq!(
                projection["journey"]["current_step"], "select_specification",
                "anchor={anchor} status={status:?}"
            );
        }
    }

    #[tokio::test]
    #[ignore = "pre-v1 cutover: planned origins are rejected before Work creation"]
    async fn planned_add_target_is_advisory_until_human_approval() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        let feature_path = temp.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path).expect("feature fixture");
        fs::write(
            feature_path,
            feature.replacen(
                "criterion: REQ-FIXTURE-001#criterion.add-file",
                "criterion: REQ-FIXTURE-001#criterion.add-symbol",
                1,
            ),
        )
        .expect("two Add targets fixture");
        let config_path = temp.path().join("mitase.yaml");
        let config = fs::read_to_string(&config_path).expect("config fixture");
        fs::write(
            &config_path,
            config.replace("max_total_bytes: 120000", "max_total_bytes: 700"),
        )
        .expect("small slicing budget");
        let requirement_path = temp.path().join("spec/requirement.yaml");
        let requirement = fs::read_to_string(&requirement_path).expect("requirement fixture");
        fs::write(
            requirement_path,
            requirement.replacen("status: implemented", "status: planned", 1),
        )
        .expect("planned requirement fixture");
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let suggestion_path =
            "/api/specifications/REQ-FIXTURE-001%23criterion.add-symbol/target-suggestions";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("planned add suggestions");
        let add_candidates = suggestions
            .suggestions
            .iter()
            .filter(|candidate| candidate.transition == mitase_work_model::TargetTransition::Add)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(add_candidates.len(), 2);
        let add = add_candidates
            .first()
            .cloned()
            .expect("planned Add suggestion");
        assert_eq!(
            add.lifecycle,
            mitase_work_model::TargetLifecycle::EnsurePresent
        );
        assert_eq!(add.path, "src/lib.rs");
        assert!(add.existing_file);
        assert_eq!(add.budget_bytes, Some(512));
        assert_eq!(add.budget_lines, Some(32));
        assert!(service.session.read().unwrap().draft_request.is_none());
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis: basis.clone(),
                suggestion_token: suggestions.suggestion_token.clone(),
                suggestion_ids: add_candidates
                    .iter()
                    .map(|candidate| candidate.id.clone())
                    .collect(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let bulk_approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("bulk planned add approval");
        assert!(bulk_approval.approved_ids.is_empty());
        assert_eq!(
            bulk_approval
                .split_recommendation
                .as_ref()
                .expect("budget split recommendation")
                .suggested_groups
                .len(),
            2
        );
        let split_groups = bulk_approval
            .split_recommendation
            .as_ref()
            .expect("budget split recommendation")
            .suggested_groups
            .clone();
        assert!(
            service
                .session
                .read()
                .unwrap()
                .approved_target_suggestions
                .is_empty(),
            "over-budget bulk approval must not persist any candidate"
        );
        for group in &split_groups {
            let response = json_mutation(
                &app,
                Method::POST,
                &format!("{suggestion_path}/approve"),
                &csrf,
                &TargetSuggestionApprovalCommand {
                    basis: basis.clone(),
                    suggestion_token: suggestions.suggestion_token.clone(),
                    suggestion_ids: group.clone(),
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.add-symbol" },
                "title": "reject accumulated over-budget groups"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(service.session.read().unwrap().draft_request.is_none());
        service
            .session
            .write()
            .unwrap()
            .approved_target_suggestions
            .clear();
        fs::write(&config_path, config).expect("restore slicing budget");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let restored_suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("restored planned add suggestions");

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis,
                suggestion_token: restored_suggestions.suggestion_token,
                suggestion_ids: vec![add.id.clone()],
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("planned add approval");
        assert_eq!(approval.approved_ids, vec![add.id.clone()]);
        let request = create_approved_work_request(
            &server,
            "REQ-FIXTURE-001#criterion.add-symbol",
            "add the approved target",
        )
        .await;
        assert_eq!(request.constraints.max_added_bytes_per_target, Some(512));
        assert_eq!(request.constraints.max_added_lines_per_target, Some(32));
        assert_eq!(request.requested_targets.len(), 1);
        assert_eq!(
            request.requested_targets[0].transition,
            mitase_work_model::TargetTransition::Add
        );
        assert!(matches!(
            request.origin,
            mitase_work_model::WorkOrigin::RequirementCriterion { .. }
        ));
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("planned requirement work plan");
        assert_eq!(plan.status, PlanStatus::Ready, "{plan:?}");
        assert!(service.session.read().unwrap().draft_request.is_some());
    }

    #[tokio::test]
    #[ignore = "pre-v1 cutover: planned origins are rejected before Work creation"]
    async fn planned_requirement_with_approved_add_target_can_create_ready_plan() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        fs::write(
            temp.path().join("spec/planned-requirement.yaml"),
            "schema: mitase/spec/v1\nkind: requirements\nnamespace: fixture\ncategory: Workbench recovery\nrequirements:\n  - id: REQ-PLANNED-001\n    title: A planned behavior\n    description: A planned requirement created through recovery.\n    priority: high\n    status: planned\n    criteria:\n      - id: behavior\n        kind: behavior\n        statement: Add the planned behavior.\n        governed_by: []\n",
        )
        .expect("planned requirement fixture");
        fs::write(
            temp.path().join("spec/planned-feature.yaml"),
            "schema: mitase/spec/v1\nkind: features\nnamespace: fixture\ncategory: Workbench recovery\nfeatures:\n  - id: FEAT-PLANNED-001\n    title: A planned behavior implementation\n    summary: A planned Feature target created through recovery.\n    status: planned\n    bindings:\n      - id: implementation\n        role: implementation\n        facet: work\n        responsibility: Add the planned behavior implementation.\n        targets:\n          - id: behavior\n            adapter: rust\n            path: src/lib.rs\n            selector: { kind: symbol, name: planned_behavior }\n            claims:\n              - kind: satisfies\n                criterion: REQ-PLANNED-001#criterion.behavior\n          - id: behavior-two\n            adapter: rust\n            path: src/other.rs\n            selector: { kind: symbol, name: planned_behavior_two }\n            claims:\n              - kind: satisfies\n                criterion: REQ-PLANNED-001#criterion.behavior\n",
        )
        .expect("planned Feature fixture");
        let config_path = temp.path().join("mitase.yaml");
        let config = fs::read_to_string(&config_path).expect("config fixture");
        fs::write(
            &config_path,
            config.replace("max_total_bytes: 120000", "max_total_bytes: 700"),
        )
        .expect("small slicing budget");
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let suggestion_path =
            "/api/specifications/REQ-PLANNED-001%23criterion.behavior/target-suggestions";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(response_status, StatusCode::OK, "{response_body:?}");
        let suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response_body).expect("planned Add suggestions");
        let add_candidates = suggestions
            .suggestions
            .iter()
            .filter(|candidate| candidate.transition == TargetTransition::Add)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(add_candidates.len(), 2);
        let add = add_candidates
            .first()
            .cloned()
            .expect("planned implementation Add suggestion");
        assert_eq!(add.role, BindingRole::Implementation);
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis: basis.clone(),
                suggestion_token: suggestions.suggestion_token.clone(),
                suggestion_ids: add_candidates
                    .iter()
                    .map(|candidate| candidate.id.clone())
                    .collect(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let bulk_approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("bulk planned Add approval");
        assert!(bulk_approval.approved_ids.is_empty());
        assert_eq!(
            bulk_approval
                .split_recommendation
                .as_ref()
                .expect("budget split recommendation")
                .suggested_groups
                .len(),
            2
        );
        let split_groups = bulk_approval
            .split_recommendation
            .as_ref()
            .expect("budget split recommendation")
            .suggested_groups
            .clone();
        assert!(
            service
                .session
                .read()
                .expect("workbench session lock")
                .approved_target_suggestions
                .is_empty(),
            "over-budget bulk approval must not persist any candidate"
        );
        for group in &split_groups {
            let response = json_mutation(
                &app,
                Method::POST,
                &format!("{suggestion_path}/approve"),
                &csrf,
                &TargetSuggestionApprovalCommand {
                    basis: basis.clone(),
                    suggestion_token: suggestions.suggestion_token.clone(),
                    suggestion_ids: group.clone(),
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        }
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-PLANNED-001#criterion.behavior" },
                "title": "reject accumulated over-budget groups"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(service.session.read().unwrap().draft_request.is_none());
        service
            .session
            .write()
            .unwrap()
            .approved_target_suggestions
            .clear();
        fs::write(&config_path, config).expect("restore slicing budget");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(suggestion_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let restored_suggestions: TargetSuggestionSet =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("restored planned Add suggestions");
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            &format!("{suggestion_path}/approve"),
            &csrf,
            &TargetSuggestionApprovalCommand {
                basis,
                suggestion_token: restored_suggestions.suggestion_token,
                suggestion_ids: vec![add.id.clone()],
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-PLANNED-001#criterion.behavior" },
                "title": "add the approved planned target"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            service
                .session
                .read()
                .expect("workbench session lock")
                .draft_request
                .as_ref()
                .is_some_and(|request| matches!(
                    request.origin,
                    mitase_work_model::WorkOrigin::RequirementCriterion { .. }
                ))
        );
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("planned requirement work plan");
        assert_eq!(plan.status, PlanStatus::Ready, "{plan:?}");
    }
    #[tokio::test]
    async fn journey_action_exposes_one_friendly_next_step_and_can_cancel() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();
        let (basis, csrf, initial_projection) = projection_and_basis(&app).await;
        assert_eq!(
            initial_projection["journey"]["current_step"],
            "select_specification"
        );
        assert_eq!(
            initial_projection["journey"]["steps"][0]["id"],
            "select_specification"
        );
        assert_eq!(
            initial_projection["journey"]["primary_action"]["action"],
            "choose_specification"
        );
        assert_eq!(
            initial_projection["journey"]["title_key"],
            "journey.title.select_specification"
        );
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.behavior" },
                "title": "Make the finished change understandable"
            }),
        )
        .await;
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            status,
            StatusCode::OK,
            "create body: {}",
            String::from_utf8_lossy(&body)
        );
        let projection: serde_json::Value =
            serde_json::from_slice(&body).expect("journey projection");
        assert_eq!(
            projection["journey"]["title"],
            "Make the finished change understandable"
        );
        assert_eq!(projection["journey"]["primary_action"]["action"], "prepare");
        assert_eq!(
            projection["journey"]["related_specification"],
            serde_json::json!({
                "title": "Keep the fixture behavior valid",
                "overview": "The fixture exposes one bounded behavior for Workbench post-state validation.",
                "status": "implemented",
                "criterion_statement": "The fixture behavior returns true."
            })
        );
        assert_eq!(
            projection["journey"]["advanced"]["specification_anchor"],
            "REQ-FIXTURE-001#criterion.behavior"
        );
        assert_eq!(
            service
                .session
                .read()
                .expect("journey session")
                .draft_request
                .as_ref()
                .and_then(|request| request.constraints.max_slices),
            None
        );
        assert!(
            projection["journey"]["advanced"]["request_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("work-"))
        );

        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("rename basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "rename",
                "title": "Explain the finished change"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("renamed projection");
        assert_eq!(
            projection["journey"]["title"],
            "Explain the finished change"
        );
        assert_eq!(
            projection["work"]["request"]["title"], "Make the finished change understandable",
            "renaming the display title must not invalidate the canonical request or plan"
        );
        assert_eq!(projection["journey"]["primary_action"]["action"], "prepare");

        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("fresh basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({ "basis": basis, "action": "prepare" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("prepared projection");
        assert_eq!(projection["journey"]["current_step"], "approve");
        assert_eq!(
            projection["journey"]["approved_scope"]["status"],
            "proposed"
        );
        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("prepared basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "approve",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approved projection");
        assert_eq!(projection["journey"]["current_step"], "implement");
        assert_eq!(
            projection["journey"]["approved_scope"]["status"],
            "approved"
        );
        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("approved basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "start",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("started projection");
        assert_eq!(projection["journey"]["current_step"], "verify");
        assert_eq!(
            projection["journey"]["recovery_action"]["label_key"],
            "journey.action.cancel"
        );
        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("started basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "verify",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verified projection");
        assert_eq!(projection["journey"]["primary_action"]["action"], "retry");
        assert_eq!(projection["work"]["agent"]["status"], "blocked");
        assert!(
            projection["work"]["agent_events"]
                .as_array()
                .is_some_and(|events| {
                    events
                        .iter()
                        .any(|event| event["event"]["kind"] == "verification-recorded")
                })
        );
        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("verified basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "retry",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("retried projection");
        assert_eq!(projection["journey"]["primary_action"]["action"], "verify");
        assert_eq!(projection["work"]["agent"]["status"], "active");
        let basis: MutationBasis = serde_json::from_value(serde_json::json!({
            "expected_revision": projection["snapshot"]["revision"],
            "expected_workspace_fingerprint": projection["snapshot"]["fingerprint"],
            "expected_source_hash": projection["snapshot"]["source_hash"]
        }))
        .expect("retried basis");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({ "basis": basis, "action": "cancel" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("cancelled projection");
        assert_eq!(
            projection["journey"]["current_step"],
            "select_specification"
        );
        assert!(projection["journey"]["related_specification"].is_null());

        {
            let mut session = service.session.write().expect("journey session lock");
            session.work_title = Some("Change several fixture criteria".into());
            session.draft_request = Some(WorkRequest {
                schema: WORK_REQUEST_SCHEMA.into(),
                id: "WORK-MULTIPLE-CRITERIA".into(),
                title: "Change several fixture criteria".into(),
                operation: WorkOperation::Modify,
                origin: WorkOrigin::RequirementCriterion {
                    criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                },
                constraints: WorkConstraints::default(),
                requested_targets: vec![],
            });
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projection")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("multiple criteria projection");
        assert!(!projection["journey"]["related_specification"].is_null());
    }

    #[tokio::test]
    async fn journey_prepare_returns_recovery_for_a_blocked_plan() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-BLOCKED-JOURNEY".into(),
            title: "keep the fixture behavior valid".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            },
            constraints: WorkConstraints {
                max_slices: Some(0),
                ..WorkConstraints::default()
            },
            requested_targets: vec![],
        };
        let app = WorkbenchServer::new(temp.path().to_path_buf())
            .with_request(request)
            .expect("preloaded fixture request")
            .router();
        let (basis, csrf, _) = projection_and_basis(&app).await;

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({ "basis": basis, "action": "prepare" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("blocked journey projection");
        assert_eq!(
            projection["journey"]["evidence"]["status"],
            "split_required"
        );
        assert_eq!(
            projection["journey"]["primary_action"]["action"],
            "select_slice"
        );
        let recovery = &projection["work"]["split_recovery"];
        let candidate_plan_digest = recovery["candidate_plan_digest"].as_str().unwrap();
        let candidate = recovery["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["selectable"] == true)
            .expect("blocked global slice limit exposes a replan candidate");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis_from_projection(&projection),
                "action": "select_slice",
                "schema": mitase_work_model::WORK_SELECT_SLICE_SCHEMA,
                "candidate_plan_digest": candidate_plan_digest,
                "slice_id": candidate["id"],
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let selected: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("replanned slice response");
        assert_eq!(selected["projection"]["work"]["plan"]["status"], "ready");
        assert!(selected["projection"]["work"]["selected_slice"].is_string());
    }

    #[tokio::test]
    async fn journey_verify_returns_a_fresh_projection_after_editable_change() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let app = WorkbenchServer::new(temp.path().to_path_buf()).router();
        let (basis, csrf, _) = projection_and_basis(&app).await;

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": mitase_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.behavior" },
                "title": "Make the fixture behavior pass"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("created journey projection");

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({ "basis": basis_from_projection(&projection), "action": "prepare" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("prepared journey projection");

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis_from_projection(&projection),
                "action": "approve",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approved journey projection");

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis_from_projection(&projection),
                "action": "start",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("started journey projection");
        let stale_basis = basis_from_projection(&projection);

        fs::write(
            temp.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    let result = true;\n    result\n}\n",
        )
        .expect("apply editable fixture change");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": stale_basis,
                "action": "verify",
                "execution": execution_from_projection(&projection),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("verified journey projection");
        assert_eq!(
            projection["journey"]["primary_action"]["action"],
            "finalize"
        );
        assert_eq!(projection["work"]["agent"]["status"], "completed");
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis_from_projection(&projection),
                "action": "finalize",
                "execution": execution_from_projection(&projection),
                "attempt_id": projection["journey"]["advanced"]["attempt_id"],
                "preview_token": null,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("finalized journey projection");
        assert_eq!(projection["journey"]["current_step"], "complete");
        assert_eq!(
            projection["journey"]["primary_action"]["label_key"],
            "journey.action.start_another"
        );
    }

    async fn raw_http(
        address: std::net::SocketAddr,
        method: &str,
        path: &str,
        headers: &str,
        body: &[u8],
    ) -> Vec<u8> {
        let mut stream = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect Workbench HTTP server");
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
            body.len()
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("write HTTP headers");
        stream.write_all(body).await.expect("write HTTP body");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("read HTTP response");
        response
    }

    #[tokio::test]
    async fn workbench_http_closed_loop_flow() {
        let _workspace_lock = workspace_test_lock().await;
        assert_eq!(
            run_fixture_post_state_flow(false).await,
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn workbench_http_server_transport_flow() {
        let _workspace_lock = workspace_test_lock().await;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind Workbench HTTP server");
        let address = listener.local_addr().expect("server address");
        let server = tokio::spawn(async move {
            axum::serve(listener, WorkbenchServer::new(workspace_root()).router())
                .await
                .expect("serve Workbench HTTP router");
        });

        let projection = raw_http(address, "GET", "/api/projection", "", &[]).await;
        let projection_text = String::from_utf8_lossy(&projection);
        assert!(
            projection_text.starts_with("HTTP/1.1 200"),
            "{projection_text}"
        );
        assert!(
            projection_text.contains("x-mitase-csrf-token:"),
            "{projection_text}"
        );
        assert!(
            projection_text.contains("\"specifications\""),
            "{projection_text}"
        );

        let html = raw_http(address, "GET", "/", "", &[]).await;
        let html_text = String::from_utf8_lossy(&html);
        assert!(html_text.starts_with("HTTP/1.1 200"), "{html_text}");
        assert!(html_text.contains("type=\"module\" src=\"/assets/js/main.js\""));
        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn workbench_canonical_projection_flow() {
        let _workspace_lock = workspace_test_lock().await;
        let app = WorkbenchServer::new(workspace_root()).router();
        let (_, _, projection) = projection_and_basis(&app).await;
        assert_eq!(projection["readiness"]["status"], "Not run");
        assert!(projection["specifications"]["specifications"].is_array());
        assert!(projection["config"].is_null());
    }

    #[tokio::test]
    async fn workbench_work_session_flow() {
        let _workspace_lock = workspace_test_lock().await;
        let request = WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-WORKBENCH-SESSION".into(),
            title: "plan a Workbench session".into(),
            operation: mitase_work_model::WorkOperation::Modify,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: "REQ-WORKBENCH-002#criterion.work-session".parse().unwrap(),
            },
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let app = WorkbenchServer::new(workspace_root())
            .with_request(request)
            .expect("preloaded Workbench request")
            .router();
        let (_, _, projection) = projection_and_basis(&app).await;
        assert!(projection["work"]["request"].is_object());
    }

    #[tokio::test]
    async fn select_slice_replans_the_exact_candidate_boundary() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("select-slice fixture tempdir");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-SELECT-SLICE-BOUNDARY".into(),
            title: "select the fixture behavior boundary".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        let server = WorkbenchServer::new(temp.path().to_path_buf())
            .with_request(request)
            .expect("exact preloaded request");
        let app = server.router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let candidate: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("candidate plan");
        assert_eq!(candidate.status, PlanStatus::Ready);
        let candidate_slice = candidate.slices.first().expect("candidate slice").clone();

        select_slice(
            &server.service,
            &basis,
            &candidate.canonical_digest,
            &candidate_slice.id,
        )
        .unwrap_or_else(|error| panic!("select exact candidate slice: {}", error.1));

        let session = server
            .service
            .session
            .read()
            .expect("selected session lock");
        let selected_plan = session.plan.clone().expect("selected plan");
        assert_eq!(selected_plan.slices.len(), 1);
        assert_eq!(
            session.selected_slice.as_deref(),
            Some(selected_plan.slices[0].id.as_str())
        );
        assert!(!selected_plan.request.requested_targets.is_empty());
        assert!(
            selected_plan
                .request
                .requested_targets
                .iter()
                .all(|requested| {
                    selected_plan.slices[0]
                        .editable_targets
                        .iter()
                        .chain(selected_plan.slices[0].verification_targets.iter())
                        .any(|planned| planned.reference == requested.reference)
                })
        );
        assert!(target_boundaries_match(
            &selected_plan.slices[0].editable_targets,
            &candidate_slice.editable_targets
        ));
        assert!(target_boundaries_match(
            &selected_plan.slices[0].verification_targets,
            &candidate_slice.verification_targets
        ));
        assert!(target_boundaries_match(
            &selected_plan.slices[0].readonly_context,
            &candidate_slice.readonly_context
        ));
    }

    #[test]
    fn completion_history_projection_is_store_backed() {
        let fixture = workspace_root().join("fixtures/v1/valid-workbench-flow");
        let temp = tempfile::tempdir().expect("completion history fixture");
        copy_fixture_tree(&fixture, temp.path());
        initialize_fixture_git(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).expect("workspace loads");
        let history = completion_history(&workspace).expect("completion history loads");
        assert!(history.current.is_none());
        assert!(history.previous.is_empty());
    }

    #[test]
    fn completion_history_scopes_attempts_to_the_active_plan_and_slice() {
        let attempt = |id: &str, plan_digest: &str, slice_id: &str| CompletionAttemptView {
            attempt_id: id.into(),
            plan_digest: plan_digest.into(),
            slice_id: slice_id.into(),
            status: CompletionStatus::Complete,
            demonstrated: vec![],
            blockers: vec![],
            next_action: None,
            finalized: false,
            stale: false,
        };
        let history = CompletionHistoryView {
            current: Some(attempt("attempt-old", "plan-old", "slice-old")),
            previous: vec![attempt("attempt-current", "plan-current", "slice-current")],
        };
        assert_eq!(
            history
                .current_for("plan-current", "slice-current")
                .map(|attempt| attempt.attempt_id.as_str()),
            Some("attempt-current")
        );
        assert!(history.current_for("plan-current", "slice-other").is_none());
    }

    async fn run_spec_transaction(app: Router, root: &Path) {
        let path = root.join("docs/mitase/requirements/workbench.yaml");
        let original = fs::read_to_string(&path).expect("original specification");
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let command = StructuredEditCommand {
            basis: basis.clone(),
            patch: EditPatch::Specification {
                item_id: "REQ-WORKBENCH-001".into(),
                fields: SpecificationPatchFields::Requirement {
                    title: Some("Canonical projection (previewed)".into()),
                    description: None,
                    priority: None,
                    status: None,
                },
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/REQ-WORKBENCH-001/preview",
            &csrf,
            &command,
        )
        .await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "preview response: {}",
            String::from_utf8_lossy(&response_body)
        );
        let preview: EditPreview = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(preview.old_hash, content_hash(&original));
        assert_ne!(preview.old_hash, preview.new_hash);
        assert_eq!(
            preview.workspace_fingerprint,
            basis.expected_workspace_fingerprint
        );
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/REQ-WORKBENCH-001/apply",
            &csrf,
            &command,
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/REQ-WORKBENCH-001/apply",
            &csrf,
            &StructuredEditCommand {
                basis,
                patch: command.patch,
                preview_token: Some(preview.preview_token),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_ne!(
            fs::read_to_string(&path).expect("applied specification"),
            original
        );
        fs::write(&path, original).expect("restore specification");
    }

    async fn run_config_transaction(app: Router, root: &Path) {
        let path = root.join("mitase.yaml");
        let original = fs::read_to_string(&path).expect("original config");
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let workspace = SpecWorkspace::load(root).unwrap();
        let config = workspace.config.clone();
        let command = StructuredEditCommand {
            basis: basis.clone(),
            patch: EditPatch::Config {
                config: Box::new(config.clone()),
            },
            preview_token: None,
        };
        let response =
            json_mutation(&app, Method::POST, "/api/config/preview", &csrf, &command).await;
        let response_status = response.status();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            response_status,
            StatusCode::OK,
            "preview response: {}",
            String::from_utf8_lossy(&response_body)
        );
        let preview: EditPreview = serde_json::from_slice(&response_body).unwrap();
        assert_eq!(
            preview.workspace_fingerprint,
            basis.expected_workspace_fingerprint
        );
        let response = json_mutation(&app, Method::PUT, "/api/config/apply", &csrf, &command).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/config/apply",
            &csrf,
            &StructuredEditCommand {
                basis,
                patch: EditPatch::Config {
                    config: Box::new(config),
                },
                preview_token: Some(preview.preview_token),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        fs::write(&path, original).expect("restore config");
    }

    #[tokio::test]
    async fn workbench_spec_edit_transaction() {
        let workspace = isolated_workspace_for_transactions();
        let root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace root");
        run_spec_transaction(WorkbenchServer::new(root.clone()).router(), &root).await;
    }

    #[tokio::test]
    async fn workbench_config_edit_transaction() {
        let workspace = isolated_workspace_for_transactions();
        let root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace root");
        run_config_transaction(WorkbenchServer::new(root.clone()).router(), &root).await;
    }

    #[tokio::test]
    async fn workbench_security_flow() {
        let _workspace_lock = workspace_test_lock().await;
        let app = WorkbenchServer::new(workspace_root()).router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response =
            json_mutation(&app, Method::POST, "/api/work/plan", "wrong-csrf", &basis).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/plan",
            &csrf,
            &MutationBasis {
                expected_source_hash: "stale".into(),
                ..basis
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn workbench_accessible_navigation() {
        let _workspace_lock = workspace_test_lock().await;
        let response = WorkbenchServer::new(workspace_root())
            .router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("aria-label"));
        assert!(html.contains("/assets/js/main.js"));
    }

    #[test]
    fn workbench_initial_validation_has_no_passed_or_issue_counts() {
        let run = ValidationRunView::not_run();
        assert!(matches!(run.state, ValidationRunState::NotRun));
        assert_eq!(run.evaluated_rule_count, 0);
        assert_eq!(run.issue_counts.error, 0);
        assert!(
            run.phases
                .iter()
                .all(|phase| matches!(phase.state, ValidationRunState::NotRun))
        );
    }

    #[test]
    fn workbench_completed_empty_run_distinguishes_applicable_and_skipped_phases() {
        let run = ValidationRunView::completed(
            "workspace",
            Some("abc123".into()),
            ValidationResult::default(),
            false,
            false,
            ValidationPreset::Standard,
            SystemTime::now(),
        );
        assert!(matches!(run.state, ValidationRunState::Passed));
        assert_eq!(run.applicable_phase_count, 3);
        assert_eq!(run.skipped_phase_count, 2);
        assert!(run.evaluated_rule_count > 0);
        assert_eq!(run.diagnostics.len(), 0);
    }

    #[test]
    fn workbench_diagnostic_views_carry_server_classified_phase_and_severity_counts() {
        let result = ValidationResult {
            diagnostics: vec![mitase_diagnostics::Diagnostic::error(
                "MITASE-WORK-001",
                "work issue",
                "work.yaml",
            )],
            readiness: None,
        };
        let run = ValidationRunView::completed(
            "work_plan",
            Some("PLAN-1".into()),
            result,
            false,
            true,
            ValidationPreset::Standard,
            SystemTime::now(),
        );
        assert!(matches!(run.state, ValidationRunState::Issues));
        assert_eq!(run.issue_counts.error, 1);
        assert_eq!(run.diagnostics[0].diagnostic.phase, ValidationPhase::Plan);
    }
    #[cfg(any())]
    #[tokio::test]
    async fn legacy_workbench_agent_applies_all_approved_lifecycle_writes() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let cases = [
            (
                "behavior",
                mitase_work_model::TargetTransition::Modify,
                "modify existing symbol",
            ),
            (
                "added-symbol",
                mitase_work_model::TargetTransition::Add,
                "add symbol to existing file",
            ),
            (
                "added-file",
                mitase_work_model::TargetTransition::Add,
                "add new file",
            ),
            (
                "removed-symbol",
                mitase_work_model::TargetTransition::Remove,
                "remove symbol",
            ),
            (
                "removed-file",
                mitase_work_model::TargetTransition::Remove,
                "remove file",
            ),
        ];
        for (target_id, transition, description) in cases {
            let temp = tempfile::tempdir().expect("lifecycle fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            if target_id == "removed-symbol" {
                let config_path = temp.path().join("mitase.yaml");
                let config = fs::read_to_string(&config_path).expect("fixture config");
                fs::write(
                    config_path,
                    config.replace("target: off", "target: traceable"),
                )
                .expect("enable readiness for remove-symbol lifecycle");
            }
            initialize_fixture_git(temp.path());
            let server = WorkbenchServer::new(temp.path().to_path_buf());
            let app = server.router();
            let (basis, csrf, slice, run, target) =
                start_lifecycle_agent(&server, &app, target_id, transition).await;
            assert_eq!(target.transition, transition, "{description}");
            let write = match target_id {
                "behavior" => mitase_work_model::AgentTargetWrite::Replace {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                    content: "pub fn behavior() -> bool {\n    1 == 1\n}".into(),
                },
                "added-symbol" => mitase_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool {\n    true\n}\n".into(),
                },
                "added-file" => mitase_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn added_file() {}\n".into(),
                },
                "removed-symbol" => mitase_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => mitase_work_model::AgentTargetWrite::RemoveFile {
                    target: target.reference.clone(),
                    expected_content_hash: target.content_hash.clone(),
                },
                _ => unreachable!("declared lifecycle case"),
            };
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/patch",
                &csrf,
                &AgentPatchCommand {
                    basis,
                    run_id: run.run_id.clone(),
                    patch: AgentPatch {
                        schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id.clone(),
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![write],
                    },
                },
            )
            .await;
            let status = response.status();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(
                status,
                StatusCode::OK,
                "{description}: {}",
                String::from_utf8_lossy(&body)
            );
            let patch: mitase_work_model::AgentPatchRecord =
                serde_json::from_slice(&body).expect("lifecycle patch record");
            assert_eq!(patch.changes.len(), 1, "{description}");
            assert_eq!(patch.changes[0].transition, transition, "{description}");
            assert_eq!(
                patch.changes[0].lifecycle, target.lifecycle,
                "{description}"
            );
            let (post_basis, post_csrf, _) = projection_and_basis(&app).await;
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/verify",
                &post_csrf,
                &SliceCommand {
                    basis: post_basis.clone(),
                    slice_id: slice.clone(),
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{description}");
            let attempt: CompletionAttempt =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("lifecycle completion attempt");
            assert_eq!(
                attempt.report.status,
                mitase_work_model::CompletionStatus::Complete,
                "{description}: {attempt:?}"
            );
            let receipt = attempt.receipt.expect("lifecycle receipt");
            assert_eq!(receipt.lifecycle_proofs.len(), 1, "{description}");
            assert_eq!(receipt.lifecycle_proofs[0].reference, target.reference);
            assert_eq!(receipt.lifecycle_proofs[0].transition, transition);

            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/finalize/preview",
                &post_csrf,
                &FinalizeCommand {
                    basis: post_basis.clone(),
                    attempt_id: attempt.attempt_id.clone(),
                    preview_token: None,
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{description}");
            let preview: FinalizationPreview =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("lifecycle finalization preview");
            assert_eq!(
                preview.status,
                mitase_work_model::CompletionStatus::Complete
            );
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/finalize/apply",
                &post_csrf,
                &FinalizeCommand {
                    basis: post_basis,
                    attempt_id: attempt.attempt_id,
                    preview_token: Some(preview.preview_token),
                },
            )
            .await;
            let status = response.status();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(
                status,
                StatusCode::OK,
                "{description}: {}",
                String::from_utf8_lossy(&body)
            );
            let finalization: mitase_work_model::FinalizationReceipt =
                serde_json::from_slice(&body).expect("lifecycle finalization receipt");
            assert_eq!(finalization.lifecycle_proofs.len(), 1, "{description}");
            assert_eq!(finalization.lifecycle_proofs[0].reference, target.reference);
            if target_id == "removed-symbol" {
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(readiness.status(), StatusCode::OK, "{description}");
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("post-finalization readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");
                let removed_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("finalized absent target readiness subject");
                assert!(removed_subject.ready, "{removed_subject:?}");

                // Absence evidence is durable across the commit that records
                // finalization. It must not be tied to the exact HEAD or
                // workspace fingerprint that existed immediately after the
                // lifecycle write.
                for args in [
                    vec!["add", "-A"],
                    vec!["commit", "-qm", "record finalized absence"],
                ] {
                    assert!(
                        Command::new("git")
                            .args(args)
                            .current_dir(temp.path())
                            .status()
                            .expect("record finalized absence commit")
                            .success()
                    );
                }
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("post-commit readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");

                // An unrelated, valid commit must also leave the historical
                // lifecycle proof usable. Only the current exact absence
                // obligation is relevant to this readiness decision.
                let unrelated = temp.path().join("src/unrelated.rs");
                let unrelated_content =
                    fs::read_to_string(&unrelated).expect("unrelated fixture source");
                fs::write(&unrelated, unrelated_content.replace("false", "true"))
                    .expect("unrelated approved change");
                for args in [
                    vec!["add", "src/unrelated.rs"],
                    vec!["commit", "-qm", "record unrelated approved change"],
                ] {
                    assert!(
                        Command::new("git")
                            .args(args)
                            .current_dir(temp.path())
                            .status()
                            .expect("record unrelated change commit")
                            .success()
                    );
                }
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("unrelated-change readiness");
                assert_eq!(readiness.status, "Ready", "{readiness:?}");

                // Readiness must reject a forged finalization record even
                // when its post-state fingerprint still matches. The
                // attempt, approval, exact Remove slice, and receipt proof
                // are the durable closure, not this top-level JSON alone.
                let store = DeliveryStore::for_workspace(temp.path()).expect("evidence store");
                let finalization_path =
                    fs::read_dir(store.root().join("completion/v1/finalizations"))
                        .expect("finalization directory")
                        .flatten()
                        .map(|entry| entry.path())
                        .find(|path| {
                            path.extension().and_then(|value| value.to_str()) == Some("json")
                        })
                        .expect("finalization evidence");
                let mut forged: serde_json::Value = serde_json::from_slice(
                    &fs::read(&finalization_path).expect("read finalization evidence"),
                )
                .expect("parse finalization evidence");
                forged["lifecycle_proofs"][0]["transition"] = serde_json::json!("add");
                fs::write(
                    &finalization_path,
                    serde_json::to_vec_pretty(&forged).expect("serialize forged evidence"),
                )
                .expect("forge finalization evidence");
                let readiness = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf.clone())
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("forged-evidence readiness");
                let forged_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("forged finalized absent target subject");
                assert!(!forged_subject.ready, "{forged_subject:?}");

                let source = temp.path().join("src/removable.rs");
                let mut restored = fs::read_to_string(&source).expect("removed source");
                restored.push_str("\npub fn remove_me() {}\n");
                fs::write(&source, restored).expect("restore removed target");
                let readiness = app
                    .oneshot(
                        Request::builder()
                            .method(Method::POST)
                            .uri("/api/readiness/run")
                            .header("origin", "http://127.0.0.1:7737")
                            .header("x-mitase-csrf-token", post_csrf)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(readiness.status(), StatusCode::OK, "{description}");
                let readiness: ReadinessView = serde_json::from_slice(
                    &readiness.into_body().collect().await.unwrap().to_bytes(),
                )
                .expect("restored-target readiness");
                let restored_subject = readiness
                    .axes
                    .get("seedability")
                    .and_then(|axis| {
                        axis.subjects.iter().find(|subject| {
                            subject.id.contains("criterion.remove-symbol")
                                && subject.id.contains("removed-symbol")
                        })
                    })
                    .expect("restored target readiness subject");
                assert!(!restored_subject.ready, "{restored_subject:?}");
                assert!(
                    restored_subject
                        .blockers
                        .iter()
                        .any(|blocker| blocker.contains("finalized lifecycle proof"))
                );
            }
        }
    }
    #[cfg(any())]
    #[tokio::test]
    async fn legacy_workbench_agent_rejects_stale_or_newly_existing_lifecycle_targets() {
        let _workspace_lock = workspace_test_lock().await;
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/v1/valid-workbench-flow");
        let cases = [
            (
                "added-symbol",
                mitase_work_model::TargetTransition::Add,
                "src/lib.rs",
                "\npub fn added_behavior() -> bool { true }\n",
                "now exists",
            ),
            (
                "added-file",
                mitase_work_model::TargetTransition::Add,
                "src/added.rs",
                "pub fn preexisting_file() {}\n",
                "now exists",
            ),
            (
                "removed-symbol",
                mitase_work_model::TargetTransition::Remove,
                "src/removable.rs",
                "pub fn remove_me() { panic!(\"changed\") }\n",
                "is stale",
            ),
            (
                "removed-file",
                mitase_work_model::TargetTransition::Remove,
                "remove-file.txt",
                "changed before approved removal\n",
                "is stale",
            ),
        ];
        for (target_id, transition, path, changed_content, expected_blocker) in cases {
            let temp = tempfile::tempdir().expect("lifecycle precondition fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            initialize_fixture_git(temp.path());
            let server = WorkbenchServer::new(temp.path().to_path_buf());
            let app = server.router();
            let (_, _, _, run, target) =
                start_lifecycle_agent(&server, &app, target_id, transition).await;

            let changed_path = temp.path().join(path);
            if target_id == "added-symbol" {
                let mut content = fs::read_to_string(&changed_path).expect("existing source");
                content.push_str(changed_content);
                fs::write(&changed_path, content).expect("create approved target early");
            } else {
                fs::write(&changed_path, changed_content).expect("change lifecycle target");
            }
            let (basis, csrf, _) = projection_and_basis(&app).await;
            let write = match target_id {
                "added-symbol" => mitase_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool { true }\n".into(),
                },
                "added-file" => mitase_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn approved_file() {}\n".into(),
                },
                "removed-symbol" => mitase_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => mitase_work_model::AgentTargetWrite::RemoveFile {
                    target: target.reference.clone(),
                    expected_content_hash: target.content_hash.clone(),
                },
                _ => unreachable!("declared lifecycle precondition case"),
            };
            let response = json_mutation(
                &app,
                Method::POST,
                "/api/work/agent/patch",
                &csrf,
                &AgentPatchCommand {
                    basis,
                    run_id: run.run_id.clone(),
                    patch: AgentPatch {
                        schema: mitase_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id,
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![write],
                    },
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::CONFLICT, "{target_id}");
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert!(
                String::from_utf8_lossy(&body).contains(expected_blocker),
                "{target_id}: {}",
                String::from_utf8_lossy(&body)
            );
            assert_eq!(
                fs::read_to_string(&changed_path).expect("rejected write preserves target"),
                if target_id == "added-symbol" {
                    let mut expected =
                        fs::read_to_string(fixture.join(path)).expect("fixture source");
                    expected.push_str(changed_content);
                    expected
                } else {
                    changed_content.to_string()
                },
                "{target_id}"
            );
        }
    }
}
