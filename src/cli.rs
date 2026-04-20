// FEAT-DOCS-001
// FEAT-APP-001
// FEAT-ADD-001
// FEAT-LOG-001
// FEAT-RELATE-001
// FEAT-SEARCH-001
// FEAT-BROWSE-001
// FEAT-BROWSE-002
// FEAT-INIT-007
// FEAT-INIT-006
// FEAT-INIT-005
// FEAT-INIT-004
// FEAT-INIT-003
// FEAT-LIST-002
// FEAT-TRACE-001
// FEAT-REPORT-001
// FEAT-INIT-002
// REQ-CORE-001

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, builder::BoolishValueParser};
use std::{num::NonZeroU8, path::PathBuf};

const ROOT_AFTER_HELP: &str = "\
New here?
  1. syu templates   compare starter layouts before you scaffold
  2. syu init .      scaffold a workspace in the current directory
  3. syu validate .  check the layered spec and traceability
  4. syu browse .    explore the spec in your terminal
  5. syu app .       start the local browser UI server";

const APP_AFTER_HELP: &str = concat!(
    "After startup, open the printed URL in your browser.\n",
    "Use GET /health for readiness checks once the app is serving.\n",
    "Press Ctrl-C to stop the local app server."
);

const INIT_AFTER_HELP: &str = "\
Use `syu templates` first when you want to compare starter layouts before you scaffold.

Examples:
  syu templates
  syu init .
  syu init . --interactive
  syu init . --id-prefix store
  syu init . --template rust-only
  syu init . --template ruby-only
  syu init . --template typescript-only
  syu init . --template go-only
  syu init . --template java-only
  syu init . --spec-root docs/spec
  syu init path/to/workspace --name my-project --spec-root spec/contracts --template polyglot --id-prefix store";

// FEAT-INIT-006
const TEMPLATES_AFTER_HELP: &str = "\
Use `syu templates` when you want a quick catalog of starter layouts before you scaffold.
Use `syu init --template ...` when you are ready to generate files.

Examples:
  syu templates
  syu templates --format json";

const ADD_AFTER_HELP: &str = "\
After writing a stub, `syu add` prints the reciprocal-link follow-up and matching scaffold suggestions needed before `syu validate` will pass cleanly.

Examples:
  syu add philosophy PHIL-002
  syu add policy POL-002 --file docs/syu/policies/policies.yaml
  syu add requirement REQ-AUTH-001
  syu add requirement --interactive
  syu add feature FEAT-AUTH-LOGIN-001 --kind auth
  syu add feature path/to/workspace --interactive
  syu add feature FEAT-AUTH-001 --kind auth --file docs/syu/features/auth/login.yaml";

const WORKSPACE_HELP: &str = "Workspace root or any child directory; syu walks upward to find syu.yaml and the configured spec tree";

const LIST_AFTER_HELP: &str = "\
Choose `syu list` when you want list-shaped output that can be narrowed to one layer or emitted as JSON for automation.
Choose `syu browse --non-interactive` when you want the browse snapshot instead: workspace metadata, per-layer counts, grouped items, and the current validation errors in text or JSON.

Examples:
  syu list
  syu list requirement
  syu list requirement --format json
  syu list path/to/workspace
  syu list requirement path/to/workspace
  syu list path/to/workspace requirement
  syu list requirement docs/syu
  syu list requirement docs/syu/features

Note:
  Pass the workspace root, the configured spec.root directory, or any child directory.
  syu walks upward until it finds syu.yaml, then resolves the configured spec.root from that workspace.";
const BROWSE_AFTER_HELP: &str = "\
Choose `syu browse --non-interactive` when you want the browse snapshot in text or JSON: workspace metadata, per-layer counts, grouped items, and the current validation errors.
Choose `syu list` when you want list-shaped output that can be narrowed to one layer.

Examples:
  syu browse .
  syu browse . --non-interactive
  syu browse . --non-interactive --format json
";

const SEARCH_AFTER_HELP: &str = "\
Examples:
  syu search audit
  syu search traceability --kind requirement
  syu search FEAT-CHECK-001 --format json";

const LOG_AFTER_HELP: &str = "\
Examples:
  syu log REQ-CORE-002
  syu log FEAT-CHECK-001 --kind implementation --path src/command
  syu log REQ-CORE-024 --include-related --merge-base-ref origin/main
  syu log REQ-CORE-019 --format json";

const RELATE_AFTER_HELP: &str = "\
Examples:
  syu relate REQ-CORE-018
  syu relate FEAT-SEARCH-001 --format json
  syu relate src/command/search.rs
  syu relate run_search_command
  syu relate --range main..HEAD
  syu relate --range origin/main...HEAD --format json";

