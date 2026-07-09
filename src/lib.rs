#![forbid(unsafe_code)]
mod lsp;
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use syu_app_ui::{
    WORKBENCH_APP_JS, WORKBENCH_CSS, WORKBENCH_I18N_JS, WORKBENCH_PROJECTION_JS, WorkbenchView,
    locale_catalog_script,
};
use syu_planner::{export_context, plan};
use syu_project_model::{ChangeBaseline, GitRef};
use syu_spec_model::{
    Criterion, Feature, LocalAnchorKind, LocalId, Philosophy, Policy, RepoPath, Requirement, Rule,
    SpecAnchor, SpecDocument, SpecId,
};
use syu_validation::{
    ChangeStatus, ChangedFile, ChangedRange, PlanValidationMode, ValidationContext, validate,
};
use syu_work_model::{WorkPlan, WorkRequest};
use syu_workbench_server::project as project_workbench;
use syu_workspace::SpecWorkspace;
use tokio::sync::RwLock;

#[derive(Clone)]
struct WorkbenchWebState {
    workspace_root: PathBuf,
    request: Arc<RwLock<Option<WorkRequest>>>,
    preview_proofs: Arc<RwLock<HashMap<String, PreviewProof>>>,
}

#[derive(Debug, Clone)]
struct PreviewProof {
    expected_hash: String,
    new_hash: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug)]
struct ApiError(StatusCode, anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ApiErrorBody {
                error: format!("{:#}", self.1),
            }),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self(StatusCode::BAD_REQUEST, error)
    }
}

#[derive(Debug, Deserialize)]
struct FileQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct FileEdit {
    path: String,
    content: String,
    expected_hash: String,
    #[serde(default)]
    preview_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileSource {
    path: String,
    content: String,
    hash: String,
}

#[derive(Debug, Serialize)]
struct FilePreview {
    path: String,
    old_hash: String,
    new_hash: String,
    changed_lines: usize,
    validation_errors: Vec<String>,
    preview_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BrowserValidationRequest {
    context: String,
    #[serde(default)]
    slice: Option<String>,
    #[serde(default)]
    range: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigEdit {
    config: syu_project_model::ProjectConfig,
    expected_hash: String,
    #[serde(default)]
    preview_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BranchScopeQuery {
    range: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ItemEditPayload {
    id: String,
    kind: String,
    path: String,
    title: String,
    summary: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    principles: Vec<syu_workbench_server::PrincipleSummary>,
    rules: Vec<syu_workbench_server::RuleSummary>,
    criteria: Vec<syu_workbench_server::CriterionSummary>,
    expected_hash: String,
    #[serde(default)]
    preview_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfigSource {
    config: syu_project_model::ProjectConfig,
    hash: String,
}

struct ValidationInputs {
    changed_files: Option<Vec<ChangedFile>>,
    reported_changed_files: Option<Vec<ChangedFile>>,
    change_base_revision: Option<String>,
    plan_mode: PlanValidationMode,
}

#[derive(Debug, Parser)]
#[command(name="syu", version=env!("SYU_GIT_VERSION"), about="Exact specification work planning and validation")]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}
#[derive(Debug, Subcommand)]
enum CommandKind {
    Validate(ValidateArgs),
    Work(WorkArgs),
    Workbench(WorkbenchArgs),
    Lsp,
}
#[derive(Debug, Args)]
struct ValidateArgs {
    #[arg(default_value = ".")]
    workspace: PathBuf,
    #[arg(long)]
    range: Option<String>,
    #[arg(long)]
    baseline: Option<String>,
    #[arg(long)]
    plan: Option<PathBuf>,
    #[arg(long)]
    slice: Option<String>,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
}
#[derive(Debug, Args)]
struct WorkArgs {
    #[command(subcommand)]
    command: WorkCommand,
}
#[derive(Debug, Args)]
#[command(
    about = "Run the Workbench server or print its canonical projection",
    after_help = "Serve options: --bind <IP> --port <PORT> --allow-remote-bind --show-log"
)]
struct WorkbenchArgs {
    #[command(subcommand)]
    command: WorkbenchCommand,
}
#[derive(Debug, Subcommand)]
enum WorkCommand {
    Plan {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    Show {
        #[arg(long)]
        plan: PathBuf,
    },
    ExportContext {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        slice: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}
#[derive(Debug, Subcommand)]
enum WorkbenchCommand {
    Project {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "json")]
        format: WorkbenchFormat,
    },
    /// Serve the canonical Workbench browser UI.
    Serve {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long, default_value = "127.0.0.1")]
        bind: IpAddr,
        #[arg(long, default_value_t = 7737)]
        port: u16,
        #[arg(long)]
        allow_remote_bind: bool,
        #[arg(long)]
        show_log: bool,
    },
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum WorkbenchFormat {
    Json,
    Yaml,
}

pub fn run() -> Result<i32> {
    match Cli::parse().command {
        CommandKind::Validate(args) => run_validate(args),
        CommandKind::Work(args) => run_work(args),
        CommandKind::Workbench(args) => run_workbench(args),
        CommandKind::Lsp => {
            lsp::run_lsp_server()?;
            Ok(0)
        }
    }
}
fn run_validate(args: ValidateArgs) -> Result<i32> {
    let workspace = SpecWorkspace::load(&args.workspace)?;
    let index = workspace.index()?;
    let plan = args
        .plan
        .as_ref()
        .map(|path| read_yaml::<WorkPlan>(path))
        .transpose()?;
    let needs_revision = args.range.is_some()
        || args.baseline.is_some()
        || workspace.config.validation.changed.baseline.is_some()
        || plan.is_some();
    let revision = needs_revision
        .then(|| revision(&workspace.root))
        .transpose()?;
    let selected = match (&plan, &args.slice) {
        (Some(p), Some(id)) => Some(
            p.slices
                .iter()
                .find(|s| &s.id == id)
                .with_context(|| format!("slice {id} not found"))?,
        ),
        (None, Some(_)) => bail!("--slice requires --plan"),
        _ => None,
    };
    let validation_inputs = validation_inputs_for_cli(&workspace, &args, plan.as_ref())?;
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: validation_inputs.changed_files.as_deref(),
        reported_changed_files: validation_inputs.reported_changed_files.as_deref(),
        work_plan: plan.as_ref(),
        selected_slice: selected,
        plan_mode: validation_inputs.plan_mode,
        preset: workspace.config.validation.preset,
        revision: revision.as_deref(),
        change_base_revision: validation_inputs.change_base_revision.as_deref(),
    });
    match args.format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        Format::Text => {
            for d in &result.diagnostics {
                println!(
                    "{:?} {} {}: {}",
                    d.severity, d.rule_id, d.primary.path, d.message
                );
            }
            println!("{} diagnostic(s)", result.diagnostics.len());
        }
    }
    Ok(if result.is_valid() { 0 } else { 1 })
}
fn run_work(args: WorkArgs) -> Result<i32> {
    match args.command {
        WorkCommand::Plan {
            request,
            out,
            workspace,
        } => {
            let workspace = SpecWorkspace::load(workspace)?;
            let index = workspace.index()?;
            let request: WorkRequest = read_yaml(&request)?;
            ensure_clean_plan_workspace(&workspace)?;
            let plan = plan(&request, &workspace, &index, &revision(&workspace.root)?)?;
            write_yaml(&out, &plan)?;
            println!("wrote {} ({:?})", out.display(), plan.status);
            Ok(
                if matches!(plan.status, syu_work_model::PlanStatus::Ready) {
                    0
                } else {
                    1
                },
            )
        }
        WorkCommand::Show { plan } => {
            let plan: WorkPlan = read_yaml(&plan)?;
            println!("{}", serde_yaml::to_string(&plan)?);
            Ok(0)
        }
        WorkCommand::ExportContext {
            plan,
            slice,
            workspace,
            out,
        } => {
            let workspace = SpecWorkspace::load(workspace)?;
            let index = workspace.index()?;
            let plan: WorkPlan = read_yaml(&plan)?;
            plan.slices
                .iter()
                .find(|s| s.id == slice)
                .with_context(|| format!("slice {slice} not found"))?;
            let current_revision = revision(&workspace.root)?;
            let context = export_context(&plan, &slice, &workspace, &index, &current_revision)?;
            let yaml = serde_yaml::to_string(&context)?;
            if let Some(path) = out {
                fs::write(&path, yaml)?;
                println!("wrote {}", path.display());
            } else {
                print!("{yaml}");
            }
            Ok(0)
        }
    }
}
fn run_workbench(args: WorkbenchArgs) -> Result<i32> {
    match args.command {
        WorkbenchCommand::Project {
            workspace,
            request,
            format,
        } => {
            let workspace = SpecWorkspace::load(workspace)?;
            let request = request
                .as_ref()
                .map(|path| read_yaml::<WorkRequest>(path))
                .transpose()?;
            let projection =
                project_workbench(&workspace, request.as_ref(), &revision(&workspace.root)?)?;
            match format {
                WorkbenchFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&projection)?);
                }
                WorkbenchFormat::Yaml => {
                    print!("{}", serde_yaml::to_string(&projection)?);
                }
            }
            Ok(0)
        }
        WorkbenchCommand::Serve {
            workspace,
            request,
            bind,
            port,
            allow_remote_bind,
            show_log,
        } => {
            if !bind.is_loopback() && !allow_remote_bind {
                bail!("remote --bind requires --allow-remote-bind");
            }
            let workspace = SpecWorkspace::load(workspace)?;
            let request = request
                .as_ref()
                .map(|path| read_yaml::<WorkRequest>(path))
                .transpose()?;
            let state = WorkbenchWebState {
                workspace_root: workspace.root,
                request: Arc::new(RwLock::new(request)),
                preview_proofs: Arc::new(RwLock::new(HashMap::new())),
            };
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(async move {
                let app = Router::new()
                    .route("/", get(web_index))
                    .route("/assets/{name}", get(web_asset))
                    .route("/api/projection", get(web_projection))
                    .route("/api/work/request", put(web_update_request))
                    .route("/api/work/replan", post(web_replan))
                    .route("/api/scope/branch", get(web_branch_scope))
                    .route(
                        "/api/context/{slice}",
                        post(web_export_context).get(web_export_context),
                    )
                    .route("/api/validate", post(web_validate))
                    .route("/api/items/{id}/preview", post(web_preview_item))
                    .route("/api/items/{id}/apply", put(web_apply_item))
                    .route("/api/source", get(web_source))
                    .route("/api/file/preview", post(web_preview_file))
                    .route("/api/file/apply", put(web_apply_file))
                    .route("/api/config", get(web_config))
                    .route("/api/config/preview", post(web_preview_config))
                    .route("/api/config/apply", put(web_apply_config))
                    .with_state(state);
                let listener = tokio::net::TcpListener::bind((bind, port)).await?;
                println!("Syu Workbench listening on http://{bind}:{port}");
                if show_log {
                    println!("Workbench request logging enabled");
                }
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = tokio::signal::ctrl_c().await;
                    })
                    .await?;
                Ok::<(), anyhow::Error>(())
            })?;
            Ok(0)
        }
    }
}

