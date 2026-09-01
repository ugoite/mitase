#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

pub const SPEC_SCHEMA: &str = "mitase/spec/v1";
pub const SHA256_PREFIX: &str = "sha256:";

/// Encodes bytes as lowercase hexadecimal for stable serialized identifiers.
pub fn lowercase_hex(bytes: impl AsRef<[u8]>) -> String {
    use std::fmt::Write as _;

    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

/// Encodes a SHA-256 digest in the canonical form used by serialized mitase data.
pub fn format_sha256(digest: impl AsRef<[u8]>) -> String {
    format!("{SHA256_PREFIX}{}", lowercase_hex(digest))
}

macro_rules! string_id {
    ($name:ident, $validator:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                if !$validator(&value) {
                    return Err(serde::de::Error::custom(concat!(
                        "invalid ",
                        stringify!($name)
                    )));
                }
                Ok(Self(value))
            }
        }
    };
}
string_id!(SpecId, is_spec_id);
string_id!(LocalId, is_local_id);

fn is_spec_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        && !value.contains("--")
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(PathBuf);
impl RepoPath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, String> {
        let value = value.into();
        if value.as_os_str().is_empty() || value.is_absolute() {
            return Err("repository path must be non-empty and relative".into());
        }
        if value
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(
                "repository path must not contain root, prefix, dot, or parent components".into(),
            );
        }
        let text = value.to_string_lossy();
        if text.contains('\\') {
            return Err("repository path must use forward-slash separators".into());
        }
        Ok(Self(value))
    }

    /// Build a repository path from a filesystem-relative path.
    ///
    /// Repository paths are serialized with forward slashes on every
    /// platform, while `Path::strip_prefix` uses the host platform's native
    /// separator. Normalize only at this filesystem boundary so the model's
    /// strict serialized-path invariant remains unchanged.
    pub fn from_path(value: impl AsRef<Path>) -> Result<Self, String> {
        let normalized = value.as_ref().to_string_lossy().replace('\\', "/");
        Self::new(normalized)
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.0.to_string_lossy()
    }
}
impl AsRef<Path> for RepoPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}
impl Serialize for RepoPath {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string_lossy())
    }
}
impl<'de> Deserialize<'de> for RepoPath {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalAnchorKind {
    Principle,
    Rule,
    Criterion,
    Binding,
    Contract,
}

impl LocalAnchorKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Principle => "principle",
            Self::Rule => "rule",
            Self::Criterion => "criterion",
            Self::Binding => "binding",
            Self::Contract => "contract",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpecAnchor {
    pub item: SpecId,
    pub kind: LocalAnchorKind,
    pub local_id: LocalId,
}

impl fmt::Display for SpecAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}.{}", self.item, self.kind.label(), self.local_id)
    }
}
impl FromStr for SpecAnchor {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (item, local) = value.split_once('#').ok_or("anchor must contain #")?;
        let (kind, id) = local
            .split_once('.')
            .ok_or("anchor must contain kind.local-id")?;
        if !is_spec_id(item) || !is_local_id(id) {
            return Err("anchor contains an invalid id".into());
        }
        let kind = match kind {
            "principle" => LocalAnchorKind::Principle,
            "rule" => LocalAnchorKind::Rule,
            "criterion" => LocalAnchorKind::Criterion,
            "binding" => LocalAnchorKind::Binding,
            "contract" => LocalAnchorKind::Contract,
            _ => return Err("unknown anchor kind".into()),
        };
        Ok(Self {
            item: SpecId(item.into()),
            kind,
            local_id: LocalId(id.into()),
        })
    }
}
impl Serialize for SpecAnchor {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for SpecAnchor {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpecItemRef(pub SpecId);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundTargetRef {
    pub binding: SpecAnchor,
    pub target_id: LocalId,
}
impl fmt::Display for BoundTargetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/target.{}", self.binding, self.target_id)
    }
}
impl FromStr for BoundTargetRef {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (binding, target) = value
            .rsplit_once("/target.")
            .ok_or("target reference must contain /target.")?;
        let binding: SpecAnchor = binding.parse()?;
        if binding.kind != LocalAnchorKind::Binding || !is_local_id(target) {
            return Err("target reference must use a binding anchor and valid target id".into());
        }
        Ok(Self {
            binding,
            target_id: LocalId(target.into()),
        })
    }
}
impl Serialize for BoundTargetRef {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for BoundTargetRef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

fn is_local_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.contains("--")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OwnershipSelector {
    File,
    Module { name: String },
    PathPrefix { value: RepoPath },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExactSelector {
    File,
    Symbol { name: String },
    Test { name: String },
    Operation { method: String, path: String },
    Heading { value: String },
    JsonPointer { value: String },
    Marker { value: String },
}

/// `Selector` used to be the public name for artifact selection.  Keep the
/// type name stable while replacing its shape with one exact identity.
pub type Selector = ExactSelector;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipScope {
    pub id: LocalId,
    pub adapter: String,
    pub path: RepoPath,
    pub selector: OwnershipSelector,
    #[serde(default)]
    pub supports: Vec<SpecAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TargetClaim {
    Satisfies {
        criterion: SpecAnchor,
    },
    Verifies {
        criterion: SpecAnchor,
        covers: Vec<BoundTargetRef>,
        runner: VerificationRunnerRef,
    },
    Documents {
        anchor: SpecAnchor,
    },
    Enforces {
        rule: SpecAnchor,
    },
    GeneratedFrom {
        targets: Vec<BoundTargetRef>,
    },
    Exposes {
        target: BoundTargetRef,
    },
    Evidences {
        anchor: SpecAnchor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRunnerRef {
    pub runner: String,
    #[serde(default)]
    pub arguments: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTarget {
    pub id: LocalId,
    pub adapter: String,
    pub path: RepoPath,
    pub selector: ExactSelector,
    /// The state this target must have once its owning item is implemented.
    ///
    /// `Absent` keeps a removed target in the specification as an explicit
    /// lifecycle obligation, rather than silently dropping its ownership and
    /// finalization evidence.
    #[serde(default, skip_serializing_if = "is_present_target_lifecycle")]
    pub lifecycle: ArtifactTargetLifecycle,
    #[serde(default)]
    pub claims: Vec<TargetClaim>,
}

fn is_present_target_lifecycle(value: &ArtifactTargetLifecycle) -> bool {
    matches!(value, ArtifactTargetLifecycle::Present)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactTargetLifecycle {
    #[default]
    Present,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingRole {
    Implementation,
    Verification,
    Documentation,
    Enforcement,
    ContractSource,
    Configuration,
    Generated,
    Migration,
    Operation,
    Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactBinding {
    pub id: LocalId,
    pub role: BindingRole,
    pub facet: String,
    pub responsibility: String,
    #[serde(default)]
    pub owns: Vec<OwnershipScope>,
    pub targets: Vec<ArtifactTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Principle {
    pub id: LocalId,
    pub statement: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleLevel {
    /// The source did not declare a normative strength for this rule.
    ///
    /// This preserves source meaning during migration without treating an
    /// inferred default as a governance decision.
    Unspecified,
    Must,
    Should,
    May,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleAppliesTo {
    #[serde(default)]
    pub roles: Vec<BindingRole>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleEnforcement {
    External(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: LocalId,
    pub level: RuleLevel,
    pub statement: String,
    pub governed_by: Vec<SpecAnchor>,
    #[serde(default)]
    pub applies_to: RuleAppliesTo,
    #[serde(default)]
    pub enforcement: Option<RuleEnforcement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CriterionKind {
    Behavior,
    Quality,
    Security,
    Operational,
    Documentation,
    Compatibility,
    Custom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: LocalId,
    pub kind: CriterionKind,
    pub statement: String,
    pub governed_by: Vec<SpecAnchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemStatus {
    Planned,
    Implemented,
    Deprecated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Philosophy {
    pub id: SpecId,
    pub title: String,
    pub summary: String,
    pub principles: Vec<Principle>,
    #[serde(default)]
    pub bindings: Vec<ArtifactBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub id: SpecId,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub bindings: Vec<ArtifactBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: SpecId,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub status: ItemStatus,
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub bindings: Vec<ArtifactBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractKind {
    Http,
    Event,
    Function,
    Schema,
    Cli,
    File,
    Custom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractParticipant {
    pub target: BoundTargetRef,
    pub role: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub id: LocalId,
    pub kind: ContractKind,
    pub source: BoundTargetRef,
    pub participants: Vec<ContractParticipant>,
    #[serde(default)]
    pub guarantees: Vec<SpecAnchor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub id: SpecId,
    pub title: String,
    pub summary: String,
    pub status: ItemStatus,
    pub bindings: Vec<ArtifactBinding>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SpecDocument {
    Philosophies {
        schema: String,
        namespace: String,
        category: String,
        philosophies: Vec<Philosophy>,
    },
    Policies {
        schema: String,
        namespace: String,
        category: String,
        policies: Vec<Policy>,
    },
    Requirements {
        schema: String,
        namespace: String,
        category: String,
        requirements: Vec<Requirement>,
    },
    Features {
        schema: String,
        namespace: String,
        category: String,
        features: Vec<Feature>,
    },
}
impl SpecDocument {
    pub fn schema(&self) -> &str {
        match self {
            Self::Philosophies { schema, .. }
            | Self::Policies { schema, .. }
            | Self::Requirements { schema, .. }
            | Self::Features { schema, .. } => schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_digests_use_the_canonical_lowercase_representation() {
        assert_eq!(lowercase_hex([0xAB, 0xCD]), "abcd");
        assert_eq!(format_sha256([0xAB, 0xCD]), "sha256:abcd");
    }

    #[test]
    fn anchors_roundtrip() {
        for text in [
            "PHIL-005#principle.authority-without-workflow-ownership",
            "POL-1#rule.bounded-work",
            "REQ-1#criterion.result",
            "FEAT-1#binding.backend",
            "FEAT-1#contract.http",
        ] {
            let parsed: SpecAnchor = text.parse().unwrap();
            assert_eq!(
                serde_yaml::from_str::<SpecAnchor>(&serde_yaml::to_string(&parsed).unwrap())
                    .unwrap(),
                parsed
            );
        }
    }
    #[test]
    fn target_refs_do_not_use_array_positions() {
        let text = "FEAT-1#binding.backend/target.handler";
        assert_eq!(text.parse::<BoundTargetRef>().unwrap().to_string(), text);
    }
    #[test]
    fn old_shape_is_rejected() {
        assert!(serde_yaml::from_str::<SpecDocument>("schema: mitase/spec/v1\nkind: requirements\nnamespace: x\ncategory: X\nrequirements:\n- id: REQ-1\n  title: x\n  description: x\n  priority: high\n  status: implemented\n  tests: {}\n").is_err());
    }
    #[test]
    fn invalid_ids_and_repository_paths_are_rejected() {
        assert!(serde_yaml::from_str::<SpecId>("bad-id").is_err());
        assert!(serde_yaml::from_str::<LocalId>("Bad_ID").is_err());
        assert!(serde_yaml::from_str::<RepoPath>("../outside").is_err());
        assert!(serde_yaml::from_str::<RepoPath>("/absolute").is_err());
    }

    #[test]
    fn filesystem_paths_are_normalized_to_repository_paths() {
        let path = RepoPath::from_path(Path::new(r"api\login.rs")).unwrap();
        assert_eq!(path.to_string_lossy(), "api/login.rs");
    }

    #[test]
    fn selectors_are_single_exact_identities() {
        assert!(
            serde_yaml::from_str::<ExactSelector>("kind: symbol\nnames: [one, two]\n").is_err()
        );
        assert!(
            serde_yaml::from_str::<ExactSelector>("kind: symbol\nname: one\nextra: true\n")
                .is_err()
        );
        assert!(matches!(
            serde_yaml::from_str::<ExactSelector>("kind: test\nname: keyword-first\n").unwrap(),
            ExactSelector::Test { name } if name == "keyword-first"
        ));
    }

    #[test]
    fn artifact_target_lifecycle_defaults_to_present_and_accepts_absent() {
        let present: ArtifactTarget = serde_yaml::from_str(
            "id: subject\nadapter: rust\npath: src/lib.rs\nselector: { kind: symbol, name: subject }\n",
        )
        .expect("default lifecycle target");
        assert_eq!(present.lifecycle, ArtifactTargetLifecycle::Present);

        let absent: ArtifactTarget = serde_yaml::from_str(
            "id: subject\nadapter: rust\npath: src/lib.rs\nselector: { kind: symbol, name: subject }\nlifecycle: absent\n",
        )
        .expect("absent lifecycle target");
        assert_eq!(absent.lifecycle, ArtifactTargetLifecycle::Absent);
    }

    #[test]
    fn unspecified_rule_level_round_trips_without_becoming_enforceable() {
        let rule: Rule = serde_yaml::from_str(
            "id: governance\nlevel: unspecified\nstatement: Preserve source meaning.\ngoverned_by: [PHIL-001#principle.source]\n",
        )
        .expect("unspecified rule level");
        assert_eq!(rule.level, RuleLevel::Unspecified);
        assert_eq!(
            serde_yaml::to_string(&rule).expect("serialize rule"),
            "id: governance\nlevel: unspecified\nstatement: Preserve source meaning.\ngoverned_by:\n- PHIL-001#principle.source\napplies_to:\n  roles: []\nenforcement: null\n"
        );
    }

    #[test]
    fn binding_level_relations_and_non_target_contract_refs_are_rejected() {
        let binding_relation = r#"
schema: mitase/spec/v1
kind: features
namespace: x
category: X
features:
  - id: FEAT-1
    title: x
    summary: x
    status: planned
    bindings:
      - id: b
        role: implementation
        facet: x
        responsibility: x
        satisfies: [REQ-1#criterion.x]
        targets: []
"#;
        assert!(serde_yaml::from_str::<SpecDocument>(binding_relation).is_err());
        assert!("FEAT-1#binding.b".parse::<BoundTargetRef>().is_err());
    }
}
