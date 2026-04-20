// FEAT-ADD-001
// FEAT-LOG-001
// FEAT-RELATE-001
// FEAT-SEARCH-001
// FEAT-TRACE-001
// FEAT-BROWSE-001
// REQ-CORE-021
// REQ-CORE-023
// REQ-CORE-024

pub mod cli;
pub mod command;
pub mod config;
pub mod coverage;
pub mod inspect;
pub mod language;
pub mod model;
pub mod report;
pub mod rules;
pub mod runtime;
pub mod workspace;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::io::IsTerminal;

#[cfg(test)]
pub(crate) mod test_support {
    use std::{
        env,
        path::{Path, PathBuf},
        sync::{LazyLock, Mutex, MutexGuard},
    };

    static CURRENT_DIR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub(crate) struct CurrentDirGuard {
        previous: PathBuf,
        _lock: MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        pub(crate) fn chdir(path: &Path) -> Self {
            let lock = CURRENT_DIR_LOCK
                .lock()
                .expect("cwd test lock should not be poisoned");
            let previous = env::current_dir().expect("cwd should be readable");
            env::set_current_dir(path).expect("should chdir into tempdir");
            Self {
                previous,
                _lock: lock,
            }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).expect("should restore cwd");
        }
    }
}

enum Dispatch {
    Browse(cli::BrowseArgs),
    List(cli::ListArgs),
    Show(cli::ShowArgs),
    Search(cli::SearchArgs),
    Log(cli::LogArgs),
    Relate(cli::RelateArgs),
    Trace(cli::TraceArgs),
    App(cli::AppArgs),
    PrintHelp,
    Validate(cli::ValidateArgs),
    Report(cli::ReportArgs),
    Init(cli::InitArgs),
    Templates(cli::TemplatesArgs),
    Add(cli::AddArgs),
}

fn dispatch(cli: cli::Cli, stdin_is_terminal: bool, stdout_is_terminal: bool) -> Dispatch {
    match cli.command {
        Some(cli::Commands::Browse(args)) => Dispatch::Browse(args),
        Some(cli::Commands::List(args)) => Dispatch::List(args),
        Some(cli::Commands::Show(args)) => Dispatch::Show(args),
        Some(cli::Commands::Search(args)) => Dispatch::Search(args),
        Some(cli::Commands::Log(args)) => Dispatch::Log(args),
        Some(cli::Commands::Relate(args)) => Dispatch::Relate(args),
        Some(cli::Commands::Trace(args)) => Dispatch::Trace(args),
        Some(cli::Commands::App(args)) => Dispatch::App(args),
        None if stdin_is_terminal && stdout_is_terminal => {
            Dispatch::Browse(cli::BrowseArgs::default())
        }
        None => Dispatch::PrintHelp,
        Some(cli::Commands::Validate(args)) => Dispatch::Validate(args),
        Some(cli::Commands::Report(args)) => Dispatch::Report(args),
        Some(cli::Commands::Init(args)) => Dispatch::Init(args),
        Some(cli::Commands::Templates(args)) => Dispatch::Templates(args),
        Some(cli::Commands::Add(args)) => Dispatch::Add(args),
    }
}

fn run_dispatch(dispatch: Dispatch) -> Result<i32> {
    match dispatch {
        Dispatch::Browse(args) => command::browse::run_browse_command(&args),
        Dispatch::List(args) => command::list::run_list_command(&args),
        Dispatch::Show(args) => command::show::run_show_command(&args),
        Dispatch::Search(args) => command::search::run_search_command(&args),
        Dispatch::Log(args) => command::log::run_log_command(&args),
        Dispatch::Relate(args) => command::relate::run_relate_command(&args),
        Dispatch::Trace(args) => command::trace::run_trace_command(&args),
        Dispatch::App(args) => command::app::run_app_command(&args),
        Dispatch::PrintHelp => {
            let mut command = cli::Cli::command();
            command.print_help()?;
            println!();
            Ok(0)
        }
        Dispatch::Validate(args) => command::check::run_check_command(&args),
        Dispatch::Report(args) => command::report::run_report_command(&args),
        Dispatch::Init(args) => command::init::run_init_command(&args),
        Dispatch::Templates(args) => command::templates::run_templates_command(&args),
        Dispatch::Add(args) => command::add::run_add_command(&args),
    }
}

