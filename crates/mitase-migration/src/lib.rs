//! Legacy `mitase/spec/v1` migration input, isolated from normal loading.
//!
//! Only the explicit read-only `mitase migrate <source> --stdout` command
//! uses this crate. Normal workspace loading, validation, queries, reports,
//! and editor integration never depend on it.

#![forbid(unsafe_code)]

use mitase_authoring::{AUTHORING_SCHEMA, AuthoringDocument, NormalizationError};
use mitase_spec_model::{Feature, Philosophy, Policy, Requirement, SemanticDocument};
use serde::Deserialize;
use std::{error::Error, fmt};

/// Schema identifier for legacy v1 migration input.
///
/// This constant lives only in the migration crate. Normal workspace loading
/// accepts `mitase/authoring/v2` and rejects this schema with
/// `MITASE-SOURCE-001`.
pub const LEGACY_V1_SCHEMA: &str = "mitase/spec/v1";

/// Legacy v1 input AST.
///
/// This is a migration-only input representation. It is never a canonical or
/// semantic document: callers convert it into an [`AuthoringDocument`] and
/// normalize from there.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum LegacyV1Document {
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

impl LegacyV1Document {
    /// Parse one legacy v1 source document.
    pub fn parse(source: &str) -> Result<Self, MigrationError> {
        let document: Self =
            serde_yaml::from_str(source).map_err(|error| MigrationError::InvalidSource {
                message: error.to_string(),
            })?;
        document.validate_schema()?;
        Ok(document)
    }

    fn validate_schema(&self) -> Result<(), MigrationError> {
        let schema = match self {
            Self::Philosophies { schema, .. }
            | Self::Policies { schema, .. }
            | Self::Requirements { schema, .. }
            | Self::Features { schema, .. } => schema,
        };
        if schema != LEGACY_V1_SCHEMA {
            return Err(MigrationError::WrongSourceSchema {
                expected: LEGACY_V1_SCHEMA.into(),
                actual: schema.clone(),
            });
        }
        Ok(())
    }

    /// Project this legacy input into its schema-less semantic representation.
    pub fn into_semantic(self) -> SemanticDocument {
        match self {
            Self::Philosophies {
                namespace,
                category,
                philosophies,
                ..
            } => SemanticDocument::Philosophies {
                namespace,
                category,
                philosophies,
            },
            Self::Policies {
                namespace,
                category,
                policies,
                ..
            } => SemanticDocument::Policies {
                namespace,
                category,
                policies,
            },
            Self::Requirements {
                namespace,
                category,
                requirements,
                ..
            } => SemanticDocument::Requirements {
                namespace,
                category,
                requirements,
            },
            Self::Features {
                namespace,
                category,
                features,
                ..
            } => SemanticDocument::Features {
                namespace,
                category,
                features,
            },
        }
    }

    /// Convert legacy input into explicit v0.2 authoring syntax.
    pub fn into_authoring_document(self) -> Result<AuthoringDocument, MigrationError> {
        migrate_v1_document(&self)
    }
}

/// Convert one legacy v1 source string into explicit v0.2 authoring syntax.
pub fn migrate_v1_to_v2(source: &str) -> Result<AuthoringDocument, MigrationError> {
    LegacyV1Document::parse(source)?.into_authoring_document()
}

/// Convert a parsed legacy document and prove that the v0.2 result
/// carries the exact source meaning.
pub fn migrate_v1_document(legacy: &LegacyV1Document) -> Result<AuthoringDocument, MigrationError> {
    let authoring = match legacy {
        LegacyV1Document::Philosophies {
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
        LegacyV1Document::Policies {
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
        LegacyV1Document::Requirements {
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
        LegacyV1Document::Features {
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
    if normalized.document != legacy.clone().into_semantic() {
        return Err(MigrationError::SemanticMismatch);
    }
    Ok(authoring)
}

/// Migration-specific failure model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    InvalidSource { message: String },
    WrongSourceSchema { expected: String, actual: String },
    NormalizationFailed(NormalizationError),
    SemanticMismatch,
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource { message } => {
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
                write!(formatter, "migration changed the source semantic document")
            }
        }
    }
}

impl Error for MigrationError {}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENTS: [&str; 4] = [
        r#"
schema: mitase/spec/v1
kind: philosophies
namespace: demo
category: Demo
philosophies: []
"#,
        r#"
schema: mitase/spec/v1
kind: policies
namespace: demo
category: Demo
policies: []
"#,
        r#"
schema: mitase/spec/v1
kind: requirements
namespace: demo
category: Demo
requirements: []
"#,
        r#"
schema: mitase/spec/v1
kind: features
namespace: demo
category: Demo
features: []
"#,
    ];

    #[test]
    fn legacy_schema_marker_is_isolated_in_the_migration_crate() {
        assert_eq!(LEGACY_V1_SCHEMA, "mitase/spec/v1");
    }

    #[test]
    fn migration_preserves_every_legacy_document_kind() {
        for source in DOCUMENTS {
            let legacy = LegacyV1Document::parse(source).expect("legacy source");
            let migrated = migrate_v1_document(&legacy).expect("migration");
            let normalized = migrated.normalize().expect("normalization");
            assert_eq!(normalized.document, legacy.clone().into_semantic());
            assert_eq!(migrated.schema(), AUTHORING_SCHEMA);
        }
    }

    #[test]
    fn migration_is_deterministic_and_rejects_non_legacy_sources() {
        let first = migrate_v1_to_v2(DOCUMENTS[3]).expect("migration");
        let second = migrate_v1_to_v2(DOCUMENTS[3]).expect("migration");
        assert_eq!(
            serde_yaml::to_string(&first).expect("serialize migration"),
            serde_yaml::to_string(&second).expect("serialize migration")
        );

        let wrong_schema = DOCUMENTS[3].replace(LEGACY_V1_SCHEMA, AUTHORING_SCHEMA);
        assert!(matches!(
            migrate_v1_to_v2(&wrong_schema),
            Err(MigrationError::WrongSourceSchema { .. })
        ));
        assert!(matches!(
            migrate_v1_to_v2("not: valid"),
            Err(MigrationError::InvalidSource { .. })
        ));
    }
}