const TRACE_AFTER_HELP: &str = "\
Examples:
  syu trace src/rust_feature.rs
  syu trace src/rust_feature.rs --symbol feature_trace_rust
  syu trace src/rust_feature.rs path/to/workspace --format json
  syu trace --range main..HEAD
  syu trace --range origin/main...HEAD --format json";

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
        about = "Browse the specification in your terminal (interactive prompts or text output)",
        after_help = BROWSE_AFTER_HELP
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
        about = "Search spec items by ID, title, summary, or description",
        after_help = SEARCH_AFTER_HELP
    )]
    Search(SearchArgs),
    #[command(
        about = "Show git history for one traced requirement or feature",
        after_help = LOG_AFTER_HELP
    )]
    Log(LogArgs),
    #[command(
        about = "Inspect the connected graph around one ID, path, or source symbol",
        after_help = RELATE_AFTER_HELP
    )]
    Relate(RelateArgs),
    #[command(
        about = "Resolve linked requirements, features, policies, and philosophies from a traced file or symbol",
        after_help = TRACE_AFTER_HELP
    )]
    Trace(TraceArgs),
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
    #[command(
        about = "Scaffold a version-matched syu workspace",
        after_help = INIT_AFTER_HELP
    )]
    Init(InitArgs),
    #[command(
        about = "List starter templates and related checked-in examples",
        after_help = TEMPLATES_AFTER_HELP
    )]
    Templates(TemplatesArgs),
    #[command(
        about = "Scaffold a new philosophy, policy, requirement, or feature stub",
        after_help = ADD_AFTER_HELP
    )]
    Add(AddArgs),
    #[command(about = "Start LSP server for editor integrations (JSON-RPC 2.0 over stdio)")]
    Lsp,
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

    #[arg(help = "Output format for non-interactive browse snapshots")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

impl Default for BrowseArgs {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
            non_interactive: false,
            format: OutputFormat::Text,
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
pub struct SearchArgs {
    #[arg(
        help = "Case-insensitive search query matched against IDs, titles, summaries, and descriptions"
    )]
    pub query: String,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Limit matches to one layer kind")]
    #[arg(long, value_enum)]
    pub kind: Option<LookupKind>,

    #[arg(help = "Output format for matched items")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum HistoryKind {
    All,
    #[value(alias = "spec")]
    Definition,
    #[value(alias = "tests")]
    Test,
    #[value(alias = "implementations")]
    Implementation,
}

