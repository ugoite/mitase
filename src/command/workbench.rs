use anyhow::Result;

use crate::{
    cli::WorkbenchArgs,
    config::{load_config, resolve_spec_root},
    workspace::resolve_workspace_root,
};
use syu_workbench_server::{WorkbenchLaunchConfig, WorkbenchServer};

pub fn run_workbench_command(args: &WorkbenchArgs) -> Result<i32> {
    let launch = resolve_workbench_launch(args)?;
    let server = WorkbenchServer::new(launch)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(server.serve())?;
    Ok(0)
}

fn resolve_workbench_launch(args: &WorkbenchArgs) -> Result<WorkbenchLaunchConfig> {
    let workspace_root = resolve_workspace_root(&args.workspace)?;
    let loaded_config = load_config(&workspace_root)?;
    let bind = args
        .bind
        .clone()
        .unwrap_or_else(|| loaded_config.config.workbench.bind.clone());
    let port = args.port.unwrap_or(loaded_config.config.workbench.port);
    Ok(WorkbenchLaunchConfig {
        spec_root: resolve_spec_root(&workspace_root, &loaded_config.config),
        workspace_root,
        bind,
        port,
        allow_remote_bind: args.allow_remote_bind,
        show_log: args.show_log,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninitialized_directory_resolves_to_a_workbench_launch() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let launch = resolve_workbench_launch(&WorkbenchArgs {
            workspace: tempdir.path().to_path_buf(),
            bind: None,
            port: None,
            allow_remote_bind: false,
            show_log: false,
        })
        .expect("launch");

        assert_eq!(
            launch.workspace_root,
            tempdir.path().canonicalize().unwrap()
        );
        assert_eq!(launch.spec_root, launch.workspace_root.join("docs/syu"));
        assert_eq!(launch.bind, "127.0.0.1");
        assert_eq!(launch.port, 3000);
    }
}
