// FEAT-CONTRIB-001
// REQ-CORE-009

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::generate;
use std::io;

use crate::cli::{Cli, CompletionArgs};

pub fn run_completion_command(args: &CompletionArgs) -> Result<i32> {
    let mut command = Cli::command();
    generate(args.shell, &mut command, "syu", &mut io::stdout());
    Ok(0)
}
