#![forbid(unsafe_code)]
mod lsp;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
};
use syu_delivery::DeliveryStore;
use syu_inventory::{InventoryContext, InventoryRegistry};
use syu_planner::{export_context, plan, validate_work_origin, validate_work_request};
use syu_project_model::{ChangeBaseline, GitRef};
use syu_spec_model::RepoPath;
use syu_validation::{
    ChangeStatus, ChangedFile, ChangedRange, PlanValidationMode, ValidationContext, validate,
};
use syu_work_model::{
    CompletionStatus, ExecutionIdentity, PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus,
    VerificationAttemptStatus, WorkPlan, WorkRequest,
};
use syu_workbench_server::project as project_workbench;
use syu_workspace::SpecWorkspace;

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
    Readiness(ReadinessArgs),
    Work(WorkArgs),
    Task(TaskArgs),
    Workbench(WorkbenchArgs),
    Lsp,
}
#[derive(Debug, Args)]
struct ReadinessArgs {
    #[command(subcommand)]
    command: ReadinessCommand,
}
#[derive(Debug, Subcommand)]
enum ReadinessCommand {
    Report {
        #[arg(default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
}
#[derive(Debug, Args)]
struct ValidateArgs {
    #[command(subcommand)]
    command: ValidateCommand,
}
#[derive(Debug, Subcommand)]
enum ValidateCommand {
    Workspace(ValidateOptions),
    Change(ValidateOptions),
    Plan(ValidateOptions),
    Result(ValidateResultOptions),
}
#[derive(Debug, Args)]
struct ValidateOptions {
    #[arg(default_value = ".")]
    workspace: PathBuf,
    #[arg(long)]
    range: Option<String>,
    #[arg(long)]
    baseline: Option<String>,
    #[arg(long)]
    staged: bool,
    #[arg(long)]
    plan: Option<PathBuf>,
    #[arg(long)]
    plan_digest: Option<String>,
    #[arg(long)]
    slice_id: Option<String>,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
}
#[derive(Debug, Args)]
struct ValidateResultOptions {
    #[command(flatten)]
    validate: ValidateOptions,
    #[arg(long)]
    attempt_id: String,
    #[arg(long)]
    receipt: PathBuf,
}
#[derive(Debug, Args)]
struct WorkArgs {
    #[command(subcommand)]
    command: WorkCommand,
}
#[derive(Debug, Args)]
struct TaskArgs {
    #[command(subcommand)]
    command: TaskCommand,
}
#[derive(Debug, Subcommand)]
enum TaskCommand {
    Approve {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    Verify {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    Attempts {
        #[command(subcommand)]
        command: AttemptCommand,
    },
    Finalize {
        #[command(subcommand)]
        command: FinalizeCommand,
    },
}
#[derive(Debug, Subcommand)]
enum AttemptCommand {
    List {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        plan_digest: Option<String>,
        #[arg(long)]
        slice_id: Option<String>,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    Show {
        attempt_id: String,
        #[arg(long)]
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: Format,
    },
}
#[derive(Debug, Subcommand)]
enum FinalizeCommand {
    Preview {
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: Format,
    },
    Apply {
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
        #[arg(long)]
        preview_token: String,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value = "json")]
        format: Format,
    },
}
#[derive(Debug, Args)]
#[command(
    about = "Run the Workbench server or print its canonical projection",
    after_help = "Serve options: --bind <IP> --port <PORT> --allow-remote-bind --show-log"
)]
struct WorkbenchArgs {
    #[command(subcommand)]
    command: Option<WorkbenchCommand>,
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
    session_token: Option<String>,
    #[arg(long)]
    no_open: bool,
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
        plan_digest: String,
        #[arg(long)]
        slice_id: String,
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
        session_token: Option<String>,
        #[arg(long)]
        show_log: bool,
        #[arg(long)]
        no_open: bool,
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
        CommandKind::Readiness(args) => run_readiness(args),
        CommandKind::Work(args) => run_work(args),
        CommandKind::Task(args) => run_task(args),
        CommandKind::Workbench(args) => run_workbench(args),
        CommandKind::Lsp => {
            lsp::run_lsp_server()?;
            Ok(0)
        }
    }
}
fn run_readiness(args: ReadinessArgs) -> Result<i32> {
    let ReadinessCommand::Report { workspace, format } = args.command;
    let workspace = SpecWorkspace::load(workspace)?;
    let index = workspace.index()?;
    let profile = workspace
        .config
        .inventory
        .profiles
        .iter()
        .find(|profile| profile.id == workspace.config.inventory.active_profile)
        .context("active inventory profile is not defined")?;
    let inventory_result = InventoryRegistry::discover(
        &InventoryContext {
            workspace_root: workspace.root.clone(),
            profile: profile.id.clone(),
            settings: serde_yaml::Value::Null,
            excludes: workspace
                .config
                .workspace
                .excludes
                .iter()
                .map(|pattern| pattern.0.clone())
                .collect(),
            overlays: std::collections::BTreeMap::new(),
        },
        profile,
    );
    if let Err(error) = inventory_result {
        bail!("inventory readiness failed: {error}");
    }
    let report =
        syu_validation::evaluate_readiness(&workspace, &index, &revision(&workspace.root)?, true)?;
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Text => println!(
            "Readiness target: {}\nInventory: {}/{}\nWorkability: {}/{}",
            report.target,
            report.inventory.ready,
            report.inventory.required,
            report.workability.ready,
            report.workability.required
        ),
    }
    Ok(if report.meets_configured(&workspace.config) {
        0
    } else {
        1
    })
}
fn run_validate(args: ValidateArgs) -> Result<i32> {
    if let ValidateCommand::Result(result) = args.command {
        return run_validate_result(result);
    }
    let (is_change, force_post_state, enforce_readiness, args) = match args.command {
        ValidateCommand::Workspace(args) => (false, false, true, args),
        ValidateCommand::Change(args) => (true, false, false, args),
        ValidateCommand::Plan(args) => (false, false, false, args),
        ValidateCommand::Result(_) => unreachable!("result validation handled above"),
    };
    if args.staged {
        if !is_change {
            bail!("--staged is only supported by validate change");
        }
        if args.range.is_some() || args.baseline.is_some() {
            bail!("--staged cannot be combined with --range or --baseline");
        }
        if args.plan.is_some() || args.slice_id.is_some() {
            bail!("--staged cannot be combined with --plan or --slice-id");
        }
    }
    let staged_snapshot = args
        .staged
        .then(|| staged_workspace_snapshot(&args.workspace))
        .transpose()?;
    let workspace_path = staged_snapshot
        .as_ref()
        .map(|(_, path)| path)
        .unwrap_or(&args.workspace);
    let workspace = SpecWorkspace::load(workspace_path)?;
    let index = workspace.index()?;
    let plan = args
        .plan
        .as_ref()
        .map(|path| read_yaml::<WorkPlan>(path))
        .transpose()?;
    match (&plan, &args.plan_digest) {
        (Some(plan), Some(digest)) if plan.canonical_digest == *digest => {}
        (Some(_), Some(_)) => bail!("submitted plan does not match --plan-digest"),
        (Some(_), None) => bail!("--plan requires --plan-digest"),
        (None, Some(_)) => bail!("--plan-digest requires --plan"),
        (None, None) => {}
    }
    // Every validation invocation gets one explicit revision. Changed-unit
    // probes use the same resolved baseline plus staged, working-tree, and
    // untracked changes below.
    let revision = Some(revision(&workspace.root)?);
    let selected = match (&plan, &args.slice_id) {
        (Some(p), Some(id)) => Some(
            p.slices
                .iter()
                .find(|s| &s.id == id)
                .with_context(|| format!("slice {id} not found"))?,
        ),
        (None, Some(_)) => bail!("--slice-id requires --plan"),
        (Some(_), None) => bail!("--plan requires --slice-id"),
        (None, None) => None,
    };
    let mut validation_inputs = validation_inputs_for_cli(&workspace, &args, plan.as_ref())?;
    if force_post_state {
        if plan.as_ref().is_some_and(|plan| plan.slices.len() > 1) && args.slice_id.is_none() {
            bail!("validate result requires the receipt-selected slice for a multi-slice plan");
        }
        validation_inputs.plan_mode = PlanValidationMode::PostState;
        // Result validation is the selected receipt slice's post-state
        // closure. Always inspect the real diff from the plan basis so an
        // omitted file cannot bypass scope validation.
        validation_inputs.changed_files = Some(changed_files_against_revision(
            &workspace.root,
            &plan
                .as_ref()
                .expect("result validation requires a plan")
                .basis
                .revision,
        )?);
        validation_inputs.reported_changed_files = None;
        validation_inputs.change_base_revision = Some(
            plan.as_ref()
                .expect("result validation requires a plan")
                .basis
                .revision
                .clone(),
        );
    }
    let context = ValidationContext {
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
    };
    let result = if enforce_readiness {
        syu_validation::validate_workspace(&context)
    } else {
        validate(&context)
    };
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

fn run_validate_result(args: ValidateResultOptions) -> Result<i32> {
    let plan_path = args
        .validate
        .plan
        .as_ref()
        .context("validate result requires --plan")?;
    let submitted_plan: WorkPlan = read_yaml(plan_path)?;
    let plan_digest = args
        .validate
        .plan_digest
        .as_deref()
        .context("validate result requires --plan-digest")?;
    let slice_id = args
        .validate
        .slice_id
        .as_deref()
        .context("validate result requires --slice-id")?;
    if submitted_plan.canonical_digest != plan_digest {
        bail!("submitted plan does not match --plan-digest");
    }
    let workspace = SpecWorkspace::load(&args.validate.workspace)?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let identity = ExecutionIdentity {
        plan_digest: plan_digest.to_owned(),
        slice_id: slice_id.to_owned(),
    };
    let attempt = store.attempt(&identity, &args.attempt_id)?;
    if attempt.attempt_id != args.attempt_id
        || attempt.report.attempt_id != args.attempt_id
        || attempt.report.status != CompletionStatus::Complete
    {
        bail!("attempt is not a complete durable verification attempt");
    }
    let approval = store.approval(&identity)?;
    if approval.plan != submitted_plan {
        bail!("submitted plan differs from the approved durable plan");
    }
    let receipt: syu_work_model::VerificationReceipt = read_yaml(&args.receipt)?;
    let canonical = attempt
        .receipt
        .as_ref()
        .context("complete attempt has no durable verification receipt")?;
    if receipt != *canonical
        || receipt.plan_digest != plan_digest
        || receipt.slice_id != slice_id
        || attempt.report.receipt_digest.as_deref()
            != Some(DeliveryStore::verification_digest(canonical)?.as_str())
    {
        bail!("receipt does not match the exact durable verification attempt");
    }
    let index = workspace.index()?;
    let report = syu_validation::evaluate_completion(&workspace, &index, &approval.plan, &receipt)?;
    match args.validate.format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Text => {
            println!(
                "Completion: {:?}\nPlan: {}\nSlice: {}",
                report.status, report.plan_digest, report.slice_id
            );
            if report.demonstrated.is_empty() {
                println!("Demonstrated criteria: none");
            } else {
                println!("Demonstrated criteria:");
                for criterion in &report.demonstrated {
                    println!("- {}: {}", criterion.anchor, criterion.statement);
                }
            }
            println!("Completion checks: {}", report.checks.len());
            for blocker in &report.blockers {
                println!(
                    "BLOCKED {}: {}\n  Next action: {}",
                    blocker.code, blocker.message, blocker.next_action
                );
            }
        }
    }
    Ok(if report.status == CompletionStatus::Complete {
        0
    } else {
        1
    })
}

