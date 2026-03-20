// FEAT-APP-001

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Philosophy,
    Policies,
    Features,
    Requirements,
}

impl SectionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Philosophy => "philosophy",
            Self::Policies => "policies",
            Self::Features => "features",
            Self::Requirements => "requirements",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub section: SectionKind,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefinitionCounts {
    pub philosophies: usize,
    pub policies: usize,
    pub requirements: usize,
    pub features: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceSummary {
    pub requirement_traces: TraceCount,
    pub feature_traces: TraceCount,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceCount {
    pub declared: usize,
    pub validated: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: Severity,
    pub subject: String,
    pub location: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencedRule {
    pub genre: String,
    pub code: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationSnapshot {
    pub definition_counts: DefinitionCounts,
    pub trace_summary: TraceSummary,
    pub issues: Vec<ValidationIssue>,
    pub referenced_rules: Vec<ReferencedRule>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppPayload {
    pub workspace_root: String,
    pub spec_root: String,
    pub source_documents: Vec<SourceDocument>,
    pub validation: ValidationSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserWorkspace {
    pub workspace_root: String,
    pub spec_root: String,
    pub sections: Vec<BrowserSection>,
    pub item_index: BTreeMap<String, BrowserIndexEntry>,
    pub validation: ValidationSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSection {
    pub kind: SectionKind,
    pub label: String,
    pub documents: Vec<BrowserDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDocument {
    pub section: SectionKind,
    pub path: String,
    pub title: String,
    pub folder_segments: Vec<String>,
    pub raw_yaml: String,
    pub parse_error: Option<String>,
    pub items: Vec<BrowserItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserItem {
    pub kind: SectionKind,
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub product_design_principle: Option<String>,
    pub coding_guideline: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub linked_philosophies: Vec<String>,
    pub linked_policies: Vec<String>,
    pub linked_requirements: Vec<String>,
    pub linked_features: Vec<String>,
    pub tests: Vec<BrowserTraceGroup>,
    pub implementations: Vec<BrowserTraceGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserIndexEntry {
    pub id: String,
    pub title: String,
    pub kind: SectionKind,
    pub document_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTraceGroup {
    pub language: String,
    pub references: Vec<BrowserTraceReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTraceReference {
    pub file: String,
    pub symbols: Vec<String>,
    pub doc_contains: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhilosophyDocument {
    category: String,
    version: u32,
    language: Option<String>,
    philosophies: Vec<Philosophy>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Philosophy {
    id: String,
    title: String,
    product_design_principle: String,
    coding_guideline: String,
    #[serde(default)]
    linked_policies: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyDocument {
    category: String,
    version: u32,
    language: Option<String>,
    policies: Vec<Policy>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    id: String,
    title: String,
    summary: String,
    description: String,
    #[serde(default)]
    linked_philosophies: Vec<String>,
    #[serde(default)]
    linked_requirements: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementDocument {
    category: String,
    prefix: String,
    requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Requirement {
    id: String,
    title: String,
    description: String,
    priority: String,
    status: String,
    #[serde(default)]
    linked_policies: Vec<String>,
    #[serde(default)]
    linked_features: Vec<String>,
    #[serde(default)]
    tests: BTreeMap<String, Vec<TraceReference>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureDocument {
    category: String,
    version: u32,
    features: Vec<Feature>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Feature {
    id: String,
    title: String,
    summary: String,
    status: String,
    #[serde(default)]
    linked_requirements: Vec<String>,
    #[serde(default)]
    implementations: BTreeMap<String, Vec<TraceReference>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceReference {
    file: String,
    #[serde(default, alias = "tests", alias = "functions")]
    symbols: Vec<String>,
    #[serde(default, alias = "docs", alias = "docstrings")]
    doc_contains: Vec<String>,
}

pub fn build_browser_workspace(payload: AppPayload) -> BrowserWorkspace {
    let mut documents_by_section: BTreeMap<SectionKind, Vec<BrowserDocument>> = BTreeMap::new();

    for source in payload.source_documents {
        documents_by_section
            .entry(source.section)
            .or_default()
            .push(parse_source_document(source));
    }

    let mut item_index = BTreeMap::new();
    let sections = [
        SectionKind::Philosophy,
        SectionKind::Policies,
        SectionKind::Features,
        SectionKind::Requirements,
    ]
    .into_iter()
    .map(|kind| {
        let mut documents = documents_by_section.remove(&kind).unwrap_or_default();
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        for document in &documents {
            for item in &document.items {
                item_index.insert(
                    item.id.clone(),
                    BrowserIndexEntry {
                        id: item.id.clone(),
                        title: item.title.clone(),
                        kind: item.kind,
                        document_path: document.path.clone(),
                    },
                );
            }
        }

        BrowserSection {
            kind,
            label: kind.label().to_string(),
            documents,
        }
    })
    .collect();

    BrowserWorkspace {
        workspace_root: payload.workspace_root,
        spec_root: payload.spec_root,
        sections,
        item_index,
        validation: payload.validation,
    }
}

fn parse_source_document(source: SourceDocument) -> BrowserDocument {
    let title_from_path = source
        .path
        .rsplit('/')
        .next()
        .unwrap_or(source.path.as_str())
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .to_string();
    let folder_segments = folder_segments(&source.path);
    let raw_yaml = source.content.clone();

    match source.section {
        SectionKind::Philosophy => {
            match serde_yaml::from_str::<PhilosophyDocument>(&source.content) {
                Ok(document) => BrowserDocument {
                    section: source.section,
                    path: source.path,
                    title: document.category,
                    folder_segments,
                    raw_yaml,
                    parse_error: None,
                    items: document
                        .philosophies
                        .into_iter()
                        .map(|item| BrowserItem {
                            kind: SectionKind::Philosophy,
                            id: item.id,
                            title: item.title,
                            summary: None,
                            description: None,
                            product_design_principle: Some(item.product_design_principle),
                            coding_guideline: Some(item.coding_guideline),
                            priority: None,
                            status: None,
                            linked_philosophies: Vec::new(),
                            linked_policies: item.linked_policies,
                            linked_requirements: Vec::new(),
                            linked_features: Vec::new(),
                            tests: Vec::new(),
                            implementations: Vec::new(),
                        })
                        .collect(),
                },
                Err(error) => invalid_document(
                    source.section,
                    source.path,
                    title_from_path,
                    folder_segments,
                    raw_yaml,
                    error,
                ),
            }
        }
        SectionKind::Policies => match serde_yaml::from_str::<PolicyDocument>(&source.content) {
            Ok(document) => BrowserDocument {
                section: source.section,
                path: source.path,
                title: document.category,
                folder_segments,
                raw_yaml,
                parse_error: None,
                items: document
                    .policies
                    .into_iter()
                    .map(|item| BrowserItem {
                        kind: SectionKind::Policies,
                        id: item.id,
                        title: item.title,
                        summary: Some(item.summary),
                        description: Some(item.description),
                        product_design_principle: None,
                        coding_guideline: None,
                        priority: None,
                        status: None,
                        linked_philosophies: item.linked_philosophies,
                        linked_policies: Vec::new(),
                        linked_requirements: item.linked_requirements,
                        linked_features: Vec::new(),
                        tests: Vec::new(),
                        implementations: Vec::new(),
                    })
                    .collect(),
            },
            Err(error) => invalid_document(
                source.section,
                source.path,
                title_from_path,
                folder_segments,
                raw_yaml,
                error,
            ),
        },
        SectionKind::Requirements => {
            match serde_yaml::from_str::<RequirementDocument>(&source.content) {
                Ok(document) => BrowserDocument {
                    section: source.section,
                    path: source.path,
                    title: document.category,
                    folder_segments,
                    raw_yaml,
                    parse_error: None,
                    items: document
                        .requirements
                        .into_iter()
                        .map(|item| BrowserItem {
                            kind: SectionKind::Requirements,
                            id: item.id,
                            title: item.title,
                            summary: None,
                            description: Some(item.description),
                            product_design_principle: None,
                            coding_guideline: None,
                            priority: Some(item.priority),
                            status: Some(item.status),
                            linked_philosophies: Vec::new(),
                            linked_policies: item.linked_policies,
                            linked_requirements: Vec::new(),
                            linked_features: item.linked_features,
                            tests: browser_trace_groups(item.tests),
                            implementations: Vec::new(),
                        })
                        .collect(),
                },
                Err(error) => invalid_document(
                    source.section,
                    source.path,
                    title_from_path,
                    folder_segments,
                    raw_yaml,
                    error,
                ),
            }
        }
        SectionKind::Features => match serde_yaml::from_str::<FeatureDocument>(&source.content) {
            Ok(document) => BrowserDocument {
                section: source.section,
                path: source.path,
                title: document.category,
                folder_segments,
                raw_yaml,
                parse_error: None,
                items: document
                    .features
                    .into_iter()
                    .map(|item| BrowserItem {
                        kind: SectionKind::Features,
                        id: item.id,
                        title: item.title,
                        summary: Some(item.summary),
                        description: None,
                        product_design_principle: None,
                        coding_guideline: None,
                        priority: None,
                        status: Some(item.status),
                        linked_philosophies: Vec::new(),
                        linked_policies: Vec::new(),
                        linked_requirements: item.linked_requirements,
                        linked_features: Vec::new(),
                        tests: Vec::new(),
                        implementations: browser_trace_groups(item.implementations),
                    })
                    .collect(),
            },
            Err(error) => invalid_document(
                source.section,
                source.path,
                title_from_path,
                folder_segments,
                raw_yaml,
                error,
            ),
        },
    }
}

fn invalid_document(
    section: SectionKind,
    path: String,
    title: String,
    folder_segments: Vec<String>,
    raw_yaml: String,
    error: serde_yaml::Error,
) -> BrowserDocument {
    BrowserDocument {
        section,
        path,
        title,
        folder_segments,
        raw_yaml,
        parse_error: Some(error.to_string()),
        items: Vec::new(),
    }
}

fn browser_trace_groups(traces: BTreeMap<String, Vec<TraceReference>>) -> Vec<BrowserTraceGroup> {
    traces
        .into_iter()
        .map(|(language, references)| BrowserTraceGroup {
            language,
            references: references
                .into_iter()
                .map(|reference| BrowserTraceReference {
                    file: reference.file,
                    symbols: reference.symbols,
                    doc_contains: reference.doc_contains,
                })
                .collect(),
        })
        .collect()
}

fn folder_segments(path: &str) -> Vec<String> {
    let mut segments: Vec<String> = path.split('/').map(str::to_string).collect();
    segments.pop();
    segments
}

#[cfg(test)]
mod tests {
    use super::{
        AppPayload, DefinitionCounts, ReferencedRule, SectionKind, Severity, SourceDocument,
        TraceCount, TraceSummary, ValidationIssue, ValidationSnapshot, build_browser_workspace,
    };

    fn sample_validation() -> ValidationSnapshot {
        ValidationSnapshot {
            definition_counts: DefinitionCounts {
                philosophies: 1,
                policies: 1,
                requirements: 1,
                features: 1,
            },
            trace_summary: TraceSummary {
                requirement_traces: TraceCount {
                    declared: 1,
                    validated: 1,
                },
                feature_traces: TraceCount {
                    declared: 1,
                    validated: 1,
                },
            },
            issues: vec![ValidationIssue {
                code: "SYU-graph-reference-001".to_string(),
                severity: Severity::Error,
                subject: "requirement".to_string(),
                location: Some("docs/syu/requirements/core.yaml".to_string()),
                message: "broken link".to_string(),
                suggestion: Some("fix the link".to_string()),
            }],
            referenced_rules: vec![ReferencedRule {
                genre: "graph".to_string(),
                code: "SYU-graph-reference-001".to_string(),
                severity: "error".to_string(),
                title: "Linked definitions must exist".to_string(),
                summary: "Missing links break the graph.".to_string(),
                description: "desc".to_string(),
            }],
        }
    }

    #[test]
    fn builds_workspace_and_indexes_items() {
        let workspace = build_browser_workspace(AppPayload {
            workspace_root: "/repo".to_string(),
            spec_root: "/repo/docs/syu".to_string(),
            source_documents: vec![
                SourceDocument {
                    section: SectionKind::Philosophy,
                    path: "foundation.yaml".to_string(),
                    content: "category: Philosophy\nversion: 1\nphilosophies:\n  - id: PHIL-001\n    title: Stable value\n    product_design_principle: Keep it explainable.\n    coding_guideline: Prefer shared logic.\n    linked_policies:\n      - POL-001\n".to_string(),
                },
                SourceDocument {
                    section: SectionKind::Policies,
                    path: "rules/core.yaml".to_string(),
                    content: "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep links explicit\n    summary: summary\n    description: description\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n".to_string(),
                },
                SourceDocument {
                    section: SectionKind::Requirements,
                    path: "core.yaml".to_string(),
                    content: "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-001\n    title: Browser view\n    description: Show the spec.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/app.rs\n          symbols:\n            - smoke_test\n".to_string(),
                },
                SourceDocument {
                    section: SectionKind::Features,
                    path: "browser/app.yaml".to_string(),
                    content: "category: App\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Browser app\n    summary: Explore layers in the browser.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/command/app.rs\n          symbols:\n            - run_app_command\n".to_string(),
                },
            ],
            validation: sample_validation(),
        });

        assert_eq!(workspace.sections.len(), 4);
        assert_eq!(workspace.sections[0].documents[0].items[0].id, "PHIL-001");
        assert_eq!(
            workspace.sections[1].documents[0].folder_segments,
            vec!["rules".to_string()]
        );
        assert_eq!(
            workspace
                .item_index
                .get("FEAT-001")
                .map(|entry| entry.document_path.as_str()),
            Some("browser/app.yaml")
        );
        assert_eq!(workspace.validation.issues.len(), 1);
    }

    #[test]
    fn preserves_parse_errors_for_invalid_documents() {
        let workspace = build_browser_workspace(AppPayload {
            workspace_root: "/repo".to_string(),
            spec_root: "/repo/docs/syu".to_string(),
            source_documents: vec![SourceDocument {
                section: SectionKind::Features,
                path: "broken.yaml".to_string(),
                content: "category: Broken\nversion: [\n".to_string(),
            }],
            validation: ValidationSnapshot::default(),
        });

        let document = &workspace.sections[2].documents[0];
        assert_eq!(document.path, "broken.yaml");
        assert!(document.parse_error.is_some());
        assert!(document.items.is_empty());
    }
}
