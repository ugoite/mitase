use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use syu_domain::{GitRange, LanguageName, SpecId, SpecKind, WorkspaceRoot};
use syu_domain::{Issue, Severity, TraceReference};

mod work;

pub use work::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchResult {
    pub id: String,
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestClassification {
    Create,
    Change,
    Delete,
}

impl RequestClassification {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "requirement_create",
            Self::Change => "requirement_change",
            Self::Delete => "requirement_delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestArtifact {
    pub version: u32,
    pub request: String,
    #[serde(default)]
    pub context: RequestArtifactContext,
}

impl RequestArtifact {
    pub fn analysis_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&self.request);
        if let Some(affected_area) = &self.context.affected_area {
            text.push('\n');
            text.push_str(affected_area);
        }
        for constraint in &self.context.repository_constraints {
            text.push('\n');
            text.push_str(constraint);
        }
        for id in &self.context.linked_ids {
            text.push('\n');
            text.push_str(id);
        }
        text
    }

    pub fn explicit_ids(&self) -> Vec<String> {
        let mut ids = self.context.linked_ids.clone();
        ids.extend(extract_spec_ids(&self.request));
        if let Some(affected_area) = &self.context.affected_area {
            ids.extend(extract_spec_ids(affected_area));
        }
        ids.sort();
        ids.dedup();
        ids
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestArtifactContext {
    #[serde(default)]
    pub affected_area: Option<String>,
    #[serde(default)]
    pub repository_constraints: Vec<String>,
    #[serde(default)]
    pub linked_ids: Vec<String>,
}

fn extract_spec_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            current.push(ch);
        } else if !current.is_empty() {
            if is_spec_id(&current) {
                ids.push(current.clone());
            }
            current.clear();
        }
    }
    if !current.is_empty() && is_spec_id(&current) {
        ids.push(current);
    }
    ids
}

