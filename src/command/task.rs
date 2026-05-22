// FEAT-TASK-001
// FEAT-TASK-003
// FEAT-TASK-004
// FEAT-TASK-005
// REQ-CORE-028
// REQ-CORE-029
// REQ-CORE-030
// REQ-CORE-031

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{
        LookupKind, OutputFormat, TaskArgs, TaskCheckArgs, TaskClassifyArgs, TaskCommands,
        TaskScaffoldArgs, TaskScopeArgs,
    },
    model::{Issue, Severity},
    workspace::load_workspace,
};

use super::issue_text::{TextIssueFormat, format_text_issue};
use super::log::resolve_git_range_changed_files;
use super::lookup::{SearchResult, WorkspaceEntity, WorkspaceLookup};

const REQUEST_ARTIFACT_VERSION: u32 = 1;
const GOAL_PLAN_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RequirementAction {
    Create,
    Change,
    Delete,
}

impl RequirementAction {
    const fn label(self) -> &'static str {
        match self {
            Self::Create => "requirement_create",
            Self::Change => "requirement_change",
            Self::Delete => "requirement_delete",
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct RequestArtifact {
    version: u32,
    request: String,
    #[serde(default)]
    context: RequestArtifactContext,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanArtifact {
    version: u32,
    kind: String,
    #[serde(default)]
    source: GoalPlanSource,
    goal: GoalPlanGoal,
    #[serde(default)]
    spec_mapping: GoalPlanSpecMapping,
    implementation_plan: GoalPlanImplementationPlan,
    test_plan: GoalPlanTestPlan,
    coverage: GoalPlanCoverage,
    completion: GoalPlanCompletion,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanSource {
    mode: GoalPlanSourceMode,
    #[serde(default)]
    request_artifact: Option<String>,
    #[serde(default)]
    range: Option<String>,
    #[serde(default)]
    confidence: Option<GoalPlanConfidence>,
}

impl Default for GoalPlanSource {
    fn default() -> Self {
        Self {
            mode: GoalPlanSourceMode::RequestDriven,
            request_artifact: None,
            range: None,
            confidence: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum GoalPlanSourceMode {
    #[default]
    #[serde(rename = "request_driven")]
    RequestDriven,
    #[serde(rename = "diff_inferred")]
    DiffInferred,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GoalPlanConfidence {
    High,
    Medium,
    Low,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanGoal {
    id: String,
    title: String,
    statement: String,
    #[serde(default)]
    non_goals: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanSpecMapping {
    #[serde(default)]
    persistent_items: GoalPlanPersistentItems,
    #[serde(default)]
    spec_updates: GoalPlanSpecUpdates,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanPersistentItems {
    #[serde(default)]
    philosophies: Vec<String>,
    #[serde(default)]
    policies: Vec<String>,
    #[serde(default)]
    requirements: Vec<String>,
    #[serde(default)]
    features: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanSpecUpdates {
    #[serde(default)]
    required: bool,
    #[serde(default)]
    expected_updates: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanImplementationPlan {
    scope: GoalPlanScope,
    #[serde(default)]
    steps: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanScope {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanTestPlan {
    selection_mode: GoalPlanSelectionMode,
    #[serde(default)]
    required_tests: BTreeMap<String, Vec<crate::model::TraceReference>>,
    #[serde(default)]
    suggested_tests: BTreeMap<String, Vec<crate::model::TraceReference>>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum GoalPlanSelectionMode {
    #[default]
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "affected")]
    Affected,
    #[serde(rename = "full")]
    Full,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanCoverage {
    mode: GoalPlanCoverageMode,
    threshold: u32,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum GoalPlanCoverageMode {
    #[default]
    #[serde(rename = "changed_lines")]
    ChangedLines,
    #[serde(rename = "affected")]
    Affected,
    #[serde(rename = "full")]
    Full,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanCompletion {
    #[serde(default)]
    must_pass: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RequestArtifactContext {
    #[serde(default)]
    affected_area: Option<String>,
    #[serde(default)]
    repository_constraints: Vec<String>,
    #[serde(default)]
    linked_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskClassifyOutput {
    request_path: String,
    request: String,
    classification: String,
    reasons: Vec<String>,
    explicit_items: Vec<SearchResult>,
    related_items: Vec<SearchResult>,
    context: JsonRequestArtifactContext,
}

#[derive(Debug, Serialize)]
struct JsonTaskScopeOutput {
    request_path: String,
    request: String,
    classification: String,
    reasons: Vec<String>,
    signals: JsonScopeSignals,
    requirements: Vec<SearchResult>,
    features: Vec<ScopeFeatureCandidate>,
    policies: Vec<SearchResult>,
    philosophies: Vec<SearchResult>,
    context: JsonRequestArtifactContext,
}

#[derive(Debug, Serialize)]
struct JsonTaskScaffoldOutput {
    request_path: String,
    request: String,
    classification: String,
    reasons: Vec<String>,
    updates: Vec<JsonScaffoldUpdate>,
    context: JsonRequestArtifactContext,
}

#[derive(Debug, Serialize)]
struct JsonTaskCheckOutput {
    plan_path: String,
    range: String,
    passed: bool,
    changed_files: Vec<String>,
    issue_count: usize,
    warning_count: usize,
    error_count: usize,
    issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
struct JsonScaffoldUpdate {
    kind: String,
    action: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    contents: String,
}

#[derive(Debug, Serialize)]
struct JsonRequestArtifactContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    affected_area: Option<String>,
    repository_constraints: Vec<String>,
    linked_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonScopeSignals {
    policy_discussion: bool,
    philosophy_discussion: bool,
    planned_feature_updates: bool,
}

#[derive(Debug)]
struct GoalPlanCheckReport {
    plan_path: String,
    range: String,
    changed_files: Vec<String>,
    issues: Vec<Issue>,
}

impl GoalPlanCheckReport {
    fn passed(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != Severity::Error)
    }

    fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Warning)
            .count()
    }

    fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
            .count()
    }
}

#[derive(Debug)]
struct ScopeOutcome {
    classification: ClassificationOutcome,
    signals: ScopeSignals,
    requirements: Vec<SearchResult>,
    features: Vec<ScopeFeatureCandidate>,
    policies: Vec<SearchResult>,
    philosophies: Vec<SearchResult>,
    notes: Vec<String>,
}

#[derive(Debug)]
struct ClassificationOutcome {
    classification: RequirementAction,
    reasons: Vec<String>,
    explicit_items: Vec<SearchResult>,
    related_items: Vec<SearchResult>,
    request: String,
    context: RequestArtifactContext,
}

#[derive(Debug, Clone, Serialize)]
struct ScopeSignals {
    policy_discussion: bool,
    philosophy_discussion: bool,
    planned_feature_updates: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ScopeFeatureCandidate {
    id: String,
    title: String,
    status: String,
    linked_requirements: Vec<String>,
    planned_state_update: bool,
}

#[derive(Debug)]
struct ScaffoldPlan {
    updates: Vec<ScaffoldUpdate>,
}

#[derive(Debug)]
struct ScaffoldUpdate {
    kind: ScaffoldUpdateKind,
    action: ScaffoldAction,
    path: String,
    id: Option<String>,
    contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaffoldUpdateKind {
    Requirement,
    Feature,
    FeatureRegistry,
}

impl ScaffoldUpdateKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Feature => "feature",
            Self::FeatureRegistry => "feature registry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaffoldAction {
    Create,
    Update,
    Append,
}

impl ScaffoldAction {
    const fn label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Append => "append",
        }
    }
}

pub fn run_task_command(args: &TaskArgs) -> Result<i32> {
    match &args.command {
        TaskCommands::Classify(classify) => run_task_classify_command(classify),
        TaskCommands::Scope(scope) => run_task_scope_command(scope),
        TaskCommands::Scaffold(scaffold) => run_task_scaffold_command(scaffold),
        TaskCommands::Check(check) => run_task_check_command(check),
    }
}

pub fn run_task_classify_command(args: &TaskClassifyArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let request_artifact = load_request_artifact(&args.request)?;
    let outcome = classify_request(&workspace, &request_artifact)?;

    match args.format {
        OutputFormat::Text => print_classify_text_output(&args.request, &outcome),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTaskClassifyOutput {
                request_path: args.request.display().to_string(),
                request: outcome.request,
                classification: outcome.classification.label().to_string(),
                reasons: outcome.reasons,
                explicit_items: outcome.explicit_items,
                related_items: outcome.related_items,
                context: JsonRequestArtifactContext {
                    affected_area: outcome.context.affected_area,
                    repository_constraints: outcome.context.repository_constraints,
                    linked_ids: outcome.context.linked_ids,
                },
            })
            .expect("serializing task classification output to JSON should succeed")
        ),
    }

    Ok(0)
}

pub fn run_task_scope_command(args: &TaskScopeArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let request_artifact = load_request_artifact(&args.request)?;
    let outcome = scope_request(&workspace, &request_artifact)?;

    match args.format {
        OutputFormat::Text => print_scope_text_output(&args.request, &outcome),
        OutputFormat::Json => {
            let ScopeOutcome {
                classification,
                signals,
                requirements,
                features,
                policies,
                philosophies,
                notes: _,
            } = outcome;
            println!(
                "{}",
                serde_json::to_string_pretty(&JsonTaskScopeOutput {
                    request_path: args.request.display().to_string(),
                    request: classification.request,
                    classification: classification.classification.label().to_string(),
                    reasons: classification.reasons,
                    signals: JsonScopeSignals {
                        policy_discussion: signals.policy_discussion,
                        philosophy_discussion: signals.philosophy_discussion,
                        planned_feature_updates: signals.planned_feature_updates,
                    },
                    requirements,
                    features,
                    policies,
                    philosophies,
                    context: JsonRequestArtifactContext {
                        affected_area: classification.context.affected_area,
                        repository_constraints: classification.context.repository_constraints,
                        linked_ids: classification.context.linked_ids,
                    },
                })
                .expect("serializing task scope output to JSON should succeed")
            )
        }
    }

    Ok(0)
}

pub fn run_task_scaffold_command(args: &TaskScaffoldArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let request_artifact = load_request_artifact(&args.request)?;
    let explicit_ids = request_artifact.explicit_ids();
    let outcome = classify_request(&workspace, &request_artifact)?;
    let plan = build_scaffold_plan(&workspace, &outcome, &explicit_ids)?;

    match args.format {
        OutputFormat::Text => print_scaffold_text_output(&args.request, &outcome, &plan),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTaskScaffoldOutput {
                request_path: args.request.display().to_string(),
                request: outcome.request,
                classification: outcome.classification.label().to_string(),
                reasons: outcome.reasons,
                updates: plan
                    .updates
                    .into_iter()
                    .map(|update| JsonScaffoldUpdate {
                        kind: update.kind.label().to_string(),
                        action: update.action.label().to_string(),
                        path: update.path,
                        id: update.id,
                        contents: update.contents,
                    })
                    .collect(),
                context: JsonRequestArtifactContext {
                    affected_area: outcome.context.affected_area,
                    repository_constraints: outcome.context.repository_constraints,
                    linked_ids: outcome.context.linked_ids,
                },
            })
            .expect("serializing task scaffold output to JSON should succeed")
        ),
    }

    Ok(0)
}

pub fn run_task_check_command(args: &TaskCheckArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let range = args.range.trim();
    if range.is_empty() {
        bail!("--range must not be empty");
    }

    let artifact = load_goal_plan_artifact(&args.plan)?;
    let mut report = check_goal_plan(&workspace, &artifact, range)?;
    report.plan_path = args.plan.display().to_string();

    match args.format {
        OutputFormat::Text => print_task_check_text_output(&args.plan, &report),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTaskCheckOutput {
                plan_path: report.plan_path.clone(),
                range: range.to_string(),
                passed: report.passed(),
                changed_files: report.changed_files.clone(),
                issue_count: report.issues.len(),
                warning_count: report.warning_count(),
                error_count: report.error_count(),
                issues: report.issues.clone(),
            })
            .expect("serializing task check output to JSON should succeed")
        ),
    }

    Ok(if report.passed() { 0 } else { 1 })
}

fn load_request_artifact(path: &PathBuf) -> Result<RequestArtifact> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read request artifact `{}`", path.display()))?;
    let artifact: RequestArtifact = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse request artifact `{}`", path.display()))?;
    if artifact.version != REQUEST_ARTIFACT_VERSION {
        bail!(
            "unsupported request artifact version `{}` in `{}`",
            artifact.version,
            path.display()
        );
    }
    Ok(artifact)
}

fn load_goal_plan_artifact(path: &PathBuf) -> Result<GoalPlanArtifact> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read goal plan artifact `{}`", path.display()))?;
    let artifact: GoalPlanArtifact = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse goal plan artifact `{}`", path.display()))?;
    if artifact.version != GOAL_PLAN_VERSION {
        bail!(
            "unsupported goal plan artifact version `{}` in `{}`",
            artifact.version,
            path.display()
        );
    }
    if artifact.kind != "syu.goal_plan" {
        bail!(
            "unsupported goal plan artifact kind `{}` in `{}`",
            artifact.kind,
            path.display()
        );
    }
    Ok(artifact)
}

fn classify_request(
    workspace: &crate::workspace::Workspace,
    artifact: &RequestArtifact,
) -> Result<ClassificationOutcome> {
    let lookup = WorkspaceLookup::new(workspace);
    let analysis_text = artifact.analysis_text();
    let lower = analysis_text.to_lowercase();
    let delete_hits = count_keyword_hits(&lower, DELETE_KEYWORDS);
    let change_hits = count_keyword_hits(&lower, CHANGE_KEYWORDS);
    let create_hits = count_keyword_hits(&lower, CREATE_KEYWORDS);

    let explicit_ids = artifact.explicit_ids();
    let explicit_items = collect_explicit_items(&lookup, &explicit_ids);
    let mut related_items = collect_related_items(&lookup, &artifact.request);
    merge_related_items(&mut related_items, lookup.search(&analysis_text, None));
    related_items.truncate(5);

    let mut reasons = Vec::new();
    if delete_hits > 0 {
        reasons.push(format!(
            "request uses delete-oriented language: {}",
            describe_keyword_hits(&lower, DELETE_KEYWORDS)
        ));
    }
    if change_hits > 0 {
        reasons.push(format!(
            "request uses change-oriented language: {}",
            describe_keyword_hits(&lower, CHANGE_KEYWORDS)
        ));
    }
    if create_hits > 0 {
        reasons.push(format!(
            "request uses create-oriented language: {}",
            describe_keyword_hits(&lower, CREATE_KEYWORDS)
        ));
    }
    if !explicit_items.is_empty() {
        reasons.push(format!(
            "request names existing spec items: {}",
            explicit_items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if explicit_items.is_empty() && !related_items.is_empty() {
        reasons.push(format!(
            "closest spec graph matches are {}",
            related_items
                .iter()
                .map(|item| format!("{} {}", item.kind, item.id))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if delete_hits == 0 && change_hits == 0 && create_hits == 0 {
        reasons.push(
            "request does not use a strong create/change/delete verb, so the graph match and linked IDs carry the decision"
                .to_string(),
        );
    }

    let classification = if delete_hits > 0 {
        RequirementAction::Delete
    } else if change_hits > 0 || !explicit_items.is_empty() {
        RequirementAction::Change
    } else {
        RequirementAction::Create
    };

    if matches!(classification, RequirementAction::Create) {
        if create_hits > 0 {
            reasons.push(
                "request uses create-oriented language and does not name an existing spec item"
                    .to_string(),
            );
        } else {
            reasons.push(
                "no existing spec item was named and the request reads like new work".to_string(),
            );
        }
    }

    Ok(ClassificationOutcome {
        classification,
        reasons,
        explicit_items,
        related_items,
        request: artifact.request.clone(),
        context: artifact.context.clone(),
    })
}

fn scope_request(
    workspace: &crate::workspace::Workspace,
    artifact: &RequestArtifact,
) -> Result<ScopeOutcome> {
    let classification = classify_request(workspace, artifact)?;
    let lookup = WorkspaceLookup::new(workspace);
    let analysis_text = artifact.analysis_text();
    let lower = analysis_text.to_lowercase();
    let explicit_ids = artifact.explicit_ids();
    let explicit_items = collect_explicit_items(&lookup, &explicit_ids);
    let search_results = lookup.search(&analysis_text, None);

    let requirements = collect_scoped_results(&explicit_items, &search_results, "requirement", 5);
    let policies = collect_scoped_results(&explicit_items, &search_results, "policy", 5);
    let philosophies = collect_scoped_results(&explicit_items, &search_results, "philosophy", 5);
    let features = collect_feature_candidates(
        &lookup,
        &explicit_items,
        &search_results,
        classification.classification,
        5,
    );

    let policy_keyword_hits = count_keyword_hits(&lower, POLICY_DISCUSSION_KEYWORDS);
    let philosophy_keyword_hits = count_keyword_hits(&lower, PHILOSOPHY_DISCUSSION_KEYWORDS);
    let policy_discussion = !policies.is_empty() || policy_keyword_hits > 0;
    let philosophy_discussion = !philosophies.is_empty() || philosophy_keyword_hits > 0;
    let planned_feature_updates = features.iter().any(|feature| feature.planned_state_update);

    let mut notes = Vec::new();
    if policy_discussion {
        if !policies.is_empty() {
            notes.push(format!(
                "request reaches policy context through {}",
                policies
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        } else {
            notes.push(format!(
                "request uses policy-oriented language: {}",
                describe_keyword_hits(&lower, POLICY_DISCUSSION_KEYWORDS)
            ));
        }
    }
    if philosophy_discussion {
        if !philosophies.is_empty() {
            notes.push(format!(
                "request reaches philosophy context through {}",
                philosophies
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        } else {
            notes.push(format!(
                "request uses philosophy-oriented language: {}",
                describe_keyword_hits(&lower, PHILOSOPHY_DISCUSSION_KEYWORDS)
            ));
        }
    }
    if planned_feature_updates {
        notes.push(
            "feature candidates include existing items that may need planned-state updates before implementation"
                .to_string(),
        );
    }

    Ok(ScopeOutcome {
        classification,
        signals: ScopeSignals {
            policy_discussion,
            philosophy_discussion,
            planned_feature_updates,
        },
        requirements,
        features,
        policies,
        philosophies,
        notes,
    })
}

fn build_scaffold_plan(
    workspace: &crate::workspace::Workspace,
    outcome: &ClassificationOutcome,
    explicit_ids: &[String],
) -> Result<ScaffoldPlan> {
    if matches!(outcome.classification, RequirementAction::Delete) {
        bail!(
            "`syu task scaffold` only supports request artifacts that classify as create or change"
        );
    }

    let lookup = WorkspaceLookup::new(workspace);
    let scaffold_stem = scaffold_stem(outcome, explicit_ids);

    let requirement_id = resolve_scaffold_id(
        &lookup,
        LookupKind::Requirement,
        explicit_ids,
        &scaffold_stem,
    );
    let feature_id =
        resolve_scaffold_id(&lookup, LookupKind::Feature, explicit_ids, &scaffold_stem);

    let requirement_title = lookup
        .title_for(LookupKind::Requirement, &requirement_id)
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| scaffold_title(&scaffold_stem));
    let feature_title = lookup
        .title_for(LookupKind::Feature, &feature_id)
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| scaffold_title(&scaffold_stem));

    let requirement_path =
        resolve_scaffold_document_path(workspace, LookupKind::Requirement, &requirement_id)?;
    let feature_path = resolve_scaffold_document_path(workspace, LookupKind::Feature, &feature_id)?;

    let requirement_action = if lookup.find(&requirement_id).is_some() {
        ScaffoldAction::Update
    } else {
        ScaffoldAction::Create
    };
    let feature_action = if lookup.find(&feature_id).is_some() {
        ScaffoldAction::Update
    } else {
        ScaffoldAction::Create
    };

    let mut updates = vec![
        ScaffoldUpdate {
            kind: ScaffoldUpdateKind::Requirement,
            action: requirement_action,
            path: requirement_path.clone(),
            id: Some(requirement_id.clone()),
            contents: render_requirement_document(
                outcome,
                &requirement_id,
                &requirement_title,
                &feature_id,
                &scaffold_stem,
            ),
        },
        ScaffoldUpdate {
            kind: ScaffoldUpdateKind::Feature,
            action: feature_action,
            path: feature_path.clone(),
            id: Some(feature_id.clone()),
            contents: render_feature_document(
                outcome,
                &feature_id,
                &feature_title,
                &requirement_id,
                &scaffold_stem,
            ),
        },
    ];

    if matches!(feature_action, ScaffoldAction::Create) {
        let registry_file = feature_registry_file_label(workspace, &feature_path)?;
        updates.push(ScaffoldUpdate {
            kind: ScaffoldUpdateKind::FeatureRegistry,
            action: ScaffoldAction::Append,
            path: workspace_relative_display(
                workspace,
                &workspace.spec_root.join("features/features.yaml"),
            ),
            id: None,
            contents: format!("  - kind: {}\n    file: {registry_file}\n", scaffold_stem),
        });
    }

    Ok(ScaffoldPlan { updates })
}

impl RequestArtifact {
    fn analysis_text(&self) -> String {
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

    fn explicit_ids(&self) -> Vec<String> {
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

fn collect_explicit_items(lookup: &WorkspaceLookup<'_>, ids: &[String]) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for id in ids {
        if let Some(item) = lookup.find(id) {
            items.insert(id.clone(), item_to_search_result(item));
        }
    }
    items.into_values().collect()
}

fn collect_related_items(lookup: &WorkspaceLookup<'_>, request: &str) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for result in lookup.search(request, None) {
        items.insert(result.id.clone(), result);
    }
    items.into_values().collect()
}

fn collect_scoped_results(
    explicit_items: &[SearchResult],
    search_results: &[SearchResult],
    kind: &'static str,
    limit: usize,
) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for result in explicit_items
        .iter()
        .chain(search_results.iter())
        .filter(|item| item.kind == kind)
    {
        items.insert(result.id.clone(), result.clone());
    }
    items.into_values().take(limit).collect()
}

fn collect_feature_candidates(
    lookup: &WorkspaceLookup<'_>,
    explicit_items: &[SearchResult],
    search_results: &[SearchResult],
    classification: RequirementAction,
    limit: usize,
) -> Vec<ScopeFeatureCandidate> {
    let mut items = BTreeMap::<String, ScopeFeatureCandidate>::new();
    for result in explicit_items
        .iter()
        .chain(search_results.iter())
        .filter(|item| item.kind == "feature")
    {
        if let Some(feature) = lookup.feature(&result.id) {
            items.insert(
                feature.id.clone(),
                ScopeFeatureCandidate {
                    id: feature.id.clone(),
                    title: feature.title.clone(),
                    status: feature.status.clone(),
                    linked_requirements: feature.linked_requirements.clone(),
                    planned_state_update: matches!(
                        classification,
                        RequirementAction::Create | RequirementAction::Change
                    ) && !feature.status.eq_ignore_ascii_case("planned"),
                },
            );
        }
    }
    items.into_values().take(limit).collect()
}

fn merge_related_items(related_items: &mut Vec<SearchResult>, additional_items: Vec<SearchResult>) {
    let mut merged = BTreeMap::<String, SearchResult>::new();
    for item in related_items.drain(..) {
        merged.insert(item.id.clone(), item);
    }
    for item in additional_items {
        merged.insert(item.id.clone(), item);
    }
    *related_items = merged.into_values().collect();
}

fn item_to_search_result(item: WorkspaceEntity<'_>) -> SearchResult {
    match item {
        WorkspaceEntity::Philosophy(item) => SearchResult {
            id: item.id.clone(),
            kind: "philosophy",
            title: item.title.clone(),
        },
        WorkspaceEntity::Policy(item) => SearchResult {
            id: item.id.clone(),
            kind: "policy",
            title: item.title.clone(),
        },
        WorkspaceEntity::Requirement(item) => SearchResult {
            id: item.id.clone(),
            kind: "requirement",
            title: item.title.clone(),
        },
        WorkspaceEntity::Feature(item) => SearchResult {
            id: item.id.clone(),
            kind: "feature",
            title: item.title.clone(),
        },
    }
}

fn print_classify_text_output(request_path: &Path, outcome: &ClassificationOutcome) {
    println!("request: {}", request_path.display());
    println!("classification: {}", outcome.classification.label());
    println!();
    println!("request text:");
    println!("{}", outcome.request.trim());
    println!();
    print_items("explicit items", &outcome.explicit_items);
    print_items("related items", &outcome.related_items);
    println!("reasons:");
    for reason in &outcome.reasons {
        println!("- {reason}");
    }
}

fn print_scope_text_output(request_path: &Path, outcome: &ScopeOutcome) {
    println!("request: {}", request_path.display());
    println!(
        "classification: {}",
        outcome.classification.classification.label()
    );
    println!();
    println!("request text:");
    println!("{}", outcome.classification.request.trim());
    println!();
    print_items("candidate requirements", &outcome.requirements);
    print_feature_candidates("candidate features", &outcome.features);
    print_items("candidate policies", &outcome.policies);
    print_items("candidate philosophies", &outcome.philosophies);
    println!("scope signals:");
    println!(
        "- policy discussion likely: {}",
        bool_label(outcome.signals.policy_discussion)
    );
    println!(
        "- philosophy discussion likely: {}",
        bool_label(outcome.signals.philosophy_discussion)
    );
    println!(
        "- candidate feature planned-state updates: {}",
        bool_label(outcome.signals.planned_feature_updates)
    );
    println!("reasons:");
    for reason in &outcome.classification.reasons {
        println!("- {reason}");
    }
    if !outcome.notes.is_empty() {
        println!();
        println!("scope notes:");
        for note in &outcome.notes {
            println!("- {note}");
        }
    }
}

fn print_scaffold_text_output(
    request_path: &Path,
    outcome: &ClassificationOutcome,
    plan: &ScaffoldPlan,
) {
    println!("request: {}", request_path.display());
    println!("classification: {}", outcome.classification.label());
    println!();
    println!("request text:");
    println!("{}", outcome.request.trim());
    println!();
    println!("reasons:");
    for reason in &outcome.reasons {
        println!("- {reason}");
    }
    println!();
    println!("planned updates:");
    for update in &plan.updates {
        println!(
            "- {} {} {}{}",
            update.action.label(),
            update.kind.label(),
            update.path,
            update
                .id
                .as_deref()
                .map(|id| format!(" for `{id}`"))
                .unwrap_or_default()
        );
        for line in update.contents.lines() {
            println!("  {line}");
        }
        println!();
    }
}

fn print_task_check_text_output(plan_path: &Path, report: &GoalPlanCheckReport) {
    println!("goal plan: {}", plan_path.display());
    println!("git range: {}", report.range);
    println!(
        "status: {}",
        if report.passed() { "passed" } else { "failed" }
    );
    println!();
    println!("changed files:");
    if report.changed_files.is_empty() {
        println!("- none");
    } else {
        for file in &report.changed_files {
            println!("- {file}");
        }
    }
    println!();
    if report.issues.is_empty() {
        println!("findings: none");
        return;
    }

    println!("findings:");
    for issue in &report.issues {
        for line in format_text_issue(issue, TextIssueFormat::Validate) {
            println!("{line}");
        }
    }
}

fn check_goal_plan(
    workspace: &crate::workspace::Workspace,
    artifact: &GoalPlanArtifact,
    range: &str,
) -> Result<GoalPlanCheckReport> {
    let lookup = WorkspaceLookup::new(workspace);
    let changed_files = resolve_git_range_changed_files(&workspace.root, range)?;
    let changed_file_strings = changed_files
        .iter()
        .map(|path| path_label(path))
        .collect::<Vec<_>>();
    let mut issues = Vec::new();

    if artifact.implementation_plan.scope.include.is_empty() {
        issues.push(Issue::error(
            "GOAL-TASK-PLAN-004",
            "implementation_plan.scope.include",
            None,
            "implementation scope does not declare any include patterns",
            Some(
                "Add the files or directories that this Goal Plan is allowed to change so range checks can evaluate scope accurately."
                    .to_string(),
            ),
        ));
    }

    if artifact.implementation_plan.steps.is_empty() {
        issues.push(Issue::warning(
            "GOAL-TASK-PLAN-005",
            "implementation_plan.steps",
            None,
            "implementation plan does not list any steps",
            Some(
                "List the bounded steps reviewers should expect before the implementation is complete."
                    .to_string(),
            ),
        ));
    }

    if artifact.test_plan.required_tests.is_empty() {
        issues.push(Issue::warning(
            "GOAL-TASK-PLAN-006",
            "test_plan.required_tests",
            None,
            "required tests are not declared",
            Some(
                "Add the repository tests that must be present before the plan can be considered complete."
                    .to_string(),
            ),
        ));
    }

    if artifact.completion.must_pass.is_empty() {
        issues.push(Issue::error(
            "GOAL-TASK-PLAN-007",
            "completion.must_pass",
            None,
            "required completion commands are not declared",
            Some(
                "List the commands reviewers or automation should require before accepting the plan."
                    .to_string(),
            ),
        ));
    }

    if matches!(artifact.source.mode, GoalPlanSourceMode::DiffInferred) {
        match artifact.source.confidence {
            Some(GoalPlanConfidence::High) => {}
            Some(GoalPlanConfidence::Medium) => issues.push(Issue::warning(
                "GOAL-TASK-PLAN-008",
                "source.confidence",
                None,
                "diff-inferred plan source confidence is medium",
                Some(
                    "Treat medium confidence as a review signal and make the risky sections explicit."
                        .to_string(),
                ),
            )),
            Some(GoalPlanConfidence::Low) => issues.push(Issue::warning(
                "GOAL-TASK-PLAN-009",
                "source.confidence",
                None,
                "diff-inferred plan source confidence is low",
                Some(
                    "Revisit the inferred plan before review because the source confidence is low."
                        .to_string(),
                ),
            )),
            None => issues.push(Issue::warning(
                "GOAL-TASK-PLAN-010",
                "source.confidence",
                None,
                "diff-inferred plan source does not declare confidence",
                Some(
                    "Add source.confidence so reviewers know how reliable the inferred plan is."
                        .to_string(),
                ),
            )),
        }
    }

    if let Some(source_range) = artifact.source.range.as_deref()
        && matches!(artifact.source.mode, GoalPlanSourceMode::DiffInferred)
        && source_range.trim() != range
    {
        issues.push(Issue::warning(
            "GOAL-TASK-PLAN-011",
            "source.range",
            None,
            "plan source range does not match the requested git range",
            Some(
                "Rebuild or update the Goal Plan so its recorded range matches the check input."
                    .to_string(),
            ),
        ));
    }

    let include_matcher = build_globset(&artifact.implementation_plan.scope.include)?;
    let exclude_matcher = build_globset(&artifact.implementation_plan.scope.exclude)?;
    if !artifact.implementation_plan.scope.include.is_empty() {
        for file in &changed_files {
            if !path_matches_scope(file, include_matcher.as_ref(), exclude_matcher.as_ref()) {
                let file_string = path_label(file);
                let production_path = is_production_path(file);
                let issue = if production_path {
                    Issue::error(
                        "GOAL-TASK-PLAN-001",
                        "implementation_plan.scope",
                        Some(file_string.clone()),
                        "changed production file is outside the implementation scope",
                        Some(
                            "Update implementation_plan.scope.include/exclude or move the change into the listed scope."
                                .to_string(),
                        ),
                    )
                } else {
                    Issue::warning(
                        "GOAL-TASK-PLAN-001",
                        "implementation_plan.scope",
                        Some(file_string),
                        "changed file is outside the implementation scope",
                        Some(
                            "Confirm whether the file should be included in the plan or excluded from the change set."
                                .to_string(),
                        ),
                    )
                };
                issues.push(issue);
            }
        }
    }

    validate_goal_plan_spec_ids(&lookup, artifact, &mut issues);
    validate_goal_plan_required_tests(workspace, artifact, &mut issues)?;

    Ok(GoalPlanCheckReport {
        plan_path: String::new(),
        range: range.to_string(),
        changed_files: changed_file_strings,
        issues,
    })
}

fn validate_goal_plan_spec_ids(
    lookup: &WorkspaceLookup<'_>,
    artifact: &GoalPlanArtifact,
    issues: &mut Vec<Issue>,
) {
    for id in artifact
        .spec_mapping
        .persistent_items
        .philosophies
        .iter()
        .chain(artifact.spec_mapping.persistent_items.policies.iter())
        .chain(artifact.spec_mapping.persistent_items.requirements.iter())
        .chain(artifact.spec_mapping.persistent_items.features.iter())
    {
        if lookup.find(id).is_none() {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-002",
                "spec_mapping.persistent_items",
                Some(id.clone()),
                "linked persistent spec ID does not exist",
                Some(
                    "Fix the Goal Plan or add the missing spec item before the plan is reviewed."
                        .to_string(),
                ),
            ));
        }
    }
}

fn validate_goal_plan_required_tests(
    workspace: &crate::workspace::Workspace,
    artifact: &GoalPlanArtifact,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    for (language, references) in &artifact.test_plan.required_tests {
        for reference in references {
            validate_goal_plan_test_reference(workspace, language, reference, issues)?;
        }
    }

    Ok(())
}

fn validate_goal_plan_test_reference(
    workspace: &crate::workspace::Workspace,
    language: &str,
    reference: &crate::model::TraceReference,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    let test_path = if reference.file.is_absolute() {
        reference.file.clone()
    } else {
        workspace.root.join(&reference.file)
    };
    let location = reference.file.display().to_string();
    let contents = match fs::read_to_string(&test_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-003",
                "test_plan.required_tests",
                Some(location.clone()),
                "required test file is missing",
                Some(
                    "Create the referenced test file or update the Goal Plan to point at an existing repository test."
                        .to_string(),
                ),
            ));
            return Ok(());
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read required test file `{}` for language `{}`",
                    reference.file.display(),
                    language
                )
            });
        }
    };

    for symbol in &reference.symbols {
        if symbol.trim() == "*" {
            continue;
        }
        if !contents.contains(symbol) {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-003",
                "test_plan.required_tests",
                Some(format!("{location}::{symbol}")),
                "required test symbol is missing",
                Some(
                    "Add the named test function, method, or symbol to the referenced test file."
                        .to_string(),
                ),
            ));
        }
    }

    for snippet in &reference.doc_contains {
        if snippet.trim() == "*" {
            continue;
        }
        if !contents.contains(snippet) {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-003",
                "test_plan.required_tests",
                Some(format!("{location}::{snippet}")),
                "required test text is missing",
                Some(
                    "Add the expected documentation text to the referenced test file.".to_string(),
                ),
            ));
        }
    }

    Ok(())
}

fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).with_context(|| format!("invalid scope glob `{pattern}`"))?);
    }

    Ok(Some(
        builder.build().context("failed to build scope glob set")?,
    ))
}

