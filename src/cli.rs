// FEAT-BROWSE-001

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "syu",
    version,
    about = "Specification-driven development for real repositories",
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(
        about = "Browse philosophies, policies, features, requirements, and validation errors"
    )]
    Browse(BrowseArgs),
    #[command(
        visible_alias = "check",
        about = "Validate the layered graph, traces, and optional autofixes"
    )]
    Validate(ValidateArgs),
    #[command(about = "Render a Markdown report from the same validation engine")]
    Report(ReportArgs),
    #[command(about = "Scaffold a version-matched syu workspace")]
    Init(InitArgs),
}

#[derive(Debug, Clone, Args)]
pub struct BrowseArgs {
    #[arg(help = "Workspace root containing syu.yaml and the spec tree (default: docs/syu)")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,
}

impl Default for BrowseArgs {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct ValidateArgs {
    #[arg(help = "Workspace root containing syu.yaml and the spec tree (default: docs/syu)")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Output format for validation results")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    #[arg(help = "Apply conservative documentation autofixes")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub fix: bool,

    #[arg(help = "Disable autofix even when syu.yaml enables it by default")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_fix: bool,
}

pub type CheckArgs = ValidateArgs;

#[derive(Debug, Clone, Args)]
pub struct ReportArgs {
    #[arg(help = "Workspace root containing syu.yaml and the spec tree (default: docs/syu)")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Write the Markdown report to a file instead of stdout")]
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct InitArgs {
    #[arg(help = "Directory to bootstrap")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Override the inferred project name")]
    #[arg(long)]
    pub name: Option<String>,

    #[arg(help = "Overwrite generated files when they already exist")]
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}