impl HistoryKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Definition => "definition",
            Self::Test => "test",
            Self::Implementation => "implementation",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct LogArgs {
    #[arg(
        help = "Philosophy, policy, requirement, or feature ID whose traced history should be inspected"
    )]
    pub id: String,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Limit matches to definition, test, or implementation paths")]
    #[arg(long, value_enum, default_value_t = HistoryKind::All)]
    pub kind: HistoryKind,

    #[arg(help = "Limit traced paths to one repository-relative file or directory prefix")]
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    #[arg(help = "Also include related spec definitions and traces from `syu relate`")]
    #[arg(long)]
    pub include_related: bool,

    #[arg(help = "Limit history to commits after the merge-base between HEAD and the given ref")]
    #[arg(long, value_name = "REF", conflicts_with = "range")]
    pub merge_base_ref: Option<String>,

    #[arg(help = "Limit history to an explicit git revision range such as origin/main..HEAD")]
    #[arg(long, value_name = "RANGE", conflicts_with = "merge_base_ref")]
    pub range: Option<String>,

    #[arg(help = "Maximum number of matching commits to show")]
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(help = "Output format for matched history")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct TraceArgs {
    #[arg(help = "Repository-relative source or test file to resolve through trace ownership")]
    pub file: Option<PathBuf>,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Optional symbol name to resolve within the traced file")]
    #[arg(long)]
    pub symbol: Option<String>,

    #[arg(help = "Git range to analyze (e.g., main..HEAD or origin/main...HEAD)")]
    #[arg(long, conflicts_with = "file")]
    pub range: Option<String>,

    #[arg(help = "Output format for trace lookup results")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct RelateArgs {
    #[arg(help = "Definition ID, repository-relative path, or traced source symbol to inspect")]
    pub selector: Option<String>,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(help = "Git range to analyze (e.g., main..HEAD or origin/main...HEAD)")]
    #[arg(long, conflicts_with = "selector")]
    pub range: Option<String>,

    #[arg(help = "Output format for the related graph")]
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

    #[arg(help = "Allow syu app to bind to a non-loopback address such as 0.0.0.0")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub allow_remote: bool,
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

    #[arg(help = "Use this exit code when validation has warnings but no errors")]
    #[arg(long, value_name = "CODE")]
    pub warning_exit_code: Option<NonZeroU8>,

    #[arg(help = "Suppress the text summary and next-step guidance in successful text output")]
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

    #[arg(help = "Prompt for starter settings in a terminal before scaffolding files")]
    #[arg(long, action = ArgAction::SetTrue)]
    pub interactive: bool,

    #[arg(help = "Override the inferred project name")]
    #[arg(long)]
    pub name: Option<String>,

    #[arg(
        help = "Repository-relative spec.root to scaffold (for example docs/spec or spec/contracts)"
    )]
    #[arg(long)]
    pub spec_root: Option<PathBuf>,

    #[arg(
        help = "Starter layout to scaffold (generic, docs-first, rust-only, python-only, ruby-only, go-only, java-only, or polyglot)"
    )]
    #[arg(long, value_enum, default_value_t = StarterTemplate::Generic)]
    pub template: StarterTemplate,

    #[arg(
        help = "Shared ID stem to render as PHIL-<stem>, POL-<stem>, REQ-<stem>, and FEAT-<stem>"
    )]
    #[arg(long, value_name = "STEM")]
    pub id_prefix: Option<String>,

    #[arg(help = "Override the philosophy ID prefix (for example PHIL-STORE)")]
    #[arg(long, value_name = "PREFIX")]
    pub philosophy_prefix: Option<String>,

    #[arg(help = "Override the policy ID prefix (for example POL-STORE)")]
    #[arg(long, value_name = "PREFIX")]
    pub policy_prefix: Option<String>,

    #[arg(help = "Override the requirement ID prefix (for example REQ-STORE)")]
    #[arg(long, value_name = "PREFIX")]
    pub requirement_prefix: Option<String>,

    #[arg(help = "Override the feature ID prefix (for example FEAT-STORE)")]
    #[arg(long, value_name = "PREFIX")]
    pub feature_prefix: Option<String>,

    #[arg(help = "Overwrite generated files when they already exist")]
    #[arg(long)]
    pub force: bool,

    #[arg(
        help = "Output format (text shows next-step guidance; json returns created file paths; --interactive supports text only)"
    )]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct TemplatesArgs {
    #[arg(help = "Output format for starter-template discovery")]
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct AddArgs {
    #[arg(help = "Layer kind to scaffold (philosophy, policy, requirement, or feature)")]
    pub layer: LookupKind,

    #[arg(
        help = "New definition ID to scaffold (for example REQ-AUTH-001 or FEAT-AUTH-LOGIN-001). Omit in a terminal to be prompted."
    )]
    pub id: Option<String>,

    #[arg(help = WORKSPACE_HELP)]
    #[arg(default_value = ".")]
    pub workspace: PathBuf,

    #[arg(
        help = "Prompt for missing values in a terminal; when the ID is omitted, syu also prompts for it"
    )]
    #[arg(long, action = ArgAction::SetTrue)]
    pub interactive: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Optional YAML document path, relative to the workspace root or spec.root"
    )]
    pub file: Option<PathBuf>,

    #[arg(
        long,
        value_name = "NAME",
        help = "Feature registry kind and default folder name (feature scaffolding only)"
    )]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum StarterTemplate {
    #[default]
    Generic,
    DocsFirst,
    RustOnly,
    PythonOnly,
    RubyOnly,
    GoOnly,
    JavaOnly,
    #[value(name = "typescript-only")]
    TypeScriptOnly,
    Polyglot,
}