fn path_matches_scope(path: &Path, include: Option<&GlobSet>, exclude: Option<&GlobSet>) -> bool {
    let included = include.is_none_or(|set| set.is_match(path));
    let excluded = exclude.is_some_and(|set| set.is_match(path));
    included && !excluded
}

fn is_production_path(path: &Path) -> bool {
    let rendered = path_label(path);
    rendered.starts_with("src/")
        || rendered.starts_with("app/")
        || rendered.starts_with("crates/")
        || rendered.starts_with("examples/")
        || rendered.starts_with("website/")
        || rendered.starts_with("repo/")
        || rendered.starts_with("scripts/")
        || matches!(rendered.as_str(), "build.rs" | "Cargo.toml" | "Cargo.lock")
}

fn print_items(heading: &str, items: &[SearchResult]) {
    println!("{heading}:");
    if items.is_empty() {
        println!("- none");
        return;
    }

    for item in items {
        println!("- {}\t{}\t{}", item.id, item.kind, item.title);
    }
}

fn print_feature_candidates(heading: &str, items: &[ScopeFeatureCandidate]) {
    println!("{heading}:");
    if items.is_empty() {
        println!("- none");
        return;
    }

    for item in items {
        println!(
            "- {}\t{}\t{}{}",
            item.id,
            item.status,
            item.title,
            if item.planned_state_update {
                " [planned-state update suggested]"
            } else {
                ""
            }
        );
        if !item.linked_requirements.is_empty() {
            println!(
                "  linked requirements: {}",
                item.linked_requirements.join(", ")
            );
        }
    }
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn render_requirement_document(
    outcome: &ClassificationOutcome,
    requirement_id: &str,
    requirement_title: &str,
    feature_id: &str,
    stem: &str,
) -> String {
    let prefix = requirement_prefix(requirement_id);
    format!(
        "category: {} Requirements\nprefix: {prefix}\n\nrequirements:\n  - id: {requirement_id}\n    title: {requirement_title}\n    description: |\n      Generated from `syu task scaffold` after `syu task classify` returned `{classification}`.\n      Request:\n{request}\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features:\n      - {feature_id}\n    tests: {{}}\n",
        title_case_slug(stem),
        classification = outcome.classification.label(),
        request = indent_block(outcome.request.trim(), 6),
    )
}

fn render_feature_document(
    outcome: &ClassificationOutcome,
    feature_id: &str,
    feature_title: &str,
    requirement_id: &str,
    stem: &str,
) -> String {
    format!(
        "category: {} Features\nversion: 1\n\nfeatures:\n  - id: {feature_id}\n    title: {feature_title}\n    summary: |\n      Generated from `syu task scaffold` after `syu task classify` returned `{classification}`.\n      Request:\n{request}\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n",
        title_case_slug(stem),
        classification = outcome.classification.label(),
        request = indent_block(outcome.request.trim(), 6),
    )
}

fn feature_registry_file_label(
    workspace: &crate::workspace::Workspace,
    feature_document: &str,
) -> Result<String> {
    let registry_root = workspace.spec_root.join("features");
    let full_path = workspace.root.join(feature_document);
    let relative = full_path.strip_prefix(&registry_root).with_context(|| {
        format!(
            "feature document `{}` must stay under `{}`",
            feature_document,
            registry_root.display()
        )
    })?;
    Ok(path_label(relative))
}

fn resolve_scaffold_document_path(
    workspace: &crate::workspace::Workspace,
    kind: LookupKind,
    id: &str,
) -> Result<String> {
    let existing = WorkspaceLookup::new(workspace)
        .document_path_for_id(id)?
        .map(|path| path_label(Path::new(&path)));
    if let Some(existing) = existing {
        return Ok(existing);
    }

    let relative = scaffold_relative_path(kind, id);
    Ok(workspace_relative_display(
        workspace,
        &workspace.spec_root.join(relative),
    ))
}

fn scaffold_relative_path(kind: LookupKind, id: &str) -> PathBuf {
    let segments = id.split('-').collect::<Vec<_>>();
    let suffix = segments
        .get(1..segments.len().saturating_sub(1))
        .unwrap_or(&[]);
    let folder = suffix
        .first()
        .copied()
        .unwrap_or_else(|| default_scaffold_folder(kind));
    let file = if suffix.len() > 1 {
        suffix[1..].join("-")
    } else {
        folder.to_string()
    };

    match kind {
        LookupKind::Requirement => PathBuf::from(format!(
            "requirements/{}/{}.yaml",
            folder.to_ascii_lowercase(),
            file.to_ascii_lowercase()
        )),
        LookupKind::Feature => PathBuf::from(format!(
            "features/{}/{}.yaml",
            folder.to_ascii_lowercase(),
            file.to_ascii_lowercase()
        )),
        _ => PathBuf::from("planned/unsupported.yaml"),
    }
}

fn default_scaffold_folder(kind: LookupKind) -> &'static str {
    match kind {
        LookupKind::Requirement => "core",
        LookupKind::Feature => "core",
        LookupKind::Philosophy => "philosophy",
        LookupKind::Policy => "policies",
    }
}

