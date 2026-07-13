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
    pub readiness: Option<ReadinessView>,
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
            .route("/api/config", get(api_config))
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredEditCommand {
    pub basis: MutationBasis,
    pub patch: EditPatch,
    #[serde(default)]
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditPatch {
    Specification {
        item_id: String,
        fields: BTreeMap<String, serde_yaml::Value>,
    },
    Config {
        config: Box<syu_project_model::ProjectConfig>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct EditPreview {
    pub path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub valid: bool,
    pub preview_token: String,
    pub changed_lines: usize,
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
fn basis(service: &WorkbenchService, expected: &MutationBasis) -> Result<SpecWorkspace> {
    if expected.expected_source_hash.is_empty() {
        anyhow::bail!("mutation requires expected_source_hash");
    }
    let workspace = SpecWorkspace::load(&service.workspace_root)?;
    let revision = current_revision(&workspace.root)?;
    if revision != expected.expected_revision
        || workspace.fingerprint() != expected.expected_workspace_fingerprint
        || workspace_source_hash(&workspace) != expected.expected_source_hash
    {
        anyhow::bail!(
            "Workspace changed since this view was loaded. Refresh the projection before applying the operation."
        );
    }
    Ok(workspace)
}

fn workspace_source_hash(workspace: &SpecWorkspace) -> String {
    let mut source = String::new();
    for document in &workspace.documents {
        source.push_str(&document.path.to_string_lossy());
        source.push_str(&fs::read_to_string(&document.path).unwrap_or_default());
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
        "app.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/app.js").into(),
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
        "js/projection.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../syu-app-ui/assets/projection.js").into(),
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
    let changed_lines = old
        .lines()
        .zip(content.lines())
        .filter(|(left, right)| left != right)
        .count()
        + old.lines().count().abs_diff(content.lines().count());
    let preview_token = content_hash(&format!("{}\n{}", path.to_string_lossy(), new_hash));
    Ok(EditPreview {
        path: path.to_string_lossy().into_owned(),
        old_hash,
        new_hash,
        valid: true,
        preview_token,
        changed_lines,
    })
}

fn specification_patch_content(
    workspace: &SpecWorkspace,
    path: &Path,
    patch: &EditPatch,
) -> Result<String> {
    let EditPatch::Specification { item_id, fields } = patch else {
        anyhow::bail!("specification endpoint requires a specification patch");
    };
    let old = fs::read_to_string(path)?;
    let document: SpecDocument = serde_yaml::from_str(&old)?;
    let collection = match document {
        SpecDocument::Philosophies { .. } => "philosophies",
        SpecDocument::Policies { .. } => "policies",
        SpecDocument::Requirements { .. } => "requirements",
        SpecDocument::Features { .. } => "features",
    };
    let mut value: serde_yaml::Value = serde_yaml::from_str(&old)?;
    let sequence = value
        .get_mut(collection)
        .and_then(serde_yaml::Value::as_sequence_mut)
        .ok_or_else(|| anyhow::anyhow!("specification collection is missing"))?;
    let item = sequence
        .iter_mut()
        .find(|item| item.get("id").and_then(serde_yaml::Value::as_str) == Some(item_id))
        .ok_or_else(|| anyhow::anyhow!("specification item {item_id} not found"))?;
    let mapping = item
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("specification item is not a mapping"))?;
    for (key, field) in fields {
        let key = serde_yaml::Value::String(key.clone());
        if !matches!(key.as_str(), Some("id" | "bindings" | "contracts")) {
            mapping.insert(key, field.clone());
        }
    }
    let content = serde_yaml::to_string(&value)?;
    let candidate: SpecDocument = serde_yaml::from_str(&content)?;
    if candidate.schema() != syu_spec_model::SPEC_SCHEMA {
        anyhow::bail!("specification schema must be syu/spec/v1");
    }
    let _ = workspace;
    Ok(content)
}

fn edit_content(workspace: &SpecWorkspace, path: &Path, patch: &EditPatch) -> Result<String> {
    match patch {
        EditPatch::Specification { .. } => specification_patch_content(workspace, path, patch),
        EditPatch::Config { config } => Ok(serde_yaml::to_string(config)?),
    }
}

fn validate_overlay(workspace: &SpecWorkspace, index: &syu_workspace::SpecIndex) -> Result<()> {
    let revision = current_revision(&workspace.root)?;
    let result = syu_validation::validate(&syu_validation::ValidationContext {
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
    let readiness = syu_validation::evaluate_readiness(workspace, index, &revision, false)?;
    if !readiness.meets(workspace.config.validation.readiness.target) {
        anyhow::bail!("candidate overlay does not meet the configured readiness target");
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
    Ok(Json(edit_preview(&path, &content)?))
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
    let old = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
    let preview = edit_preview(&path, &content)?;
    if let Some(token) = &command.preview_token
        && token != &preview.preview_token
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("preview token is stale; preview the patch again"),
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
    let old = fs::read_to_string(&path).map_err(anyhow::Error::from)?;
    let preview = edit_preview(&path, &content)?;
    if let Some(token) = &command.preview_token
        && token != &preview.preview_token
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("preview token is stale; preview the patch again"),
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
    let items = project(&workspace, None, &revision)?.specifications.items;
    let range = query.range.unwrap_or_else(|| "origin/main...HEAD".into());
    let changed = branch_changed_files(&workspace.root, &range)?;
    Ok(Json(ScopeView {
        branch: Some(branch_scope_view(&index, &items, range, &changed)),
    }))
}

#[derive(Debug, Deserialize)]
struct BranchScopeQuery {
    range: Option<String>,
}

fn branch_changed_files(root: &Path, range: &str) -> Result<Vec<syu_validation::ChangedFile>> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", range])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.split('\t');
        let Some(status) = parts.next() else { continue };
        let first_path = parts.next().unwrap_or_default();
        let second_path = parts.next();
        let new_path_text = second_path.unwrap_or(first_path);
        let old_path = (status.starts_with('D') || status.starts_with('R'))
            .then(|| syu_spec_model::RepoPath::new(first_path).ok())
            .flatten();
        let Ok(path) = syu_spec_model::RepoPath::new(new_path_text) else {
            continue;
        };
        let status = match status.chars().next().unwrap_or('M') {
            'A' => syu_validation::ChangeStatus::Added,
            'D' => syu_validation::ChangeStatus::Deleted,
            'R' => syu_validation::ChangeStatus::Renamed,
            _ => syu_validation::ChangeStatus::Modified,
        };
        files.push(syu_validation::ChangedFile {
            status,
            old_path,
            new_path: Some(path),
            hunks: vec![],
        });
    }
    let patch = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--unified=0", "--format=", range])
        .output()?;
    let mut current_path: Option<String> = None;
    for line in String::from_utf8_lossy(&patch.stdout).lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(path.into());
            continue;
        }
        let Some(hunk) = line.strip_prefix("@@ ") else {
            continue;
        };
        let mut parts = hunk.split_whitespace();
        let Some(old) = parts.next() else { continue };
        let Some(new) = parts.next() else { continue };
        let new = new.trim_start_matches('+');
        let old = old.trim_start_matches('-');
        let (old_start, old_count) = old.split_once(',').map_or((old, "1"), |parts| parts);
        let (start, count) = new.split_once(',').map_or((new, "1"), |parts| parts);
        let Ok(old_start) = old_start.parse::<usize>() else {
            continue;
        };
        let Ok(old_count) = old_count.parse::<usize>() else {
            continue;
        };
        let Ok(start) = start.parse::<usize>() else {
            continue;
        };
        let Ok(count) = count.parse::<usize>() else {
            continue;
        };
        if let Some(path) = &current_path
            && let Some(file) = files.iter_mut().find(|file| {
                file.new_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path.as_str())
            })
        {
            file.hunks.push(syu_validation::ChangedRange {
                old_start,
                old_end: old_start.saturating_add(old_count),
                new_start: start,
                new_end: start.saturating_add(count),
            });
        }
    }
    Ok(files)
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
    session.selected_slice = None;
    session.context_pack = None;
    session.verification_receipt = None;
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
    let canonical_plan = syu_planner::plan(&plan.request, &workspace, &index, &revision)?;
    if canonical_plan.canonical_digest != plan.canonical_digest
        || canonical_plan.slices != plan.slices
        || canonical_plan.status != plan.status
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("stored work plan is not the deterministic canonical plan"),
        ));
    }
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(plan),
        selected_slice: session
            .selected_slice
            .as_ref()
            .and_then(|id| plan.slices.iter().find(|slice| &slice.id == id)),
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
    let canonical_plan = syu_planner::plan(&plan.request, &workspace, &index, &revision)?;
    if canonical_plan.canonical_digest != plan.canonical_digest
        || canonical_plan.slices != plan.slices
        || canonical_plan.status != plan.status
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("stored work plan is not the deterministic canonical plan"),
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
    let receipt = execute_verification(&workspace, &index, plan, &command.slice_id, &revision)?;
    session.verification_receipt = Some(receipt.clone());
    Ok(Json(receipt))
}
async fn api_result(
    State(service): State<Arc<WorkbenchService>>,
    Json(command): Json<ResultCommand>,
) -> Result<StatusCode, ApiError> {
    let workspace = basis(&service, &command.basis)?;
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
    validate_verification_receipt(
        &workspace,
        &workspace.index()?,
        &plan,
        &canonical.slice_id,
        &canonical,
        &current_revision(&workspace.root)?,
    )?;
    let index = workspace.index()?;
    let changed = branch_changed_files(&workspace.root, "origin/main...HEAD")?;
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == canonical.slice_id);
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: Some(&changed),
        reported_changed_files: None,
        work_plan: Some(&plan),
        selected_slice: slice,
        plan_mode: PlanValidationMode::PostState,
        preset: workspace.config.validation.preset,
        revision: Some(&current_revision(&workspace.root)?),
        change_base_revision: None,
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
    pub config: syu_project_model::ProjectConfig,
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
    pub context_pack: Option<syu_work_model::ContextPack>,
    pub selected_slice: Option<String>,
    pub validation: ValidationRunView,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessView {
    pub target: String,
    pub status: String,
    pub blocking_subjects: usize,
    pub axes: BTreeMap<String, syu_validation::ReadinessAxis>,
    pub blockers: Vec<String>,
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
    pub source_hash: String,
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
    let readiness = readiness_not_run(&workspace.config);
    Ok(WorkspaceProjection {
        snapshot: WorkspaceSummary {
            root: workspace.root.display().to_string(),
            revision: revision.to_string(),
            fingerprint: workspace.fingerprint(),
            config_schema: workspace.config.schema.clone(),
            source_hash: workspace_source_hash(workspace),
        },
        config: workspace.config.clone(),
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
            request: requested_work,
            plan,
            verification_receipt: None,
            context_pack: None,
            selected_slice: None,
            validation: validation.clone(),
        },
        readiness,
        scope: ScopeView::default(),
        specifications: SpecificationCatalogView { items },
        diagnostics: DiagnosticsView { validation },
    })
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
    }
}

