#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use syu_spec_model::{ContractKind, RepoPath};

pub const CONFIG_SCHEMA: &str = "syu/config/v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub schema: String,
    pub workspace: WorkspaceConfig,
    pub profiles: ProfilesConfig,
    pub validation: ValidationConfig,
    pub work: WorkConfig,
    pub adapters: AdapterConfig,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub spec_roots: Vec<RepoPath>,
    pub artifact_roots: Vec<RepoPath>,
    #[serde(default)]
    pub excludes: Vec<RepoPathPattern>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilesConfig {
    pub active: Vec<String>,
    #[serde(default)]
    pub custom: BTreeMap<String, Profile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub facets: BTreeMap<String, FacetRule>,
    #[serde(default)]
    pub contract_rules: Vec<ContractRule>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacetRule {
    pub include: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractRule {
    pub kind: ContractKind,
    pub require_participants: Vec<ParticipantRequirement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticipantRequirement {
    pub role: String,
    pub facets: Vec<String>,
    pub min: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationPreset {
    Standard,
    Strict,
    AgentReady,
}

/// The quality of the trace from a specification item to its repository evidence.
///
/// A coverage target is qualitative: reaching a target means every required item
/// reaches that level, rather than meeting a percentage threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageLevel {
    Off,
    Connected,
    Owned,
    AgentReady,
    Verified,
    EvidenceReady,
}

impl CoverageLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Connected => "connected",
            Self::Owned => "owned",
            Self::AgentReady => "agent-ready",
            Self::Verified => "verified",
            Self::EvidenceReady => "evidence-ready",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageConfig {
    /// All discovered implementation and test symbols must have a matching
    /// specification binding at this level.
    pub artifact_ownership: CoverageLevel,
    /// All delivery anchors in the specification must have implementation and
    /// verification evidence at this level.
    pub spec_fulfillment: CoverageLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationConfig {
    pub preset: ValidationPreset,
    pub coverage: CoverageConfig,
    #[serde(default)]
    pub deny_warnings: bool,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleOverride>,
    pub changed: ChangedConfig,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOverride {
    Error,
    Warning,
    Info,
    Off,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkConfig {
    pub slicing: SliceLimits,
    pub context: ContextConfig,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextConfig {
    pub include_parent_principles: bool,
    pub include_parent_rules: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterConfig {
    pub enabled: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::CoverageLevel;

    #[test]
    fn coverage_levels_have_stable_yaml_names() {
        assert_eq!(CoverageLevel::EvidenceReady.as_str(), "evidence-ready");
    }
}
