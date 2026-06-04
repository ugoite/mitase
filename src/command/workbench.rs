use anyhow::Result;

use crate::{cli::WorkbenchArgs, workspace::load_workspace};
use syu_workbench_server::{WorkbenchLaunchConfig, WorkbenchServer};

pub fn run_workbench_command(args: &WorkbenchArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let bind = args
        .bind
        .clone()
        .unwrap_or_else(|| workspace.config.workbench.bind.clone());
    let port = args.port.unwrap_or(workspace.config.workbench.port);
    let server = WorkbenchServer::new(WorkbenchLaunchConfig {
        workspace_root: workspace.root,
        spec_root: workspace.spec_root,
        bind,
        port,
        allow_remote_bind: args.allow_remote_bind,
        show_log: args.show_log,
    })?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(server.serve())?;
    Ok(0)
}