fn is_spec_id(value: &str) -> bool {
    value.starts_with("PHIL-")
        || value.starts_with("POL-")
        || value.starts_with("REQ-")
        || value.starts_with("FEAT-")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanArtifact {
    pub version: u32,
    pub kind: String,
    #[serde(default)]
    pub request_path: Option<String>,
    #[serde(default)]
    pub request: Option<String>,
    #[serde(default)]
    pub classification: Option<String>,
    #[serde(default)]
    pub work: Option<WorkPlan>,
    #[serde(default)]
    pub source: GoalPlanSource,
    pub goal: GoalPlanGoal,
    #[serde(default)]
    pub spec_mapping: GoalPlanSpecMapping,
    pub implementation_plan: GoalPlanImplementationPlan,
    pub test_plan: GoalPlanTestPlan,
    pub coverage: GoalPlanCoverage,
    pub completion: GoalPlanCompletion,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanSourceEvidence {
    #[serde(default)]
    pub item_id: Option<String>,
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub traced_requirements: Vec<String>,
    #[serde(default)]
    pub traced_features: Vec<String>,
    #[serde(default)]
    pub traced_policies: Vec<String>,
    #[serde(default)]
    pub traced_philosophies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanSource {
    pub mode: GoalPlanSourceMode,
    #[serde(default)]
    pub request_artifact: Option<String>,
    #[serde(default)]
    pub classification: Option<String>,
    #[serde(default)]
    pub range: Option<String>,
    #[serde(default)]
    pub confidence: Option<GoalPlanConfidence>,
    #[serde(default)]
    pub evidence: Option<GoalPlanSourceEvidence>,
}

impl Default for GoalPlanSource {
    fn default() -> Self {
        Self {
            mode: GoalPlanSourceMode::RequestDriven,
            request_artifact: None,
            classification: None,
            range: None,
            confidence: None,
            evidence: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPlanSourceMode {
    #[default]
    #[serde(rename = "request_driven")]
    RequestDriven,
    #[serde(rename = "diff_inferred")]
    DiffInferred,
    #[serde(rename = "item_driven")]
    ItemDriven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPlanConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanGoal {
    pub id: String,
    pub title: String,
    pub statement: String,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub inferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanSpecMapping {
    #[serde(default)]
    pub persistent_items: GoalPlanPersistentItems,
    #[serde(default)]
    pub spec_updates: GoalPlanSpecUpdates,
    #[serde(default)]
    pub spec_updates_required: bool,
    #[serde(default)]
    pub spec_update_reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanPersistentItems {
    #[serde(default)]
    pub philosophies: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    pub policies: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    pub requirements: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    pub features: Vec<GoalPlanPersistentItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GoalPlanPersistentItem {
    Id(String),
    Item(GoalPlanPersistentItemDetails),
}

impl GoalPlanPersistentItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id,
            Self::Item(item) => &item.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanPersistentItemDetails {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub document_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanSpecUpdates {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub expected_updates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanImplementationPlan {
    #[serde(default)]
    pub confidence: Option<GoalPlanConfidence>,
    pub scope: GoalPlanScope,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanScope {
    #[serde(default)]
    pub include: Vec<GoalPlanScopeInclude>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GoalPlanScopeInclude {
    Pattern(String),
    Entry(GoalPlanScopeIncludeDetails),
}

impl GoalPlanScopeInclude {
    pub fn pattern(&self) -> &str {
        match self {
            Self::Pattern(pattern) => pattern,
            Self::Entry(entry) => &entry.file,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanScopeIncludeDetails {
    pub file: String,
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanTestPlan {
    pub selection_mode: GoalPlanSelectionMode,
    #[serde(default)]
    pub confidence: Option<GoalPlanConfidence>,
    #[serde(default)]
    pub required_tests: BTreeMap<String, Vec<TraceReference>>,
    #[serde(default)]
    pub suggested_tests: BTreeMap<String, Vec<TraceReference>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPlanSelectionMode {
    #[default]
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "affected")]
    Affected,
    #[serde(rename = "full")]
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanCoverage {
    pub mode: GoalPlanCoverageMode,
    pub threshold: u32,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPlanCoverageMode {
    #[default]
    #[serde(rename = "changed_lines")]
    ChangedLines,
    #[serde(rename = "affected")]
    Affected,
    #[serde(rename = "full")]
    Full,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanCompletion {
    #[serde(default)]
    pub must_pass: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalPlanCheckReport {
    pub plan_path: String,
    pub range: String,
    pub changed_files: Vec<String>,
    pub issues: Vec<Issue>,
}

impl GoalPlanCheckReport {
    pub fn passed(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != Severity::Error)
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Warning)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeSignals {
    pub policy_discussion: bool,
    pub philosophy_discussion: bool,
    pub planned_feature_updates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeFeatureCandidate {
    pub id: String,
    pub title: String,
    pub status: String,
    pub linked_requirements: Vec<String>,
    pub planned_state_update: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationOutcome {
    pub classification: RequestClassification,
    pub reasons: Vec<String>,
    pub explicit_items: Vec<SearchResult>,
    pub related_items: Vec<SearchResult>,
    pub request: String,
    pub context: RequestArtifactContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeOutcome {
    pub classification: ClassificationOutcome,
    pub signals: ScopeSignals,
    pub requirements: Vec<SearchResult>,
    pub features: Vec<ScopeFeatureCandidate>,
    pub policies: Vec<SearchResult>,
    pub philosophies: Vec<SearchResult>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaffoldUpdate {
    pub kind: ScaffoldUpdateKind,
    pub action: ScaffoldAction,
    pub path: String,
    pub id: Option<String>,
    pub contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaffoldUpdateKind {
    Requirement,
    Feature,
    FeatureRegistry,
}

impl ScaffoldUpdateKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Feature => "feature",
            Self::FeatureRegistry => "feature registry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaffoldAction {
    Create,
    Update,
    Append,
}

impl ScaffoldAction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Append => "append",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaffoldPlan {
    pub updates: Vec<ScaffoldUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskTestSelectionPlan {
    pub goal_id: String,
    pub goal_title: String,
    pub selection_mode: String,
    pub commands: Vec<TaskTestSelectionCommand>,
    pub escalation: TaskTestSelectionEscalation,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskTestSelectionCommand {
    pub language: String,
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskTestSelectionEscalation {
    pub level: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_artifact_yaml() -> &'static str {
        "version: 1\nrequest: Update FEAT-001 and REQ-001\ncontext:\n  affected_area: core\n  repository_constraints:\n    - keep text and JSON output\n  linked_ids:\n    - REQ-001\n"
    }

    fn goal_plan_yaml() -> &'static str {
        "version: 1\nkind: syu.goal_plan\nrequest_path: request.yaml\nrequest: Update FEAT-001 and REQ-001\nclassification: requirement_change\nsource:\n  mode: diff_inferred\n  request_artifact: request.yaml\n  classification: requirement_change\n  range: origin/main...HEAD\n  confidence: high\n  evidence:\n    changed_files:\n      - src/command/task.rs\n    traced_requirements:\n      - REQ-001\ngoal:\n  id: GOAL-001\n  title: Keep planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\n  inferred: true\n  non_goals:\n    - Add persistent task specs under spec.root\nspec_mapping:\n  persistent_items:\n    philosophies:\n      - PHIL-001\n    policies:\n      - POL-001\n    requirements:\n      - REQ-001\n    features:\n      - FEAT-001\n  spec_updates:\n    required: false\n    expected_updates: []\n  spec_updates_required: false\nimplementation_plan:\n  confidence: high\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\ntest_plan:\n  selection_mode: affected\n  confidence: high\n  required_tests:\n    rust:\n      - file: tests/task_command.rs\n        symbols:\n          - task_plan_generates_goal_from_request\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n"
    }

    #[test]
    fn request_artifact_yaml_roundtrip() {
        let artifact: RequestArtifact = serde_yaml::from_str(request_artifact_yaml())
            .expect("request artifact should deserialize");
        assert_eq!(artifact.version, 1);
        assert_eq!(artifact.context.affected_area.as_deref(), Some("core"));

        let yaml = serde_yaml::to_string(&artifact).expect("request artifact yaml");
        let roundtrip: RequestArtifact =
            serde_yaml::from_str(&yaml).expect("request artifact should roundtrip through yaml");
        assert_eq!(artifact, roundtrip);

        let json = serde_json::to_string_pretty(&artifact).expect("request artifact json");
        let roundtrip_json: RequestArtifact =
            serde_json::from_str(&json).expect("request artifact should roundtrip through json");
        assert_eq!(artifact, roundtrip_json);
    }

    #[test]
    fn goal_plan_yaml_roundtrip() {
        let artifact: GoalPlanArtifact =
            serde_yaml::from_str(goal_plan_yaml()).expect("goal plan should deserialize");
        assert_eq!(artifact.kind, "syu.goal_plan");
        assert!(matches!(
            artifact.source.mode,
            GoalPlanSourceMode::DiffInferred
        ));
        assert_eq!(artifact.goal.id, "GOAL-001");

        let yaml = serde_yaml::to_string(&artifact).expect("goal plan yaml");
        let roundtrip: GoalPlanArtifact =
            serde_yaml::from_str(&yaml).expect("goal plan should roundtrip through yaml");
        assert_eq!(artifact, roundtrip);

        let json = serde_json::to_string_pretty(&artifact).expect("goal plan json");
        let roundtrip_json: GoalPlanArtifact =
            serde_json::from_str(&json).expect("goal plan should roundtrip through json");
        assert_eq!(artifact, roundtrip_json);
    }

    #[test]
    fn rejects_unknown_fields_at_artifact_boundaries() {
        let err =
            serde_yaml::from_str::<RequestArtifact>("version: 1\nrequest: Example\nextra: nope\n")
                .expect_err("unknown request artifact field should fail");
        assert!(err.to_string().contains("extra"));

        let err = serde_yaml::from_str::<GoalPlanArtifact>(
            "version: 1\nkind: syu.goal_plan\nunexpected: nope\nsource:\n  mode: request_driven\ngoal:\n  id: GOAL-001\n  title: Example\n  statement: Example\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect_err("unknown goal plan field should fail");
        assert!(err.to_string().contains("unexpected"));
    }

    #[test]
    fn supports_request_diff_and_item_driven_sources() {
        let request_driven: GoalPlanArtifact = serde_yaml::from_str(
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: request_driven\ngoal:\n  id: GOAL-001\n  title: Example\n  statement: Example\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("request-driven plan should parse");
        assert!(matches!(
            request_driven.source.mode,
            GoalPlanSourceMode::RequestDriven
        ));

        let diff_inferred: GoalPlanArtifact = serde_yaml::from_str(
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: diff_inferred\n  range: origin/main...HEAD\n  confidence: medium\n  evidence:\n    changed_files:\n      - src/command/task.rs\ngoal:\n  id: GOAL-001\n  title: Example\n  statement: Example\n  inferred: true\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("diff-inferred plan should parse");
        assert!(matches!(
            diff_inferred.source.mode,
            GoalPlanSourceMode::DiffInferred
        ));
        assert_eq!(
            diff_inferred.source.range.as_deref(),
            Some("origin/main...HEAD")
        );
        assert_eq!(
            diff_inferred.source.confidence,
            Some(GoalPlanConfidence::Medium)
        );

        let item_driven: GoalPlanArtifact = serde_yaml::from_str(
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: item_driven\n  evidence:\n    item_id: REQ-WORKBENCH-001\ngoal:\n  id: GOAL-ITEM-001\n  title: Item Work\n  statement: Implement the Item\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("item-driven plan should parse");
        assert_eq!(item_driven.source.mode, GoalPlanSourceMode::ItemDriven);
        assert_eq!(
            item_driven
                .source
                .evidence
                .and_then(|evidence| evidence.item_id),
            Some("REQ-WORKBENCH-001".to_string())
        );
    }

    #[test]
    fn empty_required_test_symbols_are_preserved_for_validation_layers() {
        let artifact: GoalPlanArtifact = serde_yaml::from_str(
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: request_driven\ngoal:\n  id: GOAL-001\n  title: Example\n  statement: Example\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: tests/task_command.rs\n        symbols: []\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("goal plan with empty symbols should parse");

        let symbols = &artifact
            .test_plan
            .required_tests
            .get("rust")
            .expect("language should be present")[0]
            .symbols;
        assert!(symbols.is_empty());
    }
}