fn readiness_not_run(config: &syu_project_model::ProjectConfig) -> ReadinessView {
    ReadinessView {
        target: format!("{:?}", config.validation.readiness.target).to_ascii_lowercase(),
        status: "Not run".into(),
        blocking_subjects: 0,
        axes: BTreeMap::new(),
        blockers: vec![],
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
    projection.work.plan = session.plan.clone().or(projection.work.plan);
    projection.work.verification_receipt = session.verification_receipt.clone();
    projection.work.context_pack = session.context_pack.clone();
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
        projection
            .work
            .plan
            .as_ref()
            .is_some_and(|plan| plan.slices.iter().any(|slice| &slice.id == id))
    });
    let verifiable = projection
        .work
        .plan
        .as_ref()
        .is_some_and(|plan| matches!(plan.status, syu_work_model::PlanStatus::Ready))
        && plan_validated
        && slice_selected
        && projection.work.plan.as_ref().is_some_and(|plan| {
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
    if !matches!(plan.status, syu_work_model::PlanStatus::Ready) {
        anyhow::bail!("verification requires a validated ready plan");
    }
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    let started_at = epoch_seconds();
    let mut executions = Vec::new();
    if slice.verification_targets.is_empty() {
        anyhow::bail!("selected slice has no verification targets");
    }
    for planned in &slice.verification_targets {
        let target = index.target(&planned.reference).ok_or_else(|| {
            anyhow::anyhow!("verification target {} is unresolved", planned.reference)
        })?;
        let mut claim_count = 0;
        for claim in &target.claims {
            let TargetClaim::Verifies { runner, covers, .. } = claim else {
                continue;
            };
            claim_count += 1;
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
        if claim_count != 1 {
            anyhow::bail!(
                "verification target {} must have exactly one verification claim",
                planned.reference
            );
        }
    }
    let receipt = VerificationReceipt {
        schema: VERIFICATION_RECEIPT_SCHEMA.into(),
        plan_digest: plan.canonical_digest.clone(),
        slice_id: slice_id.into(),
        revision: revision.into(),
        workspace_fingerprint: workspace.fingerprint(),
        started_at: started_at.clone(),
        completed_at: epoch_seconds(),
        executions,
    };
    validate_verification_receipt(workspace, index, plan, slice_id, &receipt, revision)?;
    Ok(receipt)
}

pub fn validate_verification_receipt(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    receipt: &VerificationReceipt,
    revision: &str,
) -> Result<()> {
    if receipt.schema != VERIFICATION_RECEIPT_SCHEMA {
        anyhow::bail!("receipt schema must be {VERIFICATION_RECEIPT_SCHEMA}");
    }
    if !matches!(plan.status, syu_work_model::PlanStatus::Ready) {
        anyhow::bail!("receipt plan is not validated");
    }
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    if receipt.plan_digest != plan.canonical_digest
        || receipt.slice_id != slice_id
        || receipt.revision != revision
        || receipt.workspace_fingerprint != workspace.fingerprint()
    {
        anyhow::bail!("verification receipt basis is stale or does not match the selected slice");
    }
    let expected = slice
        .verification_targets
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let actual = receipt
        .executions
        .iter()
        .map(|execution| execution.target.clone())
        .collect::<Vec<_>>();
    if actual.len() != expected.len() || actual.into_iter().collect::<BTreeSet<_>>() != expected {
        anyhow::bail!(
            "verification receipt execution set does not exactly match the selected slice"
        );
    }
    for execution in &receipt.executions {
        if execution.exit_code != 0 {
            anyhow::bail!("verification receipt contains failed executions");
        }
        let target = index.target(&execution.target).ok_or_else(|| {
            anyhow::anyhow!("verification target {} is unresolved", execution.target)
        })?;
        let claim = target
            .claims
            .iter()
            .find_map(|claim| match claim {
                TargetClaim::Verifies { runner, covers, .. } => Some((runner, covers)),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("{} is not a verification target", execution.target))?;
        let configured = workspace
            .config
            .verification
            .runners
            .get(&claim.0.runner)
            .ok_or_else(|| {
                anyhow::anyhow!("verification runner {} is not configured", claim.0.runner)
            })?;
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| expand_runner_argument(argument, &claim.0.arguments))
            .collect::<Vec<_>>();
        let expected_command = std::iter::once(configured.executable.clone())
            .chain(arguments)
            .collect::<Vec<_>>();
        if execution.runner != claim.0.runner
            || execution.command != expected_command
            || execution.command.is_empty()
        {
            anyhow::bail!("verification receipt command does not match the configured runner");
        }
        let verification = syu_workspace::resolve_target(&workspace.root, target)?;
        if execution.verification_digest != verification.content_hash {
            anyhow::bail!("verification target digest is stale");
        }
        for covered in claim.1 {
            let covered_target = index
                .target(covered)
                .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))?;
            let resolved = syu_workspace::resolve_target(&workspace.root, covered_target)?;
            if execution.implementation_digests.get(covered) != Some(&resolved.content_hash) {
                anyhow::bail!("implementation digest for {covered} is stale");
            }
        }
        if execution.implementation_digests.len() != claim.1.len() {
            anyhow::bail!("receipt implementation digest set is not exact");
        }
    }
    Ok(())
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
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[test]
    fn workbench_server_flow_smoke() {
        let run = ValidationRunView::not_run();
        assert!(matches!(run.state, ValidationRunState::NotRun));
    }

    #[tokio::test]
    async fn workbench_http_projection_readiness_and_esm_flow() {
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
        assert_eq!(run.diagnostics[0].phase, ValidationPhase::Plan);
    }
}