async fn current_projection(
    state: &WorkbenchWebState,
) -> std::result::Result<syu_workbench_server::WorkspaceProjection, ApiError> {
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    let request = state.request.read().await.clone();
    Ok(project_workbench(
        &workspace,
        request.as_ref(),
        &revision(&workspace.root)?,
    )?)
}

async fn web_index(
    State(state): State<WorkbenchWebState>,
) -> std::result::Result<Html<String>, ApiError> {
    let projection = current_projection(&state).await?;
    Ok(Html(WorkbenchView::new(&projection).render_html()))
}

async fn web_asset(AxumPath(name): AxumPath<String>) -> Response {
    if name == "catalog.js" {
        return (
            [("content-type", "text/javascript; charset=utf-8")],
            locale_catalog_script(),
        )
            .into_response();
    }
    let (content_type, content) = match name.as_str() {
        "workbench.css" => ("text/css; charset=utf-8", WORKBENCH_CSS),
        "app.js" => ("text/javascript; charset=utf-8", WORKBENCH_APP_JS),
        "i18n.js" => ("text/javascript; charset=utf-8", WORKBENCH_I18N_JS),
        "projection.js" => ("text/javascript; charset=utf-8", WORKBENCH_PROJECTION_JS),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([("content-type", content_type)], content).into_response()
}

async fn web_projection(
    State(state): State<WorkbenchWebState>,
) -> std::result::Result<Json<syu_workbench_server::WorkspaceProjection>, ApiError> {
    Ok(Json(current_projection(&state).await?))
}

async fn web_update_request(
    State(state): State<WorkbenchWebState>,
    Json(request): Json<WorkRequest>,
) -> std::result::Result<Json<WorkPlan>, ApiError> {
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    let index = workspace.index()?;
    let plan = plan(&request, &workspace, &index, &revision(&workspace.root)?)?;
    *state.request.write().await = Some(request);
    Ok(Json(plan))
}

async fn web_replan(
    State(state): State<WorkbenchWebState>,
) -> std::result::Result<Json<WorkPlan>, ApiError> {
    let projection = current_projection(&state).await?;
    projection.plan.map(Json).ok_or_else(|| {
        ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("no work request selected"),
        )
    })
}

async fn web_export_context(
    State(state): State<WorkbenchWebState>,
    AxumPath(slice): AxumPath<String>,
) -> std::result::Result<String, ApiError> {
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    let index = workspace.index()?;
    let projection = current_projection(&state).await?;
    let plan = projection
        .plan
        .ok_or_else(|| ApiError(StatusCode::CONFLICT, anyhow::anyhow!("no work plan")))?;
    let context = export_context(
        &plan,
        &slice,
        &workspace,
        &index,
        &revision(&workspace.root)?,
    )?;
    Ok(serde_yaml::to_string(&context).map_err(anyhow::Error::from)?)
}

async fn web_branch_scope(
    State(state): State<WorkbenchWebState>,
    Query(query): Query<BranchScopeQuery>,
) -> std::result::Result<Json<syu_workbench_server::BranchScopeView>, ApiError> {
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    let index = workspace.index()?;
    let projection = current_projection(&state).await?;
    let range = match query.range.as_deref() {
        Some(range) if !range.trim().is_empty() => range.to_string(),
        _ => match default_workbench_range(&workspace) {
            Ok(range) => range,
            Err(error) => {
                return Ok(Json(syu_workbench_server::BranchScopeView::not_applicable(
                    error.to_string(),
                )));
            }
        },
    };
    let files = changed_files(&workspace.root, &range)
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, anyhow::anyhow!(error.to_string())))?;
    Ok(Json(syu_workbench_server::branch_scope_view(
        &index,
        &projection.items,
        range,
        &files,
    )))
}

async fn web_validate(
    State(state): State<WorkbenchWebState>,
    Json(request): Json<BrowserValidationRequest>,
) -> std::result::Result<Json<syu_workbench_server::ValidationRunView>, ApiError> {
    let started_at = std::time::SystemTime::now();
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    let index = workspace.index()?;
    let current_revision = revision(&workspace.root)?;
    let plan = current_projection(&state).await?.plan;
    let selected_slice = request
        .slice
        .as_deref()
        .and_then(|id| plan.as_ref()?.slices.iter().find(|slice| slice.id == id));
    let context = request.context.replace('-', "_");
    if (context == "work_plan" && plan.is_none())
        || (context == "slice" && selected_slice.is_none())
    {
        let reason = if context == "slice" {
            "Slice validation is not applicable because no valid slice is selected"
        } else {
            "Work plan validation is not applicable because no WorkPlan is selected"
        };
        return Ok(Json(
            syu_workbench_server::ValidationRunView::not_applicable(context, reason),
        ));
    }
    let (changed, basis) = if context == "git_range" {
        let range = match request.range.as_deref() {
            Some(range) if !range.trim().is_empty() => range.to_string(),
            _ => match default_workbench_range(&workspace) {
                Ok(range) => range,
                Err(error) => {
                    return Ok(Json(
                        syu_workbench_server::ValidationRunView::not_applicable(
                            context,
                            error.to_string(),
                        ),
                    ));
                }
            },
        };
        let files = match changed_files(&workspace.root, &range) {
            Ok(files) => files,
            Err(error) => {
                return Ok(Json(syu_workbench_server::ValidationRunView::failed(
                    context,
                    error.to_string(),
                    started_at,
                )));
            }
        };
        (Some(files), Some(range))
    } else {
        let basis = match context.as_str() {
            "work_plan" => plan.as_ref().map(|plan| plan.id.clone()),
            "slice" => selected_slice.map(|slice| slice.id.clone()),
            _ => Some(current_revision.clone()),
        };
        (None, basis)
    };
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: changed.as_deref(),
        reported_changed_files: None,
        work_plan: (context == "work_plan" || context == "slice")
            .then_some(plan.as_ref())
            .flatten(),
        selected_slice,
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(&current_revision),
        change_base_revision: None,
    });
    let evaluated_plan = context == "work_plan" || context == "slice";
    Ok(Json(syu_workbench_server::ValidationRunView::completed(
        context,
        basis,
        result,
        changed.is_some(),
        evaluated_plan,
        workspace.config.validation.preset,
        started_at,
    )))
}

