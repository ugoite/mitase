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
use syu_diagnostics::{Severity, ValidationPhase, ValidationResult};
use syu_planner::plan;
use syu_project_model::ValidationPreset;
use syu_spec_model::{
    ArtifactBinding, BindingRole, BoundTargetRef, Contract, ContractKind, Criterion, ItemStatus,
    LocalAnchorKind, OwnershipScope, Philosophy, Policy, Priority, Requirement, Rule, RuleLevel,
    Selector, SpecAnchor, SpecDocument, TargetClaim,
};
use syu_validation::{PlanValidationMode, ValidationContext, validate};
use syu_work_model::{
    VERIFICATION_RECEIPT_SCHEMA, VerificationExecution, VerificationReceipt, WorkPlan, WorkRequest,
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
    pub last_validation: Option<ValidationRunView>,
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
        project(&workspace, session.draft_request.as_ref(), revision)
    }
    pub fn run(self) -> Result<()> {
        let bind = self.launch.bind;
        let port = self.launch.port;
        if !bind.is_loopback()
            && self
                .launch
                .session_token
                .as_deref()
                .is_none_or(str::is_empty)
        {
            anyhow::bail!("remote --bind requires --session-token");
        }
        let service = self.service.clone();
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
        tokio::runtime::Runtime::new()?.block_on(async move {
            let app = Router::new()
                .route("/api/projection", get(api_projection))
                .route("/", get(api_index))
                .route("/assets/{*asset}", get(api_asset))
                .route("/api/readiness", get(api_readiness))
                .route("/api/specifications", get(api_specifications))
                .route("/api/specifications/{anchor}", get(api_specification))
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
                .route("/api/scope/branch", get(api_branch_scope))
                .route("/api/source", get(api_source))
                .route("/api/work/request", post(api_request))
                .route("/api/work/plan", post(api_plan))
                .route("/api/work/validate", post(api_validate))
                .route("/api/work/context", post(api_context))
                .route("/api/work/verify", post(api_verify))
                .route("/api/work/result", post(api_result))
                .route("/api/work/session", delete(api_discard))
                .layer(middleware::from_fn(mutation_guard))
                .layer(Extension(security))
                .with_state(service);
            let listener = tokio::net::TcpListener::bind((bind, port)).await?;
            println!("Syu Workbench listening on http://{bind}:{port}");
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            Ok::<(), anyhow::Error>(())
        })
    }
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
        .is_none_or(|origin| origin == security.expected_origin);
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
    #[serde(default)]
    pub expected_source_hash: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredEditCommand {
    pub basis: MutationBasis,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditPreview {
    pub path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub valid: bool,
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
        Self(StatusCode::BAD_REQUEST, value)
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
fn basis(service: &WorkbenchService, expected: &MutationBasis) -> Result<SpecWorkspace> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    if revision != expected.expected_revision
        || workspace.fingerprint() != expected.expected_workspace_fingerprint
    {
        anyhow::bail!(
            "Workspace changed since this view was loaded. Refresh the projection before applying the operation."
        );
    }
    Ok(workspace)
}
async fn api_projection(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<WorkspaceProjection>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let request = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .draft_request
        .clone();
    Ok(Json(project(&workspace, request.as_ref(), &revision)?))
}

async fn api_index(State(service): State<Arc<WorkbenchService>>) -> Result<Html<String>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let request = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?
        .draft_request
        .clone();
    let projection = project(&workspace, request.as_ref(), &revision)?;
    let json = serde_json::to_string(&projection)
        .map_err(anyhow::Error::from)?
        .replace('<', "\\u003c");
    let state = format!("<script type=\"application/json\" id=\"syu-projection\">{json}</script>");
    let html = include_str!("../../syu-app-ui/assets/workbench.html").replace(
        "<script src=\"/assets/projection.js\"></script>",
        &format!("{state}<script src=\"/assets/projection.js\"></script>"),
    );
    Ok(Html(html))
}

