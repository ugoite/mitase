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
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use syu_app_ui::WorkbenchView;
use syu_planner::{export_context, plan};
use syu_project_model::{ChangeBaseline, GitRef};
use syu_spec_model::RepoPath;
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
            };
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(async move {
                let app = Router::new()
                    .route("/", get(web_index))
                    .route("/api/projection", get(web_projection))
                    .route("/api/work/request", put(web_update_request))
                    .route("/api/work/replan", post(web_replan))
                    .route(
                        "/api/context/{slice}",
                        post(web_export_context).get(web_export_context),
                    )
                    .route("/api/validate", post(web_validate))
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
    if request.context == "slice" && selected_slice.is_none() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("slice validation requires a valid slice id"),
        ));
    }
    let (changed, basis) = if request.context == "git-range" {
        let range = match request.range.as_deref() {
            Some(range) if !range.trim().is_empty() => range.to_string(),
            _ => default_workbench_range(&workspace)?,
        };
        (Some(changed_files(&workspace.root, &range)?), Some(range))
    } else {
        (None, Some(current_revision.clone()))
    };
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: changed.as_deref(),
        reported_changed_files: None,
        work_plan: (request.context == "work-plan" || request.context == "slice")
            .then_some(plan.as_ref())
            .flatten(),
        selected_slice,
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(&current_revision),
        change_base_revision: None,
    });
    Ok(Json(syu_workbench_server::ValidationRunView::completed(
        request.context,
        basis,
        result,
        changed.is_some(),
        plan.is_some(),
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
    Ok(Json(FilePreview {
        path: edit.path,
        old_hash: content_hash(&old),
        new_hash: content_hash(&edit.content),
        changed_lines: changed_line_count(&old, &edit.content),
        validation_errors,
    }))
}

async fn web_apply_file(
    State(state): State<WorkbenchWebState>,
    Json(edit): Json<FileEdit>,
) -> std::result::Result<Json<FilePreview>, ApiError> {
    let path = safe_workspace_path(&state.workspace_root, &edit.path)?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    ensure_source_hash(&old, &edit.expected_hash)?;
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
    Ok(Json(FilePreview {
        path: edit.path,
        old_hash: content_hash(&old),
        new_hash: content_hash(&edit.content),
        changed_lines: changed_line_count(&old, &edit.content),
        validation_errors: Vec::new(),
    }))
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
        }),
    )
    .await
}

fn patch_config_source(
    source: &str,
    config: &syu_project_model::ProjectConfig,
) -> std::result::Result<String, ApiError> {
    let mut output = source.to_string();
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
    output = replace_yaml_key(&output, "spec_roots", &paths(&config.workspace.spec_roots))?;
    output = replace_yaml_key(
        &output,
        "artifact_roots",
        &paths(&config.workspace.artifact_roots),
    )?;
    output = replace_yaml_key(&output, "active", &list(&config.profiles.active))?;
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
    output = replace_inline_list(&output, "enabled", &list(&config.adapters.enabled))?;
    Ok(output)
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

fn content_hash(content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(content.as_bytes());
    format!("sha256:{:x}", digest.finalize())
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
}