fn default_workbench_range(workspace: &SpecWorkspace) -> Result<String> {
    if let Some(baseline) = &workspace.config.validation.changed.baseline {
        return range_from_baseline(&workspace.root, baseline);
    }
    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if let Ok(range) = range_from_baseline(
            &workspace.root,
            &syu_project_model::ChangeBaseline::MergeBase {
                against: syu_project_model::GitRef(candidate.into()),
            },
        ) {
            return Ok(range);
        }
    }
    bail!(
        "Git-range validation is not applicable: no configured baseline or default branch could be resolved"
    )
}

async fn web_source(
    State(state): State<WorkbenchWebState>,
    Query(query): Query<FileQuery>,
) -> std::result::Result<Json<FileSource>, ApiError> {
    let path = safe_workspace_path(&state.workspace_root, &query.path)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    Ok(Json(FileSource {
        path: query.path,
        hash: content_hash(&content),
        content,
    }))
}

async fn web_preview_file(
    State(state): State<WorkbenchWebState>,
    Json(edit): Json<FileEdit>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    let path = safe_workspace_path(&state.workspace_root, &edit.path)?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    ensure_source_hash(&old, &edit.expected_hash)?;
    let validation_errors = validate_candidate(&edit.path, &edit.content);
    let new_hash = content_hash(&edit.content);
    let preview_token = validation_errors
        .is_empty()
        .then(|| make_preview_token(&edit.path, &edit.expected_hash, &new_hash));
    if let Some(token) = &preview_token {
        state.preview_proofs.write().await.insert(
            edit.path.clone(),
            PreviewProof {
                expected_hash: edit.expected_hash.clone(),
                new_hash: new_hash.clone(),
                token: token.clone(),
            },
        );
    }
    Ok(Json(FilePreview {
        path: edit.path,
        old_hash: content_hash(&old),
        new_hash,
        changed_lines: changed_line_count(&old, &edit.content),
        validation_errors,
        preview_token,
    }))
}

async fn web_apply_file(
    State(state): State<WorkbenchWebState>,
    Json(edit): Json<FileEdit>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    let path = safe_workspace_path(&state.workspace_root, &edit.path)?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    ensure_source_hash(&old, &edit.expected_hash)?;
    let new_hash = content_hash(&edit.content);
    let proof = state.preview_proofs.read().await.get(&edit.path).cloned();
    let valid_proof = proof.is_some_and(|proof| preview_proof_matches(&edit, &new_hash, &proof));
    if !valid_proof {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!(
                "apply requires a successful preview for the same source hash and content"
            ),
        ));
    }
    let errors = validate_candidate(&edit.path, &edit.content);
    if !errors.is_empty() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            anyhow::anyhow!(errors.join("; ")),
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(anyhow::Error::from)?;
    }
    fs::write(&path, &edit.content).map_err(anyhow::Error::from)?;
    if let Err(error) = SpecWorkspace::load(&state.workspace_root)
        .and_then(|workspace| workspace.index().map(|_| ()))
    {
        if old.is_empty() {
            let _ = fs::remove_file(&path);
        } else {
            let _ = fs::write(&path, &old);
        }
        return Err(ApiError(StatusCode::UNPROCESSABLE_ENTITY, error));
    }
    state.preview_proofs.write().await.remove(&edit.path);
    Ok(Json(FilePreview {
        path: edit.path,
        old_hash: content_hash(&old),
        new_hash,
        changed_lines: changed_line_count(&old, &edit.content),
        validation_errors: Vec::new(),
        preview_token: None,
    }))
}

async fn web_preview_item(
    State(state): State<WorkbenchWebState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<ItemEditPayload>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    ensure_item_identity(&id, &payload)?;
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    ensure_item_path_in_spec_roots(&state.workspace_root, &workspace, &payload.path)?;
    let path = safe_workspace_path(&state.workspace_root, &payload.path)?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    ensure_source_hash(&old, &payload.expected_hash)?;
    let content = rewrite_item_source(&old, &payload)?;
    web_preview_file(
        State(state),
        Json(FileEdit {
            path: payload.path,
            content,
            expected_hash: payload.expected_hash,
            preview_token: None,
        }),
    )
    .await
}

async fn web_apply_item(
    State(state): State<WorkbenchWebState>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<ItemEditPayload>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    ensure_item_identity(&id, &payload)?;
    let workspace = SpecWorkspace::load(&state.workspace_root)?;
    ensure_item_path_in_spec_roots(&state.workspace_root, &workspace, &payload.path)?;
    let path = safe_workspace_path(&state.workspace_root, &payload.path)?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    ensure_source_hash(&old, &payload.expected_hash)?;
    let content = rewrite_item_source(&old, &payload)?;
    web_apply_file(
        State(state),
        Json(FileEdit {
            path: payload.path,
            content,
            expected_hash: payload.expected_hash,
            preview_token: payload.preview_token,
        }),
    )
    .await
}

async fn web_config(
    State(state): State<WorkbenchWebState>,
) -> std::result::Result<Json<ConfigSource>, ApiError> {
    let path = state.workspace_root.join("syu.yaml");
    let content = fs::read_to_string(path).map_err(anyhow::Error::from)?;
    let config = serde_yaml::from_str(&content).map_err(anyhow::Error::from)?;
    Ok(Json(ConfigSource {
        config,
        hash: content_hash(&content),
    }))
}

async fn web_preview_config(
    State(state): State<WorkbenchWebState>,
    Json(edit): Json<ConfigEdit>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    let original =
        fs::read_to_string(state.workspace_root.join("syu.yaml")).map_err(anyhow::Error::from)?;
    let content = patch_config_source(&original, &edit.config)?;
    web_preview_file(
        State(state),
        Json(FileEdit {
            path: "syu.yaml".to_string(),
            content,
            expected_hash: edit.expected_hash,
            preview_token: None,
        }),
    )
    .await
}

async fn web_apply_config(
    State(state): State<WorkbenchWebState>,
    Json(edit): Json<ConfigEdit>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    let original =
        fs::read_to_string(state.workspace_root.join("syu.yaml")).map_err(anyhow::Error::from)?;
    let content = patch_config_source(&original, &edit.config)?;
    web_apply_file(
        State(state),
        Json(FileEdit {
            path: "syu.yaml".to_string(),
            content,
            expected_hash: edit.expected_hash,
            preview_token: edit.preview_token,
        }),
    )
    .await
}

