#![forbid(unsafe_code)]

use mitase_spec_model::{
    ArtifactBinding, ArtifactTarget, BindingRole, BoundTargetRef, Criterion, ExactSelector,
    Feature, ItemStatus, LocalAnchorKind, LocalId, Philosophy, Policy, Priority, RepoPath,
    Requirement as CanonicalRequirement, SPEC_SCHEMA, SpecAnchor, SpecDocument, SpecId,
    TargetClaim, VerificationRunnerRef,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};

/// Schema identifier for the human-facing v0.2 authoring contract.
pub const AUTHORING_SCHEMA: &str = "mitase/authoring/v2";

/// Convert one current canonical document into explicit v0.2 authoring
/// syntax. This is an opt-in transformation; normal canonical loading never
/// calls it. Normal workspace loading routes authoring/v2 sources through the
/// separate parser and normalizer instead.
pub fn migrate_v1_to_v2(source: &str) -> Result<AuthoringDocument, MigrationError> {
    let canonical: SpecDocument = serde_yaml::from_str(source)
        .map_err(|error| MigrationError::InvalidSource(error.to_string()))?;
    migrate_v1_document(&canonical)
}

/// Convert a parsed canonical document and prove that the v0.2 result
/// normalizes back to the exact source graph.
pub fn migrate_v1_document(canonical: &SpecDocument) -> Result<AuthoringDocument, MigrationError> {
    if canonical.schema() != SPEC_SCHEMA {
        return Err(MigrationError::WrongSourceSchema {
            expected: SPEC_SCHEMA.into(),
            actual: canonical.schema().into(),
        });
    }

    let authoring = match canonical {
        SpecDocument::Philosophies {
            namespace,
            category,
            philosophies,
            ..
        } => AuthoringDocument::Philosophies {
            schema: AUTHORING_SCHEMA.into(),
            namespace: namespace.clone(),
            category: category.clone(),
            philosophies: philosophies.clone(),
        },
        SpecDocument::Policies {
            namespace,
            category,
            policies,
            ..
        } => AuthoringDocument::Policies {
            schema: AUTHORING_SCHEMA.into(),
            namespace: namespace.clone(),
            category: category.clone(),
            policies: policies.clone(),
        },
        SpecDocument::Requirements {
            namespace,
            category,
            requirements,
            ..
        } => AuthoringDocument::Requirements {
            schema: AUTHORING_SCHEMA.into(),
            namespace: namespace.clone(),
            category: category.clone(),
            requirements: requirements.clone(),
        },
        SpecDocument::Features {
            namespace,
            category,
            features,
            ..
        } => AuthoringDocument::Features {
            schema: AUTHORING_SCHEMA.into(),
            namespace: namespace.clone(),
            category: category.clone(),
            features: features.clone(),
        },
    };
    let normalized = authoring
        .normalize()
        .map_err(MigrationError::NormalizationFailed)?;
    if &normalized.document != canonical {
        return Err(MigrationError::SemanticMismatch);
    }
    Ok(authoring)
}

/// A strict, typed authoring document. Its semantic payload uses the
/// canonical domain types, while the schema boundary stays separate so
/// shorthand and inference can be added without weakening those types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AuthoringDocument {
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
        requirements: Vec<CanonicalRequirement>,
    },
    Features {
        schema: String,
        namespace: String,
        category: String,
        features: Vec<Feature>,
    },
    /// The first minimal short contract: one requirement, one criterion, one
    /// implementation target, and one verification target.
    Requirement {
        schema: String,
        namespace: String,
        category: String,
        requirement: Box<ShortRequirement>,
    },
}

impl AuthoringDocument {
    /// Parse one authoring document and reject non-v2 schemas at the frontend
    /// boundary. Legacy or canonical source documents are not silently
    /// reinterpreted as authoring input.
    pub fn parse(source: &str) -> Result<Self, String> {
        let document: Self = serde_yaml::from_str(source).map_err(|error| error.to_string())?;
        if document.schema() != AUTHORING_SCHEMA {
            return Err(format!(
                "authoring schema must be {AUTHORING_SCHEMA}, got {}",
                document.schema()
            ));
        }
        Ok(document)
    }

    pub fn schema(&self) -> &str {
        match self {
            Self::Philosophies { schema, .. }
            | Self::Policies { schema, .. }
            | Self::Requirements { schema, .. }
            | Self::Features { schema, .. }
            | Self::Requirement { schema, .. } => schema,
        }
    }