pub fn run() -> Result<i32> {
    let cli = cli::Cli::parse();
    run_dispatch(dispatch(
        cli,
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::cli::{
        AddArgs, AppArgs, Cli, Commands, HistoryKind, ListArgs, LogArgs, LookupKind, OutputFormat,
        RelateArgs, SearchArgs, ShowArgs, TemplatesArgs, TraceArgs,
    };

    // REQ-CORE-015
    #[test]
    fn dispatches_interactive_bare_invocations_to_browse_defaults() {
        let action = super::dispatch(Cli { command: None }, true, true);
        assert!(matches!(
            action,
            super::Dispatch::Browse(crate::cli::BrowseArgs { workspace, .. })
                if workspace == Path::new(".")
        ));
    }

    #[test]
    // REQ-CORE-015
    fn print_help_dispatch_renders_successfully() {
        let code = super::run_dispatch(super::Dispatch::PrintHelp)
            .expect("print help dispatch should succeed");
        assert_eq!(code, 0);
    }

    #[test]
    // REQ-CORE-017
    fn dispatches_app_subcommands_without_rewriting_them() {
        let action = super::dispatch(
            Cli {
                command: Some(Commands::App(AppArgs {
                    workspace: PathBuf::from("workspace"),
                    bind: Some("127.0.0.1".to_string()),
                    port: Some(4173),
                    allow_remote: false,
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            action,
            super::Dispatch::App(crate::cli::AppArgs { workspace, port, .. })
                if workspace == Path::new("workspace") && port == Some(4173)
        ));
    }

    #[test]
    // REQ-CORE-018
    fn dispatches_lookup_subcommands_without_rewriting_them() {
        let list = super::dispatch(
            Cli {
                command: Some(Commands::List(ListArgs {
                    positional: vec!["requirement".to_string(), "workspace".to_string()],
                    format: OutputFormat::Json,
                    with_path: true,
                })),
            },
            true,
            true,
        );
        assert!(matches!(
            list,
            super::Dispatch::List(crate::cli::ListArgs { ref positional, format, with_path })
                if positional == &["requirement", "workspace"]
                    && format == OutputFormat::Json
                    && with_path
        ));

        let show = super::dispatch(
            Cli {
                command: Some(Commands::Show(ShowArgs {
                    id: "REQ-CORE-018".to_string(),
                    workspace: PathBuf::from("workspace"),
                    format: OutputFormat::Text,
                })),
            },
            true,
            true,
        );
        assert!(matches!(
            show,
            super::Dispatch::Show(crate::cli::ShowArgs { id, workspace, format })
                if id == "REQ-CORE-018"
                    && workspace == Path::new("workspace")
                    && format == OutputFormat::Text
        ));

        let templates = super::dispatch(
            Cli {
                command: Some(Commands::Templates(TemplatesArgs {
                    format: OutputFormat::Json,
                })),
            },
            true,
            true,
        );
        assert!(matches!(
            templates,
            super::Dispatch::Templates(crate::cli::TemplatesArgs { format })
                if format == OutputFormat::Json
        ));
    }

    #[test]
    // REQ-CORE-021
    fn dispatches_trace_subcommands_without_rewriting_them() {
        let trace = super::dispatch(
            Cli {
                command: Some(Commands::Trace(TraceArgs {
                    file: PathBuf::from("src/lib.rs"),
                    workspace: PathBuf::from("workspace"),
                    symbol: Some("run".to_string()),
                    format: OutputFormat::Json,
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            trace,
            super::Dispatch::Trace(crate::cli::TraceArgs {
                file,
                workspace,
                symbol,
                format
            }) if file == Path::new("src/lib.rs")
                && workspace == Path::new("workspace")
                && symbol.as_deref() == Some("run")
                && format == OutputFormat::Json
        ));
    }

    #[test]
    // REQ-CORE-019
    fn dispatches_search_subcommands_without_rewriting_them() {
        let search = super::dispatch(
            Cli {
                command: Some(Commands::Search(SearchArgs {
                    query: "traceability".to_string(),
                    workspace: PathBuf::from("workspace"),
                    kind: Some(LookupKind::Feature),
                    format: OutputFormat::Json,
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            search,
            super::Dispatch::Search(crate::cli::SearchArgs { query, workspace, kind, format })
                if query == "traceability"
                    && workspace == Path::new("workspace")
                    && kind == Some(LookupKind::Feature)
                    && format == OutputFormat::Json
        ));
    }

    #[test]
    // REQ-CORE-024
    fn dispatches_log_subcommands_without_rewriting_them() {
        let log = super::dispatch(
            Cli {
                command: Some(Commands::Log(LogArgs {
                    id: "REQ-CORE-024".to_string(),
                    workspace: PathBuf::from("workspace"),
                    kind: HistoryKind::Definition,
                    path: Some(PathBuf::from("docs/syu/requirements")),
                    include_related: true,
                    merge_base_ref: Some("origin/main".to_string()),
                    range: None,
                    limit: 5,
                    format: OutputFormat::Json,
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            log,
            super::Dispatch::Log(crate::cli::LogArgs {
                id,
                workspace,
                kind,
                path,
                include_related,
                merge_base_ref,
                range,
                limit,
                format
            })
                if id == "REQ-CORE-024"
                    && workspace == Path::new("workspace")
                    && kind == HistoryKind::Definition
                    && path == Some(PathBuf::from("docs/syu/requirements"))
                    && include_related
                    && merge_base_ref.as_deref() == Some("origin/main")
                    && range.is_none()
                    && limit == 5
                    && format == OutputFormat::Json
        ));
    }

    #[test]
    // REQ-CORE-023
    fn dispatches_relate_subcommands_without_rewriting_them() {
        let relate = super::dispatch(
            Cli {
                command: Some(Commands::Relate(RelateArgs {
                    selector: "REQ-CORE-023".to_string(),
                    workspace: PathBuf::from("workspace"),
                    format: OutputFormat::Json,
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            relate,
            super::Dispatch::Relate(crate::cli::RelateArgs { selector, workspace, format })
                if selector == "REQ-CORE-023"
                    && workspace == Path::new("workspace")
                    && format == OutputFormat::Json
        ));
    }

    #[test]
    // REQ-CORE-020
    fn dispatches_add_subcommands_without_rewriting_them() {
        let add = super::dispatch(
            Cli {
                command: Some(Commands::Add(AddArgs {
                    layer: LookupKind::Feature,
                    id: Some("FEAT-AUTH-001".to_string()),
                    workspace: PathBuf::from("workspace"),
                    interactive: false,
                    file: Some(PathBuf::from("docs/syu/features/auth/login.yaml")),
                    kind: Some("auth".to_string()),
                })),
            },
            true,
            true,
        );

        assert!(matches!(
            add,
            super::Dispatch::Add(crate::cli::AddArgs {
                layer,
                id,
                workspace,
                interactive,
                file,
                kind
            })
                if layer == LookupKind::Feature
                    && id.as_deref() == Some("FEAT-AUTH-001")
                    && workspace == Path::new("workspace")
                    && !interactive
                    && file == Some(PathBuf::from("docs/syu/features/auth/login.yaml"))
                    && kind.as_deref() == Some("auth")
        ));
    }
}
