#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use syu_planner::{export_context, plan};
use syu_project_model::{ChangeBaseline, GitRef};
use syu_spec_model::RepoPath;
use syu_validation::{ChangeStatus, ChangedFile, ChangedRange, ValidationContext, validate};
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
    let needs_revision = args.range.is_some()
        || args.baseline.is_some()
        || workspace.config.validation.changed.baseline.is_some()
        || args.plan.is_some();
    let revision = needs_revision
        .then(|| revision(&workspace.root))
        .transpose()?;
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
    let changed = changed_files_for_validation(&workspace, &args)?;
    let result = validate(&ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: changed.as_deref(),
        work_plan: plan.as_ref(),
        selected_slice: selected,
        preset: workspace.config.validation.preset,
        revision: revision.as_deref(),
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

fn merge_base(root: &Path, against: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["merge-base", "HEAD", against])
        .output()?;
    if !output.status.success() {
        bail!("git merge-base HEAD {against} failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn changed_files(root: &Path, range: &str) -> Result<Vec<ChangedFile>> {
    let status_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", "-M", "--relative", range])
        .output()?;
    if !status_output.status.success() {
        bail!("git diff --name-status {range} failed");
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
    parse_changed_files(
        &String::from_utf8(status_output.stdout)?,
        &String::from_utf8(patch_output.stdout)?,
    )
}

fn parse_changed_files(status: &str, patch: &str) -> Result<Vec<ChangedFile>> {
    let mut files = Vec::new();
    for line in status.lines().filter(|line| !line.trim().is_empty()) {
        let parts = line.split('\t').collect::<Vec<_>>();
        let kind = parts.first().copied().unwrap_or_default();
        let (status, old_path, new_path) = match kind.chars().next().unwrap_or('M') {
            'A' => (ChangeStatus::Added, None, Some(parts[1])),
            'M' => (ChangeStatus::Modified, None, Some(parts[1])),
            'D' => (ChangeStatus::Deleted, Some(parts[1]), None),
            'R' => (ChangeStatus::Renamed, Some(parts[1]), Some(parts[2])),
            _ => (ChangeStatus::Modified, None, parts.get(1).copied()),
        };
        files.push(ChangedFile {
            status,
            old_path: old_path
                .map(|value| RepoPath::new(value.to_string()))
                .transpose()
                .map_err(anyhow::Error::msg)?,
            new_path: new_path
                .map(|value| RepoPath::new(value.to_string()))
                .transpose()
                .map_err(anyhow::Error::msg)?,
            hunks: Vec::new(),
        });
    }
    let mut current: Option<usize> = None;
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current = files.iter().position(|file| {
                file.new_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            });
            continue;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            if current.is_none() {
                current = files.iter().position(|file| {
                    file.old_path
                        .as_ref()
                        .is_some_and(|value| value.to_string_lossy() == path)
                });
            }
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
    let end = if len == 0 { start } else { start + len - 1 };
    Ok((start, end))
}