fn resolve_scaffold_id(
    lookup: &WorkspaceLookup<'_>,
    kind: LookupKind,
    explicit_ids: &[String],
    stem: &str,
) -> String {
    let prefix = match kind {
        LookupKind::Requirement => "REQ",
        LookupKind::Feature => "FEAT",
        LookupKind::Philosophy => "PHIL",
        LookupKind::Policy => "POL",
    };
    if let Some(existing) = explicit_ids.iter().find(|id| id.starts_with(prefix)) {
        return existing.clone();
    }

    let opposite_prefix = match kind {
        LookupKind::Requirement => "FEAT",
        LookupKind::Feature => "REQ",
        LookupKind::Philosophy => "POL",
        LookupKind::Policy => "REQ",
    };
    if let Some(other) = explicit_ids
        .iter()
        .find(|id| id.starts_with(opposite_prefix))
    {
        let candidate = rewrite_spec_prefix(other, prefix);
        if lookup.find(&candidate).is_none() {
            return candidate;
        }
    }

    next_available_scaffold_id(lookup, kind, stem)
}

fn next_available_scaffold_id(
    lookup: &WorkspaceLookup<'_>,
    kind: LookupKind,
    stem: &str,
) -> String {
    let prefix = match kind {
        LookupKind::Requirement => "REQ",
        LookupKind::Feature => "FEAT",
        LookupKind::Philosophy => "PHIL",
        LookupKind::Policy => "POL",
    };
    let stem = normalize_scaffold_stem(stem);
    let mut index = 1usize;
    loop {
        let candidate = format!("{prefix}-{}-{index:03}", stem.to_ascii_uppercase());
        if lookup.find(&candidate).is_none() {
            return candidate;
        }
        index += 1;
    }
}