fn run_task(args: TaskArgs) -> Result<i32> {
    match args.command {
        TaskCommand::Approve {
            plan: plan_path,
            plan_digest,
            slice_id,
            workspace: root,
            format,
        } => {
            let workspace = SpecWorkspace::load(&root)?;
            let store = DeliveryStore::for_workspace(&workspace.root)?;
            let _workspace_lock = store.lock_workspace()?;
            let index = workspace.index()?;
            let revision = revision(&workspace.root)?;
            let submitted: WorkPlan = read_yaml(&plan_path)?;
            let canonical = syu_validation::canonical_plan_for_execution(
                &workspace, &index, &submitted, &revision,
            )?;
            if !matches!(canonical.status, PlanStatus::Ready) {
                bail!("only a ready canonical plan can be approved");
            }
            if canonical.canonical_digest != plan_digest
                || canonical.slices.len() != 1
                || canonical.slices[0].id != slice_id
            {
                bail!("approval requires the exact --plan-digest and --slice-id pair");
            }
            let approval = PlanApproval {
                schema: PLAN_APPROVAL_SCHEMA.into(),
                approval_id: store.new_id("approval"),
                plan_digest: canonical.canonical_digest.clone(),
                slice_id,
                workspace_fingerprint: workspace.try_fingerprint()?,
                revision,
                reviewed_at: now_timestamp(),
                plan: canonical,
            };
            let approval = store.approve_while_locked(&approval)?;
            emit_task_value(&approval, format, "approved plan")?;
            Ok(0)
        }
        TaskCommand::Verify {
            plan: plan_path,
            plan_digest,
            slice_id,
            workspace: root,
            format,
        } => {
            let workspace = SpecWorkspace::load(&root)?;
            let plan: WorkPlan = read_yaml(&plan_path)?;
            let store = DeliveryStore::for_workspace(&workspace.root)?;
            if plan.canonical_digest != plan_digest {
                bail!("submitted plan does not match --plan-digest");
            }
            let approval = store
                .approval(&ExecutionIdentity {
                    plan_digest: plan.canonical_digest.clone(),
                    slice_id: slice_id.clone(),
                })
                .context("plan has not been explicitly approved; run syu task approve first")?;
            if approval.plan != plan {
                bail!(
                    "submitted plan differs from the approved plan; approve the exact plan again"
                );
            }
            let slice_value = plan
                .slices
                .iter()
                .find(|value| value.id == slice_id)
                .with_context(|| format!("slice {slice_id} not found"))?;
            if slice_value.verification_targets.is_empty() {
                bail!("selected slice has no verification targets");
            }
            let attempt = store.execute_and_append_attempt(&workspace, &plan, &slice_id)?;
            emit_task_value(&attempt, format, "completion attempt")?;
            Ok(
                if matches!(
                    attempt.verification.status,
                    VerificationAttemptStatus::Complete
                ) && attempt.report.status == CompletionStatus::Complete
                {
                    0
                } else {
                    1
                },
            )
        }
        TaskCommand::Attempts { command } => {
            let root = match &command {
                AttemptCommand::List { workspace, .. } | AttemptCommand::Show { workspace, .. } => {
                    workspace
                }
            };
            let store = DeliveryStore::for_workspace(&SpecWorkspace::load(root)?.root)?;
            match command {
                AttemptCommand::List {
                    plan_digest,
                    slice_id,
                    format,
                    ..
                } => {
                    let attempts = store
                        .attempts()?
                        .into_iter()
                        .filter(|attempt| {
                            plan_digest
                                .as_ref()
                                .is_none_or(|digest| &attempt.plan_digest == digest)
                                && slice_id.as_ref().is_none_or(|id| &attempt.slice_id == id)
                        })
                        .collect::<Vec<_>>();
                    emit_task_value(&attempts, format, "completion attempts")?;
                    Ok(0)
                }
                AttemptCommand::Show {
                    attempt_id,
                    plan_digest,
                    slice_id,
                    format,
                    ..
                } => {
                    let attempt = store.attempt(
                        &ExecutionIdentity {
                            plan_digest,
                            slice_id,
                        },
                        &attempt_id,
                    )?;
                    emit_task_value(&attempt, format, "completion attempt")?;
                    Ok(0)
                }
            }
        }
        TaskCommand::Finalize { command } => {
            let (attempt_id, plan_digest, slice_id, root, format, token) = match command {
                FinalizeCommand::Preview {
                    attempt,
                    plan_digest,
                    slice_id,
                    workspace,
                    format,
                } => (attempt, plan_digest, slice_id, workspace, format, None),
                FinalizeCommand::Apply {
                    attempt,
                    plan_digest,
                    slice_id,
                    preview_token,
                    workspace,
                    format,
                } => (
                    attempt,
                    plan_digest,
                    slice_id,
                    workspace,
                    format,
                    Some(preview_token),
                ),
            };
            let workspace = SpecWorkspace::load(&root)?;
            let store = DeliveryStore::for_workspace(&workspace.root)?;
            let identity = ExecutionIdentity {
                plan_digest,
                slice_id,
            };
            let attempt = store.attempt(&identity, &attempt_id)?;
            let preview = store.finalization_preview(&workspace, &attempt)?;
            if let Some(token) = token {
                let receipt = store.apply_finalization(&workspace, &attempt, &preview, &token)?;
                emit_task_value(&receipt, format, "finalization receipt")?;
                Ok(0)
            } else {
                let status = preview.status;
                emit_task_value(&preview, format, "finalization preview")?;
                Ok(if status == CompletionStatus::Complete {
                    0
                } else {
                    1
                })
            }
        }
    }
}

