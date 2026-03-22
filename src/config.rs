// FEAT-CHECK-001
// REQ-CORE-009

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG_FILE_NAME: &str = "syu.yaml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub exists: bool,
    pub config: SyuConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyuConfig {
    #[serde(default = "default_version", deserialize_with = "deserialize_version")]
    pub version: String,
    #[serde(default)]
    pub spec: SpecConfig,
    #[serde(default)]
    pub validate: ValidateConfig,
    #[serde(default)]
    pub runtimes: RuntimeConfigSet,
}

impl Default for SyuConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            spec: SpecConfig::default(),
            validate: ValidateConfig::default(),
            runtimes: RuntimeConfigSet::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpecConfig {
    #[serde(default = "default_spec_root")]
    pub root: PathBuf,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            root: default_spec_root(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidateConfig {
    #[serde(default)]
    pub default_fix: bool,
    #[serde(default = "default_allow_planned")]
    pub allow_planned: bool,
    #[serde(default = "default_require_non_orphaned_items")]
    pub require_non_orphaned_items: bool,
    #[serde(default = "default_require_reciprocal_links")]
    pub require_reciprocal_links: bool,
    #[serde(default)]
    pub require_symbol_trace_coverage: bool,
}

impl Default for ValidateConfig {
    fn default() -> Self {
        Self {
            default_fix: false,
            allow_planned: default_allow_planned(),
            require_non_orphaned_items: default_require_non_orphaned_items(),
            require_reciprocal_links: default_require_reciprocal_links(),
            require_symbol_trace_coverage: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigSet {
    #[serde(default)]
    pub python: RuntimeConfig,
    #[serde(default)]
    pub node: RuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default = "default_runtime_command")]
    pub command: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            command: default_runtime_command(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConfigVersionValue {
    String(String),
    Integer(u32),
}

pub fn current_cli_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn default_version() -> String {
    current_cli_version().to_string()
}

fn deserialize_version<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = ConfigVersionValue::deserialize(deserializer)?;
    match value {
        ConfigVersionValue::String(version) => {
            let trimmed = version.trim();
            if trimmed.is_empty() {
                return Err(serde::de::Error::custom(
                    "`syu.yaml` version must not be blank",
                ));
            }
            Ok(trimmed.to_string())
        }
        ConfigVersionValue::Integer(version) => Ok(version.to_string()),
    }
}

fn default_allow_planned() -> bool {
    true
}

fn default_require_non_orphaned_items() -> bool {
    true
}

fn default_require_reciprocal_links() -> bool {
    true
}

fn default_spec_root() -> PathBuf {
    PathBuf::from("docs/syu")
}

fn default_runtime_command() -> String {
    "auto".to_string()
}

pub fn config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(CONFIG_FILE_NAME)
}

// FEAT-CHECK-001
pub fn load_config(workspace_root: &Path) -> Result<LoadedConfig> {
    let path = config_path(workspace_root);
    if !path.is_file() {
        return Ok(LoadedConfig {
            path,
            exists: false,
            config: SyuConfig::default(),
        });
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config from `{}`", path.display()))?;
    let config: SyuConfig = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse config from `{}`", path.display()))?;

    Ok(LoadedConfig {
        path,
        exists: true,
        config,
    })
}

// FEAT-INIT-001
pub fn render_config(config: &SyuConfig) -> Result<String> {
    serde_yaml::to_string(config).context("failed to serialize `syu.yaml`")
}

pub fn resolve_spec_root(workspace_root: &Path, config: &SyuConfig) -> PathBuf {
    if config.spec.root.is_absolute() {
        config.spec.root.clone()
    } else {
        workspace_root.join(&config.spec.root)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

    use super::{
        CONFIG_FILE_NAME, RuntimeConfig, SyuConfig, config_path, current_cli_version, load_config,
        render_config, resolve_spec_root,
    };

    #[test]
    fn load_config_returns_defaults_when_config_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let loaded = load_config(tempdir.path()).expect("default config should load");

        assert!(!loaded.exists);
        assert_eq!(loaded.path, tempdir.path().join(CONFIG_FILE_NAME));
        assert_eq!(
            loaded.config.spec.root,
            std::path::PathBuf::from("docs/syu")
        );
        assert_eq!(loaded.config.version, current_cli_version());
        assert_eq!(loaded.config.runtimes.python.command, "auto");
        assert!(loaded.config.validate.allow_planned);
        assert!(loaded.config.validate.require_non_orphaned_items);
        assert!(loaded.config.validate.require_reciprocal_links);
        assert!(!loaded.config.validate.require_symbol_trace_coverage);
    }

    #[test]
    fn load_config_parses_workspace_override() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(
            tempdir.path().join(CONFIG_FILE_NAME),
            format!(
                "version: {version}\nspec:\n  root: spec/contracts\nvalidate:\n  default_fix: true\n  allow_planned: false\n  require_non_orphaned_items: false\n  require_reciprocal_links: false\n  require_symbol_trace_coverage: true\nruntimes:\n  python:\n    command: python3\n  node:\n    command: node\n",
                version = current_cli_version()
            ),
        )
        .expect("config should be written");

        let loaded = load_config(tempdir.path()).expect("config should parse");
        assert!(loaded.exists);
        assert_eq!(
            loaded.config.spec.root,
            std::path::PathBuf::from("spec/contracts")
        );
        assert!(loaded.config.validate.default_fix);
        assert!(!loaded.config.validate.allow_planned);
        assert!(!loaded.config.validate.require_non_orphaned_items);
        assert!(!loaded.config.validate.require_reciprocal_links);
        assert!(loaded.config.validate.require_symbol_trace_coverage);
        assert_eq!(loaded.config.runtimes.python.command, "python3");
    }

    #[test]
    fn render_config_serializes_defaults() {
        let rendered = render_config(&SyuConfig::default()).expect("config should serialize");
        assert!(rendered.contains(current_cli_version()));
        assert!(rendered.contains("default_fix: false"));
        assert!(rendered.contains("allow_planned: true"));
        assert!(rendered.contains("require_non_orphaned_items: true"));
        assert!(rendered.contains("require_reciprocal_links: true"));
        assert!(rendered.contains("require_symbol_trace_coverage: false"));
        assert!(rendered.contains("command: auto"));
    }

    #[test]
    fn resolve_spec_root_handles_relative_and_absolute_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let relative = resolve_spec_root(tempdir.path(), &SyuConfig::default());
        assert_eq!(relative, tempdir.path().join("docs/syu"));

        let absolute = SyuConfig {
            spec: super::SpecConfig {
                root: tempdir.path().join("contracts"),
            },
            ..SyuConfig::default()
        };
        assert_eq!(
            resolve_spec_root(tempdir.path(), &absolute),
            tempdir.path().join("contracts")
        );
    }

    #[test]
    fn config_path_uses_standard_filename() {
        assert_eq!(
            config_path(std::path::Path::new("/tmp/workspace")),
            std::path::PathBuf::from("/tmp/workspace/syu.yaml")
        );
    }

    #[test]
    fn runtime_config_defaults_to_auto_detection() {
        let runtime = RuntimeConfig::default();
        assert_eq!(runtime.command, "auto");
    }

    #[test]
    fn load_config_accepts_legacy_numeric_version() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join(CONFIG_FILE_NAME), "version: 1\n").expect("config");

        let loaded = load_config(tempdir.path()).expect("legacy config should parse");
        assert_eq!(loaded.config.version, "1");
    }

    #[cfg(unix)]
    #[test]
    fn load_config_reports_read_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join(CONFIG_FILE_NAME);
        fs::write(&path, format!("version: {}\n", current_cli_version()))
            .expect("config should exist");

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let error = load_config(tempdir.path()).expect_err("read errors should surface");

        let mut restore = fs::metadata(&path).expect("metadata").permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&path, restore).expect("permissions should restore");

        assert!(error.to_string().contains("failed to read config"));
    }

    #[test]
    fn load_config_reports_parse_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join(CONFIG_FILE_NAME), "version: [")
            .expect("config should exist");

        let error = load_config(tempdir.path()).expect_err("parse errors should surface");
        assert!(error.to_string().contains("failed to parse config"));
    }

    #[test]
    fn load_config_rejects_blank_string_version() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join(CONFIG_FILE_NAME), "version: \"   \"\n")
            .expect("config should exist");

        let error = load_config(tempdir.path()).expect_err("blank version should fail");
        assert!(error.to_string().contains("failed to parse config"));
    }
}