fn scaffold_stem(outcome: &ClassificationOutcome, explicit_ids: &[String]) -> String {
    if let Some(id) = explicit_ids.iter().find(|id| id.starts_with("REQ-")) {
        return id_stem(id);
    }
    if let Some(id) = explicit_ids.iter().find(|id| id.starts_with("FEAT-")) {
        return id_stem(id);
    }
    if let Some(area) = outcome.context.affected_area.as_deref() {
        let slug = slugify(area);
        if !slug.is_empty() {
            return slug;
        }
    }
    let summary = summarize_request(&outcome.request);
    let slug = slugify(&summary);
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

fn normalize_scaffold_stem(stem: &str) -> String {
    let slug = slugify(stem);
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

fn scaffold_title(stem: &str) -> String {
    title_case_slug(&normalize_scaffold_stem(stem))
}

fn id_stem(id: &str) -> String {
    let segments = id.split('-').collect::<Vec<_>>();
    if segments.len() <= 2 {
        segments.get(1).copied().unwrap_or("task").to_string()
    } else {
        segments[1..segments.len() - 1].join("-")
    }
}

fn requirement_prefix(id: &str) -> String {
    id.rsplit_once('-')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or_else(|| id.to_string())
}

fn rewrite_spec_prefix(id: &str, prefix: &str) -> String {
    let mut segments = id.split('-').collect::<Vec<_>>();
    segments[0] = prefix;
    segments.join("-")
}

fn workspace_relative_display(workspace: &crate::workspace::Workspace, path: &Path) -> String {
    path.strip_prefix(&workspace.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn indent_block(text: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn title_case_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            let first = chars.next().expect("empty segments are filtered out");
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn summarize_request(request: &str) -> String {
    let summary = request
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("planned task")
        .trim_matches(|ch: char| matches!(ch, '.' | ':' | ';'))
        .to_string();
    truncate_text(&summary, 72)
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            slug.push('-');
            previous_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

const DELETE_KEYWORDS: &[&str] = &[
    "delete",
    "remove",
    "drop",
    "retire",
    "deprecate",
    "obsolete",
    "eliminate",
    "no longer valid",
];

const CHANGE_KEYWORDS: &[&str] = &[
    "change", "update", "modify", "refine", "expand", "extend", "revise", "adjust", "replace",
    "rework", "clarify",
];

const CREATE_KEYWORDS: &[&str] = &[
    "create",
    "add",
    "introduce",
    "new",
    "implement",
    "support",
    "build",
];

const POLICY_DISCUSSION_KEYWORDS: &[&str] = &[
    "policy",
    "policies",
    "standard",
    "governance",
    "approval",
    "compliance",
    "rule",
    "guideline",
    "process",
    "constraint",
];

const PHILOSOPHY_DISCUSSION_KEYWORDS: &[&str] = &[
    "philosophy",
    "principle",
    "principles",
    "guideline",
    "guidelines",
    "values",
    "approach",
    "direction",
    "ethos",
    "design principle",
    "coding guideline",
];

fn count_keyword_hits(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .count()
}

fn describe_keyword_hits(text: &str, keywords: &[&str]) -> String {
    keywords
        .iter()
        .copied()
        .filter(|keyword| text.contains(keyword))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_spec_ids(text: &str) -> Vec<String> {
    static SPEC_ID_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = SPEC_ID_RE.get_or_init(|| {
        Regex::new(r"\b(?:PHIL|POL|REQ|FEAT)-[A-Z0-9][A-Z0-9-]*\b")
            .expect("spec id regex should compile")
    });

    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use crate::cli::{
        OutputFormat, TaskArgs, TaskCheckArgs, TaskClassifyArgs, TaskCommands, TaskScaffoldArgs,
        TaskScopeArgs,
    };

    use super::{
        ClassificationOutcome, GoalPlanConfidence, GoalPlanCoverageMode, GoalPlanSelectionMode,
        GoalPlanSourceMode, RequirementAction, SearchResult, WorkspaceLookup, build_scaffold_plan,
        classify_request, collect_feature_candidates, load_goal_plan_artifact,
        load_request_artifact, run_task_command,
    };

    fn write_request_artifact(path: &Path, request: &str, linked_ids: &[&str]) {
        let linked_ids_block = if linked_ids.is_empty() {
            "  linked_ids: []\n".to_string()
        } else {
            let list = linked_ids
                .iter()
                .map(|id| format!("    - {id}\n"))
                .collect::<String>();
            format!("  linked_ids:\n{list}")
        };
        fs::write(
            path,
            format!(
                "version: 1\nrequest: >\n  {request}\ncontext:\n  affected_area: core\n  repository_constraints:\n    - keep text and JSON output\n{linked_ids_block}",
            ),
        )
        .expect("request artifact should write");
    }

    fn write_workspace(root: &Path) {
        fs::write(
            root.join("syu.yaml"),
            "version: 1\nspec:\n  root: docs/syu\n",
        )
        .expect("workspace config");
        fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
        fs::create_dir_all(root.join("docs/syu/policies")).expect("policy dir");
        fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");
        fs::create_dir_all(root.join("docs/syu/features/core")).expect("features dir");

        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Keep planning explicit\n    product_design_principle: Request artifacts should stay reviewable.\n    coding_guideline: Prefer explicit request classification.\n    linked_policies:\n      - POL-001\n",
        )
        .expect("philosophy doc");
        fs::write(
            root.join("docs/syu/policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n",
        )
        .expect("policy doc");
        fs::write(
            root.join("docs/syu/requirements/core/classify.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-028\n    title: Classify request artifacts into requirement actions\n    description: The task classifier should decide whether a request creates, changes, or deletes a requirement.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-001\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
        )
        .expect("requirement doc");
        fs::write(
            root.join("docs/syu/requirements/core/scaffold.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-029\n    title: Scaffold planned requirement and feature updates from task planning\n    description: The scaffold command should turn request planning results into reviewable planned requirement and feature updates.\n    priority: medium\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-002\n    tests: {}\n",
        )
        .expect("scaffold requirement doc");
        fs::write(
            root.join("docs/syu/requirements/core/scope.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-030\n    title: Scope requests against requirements, policies, philosophies, and features\n    description: The task scope command should map a request artifact onto nearby spec items before planning starts.\n    priority: medium\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-003\n    tests: {}\n",
        )
        .expect("scope requirement doc");
        fs::write(
            root.join("docs/syu/requirements/core/check.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-031\n    title: Validate temporary Goal Plans against the current spec graph and git range\n    description: The task check command should validate Goal Plan conformance against changed files, linked spec IDs, required tests, and completion commands.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-004\n    tests:\n      rust:\n        - file: tests/task_command.rs\n          symbols:\n            - task_check_reports_pass_fail_results_for_goal_plans\n",
        )
        .expect("check requirement doc");
        fs::write(
            root.join("docs/syu/features/features.yaml"),
            "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n  - kind: task\n    file: core/scaffold.yaml\n  - kind: task\n    file: core/scope.yaml\n",
        )
        .expect("feature registry");
        fs::write(
            root.join("docs/syu/features/core/task.yaml"),
            "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify planned request artifacts into create, change, or delete decisions using the current spec graph and a brief explanation.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskClassifyArgs\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: planned\n    linked_requirements:\n      - REQ-CORE-030\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scope_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskScopeArgs\n  - id: FEAT-TASK-004\n    title: Goal Plan conformance checking\n    summary: Validate temporary Goal Plan artifacts against changed files, linked spec IDs, required tests, and declared completion commands before review.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-031\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_check_command\n            - load_goal_plan_artifact\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskCheckArgs\n",
        )
        .expect("feature doc");
        fs::write(
            root.join("docs/syu/features/core/scaffold.yaml"),
            "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-002\n    title: Planned task scaffold preview\n    summary: Preview reviewable planned requirement and feature updates that follow the existing add and registry conventions.\n    status: planned\n    linked_requirements:\n      - REQ-CORE-029\n    implementations: {}\n",
        )
        .expect("scaffold feature doc");
        fs::write(
            root.join("docs/syu/features/core/scope.yaml"),
            "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: planned\n    linked_requirements:\n      - REQ-CORE-030\n    implementations: {}\n",
        )
        .expect("scope feature doc");
    }

    #[test]
    fn load_request_artifact_rejects_version_mismatch() {
        let tempdir = tempdir().expect("tempdir");
        let request = tempdir.path().join("request.yaml");
        fs::write(
            &request,
            "version: 2\nrequest: Update the requirement\ncontext: {}\n",
        )
        .expect("request");

        let error = load_request_artifact(&request).expect_err("version mismatch should fail");
        assert!(
            error
                .to_string()
                .contains("unsupported request artifact version")
        );
    }

    #[test]
    fn classify_request_prefers_change_for_existing_requirement_ids() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Update REQ-CORE-028 so the request classifier stays explainable.",
            &["REQ-CORE-028"],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let outcome = classify_request(&workspace, &artifact).expect("classification");
        assert_eq!(outcome.classification, RequirementAction::Change);
        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| reason.contains("REQ-CORE-028"))
        );
    }

    #[test]
    fn classify_request_prefers_create_for_new_requests_without_existing_ids() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Create a new request summary for the upcoming planning flow.",
            &[],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let outcome = classify_request(&workspace, &artifact).expect("classification");
        assert_eq!(outcome.classification, RequirementAction::Create);
        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| reason.contains("create-oriented language"))
        );
    }

    #[test]
    fn build_scaffold_plan_prefers_existing_ids_and_registry_updates() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Update REQ-CORE-028 and FEAT-TASK-001 so the task workflow stays reviewable.",
            &["REQ-CORE-028", "FEAT-TASK-001"],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let explicit_ids = artifact.explicit_ids();
        let outcome = classify_request(&workspace, &artifact).expect("classification");
        let plan = build_scaffold_plan(&workspace, &outcome, &explicit_ids)
            .expect("scaffold plan should be created");

        assert!(plan.updates.iter().any(|update| {
            update
                .path
                .contains("docs/syu/requirements/core/classify.yaml")
        }));
        assert!(
            plan.updates
                .iter()
                .any(|update| update.path.contains("docs/syu/features/core/task.yaml"))
        );
        assert!(
            !plan
                .updates
                .iter()
                .any(|update| matches!(update.kind, super::ScaffoldUpdateKind::FeatureRegistry))
        );
    }

    #[test]
    fn build_scaffold_plan_creates_new_documents_and_registry_entry() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Create a new checkout planning flow for reviewers.",
            &[],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let explicit_ids = artifact.explicit_ids();
        let outcome = classify_request(&workspace, &artifact).expect("classification");
        let plan = build_scaffold_plan(&workspace, &outcome, &explicit_ids)
            .expect("scaffold plan should be created");

        assert!(plan.updates.iter().any(|update| {
            matches!(update.kind, super::ScaffoldUpdateKind::Requirement)
                && matches!(update.action, super::ScaffoldAction::Create)
                && update.path.contains("docs/syu/requirements/core/core.yaml")
        }));
        assert!(plan.updates.iter().any(|update| {
            matches!(update.kind, super::ScaffoldUpdateKind::Feature)
                && matches!(update.action, super::ScaffoldAction::Create)
                && update.path.contains("docs/syu/features/core/core.yaml")
        }));
        assert!(plan.updates.iter().any(|update| {
            matches!(update.kind, super::ScaffoldUpdateKind::FeatureRegistry)
                && matches!(update.action, super::ScaffoldAction::Append)
                && update.contents.contains("file: core/core.yaml")
        }));
    }

    #[test]
    fn build_scaffold_plan_rejects_delete_classifications() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Delete REQ-CORE-028 because the workflow is obsolete.",
            &["REQ-CORE-028"],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let explicit_ids = artifact.explicit_ids();
        let outcome = classify_request(&workspace, &artifact).expect("classification");
        let error = build_scaffold_plan(&workspace, &outcome, &explicit_ids)
            .expect_err("delete scaffolds should be rejected");
        assert!(
            error
                .to_string()
                .contains("only supports request artifacts")
        );
    }

    #[test]
    fn collect_feature_candidates_skips_missing_workspace_features() {
        let tempdir = tempdir().expect("tempdir");
        let workspace = crate::workspace::Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: crate::config::SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };
        let lookup = WorkspaceLookup::new(&workspace);
        let explicit_items = vec![SearchResult {
            id: "FEAT-MISSING-001".to_string(),
            kind: "feature",
            title: "Missing feature".to_string(),
        }];

        let candidates =
            collect_feature_candidates(&lookup, &explicit_items, &[], RequirementAction::Create, 5);

        assert!(candidates.is_empty());
    }

    #[test]
    fn scaffold_helpers_cover_prefixes_paths_and_fallbacks() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let lookup = crate::command::lookup::WorkspaceLookup::new(&workspace);

        assert_eq!(RequirementAction::Delete.label(), "requirement_delete");
        assert_eq!(
            super::ScaffoldUpdateKind::FeatureRegistry.label(),
            "feature registry"
        );
        assert_eq!(super::ScaffoldAction::Append.label(), "append");
        assert_eq!(
            super::scaffold_relative_path(
                crate::cli::LookupKind::Requirement,
                "REQ-AUTH-LOGIN-001"
            ),
            PathBuf::from("requirements/auth/login.yaml")
        );
        assert_eq!(
            super::scaffold_relative_path(crate::cli::LookupKind::Feature, "FEAT-001"),
            PathBuf::from("features/core/core.yaml")
        );
        assert_eq!(
            super::scaffold_relative_path(crate::cli::LookupKind::Policy, "POL-001"),
            PathBuf::from("planned/unsupported.yaml")
        );
        assert_eq!(
            super::default_scaffold_folder(crate::cli::LookupKind::Requirement),
            "core"
        );
        assert_eq!(
            super::default_scaffold_folder(crate::cli::LookupKind::Philosophy),
            "philosophy"
        );
        assert_eq!(
            super::resolve_scaffold_id(
                &lookup,
                crate::cli::LookupKind::Requirement,
                &["FEAT-AUTH-LOGIN-001".to_string()],
                "ignored",
            ),
            "REQ-AUTH-LOGIN-001"
        );
        assert_eq!(
            super::resolve_scaffold_id(
                &lookup,
                crate::cli::LookupKind::Philosophy,
                &["POL-001".to_string()],
                "governance",
            ),
            "PHIL-GOVERNANCE-001"
        );
        assert_eq!(
            super::resolve_scaffold_id(
                &lookup,
                crate::cli::LookupKind::Policy,
                &["REQ-CORE-028".to_string()],
                "governance",
            ),
            "POL-CORE-028"
        );
        assert_eq!(
            super::next_available_scaffold_id(&lookup, crate::cli::LookupKind::Feature, "task"),
            "FEAT-TASK-005"
        );
        assert_eq!(
            super::next_available_scaffold_id(
                &lookup,
                crate::cli::LookupKind::Philosophy,
                "planning"
            ),
            "PHIL-PLANNING-001"
        );
        assert_eq!(
            super::next_available_scaffold_id(&lookup, crate::cli::LookupKind::Policy, "planning"),
            "POL-PLANNING-001"
        );
        assert_eq!(super::normalize_scaffold_stem("!!!"), "task");
        assert_eq!(super::summarize_request("\n\n"), "planned task");
        assert_eq!(super::truncate_text("abcdef", 3), "...");
        assert_eq!(super::slugify("  Auth: Login++Flow  "), "auth-login-flow");
        assert_eq!(super::title_case_slug("auth-login-flow"), "Auth Login Flow");
        assert_eq!(super::id_stem("REQ"), "task");

        let error = super::feature_registry_file_label(&workspace, "outside/features/auth.yaml")
            .expect_err("external feature docs should be rejected");
        assert!(error.to_string().contains("must stay under"));

        let mut related = vec![super::SearchResult {
            id: "REQ-CORE-028".to_string(),
            kind: "requirement",
            title: "Classify request artifacts into requirement actions".to_string(),
        }];
        super::merge_related_items(
            &mut related,
            vec![super::SearchResult {
                id: "FEAT-TASK-001".to_string(),
                kind: "feature",
                title: "Request artifact classification".to_string(),
            }],
        );
        assert!(related.iter().any(|item| item.id == "FEAT-TASK-001"));
    }

    #[test]
    fn scaffold_stem_prefers_feature_area_summary_then_task_fallback() {
        let base = ClassificationOutcome {
            classification: RequirementAction::Create,
            reasons: vec!["reason".to_string()],
            explicit_items: Vec::new(),
            related_items: Vec::new(),
            request: "Create reviewer intake.".to_string(),
            context: super::RequestArtifactContext::default(),
        };

        assert_eq!(
            super::scaffold_stem(&base, &["FEAT-AUTH-LOGIN-001".to_string()]),
            "AUTH-LOGIN"
        );
        assert_eq!(super::scaffold_stem(&base, &[]), "create-reviewer-intake");

        let with_area = ClassificationOutcome {
            context: super::RequestArtifactContext {
                affected_area: Some("Checkout Flow".to_string()),
                ..super::RequestArtifactContext::default()
            },
            ..base
        };
        assert_eq!(super::scaffold_stem(&with_area, &[]), "checkout-flow");

        let fallback = ClassificationOutcome {
            request: "!!!".to_string(),
            context: super::RequestArtifactContext {
                affected_area: Some("!!!".to_string()),
                ..super::RequestArtifactContext::default()
            },
            ..with_area
        };
        assert_eq!(super::scaffold_stem(&fallback, &[]), "task");
        assert_eq!(super::scaffold_title("fallback-title"), "Fallback Title");
    }

    #[test]
    fn run_task_command_dispatches_classify_scope_scaffold_and_check() {
        let _ = TaskCommands::Classify(TaskClassifyArgs {
            request: PathBuf::from("request.yaml"),
            workspace: PathBuf::from("."),
            format: OutputFormat::Text,
        });
        let _ = TaskCommands::Scope(TaskScopeArgs {
            request: PathBuf::from("request.yaml"),
            workspace: PathBuf::from("."),
            format: OutputFormat::Text,
        });
        let _ = TaskCommands::Scaffold(TaskScaffoldArgs {
            request: PathBuf::from("request.yaml"),
            workspace: PathBuf::from("."),
            format: OutputFormat::Text,
        });
        let _ = TaskCommands::Check(TaskCheckArgs {
            plan: PathBuf::from("goal-plan.yaml"),
            range: "origin/main...HEAD".to_string(),
            workspace: PathBuf::from("."),
            format: OutputFormat::Text,
        });
        let _ = run_task_command(&TaskArgs {
            command: TaskCommands::Classify(TaskClassifyArgs {
                request: PathBuf::from("request.yaml"),
                workspace: PathBuf::from("."),
                format: OutputFormat::Text,
            }),
        });
    }

    #[test]
    fn goal_plan_artifact_supports_request_driven_and_diff_inferred_sources() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("goal-plan.yaml");
        fs::write(
            &path,
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: request_driven\n  request_artifact: request.yaml\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\n  non_goals:\n    - Add persistent task specs under spec.root\nspec_mapping:\n  persistent_items:\n    philosophies:\n      - PHIL-001\n    policies:\n      - POL-001\n    requirements:\n      - REQ-CORE-030\n    features:\n      - FEAT-TASK-003\n  spec_updates:\n    required: false\n    expected_updates: []\nimplementation_plan:\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\n    - document the temporary artifact locations\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: tests/task_command.rs\n        symbols:\n          - task_plan_generates_goal_from_request\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n",
        )
        .expect("goal plan");

        let artifact = load_goal_plan_artifact(&path).expect("goal plan should load");
        assert_eq!(artifact.kind, "syu.goal_plan");
        assert_eq!(artifact.goal.id, "GOAL-001");
        assert_eq!(artifact.source.mode, GoalPlanSourceMode::RequestDriven);
        assert_eq!(
            artifact.source.request_artifact.as_deref(),
            Some("request.yaml")
        );
        assert_eq!(artifact.coverage.mode, GoalPlanCoverageMode::ChangedLines);
        assert_eq!(
            artifact.test_plan.selection_mode,
            GoalPlanSelectionMode::Affected
        );

        fs::write(
            &path,
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: diff_inferred\n  range: origin/main...HEAD\n  confidence: high\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\n  non_goals:\n    - Add persistent task specs under spec.root\nspec_mapping:\n  persistent_items:\n    philosophies:\n      - PHIL-001\n    policies:\n      - POL-001\n    requirements:\n      - REQ-CORE-030\n    features:\n      - FEAT-TASK-003\n  spec_updates:\n    required: false\n    expected_updates: []\nimplementation_plan:\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\n    - document the temporary artifact locations\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: tests/task_command.rs\n        symbols:\n          - task_plan_generates_goal_from_request\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n",
        )
        .expect("goal plan");

        let artifact = load_goal_plan_artifact(&path).expect("goal plan should load");
        assert_eq!(artifact.source.mode, GoalPlanSourceMode::DiffInferred);
        assert_eq!(artifact.source.range.as_deref(), Some("origin/main...HEAD"));
        assert_eq!(artifact.source.confidence, Some(GoalPlanConfidence::High));
    }

    #[test]
    fn goal_plan_artifact_defaults_to_request_driven_source_when_omitted() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("goal-plan.yaml");
        fs::write(
            &path,
            "version: 1\nkind: syu.goal_plan\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("goal plan");

        let artifact = load_goal_plan_artifact(&path).expect("goal plan should load");
        assert_eq!(artifact.source.mode, GoalPlanSourceMode::RequestDriven);
        assert!(artifact.source.request_artifact.is_none());
        assert!(artifact.source.range.is_none());
        assert!(artifact.source.confidence.is_none());
    }

    #[test]
    fn goal_plan_artifact_requires_the_goal_plan_marker() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("goal-plan.yaml");
        fs::write(
            &path,
            "version: 1\nkind: syu.not_goal_plan\nsource:\n  mode: request_driven\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\nimplementation_plan:\n  scope:\n    include: []\n    exclude: []\n  steps: []\ntest_plan:\n  selection_mode: minimal\n  required_tests: {}\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\ncompletion:\n  must_pass: []\n",
        )
        .expect("goal plan");

        let error = load_goal_plan_artifact(&path).expect_err("invalid goal plan should fail");
        assert!(
            error
                .to_string()
                .contains("unsupported goal plan artifact kind")
        );
    }
}