fn patch_config_source(
    source: &str,
    config: &syu_project_model::ProjectConfig,
) -> std::result::Result<String, ApiError> {
    let mut output = source.to_string();
    let original_config = serde_yaml::from_str::<syu_project_model::ProjectConfig>(source).ok();
    let list = |values: &[String]| format!("[{}]", values.join(", "));
    let paths = |values: &[RepoPath]| {
        format!(
            "[{}]",
            values
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let patterns = |values: &[syu_project_model::RepoPathPattern]| {
        format!(
            "[{}]",
            values
                .iter()
                .map(|pattern| pattern.0.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    output = replace_yaml_key(&output, "spec_roots", &paths(&config.workspace.spec_roots))?;
    output = replace_yaml_key(
        &output,
        "artifact_roots",
        &paths(&config.workspace.artifact_roots),
    )?;
    output = replace_yaml_key(&output, "excludes", &patterns(&config.workspace.excludes))?;
    output = replace_yaml_key(&output, "active", &list(&config.profiles.active))?;
    if original_config
        .as_ref()
        .is_none_or(|original| original.profiles.custom != config.profiles.custom)
    {
        output = replace_yaml_mapping(&output, 2, "custom", &config.profiles.custom)?;
    }
    let preset = serde_yaml::to_string(&config.validation.preset)
        .map_err(anyhow::Error::from)?
        .trim()
        .to_string();
    output = replace_yaml_key(&output, "preset", &preset)?;
    output = replace_yaml_key(
        &output,
        "deny_warnings",
        &config.validation.deny_warnings.to_string(),
    )?;
    if original_config
        .as_ref()
        .is_none_or(|original| original.validation.rules != config.validation.rules)
    {
        output = replace_yaml_mapping(&output, 2, "rules", &config.validation.rules)?;
    }
    if original_config.as_ref().is_none_or(|original| {
        original.validation.changed.baseline != config.validation.changed.baseline
    }) {
        output = replace_optional_baseline(&output, config.validation.changed.baseline.as_ref())?;
    }
    output = replace_yaml_key(
        &output,
        "require_owned_changes",
        &config.validation.changed.require_owned_changes.to_string(),
    )?;
    for (key, value) in [
        ("max_editable_files", config.work.slicing.max_editable_files),
        (
            "max_editable_symbols",
            config.work.slicing.max_editable_symbols,
        ),
        (
            "max_verification_targets",
            config.work.slicing.max_verification_targets,
        ),
        (
            "max_readonly_targets",
            config.work.slicing.max_readonly_targets,
        ),
        ("max_total_bytes", config.work.slicing.max_total_bytes),
    ] {
        output = replace_inline_scalar(&output, key, &value.to_string())?;
    }
    if original_config.as_ref().is_none_or(|original| {
        original.work.context.include_parent_principles
            != config.work.context.include_parent_principles
    }) {
        output = replace_yaml_key_if_present(
            &output,
            "include_parent_principles",
            &config.work.context.include_parent_principles.to_string(),
        );
    }
    if original_config.as_ref().is_none_or(|original| {
        original.work.context.include_parent_rules != config.work.context.include_parent_rules
    }) {
        output = replace_yaml_key_if_present(
            &output,
            "include_parent_rules",
            &config.work.context.include_parent_rules.to_string(),
        );
    }
    output = replace_inline_list(&output, "enabled", &list(&config.adapters.enabled))?;
    Ok(output)
}

fn replace_yaml_mapping<T: Serialize>(
    source: &str,
    indent: usize,
    key: &str,
    value: &T,
) -> std::result::Result<String, ApiError> {
    let yaml = serde_yaml::to_string(value).map_err(anyhow::Error::from)?;
    let yaml = yaml.trim();
    let prefix = " ".repeat(indent);
    let replacement = if yaml == "{}" {
        format!("{prefix}{key}: {{}}")
    } else {
        let nested = yaml
            .lines()
            .map(|line| format!("{}{}", " ".repeat(indent + 2), line))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{prefix}{key}:\n{nested}")
    };
    replace_yaml_block(source, indent, key, &replacement)
}

fn replace_optional_baseline(
    source: &str,
    baseline: Option<&ChangeBaseline>,
) -> std::result::Result<String, ApiError> {
    if let Some(baseline) = baseline {
        let yaml = serde_yaml::to_string(baseline).map_err(anyhow::Error::from)?;
        let nested = yaml
            .trim()
            .lines()
            .map(|line| format!("      {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let replacement = format!("    baseline:\n{nested}");
        if source
            .lines()
            .any(|line| line.trim_start().starts_with("baseline:"))
        {
            replace_yaml_block(source, 4, "baseline", &replacement)
        } else {
            Ok(source.replacen("  changed:\n", &format!("  changed:\n{replacement}\n"), 1))
        }
    } else if source
        .lines()
        .any(|line| line.trim_start().starts_with("baseline:"))
    {
        replace_yaml_block(source, 4, "baseline", "")
    } else {
        Ok(source.to_string())
    }
}

fn replace_yaml_block(
    source: &str,
    indent: usize,
    key: &str,
    replacement: &str,
) -> std::result::Result<String, ApiError> {
    let lines = source.lines().collect::<Vec<_>>();
    let marker = format!("{}{key}:", " ".repeat(indent));
    let start = lines
        .iter()
        .position(|line| line.starts_with(&marker))
        .ok_or_else(|| {
            ApiError(
                StatusCode::UNPROCESSABLE_ENTITY,
                anyhow::anyhow!("config source is missing {key}"),
            )
        })?;
    let mut end = start + 1;
    while end < lines.len() {
        let line = lines[end];
        if !line.trim().is_empty() && line.len() - line.trim_start().len() <= indent {
            break;
        }
        end += 1;
    }
    let mut output = Vec::new();
    output.extend_from_slice(&lines[..start]);
    if !replacement.is_empty() {
        output.extend(replacement.lines());
    }
    output.extend_from_slice(&lines[end..]);
    Ok(format!("{}\n", output.join("\n")))
}

fn replace_yaml_key(source: &str, key: &str, value: &str) -> std::result::Result<String, ApiError> {
    let pattern = regex::Regex::new(&format!(r"(?m)^(\s*{}:\s*).*$", regex::escape(key)))
        .map_err(anyhow::Error::from)?;
    if !pattern.is_match(source) {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            anyhow::anyhow!("config source is missing {key}"),
        ));
    }
    Ok(pattern
        .replace(source, format!("${{1}}{value}"))
        .into_owned())
}

fn replace_yaml_key_if_present(source: &str, key: &str, value: &str) -> String {
    let Ok(pattern) = regex::Regex::new(&format!(r"(?m)^(\s*{}:\s*).*$", regex::escape(key)))
    else {
        return source.to_string();
    };
    pattern
        .replace(source, format!("${{1}}{value}"))
        .into_owned()
}

fn replace_inline_scalar(
    source: &str,
    key: &str,
    value: &str,
) -> std::result::Result<String, ApiError> {
    let pattern = regex::Regex::new(&format!(r"({}:\s*)\d+", regex::escape(key)))
        .map_err(anyhow::Error::from)?;
    if !pattern.is_match(source) {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            anyhow::anyhow!("config source is missing {key}"),
        ));
    }
    Ok(pattern
        .replace(source, format!("${{1}}{value}"))
        .into_owned())
}

fn replace_inline_list(
    source: &str,
    key: &str,
    value: &str,
) -> std::result::Result<String, ApiError> {
    let pattern = regex::Regex::new(&format!(r"({}:\s*)\[[^\]]*\]", regex::escape(key)))
        .map_err(anyhow::Error::from)?;
    if !pattern.is_match(source) {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            anyhow::anyhow!("config source is missing {key}"),
        ));
    }
    Ok(pattern
        .replace(source, format!("${{1}}{value}"))
        .into_owned())
}

fn safe_workspace_path(root: &Path, relative: &str) -> std::result::Result<PathBuf, ApiError> {
    let relative = RepoPath::new(relative)
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, anyhow::anyhow!(error)))?;
    Ok(root.join(relative.as_path()))
}

fn ensure_item_identity(id: &str, payload: &ItemEditPayload) -> std::result::Result<(), ApiError> {
    if payload.id != id {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("item id does not match request path"),
        ));
    }
    Ok(())
}

fn ensure_item_path_in_spec_roots(
    root: &Path,
    workspace: &SpecWorkspace,
    relative: &str,
) -> std::result::Result<(), ApiError> {
    let path = safe_workspace_path(root, relative)?;
    if workspace
        .config
        .workspace
        .spec_roots
        .iter()
        .map(|spec_root| root.join(spec_root.as_path()))
        .any(|spec_root| path.starts_with(spec_root))
    {
        return Ok(());
    }
    Err(ApiError(
        StatusCode::BAD_REQUEST,
        anyhow::anyhow!("item path must be under workspace.spec_roots"),
    ))
}

