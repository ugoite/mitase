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
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};
use syu_delivery::DeliveryStore;
use syu_diagnostics::{Severity, ValidationPhase, ValidationResult};
use syu_planner::{
    SplitWorkRecommendation, TargetSuggestion, TargetSuggestionSet, plan,
    split_work_recommendation, suggest_targets,
};
use syu_project_model::{ChangeBaseline, ReadinessLevel, ValidationPreset};
use syu_spec_model::format_sha256;
use syu_spec_model::{
    ArtifactBinding, ArtifactTarget, BindingRole, BoundTargetRef, Contract, ContractKind,
    Criterion, CriterionKind, ItemStatus, LocalAnchorKind, LocalId, OwnershipScope, Philosophy,
    Policy, Priority, RepoPath, Requirement, Rule, RuleLevel, Selector, SpecAnchor, SpecDocument,
    SpecId, TargetClaim,
};
use syu_validation::{ChangeStatus, PlanValidationMode, ValidationContext, validate};
use syu_work_model::{
    AgentBlocker, AgentEvent, AgentPatch, AgentRun, AgentRunStatus, COMPLETION_ATTEMPT_SCHEMA,
    CompletionAttempt, CompletionStatus, ExecutionIdentity, FinalizationPreview,
    FinalizationReceipt, PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus, RequestedTarget,
    TargetAccessMode, TargetTransition, VerificationReceipt, WORK_REQUEST_SCHEMA, WorkConstraints,
    WorkOperation, WorkOrigin, WorkPlan, WorkRequest,
};
use syu_workspace::{SpecIndex, SpecWorkspace};

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
    pub context_pack: Option<syu_work_model::ContextPack>,
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
        syu_planner::validate_work_request(&index, &request)
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
            println!("Syu Workbench listening on http://{bind}:{port}");
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
            let cached = Arc::new(CachedWorkspaceSnapshot {
                signature,
                workspace,
                index,
                revision,
                projection,
            });
            *self
                .snapshot
                .write()
                .map_err(|_| anyhow::anyhow!("workbench snapshot lock"))? = Some(cached.clone());
            return Ok(cached);
        }
        anyhow::bail!("workspace changed while loading the Workbench snapshot")
    }
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
    let mut hash = Sha256::new();
    hash.update(b"syu/workbench-snapshot/v1\0");
    hash.update(revision.as_bytes());
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
            response.headers_mut().insert("x-syu-csrf-token", token);
        }
        return response;
    }
    let headers = request.headers();
    let csrf_valid = headers
        .get("x-syu-csrf-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == security.csrf_token);
    let origin_valid = headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin == security.expected_origin);
    let session_valid = security.remote_session_token.as_ref().is_none_or(|token| {
        headers
            .get("x-syu-session-token")
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
        /// A journey-only exact Requirement Criterion link. Feature documents
        /// do not own criteria, but the guided authoring flow must retain the
        /// human-selected anchor while it continues to target review.
        #[serde(default)]
        criterion_anchor: Option<String>,
        /// An exact planned implementation target owned by the Feature. The
        /// target remains advisory until a human approves its suggestion.
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
        config: Box<syu_project_model::ProjectConfig>,
    },
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
        if let Some(payload) = message.strip_prefix("__SYU_STRUCTURED__")
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
        anyhow::anyhow!(format!("__SYU_STRUCTURED__{}", value)),
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
            "schema": syu_work_model::WORK_ERROR_SCHEMA,
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
            "schema": syu_work_model::WORK_ERROR_SCHEMA,
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
    Html(include_str!("../../syu-app-ui/assets/workbench.html"))
}

async fn api_asset(AxumPath(asset): AxumPath<String>) -> Response {
    let (content_type, content): (&str, String) = match asset.as_str() {
        "workbench.css" => (
            "text/css; charset=utf-8",
            include_str!("../../syu-app-ui/assets/workbench.css").into(),
        ),
        "i18n.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/i18n.js").into(),
        ),
        "catalog.js" => (
            "text/javascript; charset=utf-8",
            format!(
                "window.SYU_I18N={{en:{},ja:{}}};",
                include_str!("../../syu-app-ui/assets/locales/en.json"),
                include_str!("../../syu-app-ui/assets/locales/ja.json")
            ),
        ),
        "js/main.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/main.js").into(),
        ),
        "js/api.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/api.js").into(),
        ),
        "js/state.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/state.js").into(),
        ),
        "js/router.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/router.js").into(),
        ),
        "js/i18n.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/i18n.js").into(),
        ),
        "js/components/action.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/action.js").into(),
        ),
        "js/components/diagnostic.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/diagnostic.js").into(),
        ),
        "js/components/diff.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/diff.js").into(),
        ),
        "js/components/editor.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/editor.js").into(),
        ),
        "js/components/readiness.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/readiness.js").into(),
        ),
        "js/components/target.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/components/target.js").into(),
        ),
        "js/pages/work.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/work.js").into(),
        ),
        "js/pages/readiness.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/readiness.js").into(),
        ),
        "js/pages/scope.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/scope.js").into(),
        ),
        "js/pages/specifications.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/specifications.js").into(),
        ),
        "js/pages/diagnostics.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/diagnostics.js").into(),
        ),
        "js/pages/settings.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/js/pages/settings.js").into(),
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
    let report = syu_validation::evaluate_readiness(
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
            let result = syu_validation::validate_without_readiness(&ValidationContext {
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
            let result = syu_validation::validate_without_readiness(&ValidationContext {
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
            let canonical_plan = syu_validation::canonical_plan_for_execution(
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
            let result = syu_validation::validate_without_readiness(&ValidationContext {
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

fn filtered_target_suggestions(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
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
    if let Some(split_recommendation) = split_work_recommendation(&approved, workspace, index) {
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
    if transition_groups.len() > 1 {
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
        // Approval records the exact advisory evidence.  WorkRequest creation
        // remains an explicit journey action so a suggestion can never become
        // executable scope merely by being displayed or approved.
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
    if let Some(EditPatch::AddCriterion { requirement_id, .. }) = patch {
        affected_items.insert(requirement_id.to_string());
    }
    match patch {
        Some(EditPatch::CreateRequirement { id, .. })
        | Some(EditPatch::CreateFeature { id, .. }) => {
            affected_items.insert(id.to_string());
        }
        Some(EditPatch::AddFeatureTarget { feature_id, .. }) => {
            affected_items.insert(feature_id.to_string());
        }
        _ => {}
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
                Some(syu_workspace::AnchorValue::Criterion(_))
            )
        })
        .map(|anchor| suggest_targets(&anchor, candidate, &candidate_index))
        .collect::<Result<Vec<_>>>()?;
    let revision = current_revision(&base.root)?;
    let before = syu_validation::evaluate_readiness(base, &base_index, &revision, false)?;
    let after = syu_validation::evaluate_readiness(candidate, &candidate_index, &revision, false)?;
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
    index: &syu_workspace::SpecIndex,
    targets: &BTreeSet<syu_spec_model::BoundTargetRef>,
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

fn readiness_impact(report: &syu_validation::ReadinessReport) -> ReadinessImpact {
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
    index: &syu_workspace::SpecIndex,
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
                            lifecycle: syu_spec_model::ArtifactTargetLifecycle::Present,
                            claims,
                        }],
                    }])
                })
                .transpose()?
                .unwrap_or_default();
            let feature = syu_spec_model::Feature {
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
            let mut feature: syu_spec_model::Feature = serde_yaml::from_value(item.clone())?;
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
                lifecycle: syu_spec_model::ArtifactTargetLifecycle::Present,
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
    if candidate.schema() != syu_spec_model::SPEC_SCHEMA {
        anyhow::bail!("specification schema must be syu/spec/v1");
    }
    Ok(content)
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
        | EditPatch::CreateRequirement { .. }
        | EditPatch::CreateFeature { .. }
        | EditPatch::AddFeatureTarget { .. } => specification_patch_content(workspace, path, patch),
        EditPatch::Config { config } => Ok(serde_yaml::to_string(config)?),
    }
}

fn validate_overlay(workspace: &SpecWorkspace, index: &syu_workspace::SpecIndex) -> Result<()> {
    let revision = current_revision(&workspace.root)?;
    // Preview is a structural operation.  It must never execute a candidate
    // runner (the candidate config may contain an arbitrary executable).  The
    // explicit POST /api/readiness/run and /api/work/verify paths are the only
    // execution entry points.
    let result = syu_validation::validate_without_readiness(&syu_validation::ValidationContext {
        config: &workspace.config,
        workspace,
        index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: None,
        selected_slice: None,
        plan_mode: syu_validation::PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(&revision),
        change_base_revision: None,
    });
    if result
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, syu_diagnostics::Severity::Error))
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
        ".syu-edit-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&temporary, content)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
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
    let snapshot = basis(&service, &command.basis)?;
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
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        return Err(error.into());
    }
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
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        return Err(error.into());
    }
    if requires_replan {
        let mut session = service
            .session
            .write()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        session.plan = None;
        session.context_pack = None;
        session.verification_receipt = None;
        session.last_validation = None;
        session.selected_slice = None;
    }
    Ok(Json(preview))
}