async fn api_asset(AxumPath(asset): AxumPath<String>) -> Response {
    let (content_type, content): (&str, String) = match asset.as_str() {
        "workbench.css" => (
            "text/css; charset=utf-8",
            include_str!("../../syu-app-ui/assets/workbench.css").into(),
        ),
        "app.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/app.js").into(),
        ),
        "i18n.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/i18n.js").into(),
        ),
        "projection.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/projection.js").into(),
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
    let revision = current_revision(&workspace.root)?;
    Ok(Json(project(&workspace, None, &revision)?.readiness))
}

async fn api_specifications(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<SpecificationCatalogView>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    Ok(Json(project(&workspace, None, &revision)?.specifications))
}

async fn api_specification(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
) -> Result<Json<ItemSummary>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    let item = project(&workspace, None, &revision)?
        .specifications
        .items
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

fn specification_path(workspace: &SpecWorkspace, anchor: &str) -> Result<PathBuf> {
    let item_id = anchor.split('#').next().unwrap_or(anchor);
    let revision = current_revision(&workspace.root)?;
    let item = project(workspace, None, &revision)?
        .specifications
        .items
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| anyhow::anyhow!("specification {item_id} not found"))?;
    Ok(workspace.root.join(item.path))
}

fn edit_preview(path: &Path, content: &str) -> Result<EditPreview> {
    let old = fs::read_to_string(path)?;
    let old_hash = content_hash(&old);
    let new_hash = content_hash(content);
    Ok(EditPreview {
        path: path.to_string_lossy().into_owned(),
        old_hash,
        new_hash,
        valid: true,
    })
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
    let document: SpecDocument = serde_yaml::from_str(&command.content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    if document.schema() != syu_spec_model::SPEC_SCHEMA {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("specification schema must be syu/spec/v1"),
        ));
    }
    Ok(Json(edit_preview(&path, &command.content)?))
}

async fn api_specification_apply(
    State(service): State<Arc<WorkbenchService>>,
    AxumPath(anchor): AxumPath<String>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let path = specification_path(&workspace, &anchor)?;
    let document: SpecDocument = serde_yaml::from_str(&command.content)
        .map_err(|error| anyhow::anyhow!("strict specification parse failed: {error}"))?;
    if document.schema() != syu_spec_model::SPEC_SCHEMA {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("specification schema must be syu/spec/v1"),
        ));
    }
    let old = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
    if command
        .basis
        .expected_source_hash
        .as_deref()
        .is_some_and(|hash| hash != content_hash(&old))
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("source changed since this view was loaded"),
        ));
    }
    let preview = edit_preview(&path, &command.content)?;
    atomic_replace(&path, &command.content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        return Err(error.into());
    }
    Ok(Json(preview))
}

async fn api_config_preview(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let _: syu_project_model::ProjectConfig = serde_yaml::from_str(&command.content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    Ok(Json(edit_preview(
        &workspace.root.join("syu.yaml"),
        &command.content,
    )?))
}

async fn api_config_apply(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<StructuredEditCommand>,
) -> Result<Json<EditPreview>, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let _: syu_project_model::ProjectConfig = serde_yaml::from_str(&command.content)
        .map_err(|error| anyhow::anyhow!("strict config parse failed: {error}"))?;
    let path = workspace.root.join("syu.yaml");
    let old = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
    if command
        .basis
        .expected_source_hash
        .as_deref()
        .is_some_and(|hash| hash != content_hash(&old))
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("source changed since this view was loaded"),
        ));
    }
    let preview = edit_preview(&path, &command.content)?;
    atomic_replace(&path, &command.content)?;
    if let Err(error) = SpecWorkspace::load(&workspace.root).and_then(|candidate| candidate.index())
    {
        atomic_replace(&path, &old)?;
        return Err(error.into());
    }
    Ok(Json(preview))
}