    /// Normalize authoring syntax into the canonical semantic document.
    ///
    /// Normative text and explicit relations are never inferred. The short
    /// contract only supplies mechanical defaults and unique adapter
    /// resolution, and reports both through normalization provenance.
    pub fn normalize(&self) -> Result<NormalizedDocument, NormalizationError> {
        let (document, applied_defaults, inferred) = match self {
            Self::Philosophies {
                namespace,
                category,
                philosophies,
                ..
            } => (
                SpecDocument::Philosophies {
                    schema: SPEC_SCHEMA.into(),
                    namespace: namespace.clone(),
                    category: category.clone(),
                    philosophies: philosophies.clone(),
                },
                Vec::new(),
                Vec::new(),
            ),
            Self::Policies {
                namespace,
                category,
                policies,
                ..
            } => (
                SpecDocument::Policies {
                    schema: SPEC_SCHEMA.into(),
                    namespace: namespace.clone(),
                    category: category.clone(),
                    policies: policies.clone(),
                },
                Vec::new(),
                Vec::new(),
            ),
            Self::Requirements {
                namespace,
                category,
                requirements,
                ..
            } => (
                SpecDocument::Requirements {
                    schema: SPEC_SCHEMA.into(),
                    namespace: namespace.clone(),
                    category: category.clone(),
                    requirements: requirements.clone(),
                },
                Vec::new(),
                Vec::new(),
            ),
            Self::Features {
                namespace,
                category,
                features,
                ..
            } => (
                SpecDocument::Features {
                    schema: SPEC_SCHEMA.into(),
                    namespace: namespace.clone(),
                    category: category.clone(),
                    features: features.clone(),
                },
                Vec::new(),
                Vec::new(),
            ),
            Self::Requirement {
                namespace,
                category,
                requirement,
                ..
            } => {
                let normalized = requirement.normalize()?;
                (
                    SpecDocument::Requirements {
                        schema: SPEC_SCHEMA.into(),
                        namespace: namespace.clone(),
                        category: category.clone(),
                        requirements: vec![normalized.requirement],
                    },
                    normalized.applied_defaults,
                    normalized.inferred,
                )
            }
        };

        Ok(NormalizedDocument {
            document,
            provenance: NormalizationProvenance {
                source_schema: AUTHORING_SCHEMA.into(),
                target_schema: SPEC_SCHEMA.into(),
                applied_defaults,
                inferred,
            },
        })
    }
}