fn emit_task_value<T: serde::Serialize>(value: &T, format: Format, label: &str) -> Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(value)?),
        Format::Text => println!("{label}:\n{}", serde_yaml::to_string(value)?),
    }
    Ok(())
}

fn now_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
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
            validate_work_origin(&index, &request.origin)
                .context("work plan requires an exact implemented Work origin")?;
            validate_work_request(&index, &request)
                .context("work plan contains an out-of-scope requested target")?;
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
            plan_digest,
            slice_id,
            workspace,
            out,
        } => {
            let workspace = SpecWorkspace::load(workspace)?;
            let index = workspace.index()?;
            let plan: WorkPlan = read_yaml(&plan)?;
            if plan.canonical_digest != plan_digest {
                bail!("submitted plan does not match --plan-digest");
            }
            plan.slices
                .iter()
                .find(|s| s.id == slice_id)
                .with_context(|| format!("slice {slice_id} not found"))?;
            let current_revision = revision(&workspace.root)?;
            let context = export_context(&plan, &slice_id, &workspace, &index, &current_revision)?;
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
    let command = args.command.unwrap_or(WorkbenchCommand::Serve {
        workspace: args.workspace,
        request: args.request,
        bind: args.bind,
        port: args.port,
        allow_remote_bind: args.allow_remote_bind,
        session_token: args.session_token,
        show_log: false,
        no_open: args.no_open,
    });
    match command {
        WorkbenchCommand::Project {
            workspace,
            request,
            format,
        } => {
            let workspace = SpecWorkspace::load(workspace)?;
            let index = workspace.index()?;
            let request = request
                .as_ref()
                .map(|path| read_yaml::<WorkRequest>(path))
                .transpose()?;
            if let Some(request) = &request {
                validate_work_request(&index, request)
                    .context("workbench projection request is outside its exact origin")?;
            }
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
            session_token,
            show_log,
            no_open,
        } => {
            if !bind.is_loopback() && !allow_remote_bind {
                bail!("remote --bind requires --allow-remote-bind");
            }
            if !bind.is_loopback() && session_token.as_deref().is_none_or(str::is_empty) {
                bail!("remote --bind requires --session-token");
            }
            let workspace = SpecWorkspace::load(workspace)?;
            let request = request
                .as_ref()
                .map(|path| read_yaml::<WorkRequest>(path))
                .transpose()?;
            if show_log {
                println!("Workbench request logging is handled by the server runtime");
            }
            let server = syu_workbench_server::WorkbenchServer::new(workspace.root.clone())
                .with_launch(syu_workbench_server::WorkbenchLaunchConfig {
                    workspace_root: workspace.root,
                    bind,
                    port,
                    session_token,
                    no_open,
                });
            if let Some(request) = request {
                server.with_request(request)?.run()?;
            } else {
                server.run()?;
            }
            Ok(0)
        }
    }
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
    args: &ValidateOptions,
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
    let baseline = default_change_baseline(workspace)?;
    Ok(Some(changed_files(&workspace.root, &baseline)?))
}

