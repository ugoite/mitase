#![forbid(unsafe_code)]
use mitase_spec_model::{RepoPath, SpecAnchor};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::{Deref, DerefMut},
    path::Path,
};

pub const CONFIG_SCHEMA: &str = "mitase/config/v1";
pub const DEFAULT_SPEC_ROOT: &str = "docs/mitase";
pub const STANDARD_EXCLUDES: &[&str] = &[
    ".git/**",
    "target/**",
    "node_modules/**",
    "dist/**",
    "build/**",
    "coverage/**",
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub schema: String,
    pub workspace: WorkspaceConfig,
    pub inventory: InventoryConfig,
    pub validation: ValidationConfig,
    #[serde(default)]
    pub verification: VerificationConfig,
}

/// Strict user-authored configuration. Optional sections are resolved into a
/// complete [`ProjectConfig`] by [`EffectiveProjectConfig::from_source`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfigInput {
    pub schema: String,
    #[serde(default)]
    pub workspace: WorkspaceConfigInput,
    #[serde(default)]
    pub inventory: InventoryConfigInput,
    #[serde(default)]
    pub validation: ValidationConfigInput,
    #[serde(default)]
    pub verification: Option<VerificationConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfigInput {
    #[serde(default)]
    pub spec_roots: Option<Vec<RepoPath>>,
    #[serde(default)]
    pub excludes: Option<Vec<RepoPathPattern>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryConfigInput {
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: Option<Vec<InventoryProfileInput>>,
    /// A single profile can omit the profile wrapper and declare providers
    /// directly under `inventory`.
    #[serde(default)]
    pub providers: Option<BTreeMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryProfileInput {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub providers: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationConfigInput {
    #[serde(default)]
    pub preset: Option<ValidationPreset>,
    #[serde(default)]
    pub readiness: Option<ReadinessConfigInput>,
    #[serde(default)]
    pub changed: Option<ChangedConfigInput>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessConfigInput {
    #[serde(default)]
    pub target: Option<ReadinessLevel>,
    #[serde(default)]
    pub probes: Option<ReadinessProbes>,
    #[serde(default)]
    pub limits: Option<ReadinessLimitsInput>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessLimitsInput {
    #[serde(default)]
    pub max_ownership_scope_units: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangedConfigInput {
    #[serde(default)]
    pub baseline: Option<ChangeBaseline>,
    #[serde(default)]
    pub require_owned_changes: Option<bool>,
}

/// The complete configuration consumed by workspace loading and validation.
/// It retains the resolved canonical config and the conventions that supplied
/// omitted mechanical values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectiveProjectConfig {
    #[serde(flatten)]
    pub config: ProjectConfig,
    pub applied_conventions: Vec<String>,
}

impl Deref for EffectiveProjectConfig {
    type Target = ProjectConfig;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl DerefMut for EffectiveProjectConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl EffectiveProjectConfig {
    pub fn from_source(root: &Path, source: &str) -> Result<Self, String> {
        let input: ProjectConfigInput =
            serde_yaml::from_str(source).map_err(|error| error.to_string())?;
        input.resolve(root)
    }
}

impl ProjectConfigInput {
    pub fn resolve(self, root: &Path) -> Result<EffectiveProjectConfig, String> {
        if self.schema != CONFIG_SCHEMA {
            return Err(format!(
                "config schema must be {CONFIG_SCHEMA}, got {}",
                self.schema
            ));
        }
        let mut applied_conventions = Vec::new();
        let spec_roots = self.workspace.spec_roots.unwrap_or_else(|| {
            applied_conventions.push(format!("workspace.spec_roots={DEFAULT_SPEC_ROOT}"));
            vec![RepoPath::new(DEFAULT_SPEC_ROOT).expect("default spec root")]
        });
        let excludes = self.workspace.excludes.unwrap_or_else(|| {
            applied_conventions.push("workspace.excludes=standard".into());
            STANDARD_EXCLUDES
                .iter()
                .map(|pattern| RepoPathPattern((*pattern).into()))
                .collect()
        });
        let workspace = WorkspaceConfig {
            spec_roots: spec_roots.clone(),
            excludes,
        };
        let inventory =
            resolve_inventory(self.inventory, root, &spec_roots, &mut applied_conventions)?;
        let validation = resolve_validation(self.validation, &mut applied_conventions);
        let verification = match self.verification {
            Some(verification) => verification,
            None => {
                applied_conventions.push("verification.runners=repository-presets".into());
                VerificationConfig {
                    runners: discovered_runner_presets(root),
                }
            }
        };
        Ok(EffectiveProjectConfig {
            config: ProjectConfig {
                schema: CONFIG_SCHEMA.into(),
                workspace,
                inventory,
                validation,
                verification,
            },
            applied_conventions,
        })
    }
}

fn resolve_inventory(
    input: InventoryConfigInput,
    root: &Path,
    spec_roots: &[RepoPath],
    applied_conventions: &mut Vec<String>,
) -> Result<InventoryConfig, String> {
    if input.providers.is_some() && input.profiles.is_some() {
        return Err("inventory.providers cannot be combined with inventory.profiles".into());
    }
    let (active_profile, profiles) = match input.profiles {
        Some(profiles) => {
            if profiles.is_empty() {
                return Err("inventory.profiles must not be empty".into());
            }
            let profile_count = profiles.len();
            let profiles = profiles
                .into_iter()
                .enumerate()
                .map(|(index, profile)| {
                    let id = match profile.id {
                        Some(id) => id,
                        None if profile_count == 1 => {
                            applied_conventions.push("inventory.profiles[0].id=default".into());
                            "default".into()
                        }
                        None => {
                            return Err(format!(
                                "inventory.profiles[{index}].id is required when multiple profiles are supplied"
                            ));
                        }
                    };
                    Ok(InventoryProfile {
                        id,
                        providers: profile.providers,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut profile_ids = BTreeSet::new();
            for profile in &profiles {
                if !profile_ids.insert(profile.id.clone()) {
                    return Err(format!(
                        "inventory.profiles contains duplicate id: {}",
                        profile.id
                    ));
                }
            }
            let active = match input.active_profile {
                Some(active) => active,
                None if profiles.len() == 1 => {
                    applied_conventions
                        .push(format!("inventory.active_profile={}", profiles[0].id));
                    profiles[0].id.clone()
                }
                None => {
                    return Err(
                        "inventory.active_profile is required when multiple profiles are supplied"
                            .into(),
                    );
                }
            };
            (active, profiles)
        }
        None => {
            let active = input
                .active_profile
                .clone()
                .unwrap_or_else(|| "default".into());
            let providers = match input.providers {
                Some(providers) => providers,
                None => {
                    applied_conventions.push("inventory.providers=repository-discovery".into());
                    discover_inventory_providers(root, spec_roots)
                }
            };
            if input.active_profile.is_none() {
                applied_conventions.push(format!("inventory.active_profile={active}"));
            }
            (
                active.clone(),
                vec![InventoryProfile {
                    id: active,
                    providers,
                }],
            )
        }
    };
    if !profiles.iter().any(|profile| profile.id == active_profile) {
        return Err(format!(
            "inventory.active_profile is not defined: {active_profile}"
        ));
    }
    Ok(InventoryConfig {
        active_profile,
        profiles,
    })
}

fn resolve_validation(
    input: ValidationConfigInput,
    applied_conventions: &mut Vec<String>,
) -> ValidationConfig {
    let preset = input.preset.unwrap_or_else(|| {
        applied_conventions.push("validation.preset=standard".into());
        ValidationPreset::Standard
    });
    let readiness_input = input.readiness.unwrap_or_default();
    let readiness_target = readiness_input.target.unwrap_or_else(|| {
        applied_conventions.push("validation.readiness.target=off".into());
        ReadinessLevel::Off
    });
    let readiness_limits = readiness_input.limits.unwrap_or_default();
    let max_ownership_scope_units =
        readiness_limits
            .max_ownership_scope_units
            .unwrap_or_else(|| {
                applied_conventions
                    .push("validation.readiness.limits.max_ownership_scope_units=64".into());
                64
            });
    let changed_input = input.changed.unwrap_or_default();
    let require_owned_changes = changed_input.require_owned_changes.unwrap_or_else(|| {
        applied_conventions.push("validation.changed.require_owned_changes=false".into());
        false
    });
    ValidationConfig {
        preset,
        readiness: ReadinessConfig {
            target: readiness_target,
            probes: readiness_input.probes.unwrap_or_default(),
            limits: ReadinessLimits {
                max_ownership_scope_units,
            },
        },
        changed: ChangedConfig {
            baseline: changed_input.baseline,
            require_owned_changes,
        },
    }
}

fn discover_inventory_providers(
    root: &Path,
    spec_roots: &[RepoPath],
) -> BTreeMap<String, serde_yaml::Value> {
    let mut providers = BTreeMap::new();
    if root.join("Cargo.toml").is_file() || contains_extension(root, &["rs"]) {
        providers.insert(
            "rust".into(),
            mapping_value([
                ("mode", serde_yaml::Value::String("test".into())),
                ("include_tests", serde_yaml::Value::Bool(true)),
            ]),
        );
    }
    if root.join("tsconfig.json").is_file() || contains_extension(root, &["ts", "tsx"]) {
        providers.insert(
            "typescript".into(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    if root.join("package.json").is_file() || contains_extension(root, &["js", "jsx", "mjs", "cjs"])
    {
        providers.insert(
            "javascript".into(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let markdown_roots = spec_roots
        .iter()
        .filter(|path| root.join(path.as_path()).is_dir())
        .map(|path| serde_yaml::Value::String(path.to_string_lossy().into_owned()))
        .collect::<Vec<_>>();
    if !markdown_roots.is_empty() {
        providers.insert(
            "markdown".into(),
            mapping_value([("roots", serde_yaml::Value::Sequence(markdown_roots))]),
        );
    }
    providers
}

fn discovered_runner_presets(root: &Path) -> BTreeMap<String, VerificationRunner> {
    let mut runners = BTreeMap::new();
    if root.join("Cargo.toml").is_file() {
        runners.insert(
            "cargo-test".into(),
            VerificationRunner {
                executable: "cargo".into(),
                arguments: vec![
                    "test".into(),
                    "-p".into(),
                    "{package}".into(),
                    "{test}".into(),
                    "--".into(),
                    "--exact".into(),
                ],
            },
        );
        runners.insert(
            "cargo-test-integration".into(),
            VerificationRunner {
                executable: "cargo".into(),
                arguments: vec![
                    "test".into(),
                    "-p".into(),
                    "{package}".into(),
                    "--test".into(),
                    "{harness}".into(),
                    "{test}".into(),
                    "--".into(),
                    "--exact".into(),
                ],
            },
        );
    }
    if root.join("package.json").is_file() {
        runners.insert(
            "node-test".into(),
            VerificationRunner {
                executable: "npm".into(),
                arguments: vec!["test".into()],
            },
        );
    }
    runners
}

fn mapping_value<const N: usize>(entries: [(&str, serde_yaml::Value); N]) -> serde_yaml::Value {
    serde_yaml::Value::Mapping(
        entries
            .into_iter()
            .map(|(key, value)| (serde_yaml::Value::String(key.into()), value))
            .collect(),
    )
}

fn contains_extension(root: &Path, extensions: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        let file_type = entry.file_type().ok();
        if file_type.as_ref().is_some_and(std::fs::FileType::is_dir) {
            if matches!(
                entry.file_name().to_str(),
                Some(".git" | "target" | "node_modules" | "dist" | "build" | "coverage")
            ) {
                return false;
            }
            return contains_extension(&path, extensions);
        }
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
    })
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
    Verifiable,
}

impl fmt::Display for ReadinessLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Off => "off",
            Self::Traceable => "traceable",
            Self::Seedable => "seedable",
            Self::Verifiable => "verifiable",
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
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationConfig {
    #[serde(default)]
    pub runners: BTreeMap<String, VerificationRunner>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRunner {
    pub executable: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangedConfig {
    #[serde(default)]
    pub baseline: Option<ChangeBaseline>,
    pub require_owned_changes: bool,
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
  preset: strict
  readiness:
    target: traceable
    probes:
      public_entrypoints: { selection: all, level: seedable }
      changed_units: false
    limits: { max_ownership_scope_units: 64 }
  changed: { require_owned_changes: true }
verification: { runners: {} }
"#;
        let config: ProjectConfig = serde_yaml::from_str(source).expect("project config");
        assert_eq!(
            config.validation.readiness.target,
            ReadinessLevel::Traceable
        );
        assert!(
            config
                .validation
                .readiness
                .probes
                .implemented_criteria
                .is_empty()
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
            }
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
        for retired_level in ["work-ready", "closed-loop"] {
            let retired_source =
                source.replace("target: traceable", &format!("target: {retired_level}"));
            assert!(
                serde_yaml::from_str::<ProjectConfig>(&retired_source).is_err(),
                "retired readiness level must not remain a compatibility alias: {retired_level}"
            );
        }
        let agent_ready_source = source.replace("preset: strict", "preset: agent-ready");
        assert!(
            serde_yaml::from_str::<ProjectConfig>(&agent_ready_source).is_err(),
            "agent-ready must not remain a compatibility alias"
        );
    }

    #[test]
    fn minimal_input_resolves_repository_conventions() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let effective = EffectiveProjectConfig::from_source(&root, "schema: mitase/config/v1\n")
            .expect("minimal config");
        assert_eq!(
            effective.workspace.spec_roots,
            vec![RepoPath::new(DEFAULT_SPEC_ROOT).unwrap()]
        );
        assert_eq!(effective.validation.preset, ValidationPreset::Standard);
        assert_eq!(effective.validation.readiness.target, ReadinessLevel::Off);
        assert!(!effective.validation.changed.require_owned_changes);
        assert_eq!(effective.inventory.active_profile, "default");
        assert!(
            effective.inventory.profiles[0]
                .providers
                .contains_key("rust")
        );
        assert!(effective.verification.runners.contains_key("cargo-test"));
        assert!(
            effective
                .applied_conventions
                .contains(&"workspace.spec_roots=docs/mitase".to_string())
        );
        assert!(
            effective
                .applied_conventions
                .contains(&"inventory.providers=repository-discovery".to_string())
        );
    }

    #[test]
    fn explicit_input_preserves_the_effective_canonical_config() {
        let source = r#"
schema: mitase/config/v1
workspace:
  spec_roots: [spec]
  excludes: [target/**]
inventory:
  active_profile: custom
  profiles:
    - id: custom
      providers: { rust: { mode: source } }
validation:
  preset: strict
  readiness:
    target: traceable
    probes: { changed_units: true }
    limits: { max_ownership_scope_units: 12 }
  changed: { require_owned_changes: true }
verification: { runners: {} }
"#;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let effective = EffectiveProjectConfig::from_source(&root, source).expect("full config");
        let expected: ProjectConfig = serde_yaml::from_str(source).expect("effective config");
        assert_eq!(effective.config, expected);
        assert!(effective.applied_conventions.is_empty());
    }

    #[test]
    fn input_rejects_ambiguous_profiles_and_unknown_fields() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ambiguous = r#"
schema: mitase/config/v1
inventory:
  profiles:
    - providers: { rust: {} }
    - providers: { typescript: {} }
"#;
        assert!(
            EffectiveProjectConfig::from_source(root.as_path(), ambiguous)
                .unwrap_err()
                .contains("profiles[0].id")
        );
        assert!(
            EffectiveProjectConfig::from_source(
                root.as_path(),
                "schema: mitase/config/v1\nextra: true\n"
            )
            .is_err()
        );
        let duplicate_ids = r#"
schema: mitase/config/v1
inventory:
  active_profile: default
  profiles:
    - id: default
      providers: { rust: {} }
    - id: default
      providers: { typescript: {} }
"#;
        assert!(
            EffectiveProjectConfig::from_source(root.as_path(), duplicate_ids)
                .unwrap_err()
                .contains("duplicate id")
        );
    }
}