fn rewrite_item_source(
    source: &str,
    payload: &ItemEditPayload,
) -> std::result::Result<String, ApiError> {
    if source.trim().is_empty() {
        let item = build_new_spec_item(payload)?;
        let document = new_item_document(payload, item)?;
        return serde_yaml::to_string(&document)
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.into()));
    }

    let mut document: SpecDocument = serde_yaml::from_str(source).map_err(anyhow::Error::from)?;
    patch_existing_item_in_document(&mut document, payload)?;
    serde_yaml::to_string(&document)
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.into()))
}

enum EditableSpecItem {
    Philosophy(Philosophy),
    Policy(Policy),
    Requirement(Requirement),
    Feature(Feature),
}

fn patch_existing_item_in_document(
    document: &mut SpecDocument,
    payload: &ItemEditPayload,
) -> std::result::Result<(), ApiError> {
    let id = parse_from_string::<SpecId>(&payload.id)?;
    match document {
        SpecDocument::Philosophies { philosophies, .. } if payload.kind == "philosophy" => {
            let item = philosophies
                .iter_mut()
                .find(|entry| entry.id == id)
                .ok_or_else(|| {
                    ApiError(
                        StatusCode::NOT_FOUND,
                        anyhow::anyhow!("item {} was not found", payload.id),
                    )
                })?;
            item.title = payload.title.clone();
            item.summary = payload.summary.clone().unwrap_or_default();
            item.principles = payload
                .principles
                .iter()
                .map(principle_from_summary)
                .collect::<std::result::Result<_, _>>()?;
        }
        SpecDocument::Policies { policies, .. } if payload.kind == "policy" => {
            let item = policies
                .iter_mut()
                .find(|entry| entry.id == id)
                .ok_or_else(|| {
                    ApiError(
                        StatusCode::NOT_FOUND,
                        anyhow::anyhow!("item {} was not found", payload.id),
                    )
                })?;
            item.title = payload.title.clone();
            item.summary = payload.summary.clone().unwrap_or_default();
            item.description = payload.description.clone().unwrap_or_default();
            item.rules = payload
                .rules
                .iter()
                .map(rule_from_summary)
                .collect::<std::result::Result<_, _>>()?;
        }
        SpecDocument::Requirements { requirements, .. } if payload.kind == "requirement" => {
            let item = requirements
                .iter_mut()
                .find(|entry| entry.id == id)
                .ok_or_else(|| {
                    ApiError(
                        StatusCode::NOT_FOUND,
                        anyhow::anyhow!("item {} was not found", payload.id),
                    )
                })?;
            item.title = payload.title.clone();
            item.description = payload.description.clone().unwrap_or_default();
            item.priority = parse_enum(payload.priority.as_deref().unwrap_or("medium"))?;
            item.status = parse_enum(payload.status.as_deref().unwrap_or("planned"))?;
            item.criteria = payload
                .criteria
                .iter()
                .map(criterion_from_summary)
                .collect::<std::result::Result<_, _>>()?;
        }
        SpecDocument::Features { features, .. } if payload.kind == "feature" => {
            let item = features
                .iter_mut()
                .find(|entry| entry.id == id)
                .ok_or_else(|| {
                    ApiError(
                        StatusCode::NOT_FOUND,
                        anyhow::anyhow!("item {} was not found", payload.id),
                    )
                })?;
            item.title = payload.title.clone();
            item.summary = payload.summary.clone().unwrap_or_default();
            item.status = parse_enum(payload.status.as_deref().unwrap_or("planned"))?;
        }
        _ => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                anyhow::anyhow!("item kind does not match existing document"),
            ));
        }
    }
    Ok(())
}

fn new_item_document(
    payload: &ItemEditPayload,
    item: EditableSpecItem,
) -> std::result::Result<SpecDocument, ApiError> {
    let namespace = "workbench".to_string();
    let category = "Workbench".to_string();
    match (payload.kind.as_str(), item) {
        ("philosophy", EditableSpecItem::Philosophy(item)) => Ok(SpecDocument::Philosophies {
            schema: syu_spec_model::SPEC_SCHEMA.into(),
            namespace,
            category,
            philosophies: vec![item],
        }),
        ("policy", EditableSpecItem::Policy(item)) => Ok(SpecDocument::Policies {
            schema: syu_spec_model::SPEC_SCHEMA.into(),
            namespace,
            category,
            policies: vec![item],
        }),
        ("requirement", EditableSpecItem::Requirement(item)) => Ok(SpecDocument::Requirements {
            schema: syu_spec_model::SPEC_SCHEMA.into(),
            namespace,
            category,
            requirements: vec![item],
        }),
        ("feature", EditableSpecItem::Feature(item)) => Ok(SpecDocument::Features {
            schema: syu_spec_model::SPEC_SCHEMA.into(),
            namespace,
            category,
            features: vec![item],
        }),
        _ => Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("item kind is not supported"),
        )),
    }
}

fn build_new_spec_item(
    payload: &ItemEditPayload,
) -> std::result::Result<EditableSpecItem, ApiError> {
    let id = parse_from_string::<SpecId>(&payload.id)?;
    match payload.kind.as_str() {
        "philosophy" => Ok(EditableSpecItem::Philosophy(Philosophy {
            id,
            title: payload.title.clone(),
            summary: payload.summary.clone().unwrap_or_default(),
            principles: payload
                .principles
                .iter()
                .map(principle_from_summary)
                .collect::<std::result::Result<_, _>>()?,
            bindings: Vec::new(),
        })),
        "policy" => Ok(EditableSpecItem::Policy(Policy {
            id,
            title: payload.title.clone(),
            summary: payload.summary.clone().unwrap_or_default(),
            description: payload.description.clone().unwrap_or_default(),
            rules: payload
                .rules
                .iter()
                .map(rule_from_summary)
                .collect::<std::result::Result<_, _>>()?,
            bindings: Vec::new(),
        })),
        "requirement" => Ok(EditableSpecItem::Requirement(Requirement {
            id,
            title: payload.title.clone(),
            description: payload.description.clone().unwrap_or_default(),
            priority: parse_enum(payload.priority.as_deref().unwrap_or("medium"))?,
            status: parse_enum(payload.status.as_deref().unwrap_or("planned"))?,
            criteria: payload
                .criteria
                .iter()
                .map(criterion_from_summary)
                .collect::<std::result::Result<_, _>>()?,
            bindings: Vec::new(),
        })),
        "feature" => Ok(EditableSpecItem::Feature(Feature {
            id,
            title: payload.title.clone(),
            summary: payload.summary.clone().unwrap_or_default(),
            status: parse_enum(payload.status.as_deref().unwrap_or("planned"))?,
            bindings: Vec::new(),
            contracts: Vec::new(),
        })),
        _ => Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("item kind {} is not supported", payload.kind),
        )),
    }
}

fn principle_from_summary(
    summary: &syu_workbench_server::PrincipleSummary,
) -> std::result::Result<syu_spec_model::Principle, ApiError> {
    Ok(syu_spec_model::Principle {
        id: local_id_from_anchor(&summary.anchor, LocalAnchorKind::Principle)?,
        statement: summary.statement.clone(),
        applies_to: summary.applies_to.clone(),
    })
}

fn rule_from_summary(
    summary: &syu_workbench_server::RuleSummary,
) -> std::result::Result<Rule, ApiError> {
    Ok(Rule {
        id: local_id_from_anchor(&summary.anchor, LocalAnchorKind::Rule)?,
        level: parse_enum(&summary.level)?,
        statement: summary.statement.clone(),
        governed_by: summary
            .governed_by
            .iter()
            .map(|value| parse_from_string::<SpecAnchor>(value))
            .collect::<std::result::Result<_, _>>()?,
        applies_to: Default::default(),
        enforcement: None,
    })
}

