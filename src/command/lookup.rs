// FEAT-SEARCH-001
// FEAT-LOG-001
// REQ-CORE-019
// REQ-CORE-018

use std::path::Path;

use anyhow::{Result, bail};

use serde::Serialize;

use crate::{
    cli::LookupKind,
    model::{Feature, Philosophy, Policy, Requirement},
    workspace::{
        Workspace, load_feature_documents_with_paths, load_philosophy_documents_with_paths,
        load_policy_documents_with_paths, load_requirement_documents_with_paths,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EntitySummary {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SearchResult {
    pub id: String,
    pub kind: &'static str,
    pub title: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkspaceEntity<'a> {
    Philosophy(&'a Philosophy),
    Policy(&'a Policy),
    Requirement(&'a Requirement),
    Feature(&'a Feature),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkspaceLookup<'a> {
    workspace: &'a Workspace,
}

impl<'a> WorkspaceLookup<'a> {
    pub(crate) fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    pub(crate) fn entries(self, kind: LookupKind) -> Vec<EntitySummary> {
        match kind {
            LookupKind::Philosophy => self
                .workspace
                .philosophies
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    document_path: None,
                })
                .collect(),
            LookupKind::Policy => self
                .workspace
                .policies
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    document_path: None,
                })
                .collect(),
            LookupKind::Requirement => self
                .workspace
                .requirements
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    document_path: None,
                })
                .collect(),
            LookupKind::Feature => self
                .workspace
                .features
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    document_path: None,
                })
                .collect(),
        }
    }

    pub(crate) fn entries_with_document_paths(
        self,
        kind: LookupKind,
    ) -> Result<Vec<EntitySummary>> {
        let document_paths = self.document_paths(kind)?;
        let items = self.entries(kind);

        if items.len() != document_paths.len() {
            bail!(
                "workspace {} entries changed while collecting document paths",
                kind.label()
            );
        }

        Ok(items
            .into_iter()
            .zip(document_paths)
            .map(|(mut item, document_path)| {
                item.document_path = Some(document_path);
                item
            })
            .collect())
    }

    pub(crate) fn document_path_for_id(self, id: &str) -> Result<Option<String>> {
        let Some(kind) = kind_for_id(id) else {
            return Ok(None);
        };

        Ok(self
            .entries_with_document_paths(kind)?
            .into_iter()
            .find(|item| item.id == id)
            .and_then(|item| item.document_path))
    }

    pub(crate) fn title_for(self, kind: LookupKind, id: &str) -> Option<&'a str> {
        match kind {
            LookupKind::Philosophy => self.philosophy(id).map(|item| item.title.as_str()),
            LookupKind::Policy => self.policy(id).map(|item| item.title.as_str()),
            LookupKind::Requirement => self.requirement(id).map(|item| item.title.as_str()),
            LookupKind::Feature => self.feature(id).map(|item| item.title.as_str()),
        }
    }

    pub(crate) fn philosophy(self, id: &str) -> Option<&'a Philosophy> {
        self.workspace
            .philosophies
            .iter()
            .find(|item| item.id == id)
    }

    pub(crate) fn policy(self, id: &str) -> Option<&'a Policy> {
        self.workspace.policies.iter().find(|item| item.id == id)
    }

    pub(crate) fn requirement(self, id: &str) -> Option<&'a Requirement> {
        self.workspace
            .requirements
            .iter()
            .find(|item| item.id == id)
    }

    pub(crate) fn feature(self, id: &str) -> Option<&'a Feature> {
        self.workspace.features.iter().find(|item| item.id == id)
    }

    pub(crate) fn find(self, id: &str) -> Option<WorkspaceEntity<'a>> {
        if id.starts_with("PHIL-") {
            return self.philosophy(id).map(WorkspaceEntity::Philosophy);
        }
        if id.starts_with("POL-") {
            return self.policy(id).map(WorkspaceEntity::Policy);
        }
        if id.starts_with("REQ-") {
            return self.requirement(id).map(WorkspaceEntity::Requirement);
        }
        if id.starts_with("FEAT-") {
            return self.feature(id).map(WorkspaceEntity::Feature);
        }

        None
    }

    pub(crate) fn search(self, query: &str, kind: Option<LookupKind>) -> Vec<SearchResult> {
        let query = query.trim().to_lowercase();
        let mut results = Vec::new();

        match kind {
            Some(kind) => self.extend_search_results(kind, &query, &mut results),
            None => {
                for kind in [
                    LookupKind::Philosophy,
                    LookupKind::Policy,
                    LookupKind::Feature,
                    LookupKind::Requirement,
                ] {
                    self.extend_search_results(kind, &query, &mut results);
                }
            }
        }

        results
    }

    fn document_paths(self, kind: LookupKind) -> Result<Vec<String>> {
        match kind {
            LookupKind::Philosophy => Ok(load_philosophy_documents_with_paths(
                &self.workspace.spec_root.join("philosophy"),
            )?
            .into_iter()
            .flat_map(|loaded| {
                let path = workspace_relative_display(self.workspace, &loaded.path);
                loaded
                    .document
                    .philosophies
                    .into_iter()
                    .map(move |_| path.clone())
            })
            .collect()),
            LookupKind::Policy => Ok(load_policy_documents_with_paths(
                &self.workspace.spec_root.join("policies"),
            )?
            .into_iter()
            .flat_map(|loaded| {
                let path = workspace_relative_display(self.workspace, &loaded.path);
                loaded
                    .document
                    .policies
                    .into_iter()
                    .map(move |_| path.clone())
            })
            .collect()),
            LookupKind::Requirement => Ok(load_requirement_documents_with_paths(
                &self.workspace.spec_root.join("requirements"),
            )?
            .into_iter()
            .flat_map(|loaded| {
                let path = workspace_relative_display(self.workspace, &loaded.path);
                loaded
                    .document
                    .requirements
                    .into_iter()
                    .map(move |_| path.clone())
            })
            .collect()),
            LookupKind::Feature => Ok(load_feature_documents_with_paths(
                &self.workspace.spec_root.join("features"),
            )?
            .into_iter()
            .flat_map(|loaded| {
                let path = workspace_relative_display(self.workspace, &loaded.path);
                loaded
                    .document
                    .features
                    .into_iter()
                    .map(move |_| path.clone())
            })
            .collect()),
        }
    }

    fn extend_search_results(self, kind: LookupKind, query: &str, results: &mut Vec<SearchResult>) {
        match kind {
            LookupKind::Philosophy => {
                for item in &self.workspace.philosophies {
                    if field_matches_query(&item.id, query)
                        || field_matches_query(&item.title, query)
                    {
                        results.push(SearchResult {
                            id: item.id.clone(),
                            kind: kind.label(),
                            title: item.title.clone(),
                        });
                    }
                }
            }
            LookupKind::Policy => {
                for item in &self.workspace.policies {
                    if field_matches_query(&item.id, query)
                        || field_matches_query(&item.title, query)
                        || field_matches_query(&item.summary, query)
                        || field_matches_query(&item.description, query)
                    {
                        results.push(SearchResult {
                            id: item.id.clone(),
                            kind: kind.label(),
                            title: item.title.clone(),
                        });
                    }
                }
            }
            LookupKind::Requirement => {
                for item in &self.workspace.requirements {
                    if field_matches_query(&item.id, query)
                        || field_matches_query(&item.title, query)
                        || field_matches_query(&item.description, query)
                    {
                        results.push(SearchResult {
                            id: item.id.clone(),
                            kind: kind.label(),
                            title: item.title.clone(),
                        });
                    }
                }
            }
            LookupKind::Feature => {
                for item in &self.workspace.features {
                    if field_matches_query(&item.id, query)
                        || field_matches_query(&item.title, query)
                        || field_matches_query(&item.summary, query)
                    {
                        results.push(SearchResult {
                            id: item.id.clone(),
                            kind: kind.label(),
                            title: item.title.clone(),
                        });
                    }
                }
            }
        }
    }
}

