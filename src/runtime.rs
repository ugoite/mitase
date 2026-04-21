// FEAT-CHECK-001
// REQ-CORE-002

use std::{env, ffi::OsString, path::Path};

#[cfg(windows)]
use std::path::PathBuf;

use crate::config::SyuConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Python,
    Node,
}

impl RuntimeKind {
    fn configured_command(self, config: &SyuConfig) -> &str {
        match self {
            Self::Python => &config.runtimes.python.command,
            Self::Node => &config.runtimes.node.command,
        }
    }

    fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["python3", "python"],
            Self::Node => &["node", "bun"],
        }
    }
}

// FEAT-CHECK-001
pub fn resolve_runtime_command(config: &SyuConfig, kind: RuntimeKind) -> Option<String> {
    let configured = kind.configured_command(config).trim();
    if configured.is_empty() || configured.eq_ignore_ascii_case("auto") {
        kind.candidates()
            .iter()
            .find(|candidate| command_exists(candidate))
            .map(|candidate| (*candidate).to_string())
    } else {
        Some(configured.to_string())
    }
}

pub(crate) fn command_exists(command: &str) -> bool {
    command_exists_in_path(command, env::var_os("PATH"), env::var_os("PATHEXT"))
}

fn command_exists_in_path(
    command: &str,
    path_env: Option<OsString>,
    pathext_env: Option<OsString>,
) -> bool {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file();
    }

    let Some(path_env) = path_env else {
        return false;
    };

    #[cfg(not(windows))]
    {
        let _ = pathext_env;
        env::split_paths(&path_env).any(|directory| directory.join(command).is_file())
    }

    #[cfg(windows)]
    {
        let path_entries: Vec<PathBuf> = env::split_paths(&path_env).collect();
        let extensions = executable_extensions(command, pathext_env.as_deref());
        path_entries.into_iter().any(|directory| {
            extensions
                .iter()
                .map(|extension| {
                    if extension.is_empty() {
                        directory.join(command)
                    } else {
                        directory.join(format!("{command}{extension}"))
                    }
                })
                .any(|candidate| candidate.is_file())
        })
    }
}

#[cfg(windows)]
fn executable_extensions(command: &str, pathext: Option<&std::ffi::OsStr>) -> Vec<String> {
    if Path::new(command).extension().is_some() {
        return vec![String::new()];
    }

    let mut extensions = vec![String::new()];
    if cfg!(windows) {
        let pathext = pathext
            .and_then(|value| value.to_str())
            .unwrap_or(".COM;.EXE;.BAT;.CMD");
        extensions.extend(
            pathext
                .split(';')
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.to_ascii_lowercase()),
        );
    }
    extensions
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::config::SyuConfig;

    use super::{RuntimeKind, command_exists_in_path, resolve_runtime_command};

    #[test]
    fn resolve_runtime_uses_explicit_command_when_configured() {
        let config = SyuConfig {
            runtimes: crate::config::RuntimeConfigSet {
                python: crate::config::RuntimeConfig {
                    command: "custom-python".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            resolve_runtime_command(&config, RuntimeKind::Python),
            Some("custom-python".to_string())
        );
    }

    #[test]
    fn runtime_kind_exposes_node_configuration_and_candidates() {
        let config = SyuConfig {
            runtimes: crate::config::RuntimeConfigSet {
                node: crate::config::RuntimeConfig {
                    command: "custom-node".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(RuntimeKind::Node.configured_command(&config), "custom-node");
        assert_eq!(RuntimeKind::Node.candidates(), &["node", "bun"]);
    }

    #[test]
    fn command_exists_in_path_finds_absolute_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let executable = tempdir.path().join("tool");
        fs::write(&executable, "echo ok").expect("tool should exist");

        assert!(command_exists_in_path(
            executable.to_str().expect("utf-8 path"),
            None,
            None
        ));
    }

    #[test]
    fn command_exists_in_path_searches_custom_path_entries() {
        let tempdir = tempdir().expect("tempdir should exist");
        let bin_dir = tempdir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        fs::write(bin_dir.join("python3"), "echo ok").expect("python3 should exist");

        let joined = std::env::join_paths([bin_dir]).expect("joined path");
        assert!(command_exists_in_path("python3", Some(joined), None));
        assert!(!command_exists_in_path("python3", None, None));
    }
}