async fn api_branch_scope(
    State(service): State<Arc<WorkbenchService>>,
) -> Result<Json<ScopeView>, ApiError> {
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    Ok(Json(project(&workspace, None, &revision)?.scope))
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
    let index = workspace.index()?;
    let revision = current_revision(&workspace.root)?;
    let plan = plan(&command.request, &workspace, &index, &revision)?;
    let mut session = service
        .session
        .write()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    session.draft_request = Some(command.request);
    session.plan = Some(plan);
    Ok(Json(project(
        &workspace,
        session.draft_request.as_ref(),
        &revision,
    )?))
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
    Ok(Json(context))
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
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(plan),
        selected_slice: None,
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(&revision),
        change_base_revision: None,
    });
    let view = ValidationRunView::completed(
        "work-plan",
        Some(plan.canonical_digest.clone()),
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
) -> Result<Json<VerificationReceipt>, ApiError> {
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
    let receipt = execute_verification(&workspace, &index, plan, &command.slice_id, &revision)?;
    session.verification_receipt = Some(receipt.clone());
    Ok(Json(receipt))
}
async fn api_result(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ResultCommand>,
) -> Result<StatusCode, ApiError> {
    let workspace = basis(&service, &command.basis)?;
    let session = service
        .session
        .read()
        .map_err(|_| anyhow::anyhow!("workbench session lock"))?;
    let plan = session
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no work plan"))?;
    if command.receipt.plan_digest != plan.canonical_digest
        || command.receipt.revision != current_revision(&workspace.root)?
        || command.receipt.workspace_fingerprint != workspace.fingerprint()
        || command
            .receipt
            .executions
            .iter()
            .any(|execution| execution.exit_code != 0)
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("verification receipt does not close the selected plan"),
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
    pub request: Option<WorkRequest>,
    pub plan: Option<WorkPlan>,
    pub verification_receipt: Option<VerificationReceipt>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ReadinessView {
    pub target: String,
    pub status: String,
    pub blocking_subjects: usize,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScopeView {
    pub branch: Option<BranchScopeView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SpecificationCatalogView {
    pub items: Vec<ItemSummary>,
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
    pub phase: ValidationPhase,
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
            .map(|diagnostic| ValidationDiagnosticView {
                phase: diagnostic.phase,
                diagnostic,
            })
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
            .filter(|d| validation_phase_id(d.phase) == id)
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
}

#[derive(Debug, Clone, Serialize)]
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
    let mut items = Vec::new();
    for loaded in &workspace.documents {
        let path = relative_display(&workspace.root, &loaded.path);
        let source_hash = content_hash(&fs::read_to_string(&loaded.path).unwrap_or_default());
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                for item in philosophies {
                    items.push(item_summary_from_philosophy(
                        item,
                        &path,
                        &source_hash,
                        &index,
                    ));
                }
            }
            SpecDocument::Policies { policies, .. } => {
                for item in policies {
                    items.push(item_summary_from_policy(item, &path, &source_hash, &index));
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    items.push(item_summary_from_requirement(
                        item,
                        &path,
                        &source_hash,
                        &index,
                    ));
                }
            }
            SpecDocument::Features { features, .. } => {
                for item in features {
                    items.push(item_summary_from_feature(item, &path, &source_hash, &index));
                }
            }
        }
    }
    let plan = requested_work
        .as_ref()
        .map(|r| plan(r, workspace, &index, revision))
        .transpose()?;
    let validation = ValidationRunView::not_run();
    Ok(WorkspaceProjection {
        snapshot: WorkspaceSummary {
            root: workspace.root.display().to_string(),
            revision: revision.to_string(),
            fingerprint: workspace.fingerprint(),
            config_schema: workspace.config.schema.clone(),
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
                enabled: plan.as_ref().is_some_and(|plan| !plan.slices.is_empty()),
                disabled_reason: plan
                    .as_ref()
                    .is_none_or(|plan| plan.slices.is_empty())
                    .then(|| "Validate the selected plan before verification.".into()),
            },
        ],
        work: WorkSessionView {
            request: requested_work,
            plan,
            verification_receipt: None,
        },
        readiness: ReadinessView {
            target: "closed-loop".into(),
            status: "Not run".into(),
            blocking_subjects: 0,
        },
        scope: ScopeView::default(),
        specifications: SpecificationCatalogView { items },
        diagnostics: DiagnosticsView { validation },
    })
}

