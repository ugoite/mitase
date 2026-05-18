// FEAT-APP-001

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

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
pub struct HistoricalIdSnapshot {
    pub enabled: bool,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_ref: Option<String>,
    pub ids_by_section: BTreeMap<SectionKind, Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppPayload {
    pub workspace_root: String,
    pub spec_root: String,
    pub app_server: AppServer,
    pub source_documents: Vec<SourceDocument>,
    pub validation: ValidationSnapshot,
    pub historical_ids: HistoricalIdSnapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppServer {
    pub bind: String,
    pub port: u16,
    pub remotely_reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserWorkspace {
    pub workspace_root: String,
    pub spec_root: String,
    pub app_server: AppServer,
    pub sections: Vec<BrowserSection>,
    pub item_index: BTreeMap<String, BrowserIndexEntry>,
    pub validation: ValidationSnapshot,
    pub historical_ids: HistoricalIdSnapshot,
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
    pub method: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalPlanSourceMode {
    RequestDriven,
    DiffInferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalPlanConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalPlanSelectionMode {
    Minimal,
    Affected,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalPlanCoverageMode {
    ChangedLines,
    Affected,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlan {
    pub version: u32,
    pub kind: String,
    pub source: GoalPlanSource,
    pub goal: GoalPlanGoal,
    pub spec_mapping: GoalPlanSpecMapping,
    pub implementation_plan: GoalPlanImplementationPlan,
    pub test_plan: GoalPlanTestPlan,
    pub coverage: GoalPlanCoverage,
    pub completion: GoalPlanCompletion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanSource {
    pub mode: GoalPlanSourceMode,
    pub request_artifact: Option<PathBuf>,
    pub range: Option<String>,
    pub confidence: GoalPlanConfidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanGoal {
    pub id: String,
    pub title: String,
    pub statement: String,
    #[serde(default)]
    pub non_goals: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanSpecMapping {
    #[serde(default)]
    pub persistent_items: GoalPlanPersistentItems,
    pub spec_updates: GoalPlanSpecUpdates,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanPersistentItems {
    #[serde(default)]
    pub philosophies: Vec<String>,
    #[serde(default)]
    pub policies: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanSpecUpdates {
    pub required: bool,
    #[serde(default)]
    pub expected_updates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanImplementationPlan {
    pub scope: GoalPlanScope,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanScope {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanTestPlan {
    pub selection_mode: GoalPlanSelectionMode,
    #[serde(default)]
    pub required_tests: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub suggested_tests: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanCoverage {
    pub mode: GoalPlanCoverageMode,
    pub threshold: u32,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GoalPlanCompletion {
    #[serde(default)]
    pub must_pass: Vec<String>,
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
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    path: Option<String>,
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
        app_server: payload.app_server,
        sections,
        item_index,
        validation: payload.validation,
        historical_ids: payload.historical_ids,
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
                    method: reference.method,
                    path: reference.path,
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
        AppPayload, AppServer, DefinitionCounts, GoalPlan, GoalPlanCompletion, GoalPlanConfidence,
        GoalPlanCoverage, GoalPlanCoverageMode, GoalPlanGoal, GoalPlanImplementationPlan,
        GoalPlanPersistentItems, GoalPlanScope, GoalPlanSelectionMode, GoalPlanSource,
        GoalPlanSourceMode, GoalPlanSpecMapping, GoalPlanSpecUpdates, GoalPlanTestPlan,
        HistoricalIdSnapshot, ReferencedRule, SectionKind, Severity, SourceDocument, TraceCount,
        TraceSummary, ValidationIssue, ValidationSnapshot, build_browser_workspace,
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
            app_server: AppServer {
                bind: "127.0.0.1".to_string(),
                port: 3000,
                remotely_reachable: false,
            },
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
                    content: "category: App\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Browser app\n    summary: Explore layers in the browser.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      openapi:\n        - file: api/openapi.yaml\n          method: get\n          path: /pets/{petId}\n          symbols: []\n      rust:\n        - file: src/command/app.rs\n          symbols:\n            - run_app_command\n".to_string(),
                },
            ],
            validation: sample_validation(),
            historical_ids: HistoricalIdSnapshot::default(),
        });

        assert_eq!(workspace.sections.len(), 4);
        assert_eq!(workspace.app_server.bind, "127.0.0.1");
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
        assert_eq!(
            workspace
                .item_index
                .get("FEAT-001")
                .and_then(|entry| workspace
                    .sections
                    .iter()
                    .flat_map(|section| section.documents.iter())
                    .flat_map(|document| document.items.iter())
                    .find(|item| item.id == entry.id))
                .and_then(|item| item
                    .implementations
                    .iter()
                    .find(|group| group.language == "openapi"))
                .and_then(|group| group.references.first())
                .and_then(|reference| reference.method.as_deref()),
            Some("get")
        );
    }

    #[test]
    fn preserves_parse_errors_for_invalid_documents() {
        let workspace = build_browser_workspace(AppPayload {
            workspace_root: "/repo".to_string(),
            spec_root: "/repo/docs/syu".to_string(),
            app_server: AppServer {
                bind: "127.0.0.1".to_string(),
                port: 3000,
                remotely_reachable: false,
            },
            source_documents: vec![SourceDocument {
                section: SectionKind::Features,
                path: "broken.yaml".to_string(),
                content: "category: Broken\nversion: [\n".to_string(),
            }],
            validation: ValidationSnapshot::default(),
            historical_ids: HistoricalIdSnapshot::default(),
        });

        let document = &workspace.sections[2].documents[0];
        assert_eq!(document.path, "broken.yaml");
        assert!(document.parse_error.is_some());
        assert!(document.items.is_empty());
    }

    #[test]
    fn goal_plan_round_trips_through_yaml() {
        let plan = GoalPlan {
            version: 1,
            kind: "syu.goal_plan".to_string(),
            source: GoalPlanSource {
                mode: GoalPlanSourceMode::RequestDriven,
                request_artifact: Some(".syu/tasks/request.yaml".into()),
                range: None,
                confidence: GoalPlanConfidence::High,
            },
            goal: GoalPlanGoal {
                id: "GOAL-001".to_string(),
                title: "Ship a checkout flow".to_string(),
                statement: "Implement the checkout flow with explicit scope.".to_string(),
                non_goals: vec!["Rework the entire cart system".to_string()],
            },
            spec_mapping: GoalPlanSpecMapping {
                persistent_items: GoalPlanPersistentItems {
                    philosophies: vec!["PHIL-001".to_string()],
                    policies: vec!["POL-001".to_string()],
                    requirements: vec!["REQ-001".to_string()],
                    features: vec!["FEAT-001".to_string()],
                },
                spec_updates: GoalPlanSpecUpdates {
                    required: true,
                    expected_updates: vec!["REQ-001".to_string()],
                },
            },
            implementation_plan: GoalPlanImplementationPlan {
                scope: GoalPlanScope {
                    include: vec!["src/checkout.rs".to_string()],
                    exclude: vec!["src/cart.rs".to_string()],
                },
                steps: vec!["Draft the checkout service".to_string()],
            },
            test_plan: GoalPlanTestPlan {
                selection_mode: GoalPlanSelectionMode::Affected,
                required_tests: [("rust".to_string(), vec!["tests/checkout.rs".to_string()])]
                    .into_iter()
                    .collect(),
                suggested_tests: [("rust".to_string(), vec!["tests/cart.rs".to_string()])]
                    .into_iter()
                    .collect(),
            },
            coverage: GoalPlanCoverage {
                mode: GoalPlanCoverageMode::ChangedLines,
                threshold: 100,
                include: vec!["src/checkout.rs".to_string()],
                exclude: vec!["target".to_string()],
            },
            completion: GoalPlanCompletion {
                must_pass: vec!["syu validate .".to_string()],
            },
        };

        let yaml = serde_yaml::to_string(&plan).expect("serialize plan");
        let decoded: GoalPlan = serde_yaml::from_str(&yaml).expect("deserialize plan");
        assert_eq!(decoded, plan);
        assert!(yaml.contains("kind: syu.goal_plan"));
        assert!(yaml.contains("mode: request_driven"));
        assert!(yaml.contains("confidence: high"));
    }

    #[test]
    fn goal_plan_rejects_unknown_fields() {
        let yaml = r#"
version: 1
kind: syu.goal_plan
source:
  mode: request_driven
  request_artifact: .syu/tasks/request.yaml
  range: null
  confidence: high
goal:
  id: GOAL-001
  title: Ship a checkout flow
  statement: Implement the checkout flow with explicit scope.
  non_goals: []
spec_mapping:
  persistent_items: {}
  spec_updates:
    required: true
    expected_updates: []
implementation_plan:
  scope:
    include: []
    exclude: []
  steps: []
test_plan:
  selection_mode: minimal
  required_tests: {}
  suggested_tests: {}
coverage:
  mode: changed_lines
  threshold: 100
  include: []
  exclude: []
completion:
  must_pass: []
extra: nope
"#;

        let err = serde_yaml::from_str::<GoalPlan>(yaml).expect_err("unknown field should fail");
        assert!(err.to_string().contains("extra"));
    }
}
