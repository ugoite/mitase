#![forbid(unsafe_code)]

use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dioxus::prelude::*;
use serde::Serialize;
use mitase_app_ui::{AppShell, WorkbenchPage, WorkbenchUiState};
use mitase_workbench::{
    WorkbenchActionId, WorkbenchActionMutability, WorkbenchState, workbench_actions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLaunchConfig {
    pub workspace_root: PathBuf,
    pub spec_root: PathBuf,
    pub bind: String,
    pub port: u16,
}

impl DesktopLaunchConfig {
    pub fn from_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            spec_root: workspace_root.join("docs/mitase"),
            workspace_root,
            bind: "127.0.0.1".to_string(),
            port: 3000,
        }
    }

    pub fn local_url(&self) -> String {
        format!("http://{}:{}", self.bind, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopActionBridge {
    pub id: String,
    pub title: String,
    pub read_only: bool,
}

pub fn desktop_action_bridge() -> Vec<DesktopActionBridge> {
    workbench_actions()
        .iter()
        .map(|action| DesktopActionBridge {
            id: action.id.label().to_string(),
            title: action.title.clone(),
            read_only: action.mutability == WorkbenchActionMutability::ReadOnly,
        })
        .collect()
}

pub fn read_only_action_available(id: WorkbenchActionId) -> bool {
    workbench_actions()
        .iter()
        .any(|action| action.id == id && action.mutability == WorkbenchActionMutability::ReadOnly)
}

pub fn render_shared_workbench_shell(state: WorkbenchState) -> String {
    let ui = WorkbenchUiState::from_state(state);
    dioxus_ssr::render_element(rsx! {
        AppShell { ui, active_page: WorkbenchPage::Work, sidebar_open: true }
    })
}

pub fn default_config_from_args() -> Result<DesktopLaunchConfig> {
    let workspace = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let workspace_root = canonical_workspace_root(&workspace)?;
    Ok(DesktopLaunchConfig::from_workspace(workspace_root))
}

fn canonical_workspace_root(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("failed to resolve workspace path `{}`", path.display()))
}

pub fn run() -> Result<()> {
    run_with_config(default_config_from_args()?)
}

#[cfg(not(feature = "tauri-runtime"))]
pub fn run_with_config(config: DesktopLaunchConfig) -> Result<()> {
    println!(
        "Mitase Workbench desktop shell is available with `--features tauri-runtime`; shared server URL: {}",
        config.local_url()
    );
    Ok(())
}

#[cfg(feature = "tauri-runtime")]
pub fn run_with_config(config: DesktopLaunchConfig) -> Result<()> {
    use mitase_workbench_server::{WorkbenchLaunchConfig, WorkbenchServer};
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    use url::Url;

    let server_config = WorkbenchLaunchConfig {
        workspace_root: config.workspace_root.clone(),
        spec_root: config.spec_root.clone(),
        bind: config.bind.clone(),
        port: config.port,
        allow_remote_bind: false,
        show_log: false,
    };
    let url = config.local_url();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("desktop runtime should start");
        runtime
            .block_on(async move {
                WorkbenchServer::new(server_config)
                    .expect("workbench server should start")
                    .serve()
                    .await
            })
            .expect("workbench server should keep running");
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![desktop_actions])
        .setup(move |app| {
            let external_url = Url::parse(&url)?;
            WebviewWindowBuilder::new(app, "workbench", WebviewUrl::External(external_url))
                .title("Mitase Workbench")
                .inner_size(1280.0, 860.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .context("failed to run Mitase Workbench desktop shell")
}

#[cfg(feature = "tauri-runtime")]
#[tauri::command]
fn desktop_actions() -> Vec<DesktopActionBridge> {
    desktop_action_bridge()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_shell_points_at_the_shared_workbench_server() {
        let config = DesktopLaunchConfig::from_workspace(PathBuf::from("/tmp/project"));

        assert_eq!(config.spec_root, PathBuf::from("/tmp/project/docs/mitase"));
        assert_eq!(config.local_url(), "http://127.0.0.1:3000");
    }

    #[test]
    fn desktop_bridge_uses_the_shared_action_registry() {
        let actions = desktop_action_bridge();

        assert!(actions.iter().any(|action| action.id == "request.classify"));
        assert!(read_only_action_available(
            WorkbenchActionId::RequestClassify
        ));
    }

    #[test]
    fn desktop_shell_renders_the_shared_dioxus_workbench_ui() {
        let html = render_shared_workbench_shell(WorkbenchState::default());

        assert!(html.contains("Command Palette"));
        assert!(html.contains("request.classify"));
    }
}