/// Executes only runner declarations from the canonical configuration.  The UI
/// never supplies a shell command; both executable and argv originate in spec
/// and config data.
pub fn execute_verification(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    revision: &str,
) -> Result<VerificationReceipt> {
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    let started_at = epoch_seconds();
    let mut executions = Vec::new();
    for planned in &slice.verification_targets {
        let target = index.target(&planned.reference).ok_or_else(|| {
            anyhow::anyhow!("verification target {} is unresolved", planned.reference)
        })?;
        for claim in &target.claims {
            let TargetClaim::Verifies { runner, covers, .. } = claim else {
                continue;
            };
            if covers.is_empty() {
                anyhow::bail!("verification target {} has no covers", planned.reference);
            }
            let configured = workspace
                .config
                .verification
                .runners
                .get(&runner.runner)
                .ok_or_else(|| {
                    anyhow::anyhow!("verification runner {} is not configured", runner.runner)
                })?;
            let arguments = configured
                .arguments
                .iter()
                .map(|argument| expand_runner_argument(argument, &runner.arguments))
                .collect::<Vec<_>>();
            let output = Command::new(&configured.executable)
                .args(&arguments)
                .current_dir(&workspace.root)
                .output()?;
            let mut implementation_digests = BTreeMap::new();
            for covered in covers {
                let covered_target = index
                    .target(covered)
                    .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))?;
                let resolved = syu_workspace::resolve_target(&workspace.root, covered_target)?;
                implementation_digests.insert(covered.clone(), resolved.content_hash);
            }
            let verification = syu_workspace::resolve_target(&workspace.root, target)?;
            executions.push(VerificationExecution {
                target: planned.reference.clone(),
                runner: runner.runner.clone(),
                command: std::iter::once(configured.executable.clone())
                    .chain(arguments)
                    .collect(),
                exit_code: output.status.code().unwrap_or(-1),
                stdout_digest: digest(&output.stdout),
                stderr_digest: digest(&output.stderr),
                implementation_digests,
                verification_digest: verification.content_hash,
            });
        }
    }
    Ok(VerificationReceipt {
        schema: VERIFICATION_RECEIPT_SCHEMA.into(),
        plan_digest: plan.canonical_digest.clone(),
        slice_id: slice_id.into(),
        revision: revision.into(),
        workspace_fingerprint: workspace.fingerprint(),
        started_at: started_at.clone(),
        completed_at: epoch_seconds(),
        executions,
    })
}

fn expand_runner_argument(template: &str, values: &BTreeMap<String, String>) -> String {
    values
        .iter()
        .fold(template.to_string(), |value, (key, replacement)| {
            value.replace(&format!("{{{key}}}"), replacement)
        })
}
fn digest(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}
fn epoch_seconds() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
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
        let refs = index
            .path_to_targets
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let owners = refs
            .iter()
            .map(|reference| reference.binding.item.to_string())
            .collect::<BTreeSet<_>>();
        let anchors = refs
            .iter()
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>();
        affected_ids.extend(owners.iter().cloned());
        changed.push(BranchChangedTargetView {
            path,
            status: format!("{:?}", file.status).to_ascii_lowercase(),
            owners: owners.into_iter().collect(),
            anchors: anchors.into_iter().collect(),
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
        state: "ready".into(),
        reason: None,
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
        bindings: bindings_for(&item_id, &item.bindings),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
    }
}

fn item_summary_from_policy(
    item: &Policy,
    path: &str,
    source_hash: &str,
    index: &syu_workspace::SpecIndex,
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
        bindings: bindings_for(&item_id, &item.bindings),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
    }
}

fn item_summary_from_requirement(
    item: &Requirement,
    path: &str,
    source_hash: &str,
    index: &syu_workspace::SpecIndex,
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
        bindings: bindings_for(&item_id, &item.bindings),
        contracts: vec![],
        anchors: anchors_for(index, &item.id),
    }
}

fn item_summary_from_feature(
    item: &syu_spec_model::Feature,
    path: &str,
    source_hash: &str,
    index: &syu_workspace::SpecIndex,
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
        bindings: bindings_for(&item_id, &item.bindings),
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
                owns: binding.owns.clone(),
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
        assert_eq!(run.diagnostics[0].phase, ValidationPhase::Plan);
    }
}