/// The minimal common-case v0.2 authoring contract.
///
/// The requirement text, criterion statement, binding responsibilities,
/// claims, and verification runner remain explicit. Only mechanical target
/// details are eligible for defaults or unique inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortRequirement {
    pub id: SpecId,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub status: ItemStatus,
    pub criterion: ShortCriterion,
    pub implementation: ShortImplementation,
    pub verification: ShortVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortCriterion {
    pub id: LocalId,
    pub kind: mitase_spec_model::CriterionKind,
    pub statement: String,
    pub governed_by: Vec<SpecAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortImplementation {
    #[serde(default)]
    pub id: Option<LocalId>,
    pub facet: String,
    pub responsibility: String,
    pub target: ShortImplementationTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortImplementationTarget {
    #[serde(default)]
    pub id: Option<LocalId>,
    #[serde(default)]
    pub adapter: Option<String>,
    pub path: RepoPath,
    #[serde(default)]
    pub selector: Option<ExactSelector>,
    pub satisfies: LocalId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortVerification {
    #[serde(default)]
    pub id: Option<LocalId>,
    pub facet: String,
    pub responsibility: String,
    pub target: ShortVerificationTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortVerificationTarget {
    #[serde(default)]
    pub id: Option<LocalId>,
    #[serde(default)]
    pub adapter: Option<String>,
    pub path: RepoPath,
    #[serde(default)]
    pub selector: Option<ExactSelector>,
    pub verifies: ShortVerificationClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortVerificationClaim {
    pub criterion: LocalId,
    pub covers: Vec<LocalId>,
    pub runner: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
}

impl ShortRequirement {
    fn normalize(&self) -> Result<NormalizedShortRequirement, NormalizationError> {
        let criterion = Criterion {
            id: self.criterion.id.clone(),
            kind: self.criterion.kind,
            statement: self.criterion.statement.clone(),
            governed_by: self.criterion.governed_by.clone(),
        };
        let criterion_anchor = SpecAnchor {
            item: self.id.clone(),
            kind: LocalAnchorKind::Criterion,
            local_id: criterion.id.clone(),
        };

        let implementation_binding_id = self
            .implementation
            .id
            .clone()
            .unwrap_or_else(|| LocalId("implementation".into()));
        let verification_binding_id = self
            .verification
            .id
            .clone()
            .unwrap_or_else(|| LocalId("verification".into()));
        let implementation_target_id = self
            .implementation
            .target
            .id
            .clone()
            .unwrap_or_else(|| LocalId("source".into()));
        let verification_target_id = self
            .verification
            .target
            .id
            .clone()
            .unwrap_or_else(|| LocalId("test".into()));

        let mut applied_defaults = Vec::new();
        if self.implementation.id.is_none() {
            applied_defaults.push("requirement.implementation.id=implementation".into());
        }
        if self.verification.id.is_none() {
            applied_defaults.push("requirement.verification.id=verification".into());
        }
        if self.implementation.target.id.is_none() {
            applied_defaults.push("requirement.implementation.target.id=source".into());
        }
        if self.verification.target.id.is_none() {
            applied_defaults.push("requirement.verification.target.id=test".into());
        }

        let implementation_selector = self.implementation.target.selector.clone().or_else(|| {
            applied_defaults.push("requirement.implementation.target.selector=file".into());
            Some(ExactSelector::File)
        });
        let verification_selector = self.verification.target.selector.clone().or_else(|| {
            applied_defaults.push("requirement.verification.target.selector=file".into());
            Some(ExactSelector::File)
        });

        let (implementation_adapter, mut inferred) = adapter_from_path(
            "requirement.implementation.target.adapter",
            self.implementation.target.adapter.as_deref(),
            &self.implementation.target.path,
        )?;
        let (verification_adapter, verification_inferred) = adapter_from_path(
            "requirement.verification.target.adapter",
            self.verification.target.adapter.as_deref(),
            &self.verification.target.path,
        )?;
        inferred.extend(verification_inferred);

        let implementation_criterion = criterion_reference(
            "requirement.implementation.target.satisfies",
            &self.implementation.target.satisfies,
            &criterion_anchor,
        )?;
        let verification_criterion = criterion_reference(
            "requirement.verification.target.verifies.criterion",
            &self.verification.target.verifies.criterion,
            &criterion_anchor,
        )?;
        let implementation_binding_anchor = SpecAnchor {
            item: self.id.clone(),
            kind: LocalAnchorKind::Binding,
            local_id: implementation_binding_id.clone(),
        };
        let covered_targets = self
            .verification
            .target
            .verifies
            .covers
            .iter()
            .map(|target_id| {
                if target_id != &implementation_target_id {
                    return Err(NormalizationError::UnknownReference {
                        field: "requirement.verification.target.verifies.covers".into(),
                        value: target_id.to_string(),
                    });
                }
                Ok(BoundTargetRef {
                    binding: implementation_binding_anchor.clone(),
                    target_id: target_id.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let implementation_binding = ArtifactBinding {
            id: implementation_binding_id,
            role: BindingRole::Implementation,
            facet: self.implementation.facet.clone(),
            responsibility: self.implementation.responsibility.clone(),
            owns: Vec::new(),
            targets: vec![ArtifactTarget {
                id: implementation_target_id,
                adapter: implementation_adapter,
                path: self.implementation.target.path.clone(),
                selector: implementation_selector.expect("short selector default is present"),
                lifecycle: Default::default(),
                claims: vec![TargetClaim::Satisfies {
                    criterion: implementation_criterion,
                }],
            }],
        };
        let verification_binding = ArtifactBinding {
            id: verification_binding_id,
            role: BindingRole::Verification,
            facet: self.verification.facet.clone(),
            responsibility: self.verification.responsibility.clone(),
            owns: Vec::new(),
            targets: vec![ArtifactTarget {
                id: verification_target_id,
                adapter: verification_adapter,
                path: self.verification.target.path.clone(),
                selector: verification_selector.expect("short selector default is present"),
                lifecycle: Default::default(),
                claims: vec![TargetClaim::Verifies {
                    criterion: verification_criterion,
                    covers: covered_targets,
                    runner: VerificationRunnerRef {
                        runner: self.verification.target.verifies.runner.clone(),
                        arguments: self.verification.target.verifies.arguments.clone(),
                    },
                }],
            }],
        };

        Ok(NormalizedShortRequirement {
            requirement: CanonicalRequirement {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.clone(),
                priority: self.priority,
                status: self.status,
                criteria: vec![criterion],
                bindings: vec![implementation_binding, verification_binding],
            },
            applied_defaults,
            inferred,
        })
    }
}

fn criterion_reference(
    field: &str,
    reference: &LocalId,
    expected: &SpecAnchor,
) -> Result<SpecAnchor, NormalizationError> {
    if reference == &expected.local_id {
        Ok(expected.clone())
    } else {
        Err(NormalizationError::UnknownReference {
            field: field.into(),
            value: reference.to_string(),
        })
    }
}

fn adapter_from_path(
    field: &str,
    explicit: Option<&str>,
    path: &RepoPath,
) -> Result<(String, Vec<String>), NormalizationError> {
    if let Some(adapter) = explicit {
        return Ok((adapter.into(), Vec::new()));
    }
    let extension = path
        .as_path()
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let candidates = match extension.as_str() {
        "rs" => vec![("rust".into(), "rust".into())],
        "ts" | "tsx" | "mts" | "cts" => vec![("typescript".into(), "typescript".into())],
        "js" | "jsx" | "mjs" | "cjs" => vec![("javascript".into(), "javascript".into())],
        "py" => vec![("python".into(), "python".into())],
        "go" => vec![("go".into(), "go".into())],
        "sh" | "bash" | "zsh" => vec![("shell".into(), "shell".into())],
        "md" | "mdx" => vec![
            ("documentation".into(), "documentation".into()),
            ("markdown".into(), "markdown".into()),
        ],
        "json" => vec![
            ("json".into(), "json".into()),
            ("json-schema".into(), "json-schema".into()),
            ("openapi".into(), "openapi".into()),
        ],
        "yaml" | "yml" => vec![
            ("openapi".into(), "openapi".into()),
            ("yaml".into(), "yaml".into()),
        ],
        "html" => vec![("html".into(), "html".into())],
        _ => Vec::new(),
    };
    let (candidate, adapter) = resolve_unique(field, candidates)?;
    Ok((adapter, vec![format!("{field}={candidate}")]))
}

/// Resolve a mechanical omission only when its answer is provably unique.
/// Zero candidates and ambiguous candidates fail closed.
pub fn resolve_unique<T>(
    field: impl Into<String>,
    candidates: impl IntoIterator<Item = (String, T)>,
) -> Result<(String, T), NormalizationError> {
    let field = field.into();
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    match candidates.len() {
        0 => Err(NormalizationError::NoCandidates { field }),
        1 => Ok(candidates.into_iter().next().expect("length checked")),
        _ => Err(NormalizationError::Ambiguous {
            field,
            candidates: candidates.into_iter().map(|(label, _)| label).collect(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    NoCandidates {
        field: String,
    },
    Ambiguous {
        field: String,
        candidates: Vec<String>,
    },
    UnknownReference {
        field: String,
        value: String,
    },
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCandidates { field } => write!(
                formatter,
                "cannot infer {field}: no unique candidate is available"
            ),
            Self::Ambiguous { field, candidates } => write!(
                formatter,
                "cannot infer {field}: candidates are {}",
                candidates.join(", ")
            ),
            Self::UnknownReference { field, value } => {
                write!(formatter, "unknown reference for {field}: {value}")
            }
        }
    }
}

impl Error for NormalizationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    InvalidSource(String),
    WrongSourceSchema { expected: String, actual: String },
    NormalizationFailed(NormalizationError),
    SemanticMismatch,
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource(message) => {
                write!(formatter, "invalid migration source: {message}")
            }
            Self::WrongSourceSchema { expected, actual } => write!(
                formatter,
                "migration source schema must be {expected}, got {actual}"
            ),
            Self::NormalizationFailed(error) => {
                write!(
                    formatter,
                    "migrated authoring document does not normalize: {error}"
                )
            }
            Self::SemanticMismatch => {
                write!(
                    formatter,
                    "migration changed the canonical semantic document"
                )
            }
        }
    }
}

impl Error for MigrationError {}

struct NormalizedShortRequirement {
    requirement: CanonicalRequirement,
    applied_defaults: Vec<String>,
    inferred: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedDocument {
    pub document: SpecDocument,
    pub provenance: NormalizationProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizationProvenance {
    pub source_schema: String,
    pub target_schema: String,
    pub applied_defaults: Vec<String>,
    pub inferred: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENTS: [&str; 4] = [
        r#"
schema: mitase/authoring/v2
kind: philosophies
namespace: demo
category: Demo
philosophies: []
"#,
        r#"
schema: mitase/authoring/v2
kind: policies
namespace: demo
category: Demo
policies: []
"#,
        r#"
schema: mitase/authoring/v2
kind: requirements
namespace: demo
category: Demo
requirements: []
"#,
        r#"
schema: mitase/authoring/v2
kind: features
namespace: demo
category: Demo
features: []
"#,
    ];

    #[test]
    fn parses_and_normalizes_each_document_kind() {
        for source in DOCUMENTS {
            let document = AuthoringDocument::parse(source).expect("authoring document");
            let normalized = document.normalize().expect("normalization");
            assert_eq!(normalized.document.schema(), SPEC_SCHEMA);
            assert_eq!(
                normalized.provenance,
                NormalizationProvenance {
                    source_schema: AUTHORING_SCHEMA.into(),
                    target_schema: SPEC_SCHEMA.into(),
                    applied_defaults: vec![],
                    inferred: vec![],
                }
            );
        }
    }

    #[test]
    fn normalization_preserves_explicit_semantic_payload() {
        let source = r#"
schema: mitase/authoring/v2
kind: features
namespace: demo
category: Demo
features:
  - id: FEAT-DEMO-001
    title: Explicit feature
    summary: An explicit feature.
    status: implemented
    bindings:
      - id: implementation
        role: implementation
        facet: delivery
        responsibility: Own the exact implementation.
        targets:
          - id: source
            adapter: rust
            path: src/example.rs
            selector: { kind: file }
            claims:
              - kind: satisfies
                criterion: REQ-DEMO-001#criterion.behavior
"#;
        let authoring = AuthoringDocument::parse(source).expect("authoring document");
        let normalized = authoring.normalize().expect("normalization");
        let expected: SpecDocument =
            serde_yaml::from_str(&source.replace(AUTHORING_SCHEMA, SPEC_SCHEMA))
                .expect("canonical document");
        assert_eq!(normalized.document, expected);
    }

    const SHORT_DOCUMENT: &str = r#"
schema: mitase/authoring/v2
kind: requirement
namespace: demo
category: Demo
requirement:
  id: REQ-DEMO-001
  title: Explicit requirement
  description: The requirement intent stays explicit.
  priority: high
  status: implemented
  criterion:
    id: behavior
    kind: behavior
    statement: The behavior stays explicit.
    governed_by: []
  implementation:
    facet: delivery
    responsibility: Own the exact implementation.
    target:
      path: src/example.rs
      satisfies: behavior
  verification:
    facet: verification
    responsibility: Verify the exact behavior.
    target:
      path: tests/example.rs
      verifies:
        criterion: behavior
        covers: [source]
        runner: cargo-test
"#;

    #[test]
    fn short_contract_normalizes_one_requirement_without_inventing_meaning() {
        let authoring = AuthoringDocument::parse(SHORT_DOCUMENT).expect("short document");
        let normalized = authoring.normalize().expect("short normalization");
        let SpecDocument::Requirements { requirements, .. } = normalized.document else {
            panic!("short document must normalize to requirements");
        };
        let requirement = &requirements[0];
        assert_eq!(requirement.id, "REQ-DEMO-001".into());
        assert_eq!(
            requirement.criteria[0].statement,
            "The behavior stays explicit."
        );
        assert_eq!(requirement.bindings[0].role, BindingRole::Implementation);
        assert_eq!(requirement.bindings[0].targets[0].adapter, "rust");
        assert_eq!(
            requirement.bindings[0].targets[0].selector,
            ExactSelector::File
        );
        assert_eq!(
            requirement.bindings[0].targets[0].claims,
            vec![TargetClaim::Satisfies {
                criterion: "REQ-DEMO-001#criterion.behavior".parse().unwrap(),
            }]
        );
        assert_eq!(
            requirement.bindings[1].targets[0].claims,
            vec![TargetClaim::Verifies {
                criterion: "REQ-DEMO-001#criterion.behavior".parse().unwrap(),
                covers: vec![
                    "REQ-DEMO-001#binding.implementation/target.source"
                        .parse()
                        .unwrap()
                ],
                runner: VerificationRunnerRef {
                    runner: "cargo-test".into(),
                    arguments: BTreeMap::new(),
                },
            }]
        );
        assert_eq!(
            normalized.provenance.applied_defaults,
            vec![
                "requirement.implementation.id=implementation",
                "requirement.verification.id=verification",
                "requirement.implementation.target.id=source",
                "requirement.verification.target.id=test",
                "requirement.implementation.target.selector=file",
                "requirement.verification.target.selector=file",
            ]
        );
        assert_eq!(
            normalized.provenance.inferred,
            vec![
                "requirement.implementation.target.adapter=rust",
                "requirement.verification.target.adapter=rust",
            ]
        );
    }

    #[test]
    fn unique_resolver_fails_closed_for_zero_and_multiple_candidates() {
        assert!(matches!(
            resolve_unique::<String>("adapter", Vec::new()),
            Err(NormalizationError::NoCandidates { .. })
        ));
        assert_eq!(
            resolve_unique("adapter", vec![("rust".into(), "rust".to_string())]).unwrap(),
            ("rust".to_string(), "rust".to_string())
        );
        assert!(matches!(
            resolve_unique(
                "adapter",
                vec![
                    ("javascript".into(), "javascript".to_string()),
                    ("typescript".into(), "typescript".to_string()),
                ]
            ),
            Err(NormalizationError::Ambiguous { candidates, .. })
                if candidates == vec!["javascript", "typescript"]
        ));
    }

    #[test]
    fn short_contract_fails_closed_when_adapter_has_no_candidate() {
        let source = SHORT_DOCUMENT.replace("src/example.rs", "src/example.txt");
        let authoring = AuthoringDocument::parse(&source).expect("short document");
        assert!(matches!(
            authoring.normalize(),
            Err(NormalizationError::NoCandidates { field })
                if field == "requirement.implementation.target.adapter"
        ));
    }

    #[test]
    fn short_contract_fails_closed_when_extension_has_multiple_adapters() {
        let source = SHORT_DOCUMENT.replace("src/example.rs", "src/example.json");
        let authoring = AuthoringDocument::parse(&source).expect("short document");
        assert!(matches!(
            authoring.normalize(),
            Err(NormalizationError::Ambiguous { field, candidates })
                if field == "requirement.implementation.target.adapter"
                    && candidates == vec!["json", "json-schema", "openapi"]
        ));
    }

    #[test]
    fn rejects_unknown_fields_and_non_v2_schemas() {
        let unknown = format!("{}unknown: true\n", DOCUMENTS[0]);
        assert!(AuthoringDocument::parse(&unknown).is_err());
        assert!(
            AuthoringDocument::parse(&DOCUMENTS[0].replace(AUTHORING_SCHEMA, SPEC_SCHEMA,))
                .is_err()
        );
    }

    #[test]
    fn migration_preserves_every_canonical_document_kind() {
        for source in DOCUMENTS.map(|document| document.replace(AUTHORING_SCHEMA, SPEC_SCHEMA)) {
            let canonical: SpecDocument = serde_yaml::from_str(&source).expect("canonical source");
            let migrated = migrate_v1_document(&canonical).expect("migration");
            let normalized = migrated.normalize().expect("normalization");
            assert_eq!(normalized.document, canonical);
            assert_eq!(migrated.schema(), AUTHORING_SCHEMA);
        }
    }

    #[test]
    fn migration_is_deterministic_and_rejects_noncanonical_sources() {
        let source = DOCUMENTS[3].replace(AUTHORING_SCHEMA, SPEC_SCHEMA);
        let first = migrate_v1_to_v2(&source).expect("migration");
        let second = migrate_v1_to_v2(&source).expect("migration");
        assert_eq!(
            serde_yaml::to_string(&first).expect("serialize migration"),
            serde_yaml::to_string(&second).expect("serialize migration")
        );

        let wrong_schema = source.replace(SPEC_SCHEMA, AUTHORING_SCHEMA);
        assert!(matches!(
            migrate_v1_to_v2(&wrong_schema),
            Err(MigrationError::WrongSourceSchema { .. })
        ));
        assert!(matches!(
            migrate_v1_to_v2("not: valid"),
            Err(MigrationError::InvalidSource(_))
        ));
    }
}
