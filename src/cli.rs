// FEAT-DOCS-001
// FEAT-APP-001
// FEAT-BROWSE-001
// FEAT-BROWSE-002
// FEAT-LIST-002
// FEAT-REPORT-001
// FEAT-INIT-002
// REQ-CORE-001

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, builder::BoolishValueParser};
use std::path::PathBuf;

const ROOT_AFTER_HELP: &str = "\
New here?
  1. syu init .      scaffold a workspace in the current directory
  2. syu validate .  check the layered spec and traceability
  3. syu browse .    explore the spec in your terminal
  4. syu app .       start the local browser UI server";

const APP_AFTER_HELP: &str = concat!(
    "After startup, open the printed URL in your browser.\n",
    "Use GET /health for readiness checks once the app is serving.\n",
    "Press Ctrl-C to stop the local app server."
);

const WORKSPACE_HELP: &str = "Workspace root containing syu.yaml and the configured spec tree";

const LIST_AFTER_HELP: &str = "\
Examples:
  syu list
  syu list docs/syu
  syu list requirement
  syu list requirement docs/syu
  syu list docs/syu requirement";

#[derive(Debug, Parser)]
#[command(
    name = "syu",
    version,
    about = "Specification-driven development for real repositories",
    after_help = ROOT_AFTER_HELP,
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
        about = "Browse the specification in your terminal (interactive prompts or text output)"
    )]
    Browse(BrowseArgs),
    #[command(
        about = "List philosophies, policies, requirements, or features",
        after_help = LIST_AFTER_HELP
    )]
    List(ListArgs),
    #[command(about = "Show one philosophy, policy, requirement, or feature by ID")]
    Show(ShowArgs),
    #[command(
        about = "Start a local HTTP server and browser UI for workspace exploration, then print the URL to open in your browser",
        after_help = APP_AFTER_HELP
    )]
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
    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    /// Print the spec tree to stdout and exit without entering interactive mode.
    /// Useful in CI pipelines and scripts.
    #[arg(long, action = ArgAction::SetTrue)]
    pub non_interactive: bool,
}

impl Default for BrowseArgs {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
            non_interactive: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LookupKind {
    #[value(alias = "philosophies")]
    Philosophy,
    #[value(alias = "policies")]
    Policy,
    #[value(alias = "requirements")]
    Requirement,
    #[value(alias = "features")]
    Feature,
}

impl LookupKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Philosophy => "philosophy",
            Self::Policy => "policy",
            Self::Requirement => "requirement",
            Self::Feature => "feature",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    /// Optional layer kind and workspace path.
    /// Accepted as either `syu list [KIND] [WORKSPACE]` or `syu list [WORKSPACE] [KIND]`.
    /// With one positional argument, syu treats known kinds as layer filters and everything else as a workspace path.
    #[arg(
        num_args = 0..=2,
        value_name = "KIND_OR_WORKSPACE",
        help = "Optional kind (philosophy|policy|requirement|feature) and/or workspace path"
    )]
    pub positional: Vec<String>,

    #[arg(help = "Output format for listed items")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    #[arg(help = "Include each item's checked-in YAML document path")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub with_path: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ShowArgs {
    #[arg(help = "Definition ID to show (for example PHIL-001 or REQ-CORE-001)")]
    pub id: String,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Output format for the selected item")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct AppArgs {
    #[arg(help = WORKSPACE_HELP)]
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
    #[arg(help = WORKSPACE_HELP)]
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

    #[arg(help = "Filter visible diagnostics to one or more spec item IDs (repeat or use commas)")]
    #[arg(long, value_delimiter = ',')]
    pub id: Vec<String>,

    #[arg(help = "Apply conservative documentation autofixes")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub fix: bool,

    #[arg(help = "Disable autofix even when syu.yaml enables it by default")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_fix: bool,

    #[arg(
        long,
        value_name = "BOOL",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        help = "Temporarily override validate.allow_planned for this run"
    )]
    pub allow_planned: Option<bool>,

    #[arg(
        long,
        value_name = "BOOL",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        help = "Temporarily override validate.require_non_orphaned_items for this run"
    )]
    pub require_non_orphaned_items: Option<bool>,

    #[arg(
        long,
        value_name = "BOOL",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        help = "Temporarily override validate.require_reciprocal_links for this run"
    )]
    pub require_reciprocal_links: Option<bool>,

    #[arg(
        long,
        value_name = "BOOL",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        help = "Temporarily override validate.require_symbol_trace_coverage for this run"
    )]
    pub require_symbol_trace_coverage: Option<bool>,

    #[arg(help = "Suppress next-step guidance in successful text output")]
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub quiet: bool,
}

pub type CheckArgs = ValidateArgs;

#[derive(Debug, Clone, Args)]
pub struct ReportArgs {
    #[arg(help = WORKSPACE_HELP)]
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

    #[arg(help = "Output format (text shows next-step guidance; json returns created file paths)")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
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
    use clap::Parser;

    use super::{Cli, LookupKind, ValidationGenreFilter, ValidationSeverityFilter};

    #[test]
    fn cli_enums_expose_expected_labels() {
        assert_eq!(LookupKind::Philosophy.label(), "philosophy");
        assert_eq!(LookupKind::Policy.label(), "policy");
        assert_eq!(LookupKind::Requirement.label(), "requirement");
        assert_eq!(LookupKind::Feature.label(), "feature");
        assert_eq!(ValidationSeverityFilter::Error.as_str(), "error");
        assert_eq!(ValidationSeverityFilter::Warning.as_str(), "warning");
        assert_eq!(ValidationGenreFilter::Workspace.as_str(), "workspace");
        assert_eq!(ValidationGenreFilter::Graph.as_str(), "graph");
        assert_eq!(ValidationGenreFilter::Delivery.as_str(), "delivery");
        assert_eq!(ValidationGenreFilter::Trace.as_str(), "trace");
        assert_eq!(ValidationGenreFilter::Coverage.as_str(), "coverage");
    }

    #[test]
    fn validate_args_accept_temporary_boolean_overrides() {
        let cli = Cli::try_parse_from([
            "syu",
            "validate",
            ".",
            "--allow-planned=false",
            "--require-non-orphaned-items=false",
            "--require-reciprocal-links",
            "--require-symbol-trace-coverage",
        ])
        .expect("validate args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Validate("));
        assert!(rendered.contains("allow_planned: Some(false)"));
        assert!(rendered.contains("require_non_orphaned_items: Some(false)"));
        assert!(rendered.contains("require_reciprocal_links: Some(true)"));
        assert!(rendered.contains("require_symbol_trace_coverage: Some(true)"));
    }

    #[test]
    fn validate_args_reject_non_boolean_override_values() {
        let error = Cli::try_parse_from(["syu", "validate", ".", "--allow-planned=maybe"])
            .expect_err("non-boolean overrides should fail");

        let message = error.to_string();
        assert!(message.contains("--allow-planned"));
    }
}
