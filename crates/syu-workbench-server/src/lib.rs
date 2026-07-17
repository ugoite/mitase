#![forbid(unsafe_code)]
use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Extension, Path as AxumPath, Query, Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
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
    SplitWorkRecommendation, TargetSuggestionSet, plan, split_work_recommendation, suggest_targets,
};
use syu_project_model::{ChangeBaseline, ValidationPreset};
use syu_spec_model::{
    ArtifactBinding, BindingRole, BoundTargetRef, Contract, ContractKind, Criterion, CriterionKind,
    ItemStatus, LocalAnchorKind, LocalId, OwnershipScope, Philosophy, Policy, Priority,
    Requirement, Rule, RuleLevel, Selector, SpecAnchor, SpecDocument, SpecId, TargetClaim,
};
use syu_validation::{PlanValidationMode, ValidationContext, validate};
use syu_work_model::{
    AgentBlocker, AgentEvent, AgentPatch, AgentRun, AgentRunStatus, COMPLETION_ATTEMPT_SCHEMA,
    CompletionAttempt, FinalizationPreview, FinalizationReceipt, PLAN_APPROVAL_SCHEMA,
    PlanApproval, PlanStatus, RequestedTarget, VerificationReceipt, WORK_REQUEST_SCHEMA,
    WorkConstraints, WorkOperation, WorkPlan, WorkRequest,
};
use syu_workspace::SpecWorkspace;

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
}