fn criterion_from_summary(
    summary: &syu_workbench_server::CriterionSummary,
) -> std::result::Result<Criterion, ApiError> {
    Ok(Criterion {
        id: local_id_from_anchor(&summary.anchor, LocalAnchorKind::Criterion)?,
        kind: parse_enum(&summary.kind)?,
        statement: summary.statement.clone(),
        governed_by: summary
            .governed_by
            .iter()
            .map(|value| parse_from_string::<SpecAnchor>(value))
            .collect::<std::result::Result<_, _>>()?,
    })
}

fn local_id_from_anchor(
    anchor: &str,
    expected_kind: LocalAnchorKind,
) -> std::result::Result<LocalId, ApiError> {
    let parsed = parse_from_string::<SpecAnchor>(anchor)?;
    if parsed.kind != expected_kind {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("anchor {anchor} does not match expected kind"),
        ));
    }
    Ok(parsed.local_id)
}

fn parse_enum<T>(value: &str) -> std::result::Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    parse_from_string(value)
}

fn parse_from_string<T>(value: &str) -> std::result::Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|error| {
        ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("invalid value {value}: {error}"),
        )
    })
}

fn content_hash(content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(content.as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

fn make_preview_token(path: &str, expected_hash: &str, new_hash: &str) -> String {
    content_hash(&format!("{path}\n{expected_hash}\n{new_hash}"))
}

fn preview_proof_matches(edit: &FileEdit, new_hash: &str, proof: &PreviewProof) -> bool {
    edit.preview_token.as_deref() == Some(&proof.token)
        && edit.expected_hash == proof.expected_hash
        && new_hash == proof.new_hash
}

fn ensure_source_hash(content: &str, expected: &str) -> std::result::Result<(), ApiError> {
    let actual = content_hash(content);
    if actual != expected {
        return Err(ApiError(
            StatusCode::CONFLICT,
            anyhow::anyhow!("stale source: expected {expected}, found {actual}"),
        ));
    }
    Ok(())
}

fn validate_candidate(path: &str, content: &str) -> Vec<String> {
    let result = if path == "syu.yaml" {
        serde_yaml::from_str::<syu_project_model::ProjectConfig>(content).map(|_| ())
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str::<syu_spec_model::SpecDocument>(content).map(|_| ())
    } else {
        return Vec::new();
    };
    result
        .err()
        .map(|error| vec![error.to_string()])
        .unwrap_or_default()
}

fn changed_line_count(old: &str, new: &str) -> usize {
    let old = old.lines().collect::<Vec<_>>();
    let new = new.lines().collect::<Vec<_>>();
    let common = old.len().min(new.len());
    old[..common]
        .iter()
        .zip(&new[..common])
        .filter(|(left, right)| left != right)
        .count()
        + old.len().abs_diff(new.len())
}
fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_yaml::from_str(
        &fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("strict parse {}", path.display()))
}
fn write_yaml<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml::to_string(value)?)?;
    Ok(())
}
fn revision(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn changed_files_for_validation(
    workspace: &SpecWorkspace,
    args: &ValidateArgs,
) -> Result<Option<Vec<ChangedFile>>> {
    if let Some(range) = &args.range {
        return Ok(Some(changed_files(&workspace.root, range)?));
    }
    if let Some(baseline) = &args.baseline {
        return Ok(Some(changed_files(
            &workspace.root,
            &range_from_baseline(&workspace.root, &parse_cli_baseline(baseline)?)?,
        )?));
    }
    if let Some(baseline) = &workspace.config.validation.changed.baseline {
        return Ok(Some(changed_files(
            &workspace.root,
            &range_from_baseline(&workspace.root, baseline)?,
        )?));
    }
    Ok(None)
}

fn validation_inputs_for_cli(
    workspace: &SpecWorkspace,
    args: &ValidateArgs,
    plan: Option<&WorkPlan>,
) -> Result<ValidationInputs> {
    let reported_changed_files = changed_files_for_validation(workspace, args)?;
    if let Some(plan) = plan {
        let actual_changes = changed_files_against_revision(&workspace.root, &plan.basis.revision)?
            .into_iter()
            .filter(|file| governed_change(workspace, file))
            .collect::<Vec<_>>();
        let plan_mode = if actual_changes.is_empty() {
            PlanValidationMode::PreState
        } else {
            PlanValidationMode::PostState
        };
        return Ok(ValidationInputs {
            changed_files: (!actual_changes.is_empty()).then_some(actual_changes),
            reported_changed_files,
            change_base_revision: Some(plan.basis.revision.clone()),
            plan_mode,
        });
    }
    let change_base_revision = if let Some(range) = &args.range {
        Some(change_base_revision_for_range(workspace, range)?)
    } else if let Some(baseline) = &args.baseline {
        Some(base_revision_from_baseline(
            workspace,
            &parse_cli_baseline(baseline)?,
        )?)
    } else if let Some(baseline) = &workspace.config.validation.changed.baseline {
        Some(base_revision_from_baseline(workspace, baseline)?)
    } else {
        None
    };
    Ok(ValidationInputs {
        changed_files: reported_changed_files,
        reported_changed_files: None,
        change_base_revision,
        plan_mode: PlanValidationMode::PreState,
    })
}

fn governed_change(workspace: &SpecWorkspace, file: &ChangedFile) -> bool {
    file.old_path
        .iter()
        .chain(file.new_path.iter())
        .any(|path| {
            path.as_path() == Path::new("syu.yaml")
                || workspace.path_is_spec(path.as_path())
                || (workspace.path_is_artifact(path.as_path())
                    && !workspace.path_is_excluded(path.as_path()))
        })
}

fn parse_cli_baseline(value: &str) -> Result<ChangeBaseline> {
    if value == "parent" {
        return Ok(ChangeBaseline::Parent);
    }
    if let Some(refname) = value.strip_prefix("merge-base:") {
        return Ok(ChangeBaseline::MergeBase {
            against: GitRef(refname.into()),
        });
    }
    if let Some(refname) = value.strip_prefix("revision:") {
        return Ok(ChangeBaseline::Revision {
            revision: GitRef(refname.into()),
        });
    }
    bail!("baseline must be parent, merge-base:<ref>, or revision:<ref>");
}

fn range_from_baseline(root: &Path, baseline: &ChangeBaseline) -> Result<String> {
    match baseline {
        ChangeBaseline::MergeBase { against } => Ok(merge_base(root, &against.0)?),
        ChangeBaseline::Revision { revision } => Ok(revision.0.clone()),
        ChangeBaseline::Parent => Ok("HEAD^".into()),
    }
}

fn base_revision_from_baseline(
    workspace: &SpecWorkspace,
    baseline: &ChangeBaseline,
) -> Result<String> {
    match baseline {
        ChangeBaseline::MergeBase { against } => merge_base(&workspace.root, &against.0),
        ChangeBaseline::Revision { revision } => Ok(revision.0.clone()),
        ChangeBaseline::Parent => revision_parent(&workspace.root),
    }
}

fn change_base_revision_for_range(workspace: &SpecWorkspace, range: &str) -> Result<String> {
    if let Some((left, right)) = range.split_once("...") {
        let right = if right.is_empty() { "HEAD" } else { right };
        return merge_base_pair(&workspace.root, left, right);
    }
    if let Some((left, _)) = range.split_once("..") {
        return Ok(left.to_string());
    }
    Ok(range.to_string())
}

fn merge_base(root: &Path, against: &str) -> Result<String> {
    merge_base_pair(root, "HEAD", against)
}

fn merge_base_pair(root: &Path, left: &str, right: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["merge-base", left, right])
        .output()?;
    if !output.status.success() {
        bail!("git merge-base {left} {right} failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn revision_parent(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["rev-parse", "HEAD^"])
        .output()?;
    if !output.status.success() {
        bail!("git rev-parse HEAD^ failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn changed_files_against_revision(root: &Path, revision: &str) -> Result<Vec<ChangedFile>> {
    let status_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", "-z", "-M", "--relative", revision])
        .output()?;
    if !status_output.status.success() {
        bail!("git diff --name-status {revision} failed");
    }
    let untracked_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()?;
    if !untracked_output.status.success() {
        bail!("git ls-files --others failed");
    }
    let patch_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args([
            "diff",
            "-M",
            "--relative",
            "--unified=0",
            "--no-color",
            revision,
        ])
        .output()?;
    if !patch_output.status.success() {
        bail!("git diff --unified=0 {revision} failed");
    }
    let mut files = parse_changed_files(
        &status_output.stdout,
        &untracked_output.stdout,
        &String::from_utf8(patch_output.stdout)?,
    )?;
    synthesize_untracked_ranges(root, &mut files)?;
    Ok(files)
}

fn changed_files(root: &Path, range: &str) -> Result<Vec<ChangedFile>> {
    let status_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", "-z", "-M", "--relative", range])
        .output()?;
    if !status_output.status.success() {
        bail!("git diff --name-status {range} failed");
    }
    let untracked_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()?;
    if !untracked_output.status.success() {
        bail!("git ls-files --others failed");
    }
    let patch_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args([
            "diff",
            "-M",
            "--relative",
            "--unified=0",
            "--no-color",
            range,
        ])
        .output()?;
    if !patch_output.status.success() {
        bail!("git diff --unified=0 {range} failed");
    }
    let mut files = parse_changed_files(
        &status_output.stdout,
        &untracked_output.stdout,
        &String::from_utf8(patch_output.stdout)?,
    )?;
    synthesize_untracked_ranges(root, &mut files)?;
    Ok(files)
}

fn ensure_clean_plan_workspace(workspace: &SpecWorkspace) -> Result<()> {
    let mut args = vec![
        "-C".to_string(),
        workspace.root.to_string_lossy().into_owned(),
        "status".to_string(),
        "--porcelain".to_string(),
        "-z".to_string(),
        "--untracked-files=all".to_string(),
        "--".to_string(),
        "syu.yaml".to_string(),
    ];
    for root in &workspace.config.workspace.spec_roots {
        args.push(root.to_string_lossy().into_owned());
    }
    for root in &workspace.config.workspace.artifact_roots {
        args.push(root.to_string_lossy().into_owned());
    }
    let output = Command::new("git").args(&args).output()?;
    if !output.status.success() {
        bail!("git status for governed workspace roots failed");
    }
    if output.stdout.is_empty() {
        return Ok(());
    }
    let first_dirty = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .nth(1)
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .unwrap_or_else(|| "governed workspace".to_string());
    bail!(
        "work plan requires a clean governed workspace rooted at HEAD; dirty path: {first_dirty}"
    );
}

fn parse_changed_files(status: &[u8], untracked: &[u8], patch: &str) -> Result<Vec<ChangedFile>> {
    let mut files = Vec::new();
    let mut entries = status
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    while let Some(kind) = entries.next() {
        let kind = String::from_utf8(kind.to_vec())?;
        let (status, old_path, new_path) = match kind.chars().next().unwrap_or('M') {
            'A' => (
                ChangeStatus::Added,
                None,
                Some(String::from_utf8(
                    entries.next().context("missing added path")?.to_vec(),
                )?),
            ),
            'M' => {
                let path =
                    String::from_utf8(entries.next().context("missing modified path")?.to_vec())?;
                (ChangeStatus::Modified, Some(path.clone()), Some(path))
            }
            'D' => (
                ChangeStatus::Deleted,
                Some(String::from_utf8(
                    entries.next().context("missing deleted path")?.to_vec(),
                )?),
                None,
            ),
            'R' => (
                ChangeStatus::Renamed,
                Some(String::from_utf8(
                    entries.next().context("missing rename source")?.to_vec(),
                )?),
                Some(String::from_utf8(
                    entries.next().context("missing rename target")?.to_vec(),
                )?),
            ),
            _ => {
                let path = entries
                    .next()
                    .map(|value| String::from_utf8(value.to_vec()))
                    .transpose()?;
                (ChangeStatus::Modified, path.clone(), path)
            }
        };
        files.push(ChangedFile {
            status,
            old_path: old_path
                .map(RepoPath::new)
                .transpose()
                .map_err(anyhow::Error::msg)?,
            new_path: new_path
                .map(RepoPath::new)
                .transpose()
                .map_err(anyhow::Error::msg)?,
            hunks: Vec::new(),
        });
    }
    for path in untracked
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        files.push(ChangedFile {
            status: ChangeStatus::Untracked,
            old_path: None,
            new_path: Some(
                RepoPath::new(String::from_utf8(path.to_vec())?).map_err(anyhow::Error::msg)?,
            ),
            hunks: Vec::new(),
        });
    }
    let mut current: Option<usize> = None;
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            current = None;
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            current = files.iter().position(|file| {
                file.new_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            });
            continue;
        }
        if line == "+++ /dev/null" {
            continue;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            current = files.iter().position(|file| {
                file.old_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            });
            continue;
        }
        if let Some((old_path, new_path)) = parse_binary_patch_paths(line) {
            if let Some(index) = files.iter().position(|file| {
                file.old_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == old_path)
                    || file
                        .new_path
                        .as_ref()
                        .is_some_and(|value| value.to_string_lossy() == new_path)
            }) {
                files[index].status = ChangeStatus::Binary;
            }
            current = None;
            continue;
        }
        if let Some(hunk) = line.strip_prefix("@@ ")
            && let Some(index) = current
        {
            files[index].hunks.push(parse_hunk_header(hunk)?);
        }
    }
    Ok(files)
}

