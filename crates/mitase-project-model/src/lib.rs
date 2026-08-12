#![forbid(unsafe_code)]
use mitase_spec_model::{RepoPath, SpecAnchor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub const CONFIG_SCHEMA: &str = "mitase/config/v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub schema: String,
    pub workspace: WorkspaceConfig,
    pub inventory: InventoryConfig,
    pub validation: ValidationConfig,
    #[serde(default)]
    pub verification: VerificationConfig,
    pub work: WorkConfig,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub spec_roots: Vec<RepoPath>,
    #[serde(default)]
    pub excludes: Vec<RepoPathPattern>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryConfig {
    pub active_profile: String,
    pub profiles: Vec<InventoryProfile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryProfile {
    pub id: String,
    #[serde(default)]
    pub providers: BTreeMap<String, serde_yaml::Value>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationPreset {
    Standard,
    Strict,
    AgentReady,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationConfig {
    pub preset: ValidationPreset,
    pub readiness: ReadinessConfig,
    pub changed: ChangedConfig,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessLevel {
    Off,
    Traceable,
    Seedable,
    WorkReady,
    Verifiable,
    ClosedLoop,
}

impl fmt::Display for ReadinessLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Off => "off",
            Self::Traceable => "traceable",
            Self::Seedable => "seedable",
            Self::WorkReady => "work-ready",
            Self::Verifiable => "verifiable",
            Self::ClosedLoop => "closed-loop",
        };
        formatter.write_str(label)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessConfig {
    pub target: ReadinessLevel,
    #[serde(default)]
    pub probes: ReadinessProbes,
    pub limits: ReadinessLimits,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProbes {
    #[serde(default)]
    pub implemented_criteria: Vec<ReadinessCriterionProbe>,
    #[serde(default)]
    pub public_entrypoints: Option<ReadinessSelectionProbe>,
    #[serde(default)]
    pub contracts: Option<ReadinessSelectionProbe>,
    #[serde(default)]
    pub changed_units: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessCriterionProbe {
    pub criterion: SpecAnchor,
    pub level: ReadinessLevel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessSelection {
    All,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessSelectionProbe {
    pub selection: ReadinessSelection,
    pub level: ReadinessLevel,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessLimits {
    pub max_ownership_scope_units: usize,
    pub max_targets_per_binding: usize,
    pub max_slices_per_origin: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationConfig {
    #[serde(default)]
    pub runners: BTreeMap<String, VerificationRunner>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRunner {
    pub adapter: VerificationRunnerAdapter,
    pub executable: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

impl<'de> Deserialize<'de> for VerificationRunner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(default)]
            adapter: Option<VerificationRunnerAdapter>,
            executable: String,
            #[serde(default)]
            arguments: Vec<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        Ok(Self {
            adapter: wire.adapter.unwrap_or_else(|| {
                VerificationRunnerAdapter::infer(&wire.executable, &wire.arguments)
            }),
            executable: wire.executable,
            arguments: wire.arguments,
        })
    }
}

/// The output protocol used to turn a configured test runner's output into a
/// verification proof.  Keeping this separate from the executable lets a
/// repository use wrappers such as `python -m pytest` without making receipt
/// validation depend on a command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationRunnerAdapter {
    CargoLibtest,
    Pytest,
    NodeTest,
    GoTest,
    Shell,
}

impl VerificationRunnerAdapter {
    fn infer(executable: &str, arguments: &[String]) -> Self {
        let executable = executable.rsplit(['/', '\\']).next().unwrap_or(executable);
        let executable = if executable.len() >= 4
            && executable[executable.len() - 4..].eq_ignore_ascii_case(".exe")
        {
            &executable[..executable.len() - 4]
        } else {
            executable
        };
        if executable.eq_ignore_ascii_case("cargo") {
            Self::CargoLibtest
        } else if executable.eq_ignore_ascii_case("pytest")
            || arguments.iter().any(|argument| argument == "pytest")
        {
            Self::Pytest
        } else if executable.eq_ignore_ascii_case("node") {
            Self::NodeTest
        } else if executable.eq_ignore_ascii_case("go") {
            Self::GoTest
        } else {
            Self::Shell
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangedConfig {
    #[serde(default)]
    pub baseline: Option<ChangeBaseline>,
    pub require_owned_changes: bool,
    #[serde(default)]
    pub require_plan: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ChangeBaseline {
    MergeBase { against: GitRef },
    Revision { revision: GitRef },
    Parent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GitRef(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepoPathPattern(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkConfig {
    pub slicing: SliceLimits,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceLimits {
    pub max_editable_files: usize,
    pub max_editable_symbols: usize,
    pub max_verification_targets: usize,
    pub max_readonly_targets: usize,
    pub max_total_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_config_preserves_readiness_configuration() {
        let source = r#"
schema: mitase/config/v1
workspace: { spec_roots: [docs/mitase], excludes: [] }
inventory:
  active_profile: default
  profiles: [{ id: default, providers: { rust: {} } }]
validation:
  preset: agent-ready
  readiness:
    target: traceable
    probes:
      implemented_criteria:
        - criterion: REQ-WORK-001#criterion.exact-slice
          level: work-ready
      public_entrypoints: { selection: all, level: seedable }
      changed_units: false
    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }
  changed: { require_owned_changes: true, require_plan: true }
verification: { runners: {} }
work:
  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 8, max_readonly_targets: 12, max_total_bytes: 120000 }
"#;
        let config: ProjectConfig = serde_yaml::from_str(source).expect("project config");
        assert_eq!(
            config.validation.readiness.target,
            ReadinessLevel::Traceable
        );
        assert_eq!(
            config.validation.readiness.probes.implemented_criteria[0]
                .criterion
                .to_string(),
            "REQ-WORK-001#criterion.exact-slice"
        );
        assert_eq!(
            config
                .validation
                .readiness
                .probes
                .public_entrypoints
                .as_ref()
                .map(|probe| probe.level),
            Some(ReadinessLevel::Seedable)
        );
        let root_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mitase.yaml"),
        )
        .expect("root mitase.yaml");
        let root_config: ProjectConfig =
            serde_yaml::from_str(&root_source).expect("root project config");
        assert_eq!(
            root_config.validation.readiness.limits,
            ReadinessLimits {
                max_ownership_scope_units: 64,
                max_targets_per_binding: 12,
                max_slices_per_origin: 4,
            }
        );
        assert_eq!(
            root_config.work.slicing,
            SliceLimits {
                max_editable_files: 4,
                max_editable_symbols: 8,
                max_verification_targets: 8,
                max_readonly_targets: 12,
                max_total_bytes: 160_000,
            }
        );
        let legacy_source = source.replace(
            "verification: { runners: {} }",
            "verification:\n  runners:\n    cargo-test:\n      executable: cargo\n      arguments: [test, -p, sample, sample_test]",
        );
        let legacy_config: ProjectConfig =
            serde_yaml::from_str(&legacy_source).expect("legacy project config");
        assert_eq!(
            legacy_config.verification.runners["cargo-test"].adapter,
            VerificationRunnerAdapter::CargoLibtest
        );
        let windows_legacy_source =
            legacy_source.replace("executable: cargo", r"executable: C:\\toolchain\\cargo.exe");
        let windows_legacy_config: ProjectConfig =
            serde_yaml::from_str(&windows_legacy_source).expect("Windows legacy config");
        assert_eq!(
            windows_legacy_config.verification.runners["cargo-test"].adapter,
            VerificationRunnerAdapter::CargoLibtest
        );
        let unix_path_legacy_source =
            legacy_source.replace("executable: cargo", "executable: /usr/bin/cargo");
        let unix_path_legacy_config: ProjectConfig =
            serde_yaml::from_str(&unix_path_legacy_source).expect("Unix path legacy config");
        assert_eq!(
            unix_path_legacy_config.verification.runners["cargo-test"].adapter,
            VerificationRunnerAdapter::CargoLibtest
        );
        let mixed_case_windows_source =
            legacy_source.replace("executable: cargo", r"executable: C:\\toolchain\\cargo.ExE");
        let mixed_case_windows_config: ProjectConfig =
            serde_yaml::from_str(&mixed_case_windows_source).expect("mixed-case Windows config");
        assert_eq!(
            mixed_case_windows_config.verification.runners["cargo-test"].adapter,
            VerificationRunnerAdapter::CargoLibtest
        );
        assert!(
            serde_yaml::from_str::<ProjectConfig>(&format!("{source}unknown: true\n")).is_err()
        );
        assert!(
            serde_yaml::from_str::<ProjectConfig>(&source.replace(
                "public_entrypoints: { selection: all, level: seedable }",
                "public_entrypoints: { selection: typo-anything, level: seedable }",
            ))
            .is_err()
        );
    }
}
