// FEAT-APP-001
// FEAT-BROWSE-001
// FEAT-REPORT-001
// REQ-CORE-001

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
    #[command(about = "Start a local browser app for exploring the current workspace")]
    App(AppArgs),
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
pub struct AppArgs {
    #[arg(help = "Workspace root containing syu.yaml and the spec tree (default: docs/syu)")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "IP address to bind the local app server to (default: app.bind or 127.0.0.1)")]
    #[arg(long)]
    pub bind: Option<String>,

    #[arg(help = "Port to bind the local app server to (default: app.port or 3000)")]
    #[arg(short, long)]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Args)]
pub struct ValidateArgs {
    #[arg(help = "Workspace root containing syu.yaml and the spec tree (default: docs/syu)")]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Output format for validation results")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    #[arg(help = "Filter visible diagnostics by severity (repeat or use commas)")]
    #[arg(long, value_enum, value_delimiter = ',')]
    pub severity: Vec<ValidationSeverityFilter>,

    #[arg(help = "Filter visible diagnostics by rule genre (repeat or use commas)")]
    #[arg(long, value_enum, value_delimiter = ',')]
    pub genre: Vec<ValidationGenreFilter>,

    #[arg(help = "Filter visible diagnostics by exact rule code (repeat or use commas)")]
    #[arg(long, value_delimiter = ',')]
    pub rule: Vec<String>,

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ValidationSeverityFilter {
    Error,
    Warning,
}

impl ValidationSeverityFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ValidationGenreFilter {
    Workspace,
    Graph,
    Delivery,
    Trace,
    Coverage,
}

impl ValidationGenreFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Graph => "graph",
            Self::Delivery => "delivery",
            Self::Trace => "trace",
            Self::Coverage => "coverage",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ValidationGenreFilter, ValidationSeverityFilter};

    #[test]
    fn validation_filter_enums_expose_expected_labels() {
        assert_eq!(ValidationSeverityFilter::Error.as_str(), "error");
        assert_eq!(ValidationSeverityFilter::Warning.as_str(), "warning");
        assert_eq!(ValidationGenreFilter::Workspace.as_str(), "workspace");
        assert_eq!(ValidationGenreFilter::Graph.as_str(), "graph");
        assert_eq!(ValidationGenreFilter::Delivery.as_str(), "delivery");
        assert_eq!(ValidationGenreFilter::Trace.as_str(), "trace");
        assert_eq!(ValidationGenreFilter::Coverage.as_str(), "coverage");
    }
}