async fn api_config_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let content = edit_content(workspace, &workspace.root.join("syu.yaml"), &command.patch)?;
    let config: syu_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    Ok(Json(edit_preview(
        workspace,
        &overlay,
        &workspace.root.join("syu.yaml"),
        &content,
    )?))
}

async fn api_config(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<syu_project_model::ProjectConfig>, ApiError> {
    Ok(Json(SpecWorkspace::load(&service.workspace_root)?.config))
}

async fn api_config_apply(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let snapshot = basis(&service, &command.basis)?;
    let workspace = &snapshot.workspace;
    let content = edit_content(workspace, &workspace.root.join("syu.yaml"), &command.patch)?;
    let config: syu_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let path = workspace.root.join("syu.yaml");
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
    atomic_replace(&path, &content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        return Err(error.into());
    }
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
    changed: &[syu_validation::ChangedFile],
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
            && matches!(changed_file.status, syu_validation::ChangeStatus::Untracked)
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

fn branch_changed_files(root: &Path, range: &str) -> Result<Vec<syu_validation::ChangedFile>> {
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
        let Ok(path) = syu_spec_model::RepoPath::new(String::from_utf8_lossy(path).as_ref()) else {
            continue;
        };
        if files
            .iter()
            .all(|file| file.new_path.as_ref() != Some(&path))
        {
            files.push(syu_validation::ChangedFile {
                status: syu_validation::ChangeStatus::Untracked,
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
    files: &mut Vec<syu_validation::ChangedFile>,
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
            .map(syu_spec_model::RepoPath::new)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let new_path = new_text
            .as_deref()
            .map(syu_spec_model::RepoPath::new)
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let status = match kind {
            'A' => syu_validation::ChangeStatus::Added,
            'D' => syu_validation::ChangeStatus::Deleted,
            'R' => syu_validation::ChangeStatus::Renamed,
            _ => syu_validation::ChangeStatus::Modified,
        };
        if let Some(file) = files
            .iter_mut()
            .find(|file| file.new_path == new_path && file.old_path == old_path)
        {
            file.status = status;
        } else {
            files.push(syu_validation::ChangedFile {
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
    files: &mut [syu_validation::ChangedFile],
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
            file.hunks.push(syu_validation::ChangedRange {
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
        let resolved = syu_workspace::resolve_target_in_workspace(&snapshot.workspace, artifact)?;
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
    let relative = syu_spec_model::RepoPath::new(&source_path)
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
    syu_planner::validate_work_origin(index, origin)
}

fn validate_work_origin(
    _workspace: &SpecWorkspace,
    index: &SpecIndex,
    origin: &WorkOrigin,
) -> Result<()> {
    if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
        validate_requirement_origin(index, origin)
    } else {
        syu_planner::validate_work_origin(index, origin)
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

fn resolve_requested_targets(
    anchor: &SpecAnchor,
    suggestions: Result<Vec<TargetSuggestion>>,
    approvals: &[ApprovedTargetSuggestion],
) -> Result<Vec<RequestedTarget>> {
    Ok(
        resolve_approved_target_candidates(anchor, suggestions, approvals)?
            .into_iter()
            .map(|candidate| RequestedTarget {
                reference: candidate.reference,
                criterion: Some(anchor.clone()),
                transition: candidate.transition,
            })
            .collect(),
    )
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
                syu_spec_model::ArtifactTargetLifecycle::Absent
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
            if schema != syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA {
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
            if schema != syu_work_model::WORK_SELECT_SLICE_SCHEMA {
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
        JourneyAction::Start { execution } | JourneyAction::Retry { execution } => {
            let _ = api_agent_start(
                State(service.clone()),
                Json(AgentStartCommand {
                    basis: basis_command.clone(),
                    execution,
                }),
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
            schema: syu_work_model::WORK_SELECT_SLICE_RESPONSE_SCHEMA.into(),
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
    if store
        .has_approval_for_plan(candidate_plan_digest)
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
    let canonical_candidate_plan = syu_planner::plan(
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
    let replanned = syu_planner::plan(
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
    selected: &[syu_work_model::PlannedTarget],
    candidate: &[syu_work_model::PlannedTarget],
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
    slice: &syu_work_model::ExecutionSlice,
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
    syu_planner::validate_work_request(&snapshot.index, &request)
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
) -> Result<Json<syu_work_model::ContextPack>, ApiError> {
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
    let context = syu_planner::export_context(
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
    let canonical = syu_validation::canonical_plan_for_execution(
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
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
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
        workspace_fingerprint: snapshot.projection.snapshot.fingerprint.clone(),
        revision: snapshot.revision.clone(),
        reviewed_at: timestamp(),
        plan: canonical,
    };
    Ok(Json(store.approve(&approval)?))
}

async fn api_agent_start(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentStartCommand>,
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
    let run = syu_agent::start_run(&snapshot.workspace, &approval, &command.execution.slice_id)?;
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
) -> Result<Json<syu_work_model::AgentPatchRecord>, ApiError> {
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&snapshot.workspace, &run)?;
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
    match syu_agent::apply_scoped_patch(&snapshot.workspace, &run, &command.patch) {
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
    let run = syu_agent::current_run(&snapshot.workspace, &run)?;
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
    let event = syu_agent::record_blocker(&snapshot.workspace, &run, command.blocker)?;
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
    let run = syu_agent::current_run(&snapshot.workspace, &run)?;
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
    Ok(Json(syu_agent::request_scope_expansion(
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
    let snapshot = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&snapshot.workspace, &run)?;
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
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
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
    let attempt_id = store.new_id("attempt");
    let started_at = timestamp();
    let (verification, receipt, mut report) = syu_validation::execute_verification_attempt(
        &snapshot.workspace,
        &snapshot.index,
        &approval.plan,
        &command.execution.slice_id,
        &snapshot.revision,
        &attempt_id,
    )?;
    report.attempt_id = attempt_id.clone();
    let mut attempt = CompletionAttempt {
        schema: COMPLETION_ATTEMPT_SCHEMA.into(),
        attempt_id,
        attempt_digest: String::new(),
        plan_digest: approval.plan_digest.clone(),
        slice_id: command.execution.slice_id,
        approved_plan_digest: approval.plan_digest,
        started_at,
        completed_at: timestamp(),
        verification,
        receipt,
        report,
    };
    attempt.attempt_digest = DeliveryStore::verification_digest(&{
        let mut copy = attempt.clone();
        copy.attempt_digest.clear();
        copy
    })?;
    let attempt = store.append_attempt(&attempt)?;
    syu_agent::record_verification(&snapshot.workspace, &run, &attempt.attempt_id)?;
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
    let canonical_plan = syu_validation::canonical_plan_for_execution(
        &snapshot.workspace,
        &snapshot.index,
        plan,
        &snapshot.revision,
    )
    .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    let result = syu_validation::validate_without_readiness(&ValidationContext {
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
    let snapshot = execution_basis(&service, &command.basis)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
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
    let attempt_id = store.new_id("attempt");
    let started_at = timestamp();
    let (verification, receipt, mut report) = syu_validation::execute_verification_attempt(
        &snapshot.workspace,
        &snapshot.index,
        plan,
        &command.execution.slice_id,
        &snapshot.revision,
        &attempt_id,
    )?;
    report.attempt_id = attempt_id.clone();
    let mut attempt = CompletionAttempt {
        schema: COMPLETION_ATTEMPT_SCHEMA.into(),
        attempt_id,
        attempt_digest: String::new(),
        plan_digest: plan.canonical_digest.clone(),
        slice_id: command.execution.slice_id,
        approved_plan_digest: approval.plan_digest,
        started_at,
        completed_at: timestamp(),
        verification,
        receipt,
        report,
    };
    attempt.attempt_digest = DeliveryStore::verification_digest(&{
        let mut copy = attempt.clone();
        copy.attempt_digest.clear();
        copy
    })?;
    let attempt = store.append_attempt(&attempt)?;
    session.verification_receipt = attempt.receipt.clone();
    Ok(Json(attempt))
}

async fn api_finalize_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<FinalizeCommand>,
) -> Result<Json<FinalizationPreview>, ApiError> {
    let snapshot = execution_basis(&service, &command.basis)?;
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
    let attempt = store.attempt(&command.execution, &command.attempt_id)?;
    if attempt.plan_digest != command.execution.plan_digest
        || attempt.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("attempt does not match execution identity"),
        ));
    }
    Ok(Json(
        store.finalization_preview(&snapshot.workspace, &attempt)?,
    ))
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
    let snapshot = execution_basis(&service, &command.basis)?;
    let store = DeliveryStore::for_workspace(&snapshot.workspace.root)?;
    let attempt = store.attempt(&command.execution, &command.attempt_id)?;
    if attempt.plan_digest != command.execution.plan_digest
        || attempt.slice_id != command.execution.slice_id
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("attempt does not match execution identity"),
        ));
    }
    let preview = store.finalization_preview(&snapshot.workspace, &attempt)?;
    Ok(Json(store.apply_finalization(
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
    let snapshot = execution_basis(&service, &command.basis)?;
    let (plan, canonical) = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))
        .and_then(|session| {
            Ok((
                session
                    .plan
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("no work plan"))?,
                session.verification_receipt.clone().ok_or_else(|| {
                    anyhow::anyhow!("no server-generated verification receipt exists")
                })?,
            ))
        })?;
    if command.execution.plan_digest != plan.canonical_digest
        || command.execution.slice_id != canonical.slice_id
        || command.receipt != canonical
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("verification receipt does not close the selected plan"),
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
    let changed_files = syu_validation::changed_files_against_revision(
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
    pub acceptance: Vec<syu_work_model::AcceptanceRef>,
    pub contracts: Vec<ContractRefView>,
    pub origin_closure: syu_work_model::OriginClosure,
    pub origin_closure_digest: String,
    pub budget: syu_work_model::SliceBudgetUsage,
    pub confidence: syu_work_model::PlanConfidence,
    pub blockers: Vec<DiagnosticView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedTargetView {
    pub reference: BoundTargetRef,
    pub access: TargetAccessMode,
    pub transition: TargetTransition,
    pub lifecycle: syu_work_model::TargetLifecycle,
    pub path: String,
    pub selector: syu_work_model::ResolvedSelector,
    pub artifact_identity: Option<String>,
    pub adapter: String,
    pub facet: String,
    pub role: String,
    pub verification_claim: Option<syu_work_model::VerificationClaimRef>,
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
    pub primary: syu_diagnostics::Location,
    pub related: Vec<syu_diagnostics::RelatedLocation>,
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
    pub blockers: Vec<syu_work_model::CompletionBlocker>,
    pub next_action: Option<String>,
    pub finalized: bool,
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
    pub axes: BTreeMap<String, syu_validation::ReadinessAxis>,
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
    pub diagnostic: syu_diagnostics::Diagnostic,
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
    syu_validation::RULES
        .iter()
        .filter(|rule| {
            rule.presets.contains(&preset)
                && syu_validation::phase_for_rule(rule.id) == validation_phase_from_id(phase)
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
    pub lifecycle: syu_spec_model::ArtifactTargetLifecycle,
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
                    matches!(plan.status, syu_work_model::PlanStatus::Ready)
                        && !plan.slices.is_empty()
                }),
                disabled_reason: plan
                    .as_ref()
                    .is_none_or(|plan| {
                        !matches!(plan.status, syu_work_model::PlanStatus::Ready)
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
                        syu_spec_model::SpecDocument::Philosophies { .. } => "philosophy",
                        syu_spec_model::SpecDocument::Policies { .. } => "policy",
                        syu_spec_model::SpecDocument::Requirements { .. } => "requirement",
                        syu_spec_model::SpecDocument::Features { .. } => "feature",
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

fn target_view(target: &syu_work_model::PlannedTarget) -> TargetView {
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
    config: &syu_project_model::ProjectConfig,
    index: &SpecIndex,
) -> SplitRecoveryView {
    let criterion = request.origin.criterion().clone();
    let statement = index
        .anchors
        .get(&criterion)
        .and_then(|value| match value {
            syu_workspace::AnchorValue::Criterion(value) => Some(value.statement.clone()),
            _ => None,
        })
        .unwrap_or_else(|| criterion.to_string());
    let reason = split_reason(plan, config);
    SplitRecoveryView {
        schema: syu_work_model::WORK_SPLIT_RECOVERY_SCHEMA.into(),
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
                    origin_closure: plan.origin_closure.clone(),
                    origin_closure_digest: plan.origin_closure_digest.clone(),
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
    slice: &syu_work_model::ExecutionSlice,
    request: &WorkRequest,
    config: &syu_project_model::ProjectConfig,
    index: &SpecIndex,
    blockers: &[syu_diagnostics::Diagnostic],
) -> bool {
    plan.status == PlanStatus::Ready
        && blockers.is_empty()
        && slice.confidence != syu_work_model::PlanConfidence::Low
        && (request.operation == WorkOperation::Investigate || !slice.editable_targets.is_empty())
        && slice_budget_within_limits(slice, config)
        && slice_targets_are_active(index, slice)
        && origin_closure_is_complete(plan, request, slice, index)
        && (request.operation == WorkOperation::Investigate
            || verification_covers_editable_targets(index, slice, request.origin.criterion()))
}

fn split_candidate_blockers(
    plan: &WorkPlan,
    slice: &syu_work_model::ExecutionSlice,
    request: &WorkRequest,
    config: &syu_project_model::ProjectConfig,
    index: &SpecIndex,
) -> Vec<syu_diagnostics::Diagnostic> {
    let mut blockers = slice.blockers.clone();
    if plan.status != PlanStatus::Ready
        && !blockers
            .iter()
            .any(|blocker| blocker.rule_id == "plan-needs-review")
    {
        blockers.push(syu_diagnostics::Diagnostic::error(
            "plan-needs-review",
            "the canonical candidate plan is blocked and cannot be selected",
            "work-plan",
        ));
    }
    if slice.confidence == syu_work_model::PlanConfidence::Low {
        blockers.push(syu_diagnostics::Diagnostic::error(
            "low-confidence-slice",
            "the candidate requires exact server review before it can be selected",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate && slice.editable_targets.is_empty() {
        blockers.push(syu_diagnostics::Diagnostic::error(
            "missing-editable-target",
            "the candidate has no exact editable target",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate && slice.verification_targets.is_empty() {
        blockers.push(syu_diagnostics::Diagnostic::error(
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
        if !active_exact_target(index, &target.reference) {
            blockers.push(syu_diagnostics::Diagnostic::error(
                "target-lifecycle",
                format!(
                    "candidate target {} is not an active exact artifact",
                    target.reference
                ),
                target.resolved_path.clone(),
            ));
        }
    }
    if !origin_closure_is_complete(plan, request, slice, index) {
        blockers.push(syu_diagnostics::Diagnostic::error(
            "missing-origin-closure",
            "the candidate does not retain a complete active implementation, verification, readonly, and contract closure",
            "work-plan",
        ));
    }
    if request.operation != WorkOperation::Investigate
        && !verification_covers_editable_targets(index, slice, request.origin.criterion())
    {
        blockers.push(syu_diagnostics::Diagnostic::error(
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
        blockers.push(syu_diagnostics::Diagnostic::error(
            "missing-origin-closure",
            "the candidate does not retain the complete implementation origin closure",
            "work-plan",
        ));
    }
    if !slice_budget_within_limits(slice, config) {
        blockers.push(syu_diagnostics::Diagnostic::error(
            "budget-exceeded",
            "the candidate exceeds a configured execution limit",
            "work-plan",
        ));
    }
    blockers
}

fn active_exact_target(index: &SpecIndex, reference: &BoundTargetRef) -> bool {
    index.target(reference).is_some_and(|target| {
        !matches!(
            target.lifecycle,
            syu_spec_model::ArtifactTargetLifecycle::Absent
        )
    })
}

fn slice_targets_are_active(index: &SpecIndex, slice: &syu_work_model::ExecutionSlice) -> bool {
    slice
        .editable_targets
        .iter()
        .chain(slice.verification_targets.iter())
        .chain(slice.readonly_context.iter())
        .all(|target| active_exact_target(index, &target.reference))
}

fn verification_covers_editable_targets(
    index: &SpecIndex,
    slice: &syu_work_model::ExecutionSlice,
    criterion: &SpecAnchor,
) -> bool {
    let valid_verifications = slice
        .verification_targets
        .iter()
        .filter(|target| implemented_verification_claim(index, target, criterion))
        .collect::<Vec<_>>();
    !slice.editable_targets.is_empty()
        && slice.editable_targets.iter().all(|editable| {
            valid_verifications.iter().any(|verification| {
                index
                    .verification_by_target
                    .get(&editable.reference)
                    .is_some_and(|covered| covered.contains(&verification.reference))
            })
        })
}

fn implemented_verification_claim(
    index: &SpecIndex,
    target: &syu_work_model::PlannedTarget,
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
    if binding.role != BindingRole::Verification
        || index.item_status.get(&target.reference.binding.item) != Some(&ItemStatus::Implemented)
    {
        return false;
    }
    let Some(artifact) = index.target(&target.reference) else {
        return false;
    };
    if matches!(
        artifact.lifecycle,
        syu_spec_model::ArtifactTargetLifecycle::Absent
    ) {
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
    slice: &syu_work_model::ExecutionSlice,
    index: &SpecIndex,
) -> bool {
    let closure = &plan.origin_closure;
    if request
        .origin
        .targets()
        .iter()
        .any(|target| !closure.implementation_targets.contains(target))
    {
        return false;
    }
    if closure
        .implementation_targets
        .iter()
        .chain(closure.verification_targets.iter())
        .chain(closure.readonly_targets.iter())
        .any(|target| !active_exact_target(index, target))
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
                std::iter::once(&contract.source)
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
                || contract
                    .guarantees
                    .iter()
                    .any(|guarantee| guarantee != request.origin.criterion())
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

fn split_reason(plan: &WorkPlan, config: &syu_project_model::ProjectConfig) -> SplitReasonView {
    let budget = plan.slices.iter().fold(
        syu_work_model::SliceBudgetUsage::default(),
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
            .unwrap_or("The candidate exceeds a configured execution limit.");
        let lower = message.to_ascii_lowercase();
        let code = if lower.contains("verification") {
            "verification-budget"
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
    slice: &syu_work_model::ExecutionSlice,
    config: &syu_project_model::ProjectConfig,
) -> bool {
    let limits = &config.work.slicing;
    slice.budget.editable_files <= limits.max_editable_files
        && slice.budget.editable_symbols <= limits.max_editable_symbols
        && slice.budget.verification_targets <= limits.max_verification_targets
        && slice.budget.readonly_targets <= limits.max_readonly_targets
        && slice.budget.total_bytes <= limits.max_total_bytes
}

fn planned_target_view(target: &syu_work_model::PlannedTarget) -> PlannedTargetView {
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

fn diagnostic_view(diagnostic: &syu_diagnostics::Diagnostic) -> DiagnosticView {
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
    }
}

fn completion_history(workspace: &SpecWorkspace) -> Result<CompletionHistoryView> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let attempts = store.attempts()?;
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
        });
    }
    let mut iter = views.into_iter();
    Ok(CompletionHistoryView {
        current: iter.next(),
        previous: iter.collect(),
    })
}

fn context_pack_view(context: &syu_work_model::ContextPack) -> ContextPackView {
    ContextPackView {
        plan_digest: context.plan_digest.clone(),
        slice_id: context.slice_id.clone(),
        entry_count: context.artifact_context.len(),
    }
}

fn readiness_view(report: &syu_validation::ReadinessReport) -> ReadinessView {
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

fn readiness_not_run(config: &syu_project_model::ProjectConfig) -> ReadinessView {
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
        .map(|run| syu_agent::current_run(workspace, run))
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
        .map(|run| syu_agent::events(workspace, run))
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
    index: &syu_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    revision: &str,
) -> Result<VerificationReceipt> {
    syu_validation::execute_verification(workspace, index, plan, slice_id, revision)
}

pub fn validate_verification_receipt(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    receipt: &VerificationReceipt,
    revision: &str,
) -> Result<()> {
    syu_validation::validate_verification_receipt(
        workspace, index, plan, slice_id, receipt, revision,
    )
}

pub fn branch_scope_view(
    index: &syu_workspace::SpecIndex,
    items: &[ItemSummary],
    range: String,
    files: &[syu_validation::ChangedFile],
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
    index: &syu_workspace::SpecIndex,
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
    index: &syu_workspace::SpecIndex,
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
    index: &syu_workspace::SpecIndex,
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
    item: &syu_spec_model::Feature,
    path: &str,
    source_hash: &str,
    index: &syu_workspace::SpecIndex,
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
                let targets = binding
                    .targets
                    .iter()
                    .filter_map(|target| {
                        let reference = target.reference.parse::<BoundTargetRef>().ok()?;
                        index
                            .target(&reference)
                            .filter(|artifact| {
                                !matches!(
                                    artifact.lifecycle,
                                    syu_spec_model::ArtifactTargetLifecycle::Absent
                                )
                            })
                            .map(|_| reference)
                    })
                    .collect::<Vec<_>>();
                let mut targets = targets;
                targets.sort();
                targets.dedup();
                let criteria = binding
                    .targets
                    .iter()
                    .flat_map(|target| target.claims.iter())
                    .filter_map(|claim| match claim {
                        TargetClaim::Satisfies { criterion } => Some(criterion.clone()),
                        _ => None,
                    })
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
                    for target in targets {
                        capabilities.push(origin_capability(
                            workspace,
                            index,
                            WorkOrigin::FeatureImplementationTarget {
                                target,
                                binding: binding_anchor.clone(),
                                criterion: criterion.clone(),
                            },
                            "Implementation target",
                        ));
                    }
                } else {
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
                        schema: syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
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
            schema: syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
            origin: Some(origin),
            label: label.into(),
            enabled: true,
            disabled_code: None,
            disabled_message: None,
            nearest: vec![],
        },
        Err(error) => OriginCapabilityView {
            schema: syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA.into(),
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

fn anchors_for(index: &syu_workspace::SpecIndex, item: &syu_spec_model::SpecId) -> Vec<String> {
    index
        .item_anchors
        .get(item)
        .into_iter()
        .flatten()
        .map(ToString::to_string)
        .collect()
}

fn rule_summary(item: &syu_spec_model::SpecId, rule: &Rule) -> RuleSummary {
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
            syu_spec_model::RuleEnforcement::External(text) => text.clone(),
        }),
    }
}

fn criterion_summary(item: &syu_spec_model::SpecId, criterion: &Criterion) -> CriterionSummary {
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
    item: &syu_spec_model::SpecId,
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

fn contract_summary(item: &syu_spec_model::SpecId, contract: &Contract) -> ContractSummary {
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
    item: &syu_spec_model::SpecId,
    kind: LocalAnchorKind,
    local_id: &syu_spec_model::LocalId,
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
    use std::sync::OnceLock;
    use syu_work_model::TargetTransition;
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
        assert_eq!(source["path"], "crates/syu-workbench-server/src/lib.rs");
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
            .get("x-syu-csrf-token")
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
                    .header("x-syu-csrf-token", csrf)
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
            vec!["config", "user.email", "syu-tests@example.invalid"],
            vec!["config", "user.name", "Syu Tests"],
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
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-POST-STATE".into(),
            title: "modify the fixture behavior".into(),
            operation: syu_work_model::WorkOperation::Modify,
            origin: syu_work_model::WorkOrigin::RequirementCriterion {
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
        assert_eq!(plan.status, syu_work_model::PlanStatus::Ready, "{plan:?}");
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
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-AGENT".into(),
            title: "scoped agent fixture change".into(),
            operation: syu_work_model::WorkOperation::Modify,
            origin: syu_work_model::WorkOrigin::RequirementCriterion {
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
                        schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                        run_id: run.run_id.clone(),
                        expected_workspace_fingerprint: run
                            .context
                            .context
                            .basis
                            .workspace_fingerprint
                            .clone(),
                        writes: vec![syu_work_model::AgentTargetWrite::Replace {
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
                    schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![syu_work_model::AgentTargetWrite::Replace {
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
            target_id: syu_spec_model::LocalId("unrelated".into()),
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
                    schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![syu_work_model::AgentTargetWrite::Replace {
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
                    code: "SYU-AGENT-TEST".into(),
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
                schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
                id: "WORK-FIXTURE-NEXT".into(),
                title: "a subsequent work request".into(),
                operation: syu_work_model::WorkOperation::Modify,
                origin: syu_work_model::WorkOrigin::RequirementCriterion {
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

    async fn start_lifecycle_agent(
        server: &WorkbenchServer,
        app: &Router,
        target_id: &str,
        transition: syu_work_model::TargetTransition,
    ) -> (
        MutationBasis,
        String,
        String,
        AgentRun,
        syu_work_model::AgentTargetDigest,
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
            syu_work_model::TargetTransition::Add => syu_work_model::WorkOperation::Add,
            syu_work_model::TargetTransition::Modify => syu_work_model::WorkOperation::Modify,
            syu_work_model::TargetTransition::Remove => syu_work_model::WorkOperation::Remove,
            syu_work_model::TargetTransition::RunOnly
            | syu_work_model::TargetTransition::Readonly => unreachable!("editable lifecycle"),
        };
        let constraints = if transition == syu_work_model::TargetTransition::Add {
            syu_work_model::WorkConstraints {
                max_added_bytes_per_target: Some(512),
                max_added_lines_per_target: Some(32),
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let request = WorkRequest {
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: format!("WORK-FIXTURE-LIFECYCLE-{target_id}"),
            title: format!("apply {transition:?} to {target_id}"),
            operation,
            origin: syu_work_model::WorkOrigin::RequirementCriterion {
                criterion: format!("REQ-FIXTURE-001#criterion.{criterion}")
                    .parse()
                    .unwrap(),
            },
            constraints,
            requested_targets: vec![syu_work_model::RequestedTarget {
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
        syu_work_model::AgentTargetDigest,
    ) {
        if let Ok(mut session) = server.service.session.write() {
            session.draft_request = Some(request);
        }
        let (basis, csrf, _) = projection_and_basis(app).await;
        let response = json_mutation(app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("lifecycle plan");
        assert_eq!(plan.status, syu_work_model::PlanStatus::Ready, "{plan:?}");
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
        let config_path = temp.path().join("syu.yaml");
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
            syu_work_model::TargetTransition::Remove
        );
        assert_eq!(
            candidate.lifecycle,
            syu_work_model::TargetLifecycle::EnsureAbsent
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
        assert_eq!(request.operation, syu_work_model::WorkOperation::Remove);
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
                    schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![syu_work_model::AgentTargetWrite::Remove {
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
            syu_work_model::CompletionStatus::Complete
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
        assert_eq!(preview.status, syu_work_model::CompletionStatus::Complete);
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
    #[ignore = "pre-v1 cutover: planned origins are rejected before Work creation"]
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
                "        responsibility: Keep the existing implementation target connected to the verification proof.\n",
                "        targets:\n",
                "          - id: implementation\n",
                "            adapter: rust\n",
                "            path: src/lib.rs\n",
                "            selector: { kind: symbol, name: behavior }\n",
                "            claims:\n",
                "              - kind: satisfies\n",
                "                criterion: REQ-FIXTURE-001#criterion.add-symbol\n",
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
                "                criterion: REQ-FIXTURE-001#criterion.add-symbol\n",
                "                covers: [FEAT-LIFECYCLE-ADD-VERIFICATION-001#binding.implementation/target.implementation]\n",
                "                runner: { runner: cargo-test, arguments: { package: workbench-flow-fixture, test: added_verification_lifecycle_stays_valid } }\n",
                "  - id: FEAT-LIFECYCLE-REMOVE-SYMBOL-001"
            ),
            1,
        );
        fs::write(feature_path, feature).expect("verification feature");
        let requirement_path = temp.path().join("spec/requirement.yaml");
        let mut requirement = fs::read_to_string(&requirement_path).expect("requirement spec");
        requirement = requirement.replacen(
            "covers: [FEAT-LIFECYCLE-ADD-SYMBOL-001#binding.lifecycle/target.added-symbol]",
            "covers: [FEAT-LIFECYCLE-ADD-VERIFICATION-001#binding.implementation/target.implementation]",
            1,
        );
        fs::write(requirement_path, requirement).expect("verification requirement");
        initialize_fixture_git(temp.path());
        let server = WorkbenchServer::new(temp.path().to_path_buf());
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
        assert_eq!(candidate.transition, syu_work_model::TargetTransition::Add);
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
            "REQ-FIXTURE-001#criterion.add-symbol",
            "add the approved verification target",
        )
        .await;
        assert_eq!(request.operation, syu_work_model::WorkOperation::Add);
        let target = request.requested_targets[0].reference.clone();
        let (basis, csrf, slice, run, target_digest) =
            start_lifecycle_agent_with_request(&server, &app, target.clone(), request).await;
        assert_eq!(
            target_digest.transition,
            syu_work_model::TargetTransition::Add
        );
        assert_eq!(
            target_digest.access,
            syu_work_model::TargetAccessMode::Editable
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
                    schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                    run_id: run.run_id.clone(),
                    expected_workspace_fingerprint: run
                        .context
                        .context
                        .basis
                        .workspace_fingerprint
                        .clone(),
                    writes: vec![syu_work_model::AgentTargetWrite::AddToFile {
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
            syu_work_model::CompletionStatus::Complete
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
        assert_eq!(preview.status, syu_work_model::CompletionStatus::Complete);
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
    #[ignore = "pre-v1 cutover: planned Feature lifecycle origins are rejected before agent execution"]
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
                syu_work_model::TargetTransition::Modify,
                "modify existing symbol",
            ),
            (
                "added-symbol",
                syu_work_model::TargetTransition::Add,
                "add symbol to existing file",
            ),
            (
                "added-file",
                syu_work_model::TargetTransition::Add,
                "add new file",
            ),
            (
                "removed-symbol",
                syu_work_model::TargetTransition::Remove,
                "remove symbol",
            ),
            (
                "removed-file",
                syu_work_model::TargetTransition::Remove,
                "remove file",
            ),
        ];
        for (target_id, transition, description) in cases {
            let temp = tempfile::tempdir().expect("lifecycle fixture tempdir");
            copy_fixture_tree(&fixture, temp.path());
            if target_id == "removed-symbol" {
                let config_path = temp.path().join("syu.yaml");
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
                "behavior" => syu_work_model::AgentTargetWrite::Replace {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                    content: "pub fn behavior() -> bool {\n    1 == 1\n}".into(),
                },
                "added-symbol" => syu_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool {\n    true\n}\n".into(),
                },
                "added-file" => syu_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn added_file() {}\n".into(),
                },
                "removed-symbol" => syu_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => syu_work_model::AgentTargetWrite::RemoveFile {
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
                        schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
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
            let patch: syu_work_model::AgentPatchRecord =
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
                syu_work_model::CompletionStatus::Complete,
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
            assert_eq!(preview.status, syu_work_model::CompletionStatus::Complete);
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
            let finalization: syu_work_model::FinalizationReceipt =
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
                            .header("x-syu-csrf-token", post_csrf.clone())
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
                            .header("x-syu-csrf-token", post_csrf.clone())
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
                            .header("x-syu-csrf-token", post_csrf.clone())
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
                            .header("x-syu-csrf-token", post_csrf.clone())
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
                            .header("x-syu-csrf-token", post_csrf)
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
    #[ignore = "pre-v1 cutover: planned Feature lifecycle origins are rejected before agent execution"]
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
                syu_work_model::TargetTransition::Add,
                "src/lib.rs",
                "\npub fn added_behavior() -> bool { true }\n",
                "now exists",
            ),
            (
                "added-file",
                syu_work_model::TargetTransition::Add,
                "src/added.rs",
                "pub fn preexisting_file() {}\n",
                "now exists",
            ),
            (
                "removed-symbol",
                syu_work_model::TargetTransition::Remove,
                "src/removable.rs",
                "pub fn remove_me() { panic!(\"changed\") }\n",
                "is stale",
            ),
            (
                "removed-file",
                syu_work_model::TargetTransition::Remove,
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
                "added-symbol" => syu_work_model::AgentTargetWrite::AddToFile {
                    target: target.reference.clone(),
                    expected_path_hash: target
                        .container_content_hash
                        .clone()
                        .expect("approved insertion digest"),
                    content: "pub fn added_behavior() -> bool { true }\n".into(),
                },
                "added-file" => syu_work_model::AgentTargetWrite::CreateFile {
                    target: target.reference.clone(),
                    content: "pub fn approved_file() {}\n".into(),
                },
                "removed-symbol" => syu_work_model::AgentTargetWrite::Remove {
                    target: target.reference.clone(),
                    expected_excerpt_hash: target.excerpt_hash.clone(),
                },
                "removed-file" => syu_work_model::AgentTargetWrite::RemoveFile {
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
                        schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
        let plan = syu_planner::plan(&broadened, &workspace, &index, &revision)
            .expect("invalid exact origin becomes a blocked plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("exact Work origin is invalid") })
        );
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
        let server = WorkbenchServer::new(temp.path().to_path_buf());
        let service = server.service.clone();
        let app = server.router();

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
        assert_eq!(response.status(), StatusCode::OK);
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
        let add_criterion = StructuredEditCommand {
            basis,
            patch: EditPatch::AddCriterion {
                requirement_id: SpecId("REQ-FIXTURE-001".into()),
                criterion: NewCriterion {
                    id: "recovery".into(),
                    kind: CriterionKind::Behavior,
                    statement: "The no-match recovery path adds a reviewable behavior.".into(),
                    governed_by: vec!["POL-FIXTURE-001#rule.behavior".parse().unwrap()],
                },
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &add_criterion,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let preview: EditPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("criterion add preview");
        assert!(preview.impact.as_ref().is_some_and(|impact| {
            impact
                .changed_anchors
                .iter()
                .any(|anchor| anchor == "REQ-FIXTURE-001#criterion.recovery")
        }));
        let response = json_mutation(
            &app,
            Method::PUT,
            "/api/specifications/candidates/apply",
            &csrf,
            &StructuredEditCommand {
                preview_token: Some(preview.preview_token),
                ..add_criterion
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            fs::read_to_string(temp.path().join("spec/requirement.yaml"))
                .expect("criterion added")
                .contains("recovery")
        );

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let orphan_feature = StructuredEditCommand {
            basis,
            patch: EditPatch::CreateFeature {
                document: "spec/feature.yaml".into(),
                id: "FEAT-FIXTURE-ORPHAN".into(),
                title: "Orphan feature".into(),
                summary: "Must not enter the graph without an exact target pair.".into(),
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
            &orphan_feature,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let implemented_feature_target = StructuredEditCommand {
            basis,
            patch: EditPatch::AddFeatureTarget {
                document: "spec/feature.yaml".into(),
                feature_id: "FEAT-FIXTURE-001".into(),
                criterion_anchor: "REQ-FIXTURE-001#criterion.recovery".into(),
                target: FeatureTargetDraft {
                    id: "planned-recovery".into(),
                    adapter: "rust".into(),
                    path: "src/guided_feature.rs".into(),
                    selector: Selector::Symbol {
                        name: "guided_recovery".into(),
                    },
                },
            },
            preview_token: None,
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/candidates/preview",
            &csrf,
            &implemented_feature_target,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let feature = StructuredEditCommand {
            basis,
            patch: EditPatch::CreateFeature {
                document: "spec/feature.yaml".into(),
                id: "FEAT-FIXTURE-002".into(),
                title: "A guided feature".into(),
                summary: "Created through the same typed wizard.".into(),
                status: None,
                criterion_anchor: Some("REQ-FIXTURE-001#criterion.recovery".into()),
                target: Some(FeatureTargetDraft {
                    id: "implementation".into(),
                    adapter: "rust".into(),
                    path: "src/guided_feature.rs".into(),
                    selector: Selector::Symbol {
                        name: "guided_feature".into(),
                    },
                }),
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
        assert_eq!(response.status(), StatusCode::OK);
        let preview: EditPreview =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("feature preview");
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
        let created_feature = fs::read_to_string(temp.path().join("spec/feature.yaml"))
            .expect("created feature graph");
        assert!(created_feature.contains("REQ-FIXTURE-001#criterion.recovery"));
        assert!(created_feature.contains("guided_feature.rs"));

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let suggestions_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/specifications/REQ-FIXTURE-001%23criterion.recovery/target-suggestions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(suggestions_response.status(), StatusCode::OK);
        let suggestions: TargetSuggestionSet = serde_json::from_slice(
            &suggestions_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .expect("feature target suggestions");
        let planned_feature_target = suggestions
            .suggestions
            .iter()
            .find(|candidate| candidate.reference.to_string().contains("FEAT-FIXTURE-002"))
            .expect("persisted feature target suggestion");
        assert_eq!(
            planned_feature_target.transition,
            syu_work_model::TargetTransition::Add
        );
        assert!(service.session.read().unwrap().draft_request.is_none());
        let approve = TargetSuggestionApprovalCommand {
            basis,
            suggestion_token: suggestions.suggestion_token,
            suggestion_ids: vec![planned_feature_target.id.clone()],
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/specifications/REQ-FIXTURE-001%23criterion.recovery/target-suggestions/approve",
            &csrf,
            &approve,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approved feature target");
        assert_eq!(
            approval.approved_ids,
            vec![planned_feature_target.id.clone()]
        );
        assert!(service.session.read().unwrap().draft_request.is_none());
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
                lifecycle: syu_work_model::TargetLifecycle::Stable,
                path: "src/lib.rs".into(),
                selector: "behavior".into(),
                existing_file: true,
                budget_bytes: None,
                budget_lines: None,
                confidence: syu_planner::SuggestionConfidence::High,
                evidence: vec!["current evidence".into()],
                evidence_fingerprint: "current-fingerprint".into(),
            },
            TargetSuggestion {
                id: "target-stale".into(),
                rank: 2,
                reference: target_reference,
                role: BindingRole::Implementation,
                transition: TargetTransition::Modify,
                lifecycle: syu_work_model::TargetLifecycle::Stable,
                path: "src/lib.rs".into(),
                selector: "behavior".into(),
                existing_file: true,
                budget_bytes: None,
                budget_lines: None,
                confidence: syu_planner::SuggestionConfidence::High,
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
                suggestion_token: refreshed.suggestion_token.clone(),
                suggestion_ids: approved_ids.clone(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approval response");
        let split = approval
            .split_recommendation
            .expect("mixed transitions require separate approval groups");
        assert!(split.suggested_groups.len() >= 2);
        let mut persisted_approved_ids = Vec::new();
        for group in split.suggested_groups {
            let response = json_mutation(
                &app,
                Method::POST,
                &format!("{suggestion_path}/approve"),
                &csrf,
                &TargetSuggestionApprovalCommand {
                    basis: basis.clone(),
                    suggestion_token: refreshed.suggestion_token.clone(),
                    suggestion_ids: group.clone(),
                },
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let homogeneous: TargetSuggestionApprovalView =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .expect("homogeneous approval response");
            assert!(homogeneous.split_recommendation.is_none());
            persisted_approved_ids.extend(group);
        }
        let mut expected_approved_ids = approved_ids.clone();
        expected_approved_ids.sort();
        let mut actual_approved_ids = persisted_approved_ids.clone();
        actual_approved_ids.sort();
        assert_eq!(actual_approved_ids, expected_approved_ids);
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
        let mut persisted_view_ids = persisted.approved_ids.clone();
        persisted_view_ids.sort();
        assert_eq!(persisted_view_ids, actual_approved_ids);

        let (basis, csrf, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/action",
            &csrf,
            &serde_json::json!({
                "basis": basis,
                "action": "create",
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.behavior" },
                "title": "Do not create mixed transition work"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            service
                .session
                .read()
                .expect("target suggestion session")
                .draft_request
                .is_none()
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
                "origin": { "kind": "requirement-criterion", "criterion": "REQ-FIXTURE-001#criterion.behavior" },
                "title": "Do not create mixed transition work"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            service
                .session
                .read()
                .expect("target suggestion session")
                .draft_request
                .is_none()
        );

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
                    "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
        let config_path = temp.path().join("syu.yaml");
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
            .filter(|candidate| candidate.transition == syu_work_model::TargetTransition::Add)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(add_candidates.len(), 2);
        let add = add_candidates
            .first()
            .cloned()
            .expect("planned Add suggestion");
        assert_eq!(
            add.lifecycle,
            syu_work_model::TargetLifecycle::EnsurePresent
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
            syu_work_model::TargetTransition::Add
        );
        assert!(matches!(
            request.origin,
            syu_work_model::WorkOrigin::RequirementCriterion { .. }
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
            "schema: syu/spec/v1\nkind: requirements\nnamespace: fixture\ncategory: Workbench recovery\nrequirements:\n  - id: REQ-PLANNED-001\n    title: A planned behavior\n    description: A planned requirement created through recovery.\n    priority: high\n    status: planned\n    criteria:\n      - id: behavior\n        kind: behavior\n        statement: Add the planned behavior.\n        governed_by: []\n",
        )
        .expect("planned requirement fixture");
        fs::write(
            temp.path().join("spec/planned-feature.yaml"),
            "schema: syu/spec/v1\nkind: features\nnamespace: fixture\ncategory: Workbench recovery\nfeatures:\n  - id: FEAT-PLANNED-001\n    title: A planned behavior implementation\n    summary: A planned Feature target created through recovery.\n    status: planned\n    bindings:\n      - id: implementation\n        role: implementation\n        facet: work\n        responsibility: Add the planned behavior implementation.\n        targets:\n          - id: behavior\n            adapter: rust\n            path: src/lib.rs\n            selector: { kind: symbol, name: planned_behavior }\n            claims:\n              - kind: satisfies\n                criterion: REQ-PLANNED-001#criterion.behavior\n          - id: behavior-two\n            adapter: rust\n            path: src/other.rs\n            selector: { kind: symbol, name: planned_behavior_two }\n            claims:\n              - kind: satisfies\n                criterion: REQ-PLANNED-001#criterion.behavior\n",
        )
        .expect("planned Feature fixture");
        let config_path = temp.path().join("syu.yaml");
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
                    syu_work_model::WorkOrigin::RequirementCriterion { .. }
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
        assert_eq!(projection["journey"]["evidence"]["status"], "blocked");
        assert_eq!(
            projection["journey"]["primary_action"]["action"],
            "choose_specification"
        );
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
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
                "schema": syu_work_model::WORK_ORIGIN_CAPABILITY_SCHEMA,
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
            projection_text.contains("x-syu-csrf-token:"),
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
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-WORKBENCH-SESSION".into(),
            title: "plan a Workbench session".into(),
            operation: syu_work_model::WorkOperation::Modify,
            origin: syu_work_model::WorkOrigin::RequirementCriterion {
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
        let path = root.join("docs/syu/requirements/workbench.yaml");
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
        let path = root.join("syu.yaml");
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
            diagnostics: vec![syu_diagnostics::Diagnostic::error(
                "SYU-WORK-001",
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
}