fn default_change_baseline(workspace: &SpecWorkspace) -> Result<String> {
    if let Some(baseline) = &workspace.config.validation.changed.baseline {
        return base_revision_from_baseline(workspace, baseline);
    }
    merge_base(&workspace.root, "origin/main")
        .or_else(|_| revision_parent(&workspace.root))
        .or_else(|_| revision(&workspace.root))
}

fn validation_inputs_for_cli(
    workspace: &SpecWorkspace,
    args: &ValidateOptions,
    plan: Option<&WorkPlan>,
) -> Result<ValidationInputs> {
    if args.staged {
        return Ok(ValidationInputs {
            // Pre-commit needs to validate exactly what is staged without
            // turning unrelated staged implementation changes into a change
            // ownership review. The snapshot supplies the structural inputs;
            // pre-push and CI retain the diff-aware validation paths.
            changed_files: None,
            reported_changed_files: None,
            change_base_revision: None,
            plan_mode: PlanValidationMode::PreState,
        });
    }
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
        Some(default_change_baseline(workspace)?)
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
        ChangeBaseline::MergeBase { against } => Ok(merge_base(root, &against.0)
            .or_else(|_| revision_parent(root))
            .or_else(|_| revision(root))?),
        ChangeBaseline::Revision { revision } => Ok(revision.0.clone()),
        ChangeBaseline::Parent => Ok("HEAD^".into()),
    }
}