pub struct WorkbenchEngine;
pub struct WorkbenchService {
    pub workspace_root: PathBuf,
    pub session: RwLock<WorkbenchSession>,
    pub engine: WorkbenchEngine,
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
    pub fn with_request(self, request: WorkRequest) -> Self {
        if let Ok(mut session) = self.service.session.write() {
            session.draft_request = Some(request);
        }
        self
    }
    pub fn projection(&self, revision: &str) -> Result<WorkspaceProjection> {
        let workspace = SpecWorkspace::load(&self.service.workspace_root)?;
        let session = self.service.session.read().expect("workbench session lock");
        project_session(&workspace, &session, revision)
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
            .route("/api/source", get(api_source))
            .route("/api/work/request", post(api_request))
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
            .route("/api/work/session", delete(api_discard))
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
    format!("sha256:{:x}", hash.finalize())
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRequestCommand {
    pub basis: MutationBasis,
    pub request: WorkRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceCommand {
    pub basis: MutationBasis,
    pub slice_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCommand {
    pub basis: MutationBasis,
    pub receipt: VerificationReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStartCommand {
    pub basis: MutationBasis,
    pub slice_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPatchCommand {
    pub basis: MutationBasis,
    pub run_id: String,
    pub patch: AgentPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBlockerCommand {
    pub basis: MutationBasis,
    pub run_id: String,
    pub blocker: AgentBlocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentScopeExpansionCommand {
    pub basis: MutationBasis,
    pub run_id: String,
    pub reason: String,
    pub requested_targets: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizeCommand {
    pub basis: MutationBasis,
    pub attempt_id: String,
    #[serde(default)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<WorkRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_recommendation: Option<SplitWorkRecommendation>,
}

struct ApiError(StatusCode, anyhow::Error);
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.0,
            Json(serde_json::json!({"error": self.1.to_string()})),
        )
            .into_response()
    }
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
fn basis(service: &WorkbenchService, expected: &MutationBasis) -> Result<SpecWorkspace> {
    if expected.expected_source_hash.is_empty() {
        anyhow::bail!("mutation requires expected_source_hash");
    }
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    if revision != expected.expected_revision
        || workspace.try_fingerprint()? != expected.expected_workspace_fingerprint
        || workspace_source_hash(&workspace) != expected.expected_source_hash
    {
        anyhow::bail!(
            "Workspace changed since this view was loaded. Refresh the projection before applying the operation."
        );
    }
    Ok(workspace)
}

/// Execution is intentionally based on the plan's immutable revision rather
/// than the pre-edit workspace fingerprint. Editable source changes are the
/// expected transition between validation and verification/result; the
/// canonical plan and readonly basis checks in the validation crate still
/// reject specification, ownership, and guarded-target changes.
fn execution_basis(service: &WorkbenchService, expected: &MutationBasis) -> Result<SpecWorkspace> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    if expected.expected_revision.is_empty() || revision != expected.expected_revision {
        anyhow::bail!(
            "Workspace revision changed since this work plan was created. Refresh the projection and replan."
        );
    }
    Ok(workspace)
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
async fn api_projection(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<WorkspaceProjection>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    Ok(Json(project_session(&workspace, &session, &revision)?))
}

async fn api_index(State(service): State<Arc<WorkbenchService>>) -> Result<Html<String>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let projection = project_session(&workspace, &session, &revision)?;
    let json = serde_json::to_string(&projection)
        .map_err(anyhow::Error::from)?
        .replace('<', "\\u003c");
    let state = format!("<script type=\"application/json\" id=\"syu-projection\">{json}</script>");
    let html = include_str!("../../syu-app-ui/assets/workbench.html").replace(
        "<script type=\"application/json\" id=\"syu-projection\"></script>",
        &state,
    );
    Ok(Html(html))
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
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let index = workspace.index()?;
    let report = syu_validation::evaluate_readiness(&workspace, &index, &revision, true)?;
    let view = readiness_view(&report);
    service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .readiness = Some(view.clone());
    Ok(Json(view))
}

async fn api_specifications(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<SpecificationCatalogView>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    Ok(Json(project(&workspace, None, &revision)?.specifications))
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMatch {
    pub anchor: String,
    pub kind: String,
    pub text: String,
}

async fn api_specification_candidates(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<SpecificationCandidateQuery>,
) -> Result<Json<Vec<SpecificationCandidateView>>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let entries = project(&workspace, None, &revision)?
        .specifications
        .specifications;
    let query_text = query.q.unwrap_or_default().trim().to_ascii_lowercase();
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
            let matches = if query_text.is_empty() {
                vec![CandidateMatch {
                    anchor: item.id.clone(),
                    kind: "item".into(),
                    text: item.title.clone(),
                }]
            } else {
                fields
                    .into_iter()
                    .filter_map(|(anchor, kind, text)| {
                        text.to_ascii_lowercase()
                            .contains(&query_text)
                            .then_some(CandidateMatch { anchor, kind, text })
                    })
                    .collect::<Vec<_>>()
            };
            if matches.is_empty() {
                return None;
            }
            let mut relevance = Vec::new();
            if query_text.is_empty() {
                relevance.push("available specification".into());
            } else {
                if item.id.to_ascii_lowercase() == query_text {
                    relevance.push("exact item id".into());
                } else if item.id.to_ascii_lowercase().contains(&query_text) {
                    relevance.push("item id match".into());
                }
                if item.title.to_ascii_lowercase().contains(&query_text) {
                    relevance.push("title match".into());
                }
                if matches.iter().any(|entry| entry.kind == "criterion") {
                    relevance.push("criterion match".into());
                }
                if matches.iter().any(|entry| entry.kind == "rule") {
                    relevance.push("rule match".into());
                }
                if matches.iter().any(|entry| entry.kind == "principle") {
                    relevance.push("principle match".into());
                }
            }
            let score = relevance.len();
            Some((
                score,
                SpecificationCandidateView {
                    item,
                    matches,
                    relevance,
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
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let item = project(&workspace, None, &revision)?
        .specifications
        .specifications
        .into_iter()
        .find(|item| item.id == anchor)
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

async fn api_target_suggestions(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
) -> Result<Json<TargetSuggestionSet>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let index = workspace.index()?;
    let rejected = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .rejected_target_suggestions
        .clone();
    Ok(Json(filtered_target_suggestions(
        &workspace, &index, &criterion, &rejected,
    )?))
}

async fn api_target_suggestion_reject(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<TargetSuggestionRejectCommand>,
) -> Result<Json<TargetSuggestionSet>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let workspace = basis(&service, &command.basis)?;
    let index = workspace.index()?;
    let current = suggest_targets(&criterion, &workspace, &index)?;
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
    let rejected = {
        let mut session = service
            .session
            .write()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        session
            .rejected_target_suggestions
            .insert(candidate.id.clone(), candidate.evidence_fingerprint.clone());
        session.rejected_target_suggestions.clone()
    };
    Ok(Json(filtered_target_suggestions(
        &workspace, &index, &criterion, &rejected,
    )?))
}

async fn api_target_suggestions_approve(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<TargetSuggestionApprovalCommand>,
) -> Result<Json<TargetSuggestionApprovalView>, ApiError> {
    let criterion = anchor
        .parse::<SpecAnchor>()
        .map_err(|error| anyhow::anyhow!(error))?;
    let workspace = basis(&service, &command.basis)?;
    let index = workspace.index()?;
    let rejected = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .rejected_target_suggestions
        .clone();
    let current = filtered_target_suggestions(&workspace, &index, &criterion, &rejected)?;
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
    if let Some(split_recommendation) = split_work_recommendation(&approved, &workspace, &index) {
        return Ok(Json(TargetSuggestionApprovalView {
            request: None,
            split_recommendation: Some(split_recommendation),
        }));
    }
    let request = WorkRequest {
        schema: WORK_REQUEST_SCHEMA.into(),
        id: format!(
            "WORK-SUGGESTION-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        summary: format!("Implement approved targets for {criterion}"),
        operation: WorkOperation::Modify,
        seeds: vec![],
        constraints: WorkConstraints::default(),
        requested_targets: approved
            .into_iter()
            .map(|candidate| RequestedTarget {
                reference: candidate.reference,
                criterion: Some(criterion.clone()),
                transition: candidate.transition,
            })
            .collect(),
    };
    {
        let mut session = service
            .session
            .write()
            .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
        session.draft_request = Some(request.clone());
        session.plan = None;
        session.context_pack = None;
        session.verification_receipt = None;
        session.agent_run = None;
        session.last_validation = None;
        session.selected_slice = None;
    }
    Ok(Json(TargetSuggestionApprovalView {
        request: Some(request),
        split_recommendation: None,
    }))
}

fn specification_path(workspace: &SpecWorkspace, anchor: &str) -> Result<PathBuf> {
    let item_id = anchor.split('#').next().unwrap_or(anchor);
    let revision = current_revision(&workspace.root)?;
    let item = project(workspace, None, &revision)?
        .specifications
        .specifications
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| anyhow::anyhow!("specification {item_id} not found"))?;
    Ok(workspace.root.join(item.path))
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
        EditPatch::CreateRequirement { document, .. }
        | EditPatch::CreateFeature { document, .. } => {
            specification_document_path(workspace, document)
        }
        EditPatch::Config { .. } => anyhow::bail!("configuration is not a candidate patch"),
    }
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
    if let Some(EditPatch::CreateRequirement { id, .. } | EditPatch::CreateFeature { id, .. }) =
        patch
    {
        affected_items.insert(id.to_string());
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
            ..
        } => {
            if collection_for_value(&value)? != "features" {
                anyhow::bail!("candidate destination is not a features document");
            }
            let feature = syu_spec_model::Feature {
                id: id.clone(),
                title: title.clone(),
                summary: summary.clone(),
                status: status.unwrap_or(ItemStatus::Planned),
                bindings: vec![],
                contracts: vec![],
            };
            specification_sequence(&mut value, "features")?.push(serde_yaml::to_value(feature)?);
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
        | EditPatch::CreateRequirement { .. }
        | EditPatch::CreateFeature { .. } => specification_patch_content(workspace, path, patch),
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
    let workspace = basis(&service, &command.basis)?;
    let path = specification_path(&workspace, &anchor)?;
    let content = edit_content(&workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document.clone())?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    Ok(Json(edit_preview(&workspace, &overlay, &path, &content)?))
}

async fn api_specification_apply(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let path = specification_path(&workspace, &anchor)?;
    let content = edit_content(&workspace, &path, &command.patch)?;
    let document: SpecDocument = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    let overlay = workspace.overlay_document(&path, document.clone())?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let old = workspace.read_to_string(&path)?;
    let preview = edit_preview(&workspace, &overlay, &path, &content)?;
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
    let workspace = basis(&service, &command.basis)?;
    let path = patch_path(&workspace, &command.patch)?;
    let content = edit_content(&workspace, &path, &command.patch)?;
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
        &workspace,
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
    let workspace = basis(&service, &command.basis)?;
    let path = patch_path(&workspace, &command.patch)?;
    let content = edit_content(&workspace, &path, &command.patch)?;
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
        &workspace,
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
    let workspace = basis(&service, &command.basis)?;
    let content = edit_content(&workspace, &workspace.root.join("syu.yaml"), &command.patch)?;
    let config: syu_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    Ok(Json(edit_preview(
        &workspace,
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
    let workspace = basis(&service, &command.basis)?;
    let content = edit_content(&workspace, &workspace.root.join("syu.yaml"), &command.patch)?;
    let config: syu_project_model::ProjectConfig = serde_yaml::from_str(&content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let overlay = workspace.overlay_config(config)?;
    let overlay_index = overlay.index()?;
    validate_overlay(&overlay, &overlay_index)?;
    let path = workspace.root.join("syu.yaml");
    let old = workspace.read_to_string(&path)?;
    let preview = edit_preview(&workspace, &overlay, &path, &content)?;
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
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let index = workspace.index()?;
    let specifications = project(&workspace, None, &revision)?
        .specifications
        .specifications;
    let range = match query.range {
        Some(range) => range,
        None => configured_change_range(&workspace)?,
    };
    let changed = branch_changed_files(&workspace.root, &range)?;
    Ok(Json(ScopeView {
        branch: Some(branch_scope_view(&index, &specifications, range, &changed)),
    }))
}

#[derive(Debug, Deserialize)]
struct BranchScopeQuery {
    range: Option<String>,
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
    path: String,
}

#[derive(Debug, Serialize)]
struct SourceView {
    path: String,
    content: String,
    hash: String,
}

async fn api_source(
    State(service): State<Arc<WorkbenchService>>,
    Query(query): Query<SourceQuery>,
) -> Result<Json<SourceView>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let relative = syu_spec_model::RepoPath::new(&query.path)
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
    Ok(Json(SourceView {
        path: relative.to_string_lossy().into_owned(),
        hash: content_hash(&content),
        content,
    }))
}
async fn api_request(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<WorkRequestCommand>,
) -> Result<Json<WorkspaceProjection>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let revision = current_revision(&workspace.root)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    session.draft_request = Some(command.request);
    session.plan = None;
    session.selected_slice = None;
    session.context_pack = None;
    session.verification_receipt = None;
    session.agent_run = None;
    session.last_validation = None;
    Ok(Json(project_session(&workspace, &session, &revision)?))
}
async fn api_plan(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<MutationBasis>,
) -> Result<Json<WorkPlan>, ApiError> {
    let workspace = basis(&service, &command)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let request = session
        .draft_request
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no work request selected"))?;
    let plan = plan(&request, &workspace, &index, &revision)?;
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
    Json(command): Json<SliceCommand>,
) -> Result<Json<syu_work_model::ContextPack>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let context =
        syu_planner::export_context(plan, &command.slice_id, &workspace, &index, &revision)?;
    session.context_pack = Some(context.clone());
    session.selected_slice = Some(command.slice_id.clone());
    Ok(Json(context))
}
async fn api_approve(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<MutationBasis>,
) -> Result<Json<PlanApproval>, ApiError> {
    let workspace = basis(&service, &command)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let canonical =
        syu_validation::canonical_plan_for_execution(&workspace, &index, plan, &revision)
            .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    if !matches!(canonical.status, PlanStatus::Ready) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("only a ready plan can be approved"),
        ));
    }
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let approval = PlanApproval {
        schema: PLAN_APPROVAL_SCHEMA.into(),
        approval_id: store.new_id("approval"),
        plan_digest: canonical.canonical_digest.clone(),
        workspace_fingerprint: workspace.try_fingerprint()?,
        revision,
        reviewed_at: timestamp(),
        plan: canonical,
    };
    Ok(Json(store.approve(&approval)?))
}

async fn api_agent_start(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentStartCommand>,
) -> Result<Json<AgentRun>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
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
    if selected_slice.as_deref() != Some(command.slice_id.as_str()) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("select the requested slice before starting the agent"),
        ));
    }
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let approval = store.approval(&plan.canonical_digest).map_err(|error| {
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
    let run = syu_agent::start_run(&workspace, &approval, &command.slice_id)?;
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
    let workspace = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    match syu_agent::apply_scoped_patch(&workspace, &run, &command.patch) {
        Ok(record) => Ok(Json(record)),
        Err(error) => Err(ApiError(StatusCode::CONFLICT, error)),
    }
}

async fn api_agent_blocker(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentBlockerCommand>,
) -> Result<Json<AgentEvent>, ApiError> {
    let workspace = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    let event = syu_agent::record_blocker(&workspace, &run, command.blocker)?;
    Ok(Json(event))
}

async fn api_agent_scope_expansion(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<AgentScopeExpansionCommand>,
) -> Result<Json<AgentEvent>, ApiError> {
    let workspace = execution_basis(&service, &command.basis)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&workspace, &run)?;
    if run.run_id != command.run_id {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent run does not match the active Workbench session"),
        ));
    }
    Ok(Json(syu_agent::request_scope_expansion(
        &workspace,
        &run,
        command.reason,
        command.requested_targets,
    )?))
}

async fn api_agent_verify(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<SliceCommand>,
) -> Result<Json<CompletionAttempt>, ApiError> {
    let workspace = execution_basis(&service, &command.basis)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let run = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .agent_run
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active agent run"))?;
    let run = syu_agent::current_run(&workspace, &run)?;
    if run.slice_id != command.slice_id {
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
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let approval = store.approval(&run.plan_digest).map_err(|error| {
        ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("agent verification requires an approved plan: {error}"),
        )
    })?;
    let attempt_id = store.new_id("attempt");
    let started_at = timestamp();
    let (verification, receipt, mut report) = syu_validation::execute_verification_attempt(
        &workspace,
        &index,
        &approval.plan,
        &command.slice_id,
        &revision,
        &attempt_id,
    )?;
    report.attempt_id = attempt_id.clone();
    let mut attempt = CompletionAttempt {
        schema: COMPLETION_ATTEMPT_SCHEMA.into(),
        attempt_id,
        attempt_digest: String::new(),
        plan_digest: approval.plan_digest.clone(),
        slice_id: command.slice_id,
        approved_plan_digest: approval.plan_digest,
        started_at,
        completed_at: timestamp(),
        verification,
        receipt,
        report,
    };
    attempt.attempt_digest = DeliveryStore::digest(&{
        let mut copy = attempt.clone();
        copy.attempt_digest.clear();
        copy
    })?;
    let attempt = store.append_attempt(&attempt)?;
    syu_agent::record_verification(&workspace, &run, &attempt.attempt_id)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    session.verification_receipt = attempt.receipt.clone();
    Ok(Json(attempt))
}

async fn api_validate(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<MutationBasis>,
) -> Result<Json<ValidationRunView>, ApiError> {
    let workspace = basis(&service, &command)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let started = SystemTime::now();
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let canonical_plan =
        syu_validation::canonical_plan_for_execution(&workspace, &index, plan, &revision)
            .map_err(|error| ApiError(StatusCode::CONFLICT, error))?;
    let result = syu_validation::validate_without_readiness(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(&canonical_plan),
        selected_slice: session
            .selected_slice
            .as_ref()
            .and_then(|id| canonical_plan.slices.iter().find(|slice| &slice.id == id)),
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(&revision),
        change_base_revision: None,
    });
    let view = ValidationRunView::completed(
        "work-plan",
        Some(canonical_plan.canonical_digest.clone()),
        result,
        false,
        true,
        workspace.config.validation.preset,
        started,
    );
    session.last_validation = Some(view.clone());
    Ok(Json(view))
}
async fn api_verify(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<SliceCommand>,
) -> Result<Json<CompletionAttempt>, ApiError> {
    let workspace = execution_basis(&service, &command.basis)?;
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let approval = store.approval(&plan.canonical_digest).map_err(|error| {
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
    if session
        .selected_slice
        .as_deref()
        .is_none_or(|selected| selected != command.slice_id)
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
        &workspace,
        &index,
        plan,
        &command.slice_id,
        &revision,
        &attempt_id,
    )?;
    report.attempt_id = attempt_id.clone();
    let mut attempt = CompletionAttempt {
        schema: COMPLETION_ATTEMPT_SCHEMA.into(),
        attempt_id,
        attempt_digest: String::new(),
        plan_digest: plan.canonical_digest.clone(),
        slice_id: command.slice_id,
        approved_plan_digest: approval.plan_digest,
        started_at,
        completed_at: timestamp(),
        verification,
        receipt,
        report,
    };
    attempt.attempt_digest = DeliveryStore::digest(&{
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
    let workspace = execution_basis(&service, &command.basis)?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let attempt = store.attempt(&command.attempt_id)?;
    Ok(Json(store.finalization_preview(&workspace, &attempt)?))
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
    let workspace = execution_basis(&service, &command.basis)?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let attempt = store.attempt(&command.attempt_id)?;
    let preview = store.finalization_preview(&workspace, &attempt)?;
    Ok(Json(store.apply_finalization(
        &workspace, &attempt, &preview, token,
    )?))
}

async fn api_result(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ResultCommand>,
) -> Result<StatusCode, ApiError> {
    let workspace = execution_basis(&service, &command.basis)?;
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
    if command.receipt != canonical {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("verification receipt does not close the selected plan"),
        ));
    }
    let index = workspace.index()?;
    validate_verification_receipt(
        &workspace,
        &index,
        &plan,
        &canonical.slice_id,
        &canonical,
        &plan.basis.revision,
    )?;
    let changed_files =
        syu_validation::changed_files_against_revision(&workspace.root, &plan.basis.revision)?;
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
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        // Result validation must inspect the real diff from the plan basis.
        // This is what rejects unrelated files and readonly changes even when
        // verification itself succeeded.
        changed_files: Some(&changed_files),
        reported_changed_files: None,
        work_plan: Some(&plan),
        selected_slice: slice,
        plan_mode: PlanValidationMode::PostState,
        preset: workspace.config.validation.preset,
        revision: Some(&current_revision(&workspace.root)?),
        change_base_revision: Some(&plan.basis.revision),
    });
    let view = ValidationRunView::completed(
        "work-result",
        Some(plan.canonical_digest.clone()),
        result.clone(),
        false,
        true,
        workspace.config.validation.preset,
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
    pub capabilities: Vec<ActionCapabilityView>,
    pub work: WorkSessionView,
    pub readiness: ReadinessView,
    pub scope: ScopeView,
    pub specifications: SpecificationCatalogView,
    pub diagnostics: DiagnosticsView,
}
pub type WorkbenchProjection = WorkspaceProjection;
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
}
#[derive(Debug, Clone, Serialize)]
pub struct CompletionHistoryView {
    pub current: Option<CompletionAttemptView>,
    pub previous: Vec<CompletionAttemptView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CompletionAttemptView {
    pub attempt_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub status: String,
    pub demonstrated: Vec<String>,
    pub blockers: Vec<syu_work_model::CompletionBlocker>,
    pub next_action: Option<String>,
    pub finalized: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct WorkRequestView {
    pub summary: String,
    pub operation: String,
    pub seed_count: usize,
    pub requested_target_count: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlanView {
    pub id: String,
    pub status: String,
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
    pub access: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct VerificationReceiptView {
    pub slice_id: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ContextPackView {
    pub slice_id: String,
    pub entry_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessView {
    pub target: String,
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
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub principles: Vec<PrincipleSummary>,
    pub rules: Vec<RuleSummary>,
    pub criteria: Vec<CriterionSummary>,
    pub bindings: Vec<BindingSummary>,
    pub contracts: Vec<ContractSummary>,
    /// Exact canonical anchors that may seed a WorkRequest. Item ids alone are
    /// intentionally not accepted by the browser create-work flow.
    pub anchors: Vec<String>,
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
    pub status: String,
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
    pub kind: String,
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
                        &index,
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
                        &index,
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
                        &index,
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
                        &index,
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
    Ok(WorkspaceProjection {
        snapshot: WorkspaceSummary {
            root: workspace.root.display().to_string(),
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
        },
        diagnostics: DiagnosticsView { validation },
    })
}

fn request_view(request: &WorkRequest) -> WorkRequestView {
    WorkRequestView {
        summary: request.summary.clone(),
        operation: format!("{:?}", request.operation).to_ascii_lowercase(),
        seed_count: request.seeds.len(),
        requested_target_count: request.requested_targets.len(),
    }
}

fn target_view(target: &syu_work_model::PlannedTarget) -> TargetView {
    TargetView {
        reference: target.reference.to_string(),
        access: format!("{:?}", target.access).to_ascii_lowercase(),
        path: target.resolved_path.clone(),
    }
}

fn plan_view(plan: &WorkPlan) -> PlanView {
    PlanView {
        id: plan.id.clone(),
        status: format!("{:?}", plan.status).to_ascii_lowercase(),
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
        let finalized = store.finalization(&attempt.attempt_id)?.is_some();
        views.push(CompletionAttemptView {
            attempt_id: attempt.attempt_id,
            plan_digest: attempt.plan_digest,
            slice_id: attempt.slice_id,
            status: format!("{:?}", attempt.report.status).to_ascii_lowercase(),
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
        slice_id: context.slice.clone(),
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
    let blocker_details = axes
        .values()
        .flat_map(|axis| axis.blockers.clone())
        .collect::<Vec<_>>();
    let has_subjects = axes.values().any(|axis| axis.required > 0);
    ReadinessView {
        target: report.target.clone(),
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
        target: format!("{:?}", config.validation.readiness.target).to_ascii_lowercase(),
        status: "Not run".into(),
        blocking_subjects: 0,
        axes: BTreeMap::new(),
        blockers: vec![],
        execution_state: "not-run".into(),
    }
}

fn project_session(
    workspace: &SpecWorkspace,
    session: &WorkbenchSession,
    revision: &str,
) -> Result<WorkspaceProjection> {
    let mut projection = project(workspace, session.draft_request.as_ref(), revision)?;
    if let Some(readiness) = &session.readiness {
        projection.readiness = readiness.clone();
    }
    projection.work.request = session.draft_request.as_ref().map(request_view);
    projection.work.plan = session
        .plan
        .as_ref()
        .map(plan_view)
        .or(projection.work.plan);
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
        .map(|run| syu_agent::events(workspace, &run.run_id))
        .transpose()?
        .unwrap_or_default();
    projection.work.context_pack = session.context_pack.as_ref().map(context_pack_view);
    projection.work.selected_slice = session.selected_slice.clone();
    projection.work.validation = session
        .last_validation
        .clone()
        .unwrap_or_else(ValidationRunView::not_run);
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
        .is_some_and(|plan| plan.status == "ready")
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
    Ok(projection)
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
            status: format!("{:?}", file.status).to_ascii_lowercase(),
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
    }
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
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
        kind: format!("{:?}", criterion.kind).to_ascii_lowercase(),
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

    async fn run_fixture_post_state_flow(out_of_scope: bool) -> StatusCode {
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
        let request = WorkRequest {
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-POST-STATE".into(),
            summary: "modify the fixture behavior".into(),
            operation: syu_work_model::WorkOperation::Modify,
            seeds: vec![syu_work_model::WorkSeed::Anchor(
                "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            )],
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/request",
            &csrf,
            &WorkRequestCommand {
                basis: basis.clone(),
                request,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response_body = response.into_body().collect().await.unwrap().to_bytes();
        let request_projection: serde_json::Value =
            serde_json::from_slice(&response_body).expect("request projection");
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
                slice_id: slice.clone(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(&app, Method::POST, "/api/work/validate", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let validation: ValidationRunView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("fixture pre-state validation");
        assert!(
            matches!(validation.state, ValidationRunState::Passed),
            "{validation:?}"
        );
        let response = json_mutation(&app, Method::POST, "/api/work/approve", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);

        let source = temp.path().join("src/lib.rs");
        if out_of_scope {
            fs::write(
                temp.path().join("src/unrelated.rs"),
                "pub const UNRELATED: bool = true;\n",
            )
            .expect("modify unrelated fixture source");
        } else {
            fs::write(&source, "pub fn behavior() -> bool {\n    1 == 1\n}\n")
                .expect("modify editable fixture source");
        }

        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/verify",
            &csrf,
            &SliceCommand {
                basis: basis.clone(),
                slice_id: slice,
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
            &ResultCommand { basis, receipt },
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
        let app = WorkbenchServer::new(temp.path().to_path_buf()).router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let request = WorkRequest {
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-FIXTURE-AGENT".into(),
            summary: "scoped agent fixture change".into(),
            operation: syu_work_model::WorkOperation::Modify,
            seeds: vec![syu_work_model::WorkSeed::Anchor(
                "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            )],
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/request",
            &csrf,
            &WorkRequestCommand {
                basis: basis.clone(),
                request,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
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
                slice_id: slice.clone(),
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(&app, Method::POST, "/api/work/validate", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(&app, Method::POST, "/api/work/approve", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/start",
            &csrf,
            &AgentStartCommand {
                basis: basis.clone(),
                slice_id: slice,
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
                basis,
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
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(fs::read_to_string(source).unwrap(), before_rejected);

        let (basis, _, _) = projection_and_basis(&app).await;
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/agent/blocker",
            &csrf,
            &AgentBlockerCommand {
                basis: basis.clone(),
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

        let restarted = WorkbenchServer::new(temp.path().to_path_buf()).router();
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

        let (basis, csrf, _) = projection_and_basis(&restarted).await;
        let response = json_mutation(
            &restarted,
            Method::POST,
            "/api/work/request",
            &csrf,
            &WorkRequestCommand {
                basis,
                request: WorkRequest {
                    schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
                    id: "WORK-FIXTURE-NEXT".into(),
                    summary: "a subsequent work request".into(),
                    operation: syu_work_model::WorkOperation::Modify,
                    seeds: vec![syu_work_model::WorkSeed::Anchor(
                        "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                    )],
                    constraints: Default::default(),
                    requested_targets: vec![],
                },
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let projection: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("subsequent request projection");
        assert!(projection["work"]["agent"].is_null());
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
        assert_eq!(response.status(), StatusCode::CONFLICT);
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
        let feature = StructuredEditCommand {
            basis,
            patch: EditPatch::CreateFeature {
                document: "spec/feature.yaml".into(),
                id: "FEAT-FIXTURE-002".into(),
                title: "A guided feature".into(),
                summary: "Created through the same typed wizard.".into(),
                status: None,
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
    }

    #[tokio::test]
    async fn target_suggestions_require_review_before_exact_work_request() {
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
                suggestion_ids: approved_ids,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let approval: TargetSuggestionApprovalView =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approval response");
        let request = approval.request.expect("approved request");
        assert!(request.seeds.is_empty());
        assert_eq!(request.requested_targets.len(), 2);
        assert!(request.requested_targets.iter().all(|target| {
            target.criterion.as_ref()
                == Some(&"REQ-FIXTURE-001#criterion.behavior".parse().unwrap())
        }));
        let response = json_mutation(&app, Method::POST, "/api/work/plan", &csrf, &basis).await;
        assert_eq!(response.status(), StatusCode::OK);
        let plan: WorkPlan =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .expect("approved target plan");
        assert_eq!(plan.request.requested_targets, request.requested_targets);
        assert!(plan.diagnostics.is_empty());
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
        let app = WorkbenchServer::new(workspace_root()).router();
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let request = WorkRequest {
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-WORKBENCH-SESSION".into(),
            summary: "plan a Workbench session".into(),
            operation: syu_work_model::WorkOperation::Modify,
            seeds: vec![syu_work_model::WorkSeed::Anchor(
                "REQ-WORKBENCH-002#criterion.work-session".parse().unwrap(),
            )],
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let response = json_mutation(
            &app,
            Method::POST,
            "/api/work/request",
            &csrf,
            &WorkRequestCommand { basis, request },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn completion_history_projection_is_store_backed() {
        let workspace = SpecWorkspace::load(workspace_root()).expect("workspace loads");
        let history = completion_history(&workspace).expect("completion history loads");
        assert!(history.current.is_none());
        assert!(history.previous.is_empty());
    }

    async fn run_spec_transaction(app: Router) {
        let path = workspace_root().join("docs/syu/workbench.yaml");
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

    async fn run_config_transaction(app: Router) {
        let path = workspace_root().join("syu.yaml");
        let original = fs::read_to_string(&path).expect("original config");
        let (basis, csrf, _) = projection_and_basis(&app).await;
        let workspace = SpecWorkspace::load(workspace_root()).unwrap();
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
        let _workspace_lock = workspace_test_lock().await;
        run_spec_transaction(WorkbenchServer::new(workspace_root()).router()).await;
    }

    #[tokio::test]
    async fn workbench_config_edit_transaction() {
        let _workspace_lock = workspace_test_lock().await;
        run_config_transaction(WorkbenchServer::new(workspace_root()).router()).await;
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
