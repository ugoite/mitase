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
    App(cli::AppArgs),
    PrintHelp,
    Validate(cli::ValidateArgs),
    Report(cli::ReportArgs),
    Init(cli::InitArgs),
}

fn dispatch(cli: cli::Cli, stdin_is_terminal: bool, stdout_is_terminal: bool) -> Dispatch {
    match cli.command {
        Some(cli::Commands::Browse(args)) => Dispatch::Browse(args),
        Some(cli::Commands::App(args)) => Dispatch::App(args),
        None if stdin_is_terminal && stdout_is_terminal => {
            Dispatch::Browse(cli::BrowseArgs::default())
        }
        None => Dispatch::PrintHelp,
        Some(cli::Commands::Validate(args)) => Dispatch::Validate(args),
        Some(cli::Commands::Report(args)) => Dispatch::Report(args),
        Some(cli::Commands::Init(args)) => Dispatch::Init(args),
    }
}

fn run_dispatch(dispatch: Dispatch) -> Result<i32> {
    match dispatch {
        Dispatch::Browse(args) => command::browse::run_browse_command(&args),
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

    use crate::cli::{AppArgs, Cli, Commands};

    // REQ-CORE-015
    #[test]
    fn dispatches_interactive_bare_invocations_to_browse_defaults() {
        let action = super::dispatch(Cli { command: None }, true, true);
        assert!(matches!(
            action,
            super::Dispatch::Browse(crate::cli::BrowseArgs { workspace })
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
}
