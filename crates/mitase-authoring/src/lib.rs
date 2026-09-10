#![forbid(unsafe_code)]

use mitase_spec_model::{Feature, Philosophy, Policy, Requirement, SPEC_SCHEMA, SpecDocument};
use serde::{Deserialize, Serialize};

/// Schema identifier for the human-facing v0.2 authoring contract.
pub const AUTHORING_SCHEMA: &str = "mitase/authoring/v2";

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
        requirements: Vec<Requirement>,
    },
    Features {
        schema: String,
        namespace: String,
        category: String,
        features: Vec<Feature>,
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
            | Self::Features { schema, .. } => schema,
        }
    }

    /// Normalize authoring syntax into the canonical semantic document.
    ///
    /// This method only changes the schema boundary in this first slice. It
    /// does not infer normative meaning or alter IDs, relations, targets, or
    /// claims.
    pub fn normalize(&self) -> NormalizedDocument {
        let document = match self {
            Self::Philosophies {
                namespace,
                category,
                philosophies,
                ..
            } => SpecDocument::Philosophies {
                schema: SPEC_SCHEMA.into(),
                namespace: namespace.clone(),
                category: category.clone(),
                philosophies: philosophies.clone(),
            },
            Self::Policies {
                namespace,
                category,
                policies,
                ..
            } => SpecDocument::Policies {
                schema: SPEC_SCHEMA.into(),
                namespace: namespace.clone(),
                category: category.clone(),
                policies: policies.clone(),
            },
            Self::Requirements {
                namespace,
                category,
                requirements,
                ..
            } => SpecDocument::Requirements {
                schema: SPEC_SCHEMA.into(),
                namespace: namespace.clone(),
                category: category.clone(),
                requirements: requirements.clone(),
            },
            Self::Features {
                namespace,
                category,
                features,
                ..
            } => SpecDocument::Features {
                schema: SPEC_SCHEMA.into(),
                namespace: namespace.clone(),
                category: category.clone(),
                features: features.clone(),
            },
        };
        NormalizedDocument {
            document,
            provenance: NormalizationProvenance {
                source_schema: AUTHORING_SCHEMA.into(),
                target_schema: SPEC_SCHEMA.into(),
                applied_defaults: Vec::new(),
                inferred: Vec::new(),
            },
        }
    }
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
            let normalized = document.normalize();
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
        let normalized = authoring.normalize();
        let expected: SpecDocument =
            serde_yaml::from_str(&source.replace(AUTHORING_SCHEMA, SPEC_SCHEMA))
                .expect("canonical document");
        assert_eq!(normalized.document, expected);
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
}