impl StarterTemplate {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::DocsFirst => "docs-first",
            Self::RustOnly => "rust-only",
            Self::PythonOnly => "python-only",
            Self::RubyOnly => "ruby-only",
            Self::GoOnly => "go-only",
            Self::JavaOnly => "java-only",
            Self::TypeScriptOnly => "typescript-only",
            Self::Polyglot => "polyglot",
        }
    }
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
    use super::{
        Cli, LookupKind, StarterTemplate, ValidationGenreFilter, ValidationSeverityFilter,
    };
    use clap::Parser;

    #[test]
    fn cli_enums_expose_expected_labels() {
        assert_eq!(LookupKind::Philosophy.label(), "philosophy");
        assert_eq!(LookupKind::Policy.label(), "policy");
        assert_eq!(LookupKind::Requirement.label(), "requirement");
        assert_eq!(LookupKind::Feature.label(), "feature");
        assert_eq!(StarterTemplate::Generic.label(), "generic");
        assert_eq!(StarterTemplate::DocsFirst.label(), "docs-first");
        assert_eq!(StarterTemplate::RustOnly.label(), "rust-only");
        assert_eq!(StarterTemplate::PythonOnly.label(), "python-only");
        assert_eq!(StarterTemplate::RubyOnly.label(), "ruby-only");
        assert_eq!(StarterTemplate::GoOnly.label(), "go-only");
        assert_eq!(StarterTemplate::JavaOnly.label(), "java-only");
        assert_eq!(StarterTemplate::TypeScriptOnly.label(), "typescript-only");
        assert_eq!(StarterTemplate::Polyglot.label(), "polyglot");
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

    #[test]
    fn init_args_accept_template_values() {
        let cli = Cli::try_parse_from(["syu", "init", ".", "--template", "rust-only"])
            .expect("init args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Init("));
        assert!(rendered.contains("template: RustOnly"));
    }

    #[test]
    fn templates_args_accept_json_format() {
        let cli = Cli::try_parse_from(["syu", "templates", "--format", "json"])
            .expect("templates args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Templates("));
        assert!(rendered.contains("format: Json"));
    }

    #[test]
    fn templates_args_default_to_text_format() {
        let cli = Cli::try_parse_from(["syu", "templates"]).expect("templates args should parse");
        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Templates("));
        assert!(rendered.contains("format: Text"));
    }

    #[test]
    fn init_args_accept_custom_spec_root() {
        let cli = Cli::try_parse_from(["syu", "init", ".", "--spec-root", "docs/spec"])
            .expect("init args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Init("));
        assert!(rendered.contains("spec_root: Some(\"docs/spec\")"));
    }

    #[test]
    fn search_args_accept_kind_filters_and_json_output() {
        let cli = Cli::try_parse_from([
            "syu",
            "search",
            "traceability",
            ".",
            "--kind",
            "feature",
            "--format",
            "json",
        ])
        .expect("search args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Search("));
        assert!(rendered.contains("query: \"traceability\""));
        assert!(rendered.contains("workspace: \".\""));
        assert!(rendered.contains("kind: Some(Feature)"));
        assert!(rendered.contains("format: Json"));
    }

    #[test]
    fn init_args_accept_shared_id_prefix_stems() {
        let cli = Cli::try_parse_from(["syu", "init", ".", "--id-prefix", "store"])
            .expect("init args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Init("));
        assert!(rendered.contains("id_prefix: Some(\"store\")"));
    }

    #[test]
    fn init_args_support_interactive_mode() {
        let cli = Cli::try_parse_from(["syu", "init", ".", "--interactive"])
            .expect("interactive init args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Init("));
        assert!(rendered.contains("workspace: \".\""));
        assert!(rendered.contains("interactive: true"));
    }

    #[test]
    fn init_args_accept_individual_id_prefix_overrides() {
        let cli = Cli::try_parse_from([
            "syu",
            "init",
            ".",
            "--philosophy-prefix",
            "PHIL-STORE",
            "--policy-prefix",
            "POL-STORE",
            "--requirement-prefix",
            "REQ-STORE",
            "--feature-prefix",
            "FEAT-STORE",
        ])
        .expect("init args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("philosophy_prefix: Some(\"PHIL-STORE\")"));
        assert!(rendered.contains("policy_prefix: Some(\"POL-STORE\")"));
        assert!(rendered.contains("requirement_prefix: Some(\"REQ-STORE\")"));
        assert!(rendered.contains("feature_prefix: Some(\"FEAT-STORE\")"));
    }

    #[test]
    fn add_args_accept_explicit_files_and_feature_kinds() {
        let cli = Cli::try_parse_from([
            "syu",
            "add",
            "feature",
            "FEAT-AUTH-001",
            ".",
            "--kind",
            "auth",
            "--file",
            "docs/syu/features/auth/login.yaml",
        ])
        .expect("add args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Add("));
        assert!(rendered.contains("layer: Feature"));
        assert!(rendered.contains("id: Some(\"FEAT-AUTH-001\")"));
        assert!(rendered.contains("workspace: \".\""));
        assert!(rendered.contains("interactive: false"));
        assert!(rendered.contains("file: Some(\"docs/syu/features/auth/login.yaml\")"));
        assert!(rendered.contains("kind: Some(\"auth\")"));
    }

    #[test]
    fn add_args_support_interactive_mode_without_an_id() {
        let cli = Cli::try_parse_from(["syu", "add", "requirement", "--interactive"])
            .expect("interactive add args should parse");

        let rendered = format!("{cli:?}");
        assert!(rendered.contains("command: Some(Add("));
        assert!(rendered.contains("layer: Requirement"));
        assert!(rendered.contains("id: None"));
        assert!(rendered.contains("workspace: \".\""));
        assert!(rendered.contains("interactive: true"));
    }
}
