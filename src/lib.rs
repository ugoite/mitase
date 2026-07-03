#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use syu_planner::{export_context, plan};
use syu_validation::{ValidationContext, validate};
use syu_work_model::{WorkPlan, WorkRequest};
use syu_workspace::SpecWorkspace;

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
}
#[derive(Debug, Args)]
struct ValidateArgs {
    #[arg(default_value = ".")]
    workspace: PathBuf,
    #[arg(long)]
    range: Option<String>,
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
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

pub fn run() -> Result<i32> {
    match Cli::parse().command {
        CommandKind::Validate(args) => run_validate(args),
        CommandKind::Work(args) => run_work(args),
    }
}
fn run_validate(args: ValidateArgs) -> Result<i32> {
    let workspace = SpecWorkspace::load(&args.workspace)?;
    let index = workspace.index()?;
    let revision = revision(&workspace.root)?;
    let plan = args
        .plan
        .as_ref()
        .map(|path| read_yaml::<WorkPlan>(path))
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
    let changed = args
        .range
        .as_ref()
        .map(|r| changed_paths(&workspace.root, r))
        .transpose()?;
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_paths: changed.as_deref(),
        work_plan: plan.as_ref(),
        selected_slice: selected,
        preset: workspace.config.validation.preset,
        revision: &revision,
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
            let context = export_context(&plan, &slice, &workspace, &index)?;
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
fn changed_paths(root: &Path, range: &str) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", "-M", "--relative", range])
        .output()?;
    if !output.status.success() {
        bail!("git diff --name-only {range} failed");
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| line.split('\t').next_back())
        .map(PathBuf::from)
        .collect())
}
