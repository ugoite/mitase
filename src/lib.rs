// FEAT-ADD-001
// FEAT-SEARCH-001
// FEAT-BROWSE-001

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

enum Dispatch {
    Browse(cli::BrowseArgs),
    List(cli::ListArgs),
    Show(cli::ShowArgs),
    Search(cli::SearchArgs),
    App(cli::AppArgs),
    PrintHelp,
    Validate(cli::ValidateArgs),
    Report(cli::ReportArgs),
    Init(cli::InitArgs),
    Add(cli::AddArgs),
}

fn dispatch(cli: cli::Cli, stdin_is_terminal: bool, stdout_is_terminal: bool) -> Dispatch {
    match cli.command {
        Some(cli::Commands::Browse(args)) => Dispatch::Browse(args),
        Some(cli::Commands::List(args)) => Dispatch::List(args),
        Some(cli::Commands::Show(args)) => Dispatch::Show(args),
        Some(cli::Commands::Search(args)) => Dispatch::Search(args),
        Some(cli::Commands::App(args)) => Dispatch::App(args),
        None if stdin_is_terminal && stdout_is_terminal => {
            Dispatch::Browse(cli::BrowseArgs::default())
        }
        None => Dispatch::PrintHelp,
        Some(cli::Commands::Validate(args)) => Dispatch::Validate(args),
        Some(cli::Commands::Report(args)) => Dispatch::Report(args),
        Some(cli::Commands::Init(args)) => Dispatch::Init(args),
        Some(cli::Commands::Add(args)) => Dispatch::Add(args),
    }
}

fn run_dispatch(dispatch: Dispatch) -> Result<i32> {
    match dispatch {
        Dispatch::Browse(args) => command::browse::run_browse_command(&args),
        Dispatch::List(args) => command::list::run_list_command(&args),
        Dispatch::Show(args) => command::show::run_show_command(&args),
        Dispatch::Search(args) => command::search::run_search_command(&args),
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
        AddArgs, AppArgs, Cli, Commands, ListArgs, LookupKind, OutputFormat, SearchArgs, ShowArgs,
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
    // REQ-CORE-017
    fn dispatches_app_subcommands_without_rewriting_them() {
        let action = super::dispatch(
            Cli {
                command: Some(Commands::App(AppArgs {
                    workspace: PathBuf::from("workspace"),
                    bind: Some("127.0.0.1".to_string()),
                    port: Some(4173),
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