fn synthesize_untracked_ranges(root: &Path, files: &mut [ChangedFile]) -> Result<()> {
    for file in files {
        if file.status != ChangeStatus::Untracked || !file.hunks.is_empty() {
            continue;
        }
        let Some(path) = file.new_path.as_ref() else {
            continue;
        };
        let contents = match fs::read(root.join(path.as_path())) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        if std::str::from_utf8(&contents).is_err() {
            continue;
        }
        let line_count = contents.iter().filter(|byte| **byte == b'\n').count()
            + usize::from(!contents.is_empty() && *contents.last().unwrap_or(&b'\n') != b'\n');
        file.hunks.push(ChangedRange {
            old_start: 0,
            old_end: 0,
            new_start: 1,
            new_end: line_count.max(1),
        });
    }
    Ok(())
}

fn parse_binary_patch_paths(line: &str) -> Option<(&str, &str)> {
    let paths = line.strip_prefix("Binary files ")?;
    let (old_path, remainder) = paths.split_once(" and ")?;
    let new_path = remainder.strip_suffix(" differ")?;
    let old_path = old_path.strip_prefix("a/").unwrap_or(old_path);
    let new_path = new_path.strip_prefix("b/").unwrap_or(new_path);
    Some((old_path, new_path))
}

fn parse_hunk_header(header: &str) -> Result<ChangedRange> {
    let range = header
        .split(" @@")
        .next()
        .context("malformed hunk header")?;
    let mut parts = range.split_whitespace();
    let old = parts.next().context("missing old hunk range")?;
    let new = parts.next().context("missing new hunk range")?;
    let (old_start, old_end) = parse_diff_span(old.trim_start_matches('-'))?;
    let (new_start, new_end) = parse_diff_span(new.trim_start_matches('+'))?;
    Ok(ChangedRange {
        old_start,
        old_end,
        new_start,
        new_end,
    })
}