fn base_revision_from_baseline(
    workspace: &SpecWorkspace,
    baseline: &ChangeBaseline,
) -> Result<String> {
    match baseline {
        ChangeBaseline::MergeBase { against } => merge_base(&workspace.root, &against.0)
            .or_else(|_| revision_parent(&workspace.root))
            .or_else(|_| revision(&workspace.root)),
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

fn staged_workspace_snapshot(workspace: &Path) -> Result<(tempfile::TempDir, PathBuf)> {
    let workspace = workspace
        .canonicalize()
        .with_context(|| format!("resolve workspace path {}", workspace.display()))?;
    let repo_root_output = Command::new("git")
        .args(["-C"])
        .arg(&workspace)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !repo_root_output.status.success() {
        bail!("git rev-parse --show-toplevel failed");
    }
    let repo_root = PathBuf::from(String::from_utf8(repo_root_output.stdout)?.trim());
    let relative_workspace = workspace
        .strip_prefix(&repo_root)
        .with_context(|| format!("workspace {} is outside repository", workspace.display()))?;
    let git_dir_output = Command::new("git")
        .args(["-C"])
        .arg(&repo_root)
        .args(["rev-parse", "--absolute-git-dir"])
        .output()?;
    if !git_dir_output.status.success() {
        bail!("git rev-parse --absolute-git-dir failed");
    }
    let snapshot = tempfile::tempdir()?;
    let checkout_status = Command::new("git")
        .args(["-C"])
        .arg(&repo_root)
        .args(["checkout-index", "--all", "--prefix"])
        .arg(format!("{}/", snapshot.path().display()))
        .status()?;
    if !checkout_status.success() {
        bail!("git checkout-index failed");
    }
    fs::write(
        snapshot.path().join(".git"),
        format!(
            "gitdir: {}\n",
            String::from_utf8(git_dir_output.stdout)?.trim()
        ),
    )?;
    let snapshot_workspace = snapshot.path().join(relative_workspace);
    Ok((snapshot, snapshot_workspace))
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
