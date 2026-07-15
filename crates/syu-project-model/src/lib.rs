#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use syu_spec_model::RepoPath;

pub const CONFIG_SCHEMA: &str = "syu/config/v1";
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessConfig {
    pub target: ReadinessLevel,
    #[serde(default)]
    pub scopes: BTreeMap<String, ReadinessLevel>,
    #[serde(default)]
    pub probes: ReadinessProbes,
    pub limits: ReadinessLimits,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProbes {
    #[serde(default)]
    pub implemented_criteria: Option<String>,
    #[serde(default)]
    pub public_entrypoints: Option<String>,
    #[serde(default)]
    pub contracts: Option<String>,
    #[serde(default)]
    pub changed_units: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessLimits {
    pub max_ownership_scope_units: usize,
    pub max_targets_per_binding: usize,
    pub max_slices_per_seed: usize,
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