fn parse_diff_span(span: &str) -> Result<(usize, usize)> {
    let (start, len) = span
        .split_once(',')
        .map_or((span, "1"), |(start, len)| (start, len));
    let start = start.parse::<usize>()?;
    let len = len.parse::<usize>()?;
    if len == 0 {
        return Ok((0, 0));
    }
    let end = start + len - 1;
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_changed_files_keeps_deleted_file_hunks_on_the_deleted_path() {
        let files = parse_changed_files(
            b"M\0src/a.rs\0D\0src/b.rs\0",
            b"",
            "\
diff --git a/src/a.rs b/src/a.rs\n\
--- a/src/a.rs\n\
+++ b/src/a.rs\n\
@@ -1 +1 @@\n\
diff --git a/src/b.rs b/src/b.rs\n\
--- a/src/b.rs\n\
+++ /dev/null\n\
@@ -2,2 +0,0 @@\n",
        )
        .unwrap();
        assert_eq!(files[1].status, ChangeStatus::Deleted);
        assert_eq!(files[1].hunks.len(), 1);
        assert!(files[0].hunks.len() <= 1);
    }

    #[test]
    fn parse_changed_files_collects_untracked_paths() {
        let files = parse_changed_files(b"", b"src/new.rs\0", "").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, ChangeStatus::Untracked);
        assert_eq!(
            files[0].new_path.as_ref().unwrap().to_string_lossy(),
            "src/new.rs"
        );
    }

    #[test]
    fn parse_hunk_header_preserves_zero_length_sides_as_empty() {
        let hunk = parse_hunk_header("-4,0 +5,3 @@").unwrap();
        assert_eq!(
            hunk,
            ChangedRange {
                old_start: 0,
                old_end: 0,
                new_start: 5,
                new_end: 7,
            }
        );
    }

    #[test]
    fn parse_changed_files_keeps_modified_path_on_both_sides() {
        let files = parse_changed_files(b"M\0src/app.rs\0", b"", "").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, ChangeStatus::Modified);
        assert_eq!(
            files[0].old_path.as_ref().unwrap().to_string_lossy(),
            "src/app.rs"
        );
        assert_eq!(
            files[0].new_path.as_ref().unwrap().to_string_lossy(),
            "src/app.rs"
        );
    }

    #[test]
    fn synthesize_untracked_ranges_covers_full_text_file() {
        let tempdir = tempdir().unwrap();
        fs::create_dir_all(tempdir.path().join("src")).unwrap();
        fs::write(tempdir.path().join("src/new.rs"), "fn a() {}\nfn b() {}\n").unwrap();
        let mut files = parse_changed_files(b"", b"src/new.rs\0", "").unwrap();
        synthesize_untracked_ranges(tempdir.path(), &mut files).unwrap();
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(
            files[0].hunks[0],
            ChangedRange {
                old_start: 0,
                old_end: 0,
                new_start: 1,
                new_end: 2,
            }
        );
    }

    #[test]
    fn parse_changed_files_marks_binary_patches_from_patch_output() {
        let files = parse_changed_files(
            b"M\0assets/logo.png\0",
            b"",
            "\
diff --git a/assets/logo.png b/assets/logo.png\n\
Binary files a/assets/logo.png and b/assets/logo.png differ\n",
        )
        .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, ChangeStatus::Binary);
    }

    #[test]
    fn parse_changed_files_does_not_treat_type_changes_as_binary() {
        let files = parse_changed_files(b"T\0src/app.rs\0", b"", "").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, ChangeStatus::Modified);
        assert_eq!(
            files[0].new_path.as_ref().unwrap().to_string_lossy(),
            "src/app.rs"
        );
    }

    #[test]
    fn workbench_file_edits_reject_stale_and_escaping_sources() {
        let hash = content_hash("before\n");
        assert!(ensure_source_hash("before\n", &hash).is_ok());
        assert!(ensure_source_hash("after\n", &hash).is_err());
        let root = Path::new("/tmp/workspace");
        assert!(safe_workspace_path(root, "spec/requirement.yaml").is_ok());
        assert!(safe_workspace_path(root, "../outside.yaml").is_err());
        assert!(safe_workspace_path(root, "/etc/passwd").is_err());
    }

    #[test]
    fn workbench_preview_strictly_validates_config_and_spec_yaml() {
        assert!(validate_candidate("syu.yaml", "schema: wrong\n").len() == 1);
        assert!(validate_candidate("spec/item.yaml", "schema: wrong\n").len() == 1);
        assert!(validate_candidate("README.md", "anything").is_empty());
        assert_eq!(changed_line_count("a\nb\n", "a\nc\nd\n"), 2);
    }

    #[test]
    fn apply_proof_is_bound_to_path_hash_and_previewed_content() {
        let expected_hash = content_hash("before");
        let new_hash = content_hash("after");
        let token = make_preview_token("syu.yaml", &expected_hash, &new_hash);
        let proof = PreviewProof {
            expected_hash: expected_hash.clone(),
            new_hash: new_hash.clone(),
            token: token.clone(),
        };
        let edit = FileEdit {
            path: "syu.yaml".into(),
            content: "after".into(),
            expected_hash,
            preview_token: Some(token),
        };
        assert!(preview_proof_matches(&edit, &new_hash, &proof));
        assert!(!preview_proof_matches(
            &edit,
            &content_hash("changed"),
            &proof
        ));
    }

    #[test]
    fn new_item_document_validates_under_spec_root() {
        let workspace = SpecWorkspace::load("fixtures/v1/valid-web-app").unwrap();
        let root = Path::new("fixtures/v1/valid-web-app");
        assert!(
            ensure_item_path_in_spec_roots(root, &workspace, "spec/requirements/req-new.yaml")
                .is_ok()
        );
        assert!(
            ensure_item_path_in_spec_roots(root, &workspace, "docs/syu/requirements/req-new.yaml")
                .is_err()
        );
    }

    #[test]
    fn structured_config_edits_preserve_unrelated_source_lines() {
        let source = fs::read_to_string("fixtures/v1/valid-web-app/syu.yaml").unwrap();
        let mut config: syu_project_model::ProjectConfig = serde_yaml::from_str(&source).unwrap();
        config.validation.deny_warnings = !config.validation.deny_warnings;
        let updated = patch_config_source(&source, &config).unwrap();
        assert_eq!(changed_line_count(&source, &updated), 1);
        assert!(updated.contains("contract_rules:"));
        assert!(updated.contains("facets:"));
        let reparsed: syu_project_model::ProjectConfig = serde_yaml::from_str(&updated).unwrap();
        assert_eq!(reparsed, config);
    }

    #[test]
    fn structured_config_edits_cover_every_workspace_subpage() {
        let source = fs::read_to_string("syu.yaml").unwrap();
        let mut config: syu_project_model::ProjectConfig = serde_yaml::from_str(&source).unwrap();
        config
            .workspace
            .excludes
            .push(syu_project_model::RepoPathPattern("tmp/**".into()));
        config.validation.rules.insert(
            "SYU-DOC-001".into(),
            syu_project_model::RuleOverride::Warning,
        );
        config.validation.changed.baseline = Some(ChangeBaseline::MergeBase {
            against: GitRef("origin/main".into()),
        });
        config.work.context.include_parent_rules = !config.work.context.include_parent_rules;
        let updated = patch_config_source(&source, &config).unwrap();
        let reparsed: syu_project_model::ProjectConfig = serde_yaml::from_str(&updated).unwrap();
        assert_eq!(reparsed, config);
    }

    #[test]
    fn item_edit_preserves_bindings_and_contracts() {
        let source = r#"
schema: syu/spec/v1
kind: features
namespace: test
category: Test
features:
  - id: FEAT-1
    title: Old
    summary: Old summary
    status: implemented
    bindings:
      - id: impl
        role: implementation
        facet: ui
        responsibility: Keep me
        targets:
          - id: target
            path: web/login.ts
            selector: { kind: file }
            adapter: typescript
        satisfies: []
        verifies: []
        documents: []
        enforces: []
        generated_from: []
        evidences: []
    contracts:
      - id: http
        kind: http
        source: FEAT-1#binding.impl/target.target
        participants:
          - binding: FEAT-1#binding.impl
            role: provider
        guarantees: []
"#;
        let payload = ItemEditPayload {
            id: "FEAT-1".into(),
            kind: "feature".into(),
            path: "spec/features/feat-1.yaml".into(),
            title: "New".into(),
            summary: Some("New summary".into()),
            description: None,
            status: Some("implemented".into()),
            priority: None,
            principles: vec![],
            rules: vec![],
            criteria: vec![],
            expected_hash: content_hash(source),
            preview_token: None,
        };

        let next = rewrite_item_source(source, &payload).unwrap();
        assert!(next.contains("title: New"));
        assert!(next.contains("responsibility: Keep me"));
        assert!(next.contains("kind: http"));
        assert!(serde_yaml::from_str::<SpecDocument>(&next).is_ok());
    }
}
