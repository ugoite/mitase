#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, str::FromStr};

pub const SPEC_SCHEMA: &str = "syu/spec/v1";

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    };
}
string_id!(SpecId);
string_id!(LocalId);

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
        if item.is_empty() || id.is_empty() || !is_local_id(id) {
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
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selector {
    File,
    Symbol { names: Vec<String> },
    Operation { method: String, path: String },
    Heading { value: String },
    JsonPointer { value: String },
    Marker { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTarget {
    pub id: LocalId,
    pub adapter: String,
    pub path: PathBuf,
    pub selector: Selector,
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
    pub targets: Vec<ArtifactTarget>,
    #[serde(default)]
    pub satisfies: Vec<SpecAnchor>,
    #[serde(default)]
    pub verifies: Vec<SpecAnchor>,
    #[serde(default)]
    pub documents: Vec<SpecAnchor>,
    #[serde(default)]
    pub enforces: Vec<SpecAnchor>,
    #[serde(default)]
    pub generated_from: Vec<BoundTargetRef>,
    #[serde(default)]
    pub evidences: Vec<SpecAnchor>,
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
    pub binding: SpecAnchor,
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
    fn anchors_roundtrip() {
        for text in [
            "PHIL-001#principle.intent-before-code",
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
        assert!(serde_yaml::from_str::<SpecDocument>("schema: syu/spec/v1\nkind: requirements\nnamespace: x\ncategory: X\nrequirements:\n- id: REQ-1\n  title: x\n  description: x\n  priority: high\n  status: implemented\n  tests: {}\n").is_err());
    }
}