fn workspace_relative_display(workspace: &Workspace, path: &Path) -> String {
    path.strip_prefix(&workspace.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn field_matches_query(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

fn kind_for_id(id: &str) -> Option<LookupKind> {
    if id.starts_with("PHIL-") {
        return Some(LookupKind::Philosophy);
    }
    if id.starts_with("POL-") {
        return Some(LookupKind::Policy);
    }
    if id.starts_with("REQ-") {
        return Some(LookupKind::Requirement);
    }
    if id.starts_with("FEAT-") {
        return Some(LookupKind::Feature);
    }

    None
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;

    use crate::{
        cli::LookupKind,
        config::SyuConfig,
        model::{Feature, Policy, Requirement},
        workspace::Workspace,
    };

    use super::{WorkspaceLookup, kind_for_id};

    #[test]
    fn entries_with_document_paths_reports_missing_philosophy_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("external-spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Philosophy)
            .expect_err("missing philosophy directory should surface an error");
        assert!(error.to_string().contains("missing philosophy directory"));
    }

    #[test]
    fn entries_with_document_paths_reports_missing_requirement_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("external-spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Requirement)
            .expect_err("missing requirement directory should surface an error");
        assert!(error.to_string().contains("missing requirement directory"));
    }

    #[test]
    fn entries_with_document_paths_reports_missing_policy_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("external-spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Policy)
            .expect_err("missing policy directory should surface an error");
        assert!(error.to_string().contains("missing policy directory"));
    }

    #[test]
    fn entries_with_document_paths_reports_missing_feature_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("external-spec"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Feature)
            .expect_err("missing feature directory should surface an error");
        assert!(error.to_string().contains("feature registry"));
    }

    #[test]
    fn document_path_for_id_returns_none_for_unknown_prefixes() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let document_path = WorkspaceLookup::new(&workspace)
            .document_path_for_id("UNKNOWN-001")
            .expect("unknown prefixes should not error");
        assert!(document_path.is_none());
    }

    #[test]
    fn kind_for_id_recognizes_supported_prefixes() {
        assert!(matches!(
            kind_for_id("PHIL-001"),
            Some(LookupKind::Philosophy)
        ));
        assert!(matches!(kind_for_id("POL-001"), Some(LookupKind::Policy)));
        assert!(matches!(
            kind_for_id("REQ-001"),
            Some(LookupKind::Requirement)
        ));
        assert!(matches!(kind_for_id("FEAT-001"), Some(LookupKind::Feature)));
        assert!(kind_for_id("UNKNOWN-001").is_none());
    }

    #[test]
    fn entries_with_document_paths_keep_duplicate_requirement_ids_on_their_own_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let spec_root = workspace_root.join("docs/syu");
        std::fs::create_dir_all(spec_root.join("requirements/core")).expect("requirements dir");
        std::fs::write(
            spec_root.join("requirements/core/alpha.yaml"),
            "category: Core Requirements\nprefix: REQ-DUP\n\nrequirements:\n  - id: REQ-DUP-001\n    title: Alpha copy\n    description: First duplicate.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("alpha requirement should write");
        std::fs::write(
            spec_root.join("requirements/core/beta.yaml"),
            "category: Core Requirements\nprefix: REQ-DUP\n\nrequirements:\n  - id: REQ-DUP-001\n    title: Beta copy\n    description: Second duplicate.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("beta requirement should write");

        let workspace = Workspace {
            root: workspace_root,
            spec_root,
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: vec![
                Requirement {
                    id: "REQ-DUP-001".to_string(),
                    title: "Alpha copy".to_string(),
                    description: "First duplicate.".to_string(),
                    priority: "high".to_string(),
                    status: "planned".to_string(),
                    linked_policies: Vec::new(),
                    linked_features: Vec::new(),
                    tests: BTreeMap::new(),
                },
                Requirement {
                    id: "REQ-DUP-001".to_string(),
                    title: "Beta copy".to_string(),
                    description: "Second duplicate.".to_string(),
                    priority: "high".to_string(),
                    status: "planned".to_string(),
                    linked_policies: Vec::new(),
                    linked_features: Vec::new(),
                    tests: BTreeMap::new(),
                },
            ],
            features: Vec::new(),
        };

        let entries = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Requirement)
            .expect("duplicate IDs should still keep per-document paths");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "REQ-DUP-001");
        assert_eq!(entries[0].title, "Alpha copy");
        assert_eq!(
            entries[0].document_path.as_deref(),
            Some("docs/syu/requirements/core/alpha.yaml")
        );
        assert_eq!(entries[1].id, "REQ-DUP-001");
        assert_eq!(entries[1].title, "Beta copy");
        assert_eq!(
            entries[1].document_path.as_deref(),
            Some("docs/syu/requirements/core/beta.yaml")
        );
    }

    #[test]
    fn entries_with_document_paths_attach_policy_and_feature_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let spec_root = workspace_root.join("docs/syu");
        std::fs::create_dir_all(spec_root.join("policies")).expect("policies dir");
        std::fs::create_dir_all(spec_root.join("features/core")).expect("features dir");
        std::fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-TEST-001\n    title: Policy path\n    summary: Policy stays discoverable.\n    description: Keep the source file visible in list output.\n    linked_philosophies: []\n    linked_requirements: []\n",
        )
        .expect("policy should write");
        std::fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.0.1-alpha.7'\nupdated: 'generated by test'\n\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
        )
        .expect("feature registry should write");
        std::fs::write(
            spec_root.join("features/core/core.yaml"),
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-TEST-001\n    title: Feature path\n    summary: Feature stays discoverable.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
        )
        .expect("feature should write");

        let workspace = Workspace {
            root: workspace_root,
            spec_root,
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: vec![Policy {
                id: "POL-TEST-001".to_string(),
                title: "Policy path".to_string(),
                summary: "Policy stays discoverable.".to_string(),
                description: "Keep the source file visible in list output.".to_string(),
                linked_philosophies: Vec::new(),
                linked_requirements: Vec::new(),
            }],
            requirements: Vec::new(),
            features: vec![Feature {
                id: "FEAT-TEST-001".to_string(),
                title: "Feature path".to_string(),
                summary: "Feature stays discoverable.".to_string(),
                status: "planned".to_string(),
                linked_requirements: Vec::new(),
                implementations: BTreeMap::new(),
            }],
        };

        let policy_entries = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Policy)
            .expect("policy paths should load");
        assert_eq!(
            policy_entries[0].document_path.as_deref(),
            Some("docs/syu/policies/policies.yaml")
        );

        let feature_entries = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Feature)
            .expect("feature paths should load");
        assert_eq!(
            feature_entries[0].document_path.as_deref(),
            Some("docs/syu/features/core/core.yaml")
        );
    }

    #[test]
    fn entries_with_document_paths_error_when_workspace_items_do_not_match_loaded_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let spec_root = workspace_root.join("docs/syu");
        std::fs::create_dir_all(spec_root.join("requirements/core")).expect("requirements dir");
        std::fs::write(
            spec_root.join("requirements/core/core.yaml"),
            "category: Core Requirements\nprefix: REQ-SYNC\n\nrequirements:\n  - id: REQ-SYNC-001\n    title: On disk only\n    description: Exercises mismatch handling.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("requirement should write");

        let workspace = Workspace {
            root: workspace_root,
            spec_root,
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = WorkspaceLookup::new(&workspace)
            .entries_with_document_paths(LookupKind::Requirement)
            .expect_err("mismatched workspace items should error");
        assert!(
            error
                .to_string()
                .contains("workspace requirement entries changed while collecting document paths")
        );
    }

    #[test]
    fn search_matches_browser_search_fields_only() {
        let workspace = Workspace {
            root: std::path::PathBuf::from("."),
            spec_root: std::path::PathBuf::from("./docs/syu"),
            config: SyuConfig::default(),
            philosophies: vec![crate::model::Philosophy {
                id: "PHIL-001".to_string(),
                title: "Trace everything".to_string(),
                product_design_principle: "Long-form principle text".to_string(),
                coding_guideline: "Long-form coding text".to_string(),
                linked_policies: Vec::new(),
            }],
            policies: vec![Policy {
                id: "POL-001".to_string(),
                title: "Policy title".to_string(),
                summary: "Summary field".to_string(),
                description: "Description field".to_string(),
                linked_philosophies: Vec::new(),
                linked_requirements: Vec::new(),
            }],
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Requirement title".to_string(),
                description: "Requirement description".to_string(),
                priority: "high".to_string(),
                status: "planned".to_string(),
                linked_policies: Vec::new(),
                linked_features: Vec::new(),
                tests: BTreeMap::new(),
            }],
            features: vec![Feature {
                id: "FEAT-001".to_string(),
                title: "Feature title".to_string(),
                summary: "Feature summary".to_string(),
                status: "planned".to_string(),
                linked_requirements: Vec::new(),
                implementations: BTreeMap::new(),
            }],
        };

        let lookup = WorkspaceLookup::new(&workspace);
        assert_eq!(lookup.search("summary", None)[0].id, "POL-001");
        assert_eq!(lookup.search("description", None)[0].id, "POL-001");
        assert_eq!(lookup.search("feature summary", None)[0].id, "FEAT-001");
        assert!(lookup.search("Long-form principle", None).is_empty());
    }
}
