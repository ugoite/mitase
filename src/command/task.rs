// FEAT-TASK-001
// FEAT-TASK-003
// FEAT-TASK-004
// FEAT-TASK-005
// REQ-CORE-028
// REQ-CORE-029
// REQ-CORE-030
// REQ-CORE-031

use std::{
    collections::{BTreeMap, BTreeSet},
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
        TaskInferArgs, TaskPlanArgs, TaskPlanFormat, TaskScaffoldArgs, TaskScopeArgs,
        TaskTestSelectArgs,
    },
    coverage::normalize_relative_path,
    language::adapter_for_language,
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
    request_path: Option<String>,
    #[serde(default)]
    request: Option<String>,
    #[serde(default)]
    classification: Option<String>,
    #[serde(default)]
    source: GoalPlanSource,
    goal: GoalPlanGoal,
    #[serde(default)]
    spec_mapping: GoalPlanSpecMapping,
    implementation_plan: GoalPlanImplementationPlan,
    test_plan: GoalPlanTestPlan,
    coverage: GoalPlanCoverage,
    completion: GoalPlanCompletion,
    #[serde(default)]
    warnings: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct GoalPlanSourceEvidence {
    #[serde(default)]
    changed_files: Vec<String>,
    #[serde(default)]
    traced_requirements: Vec<String>,
    #[serde(default)]
    traced_features: Vec<String>,
    #[serde(default)]
    traced_policies: Vec<String>,
    #[serde(default)]
    traced_philosophies: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanSource {
    mode: GoalPlanSourceMode,
    #[serde(default)]
    request_artifact: Option<String>,
    #[serde(default)]
    classification: Option<String>,
    #[serde(default)]
    range: Option<String>,
    #[serde(default)]
    confidence: Option<GoalPlanConfidence>,
    #[serde(default)]
    evidence: Option<GoalPlanSourceEvidence>,
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
    #[serde(default)]
    inferred: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanSpecMapping {
    #[serde(default)]
    persistent_items: GoalPlanPersistentItems,
    #[serde(default)]
    spec_updates: GoalPlanSpecUpdates,
    #[serde(default)]
    spec_updates_required: bool,
    #[serde(default)]
    spec_update_reasons: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanPersistentItems {
    #[serde(default)]
    philosophies: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    policies: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    requirements: Vec<GoalPlanPersistentItem>,
    #[serde(default)]
    features: Vec<GoalPlanPersistentItem>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum GoalPlanPersistentItem {
    Id(String),
    Item(GoalPlanPersistentItemDetails),
}

impl GoalPlanPersistentItem {
    fn id(&self) -> &str {
        match self {
            GoalPlanPersistentItem::Id(id) => id,
            GoalPlanPersistentItem::Item(item) => &item.id,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanPersistentItemDetails {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    document_path: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum GoalPlanScopeInclude {
    Pattern(String),
    Entry(GoalPlanScopeIncludeDetails),
}

impl GoalPlanScopeInclude {
    fn pattern(&self) -> &str {
        match self {
            GoalPlanScopeInclude::Pattern(pattern) => pattern,
            GoalPlanScopeInclude::Entry(entry) => &entry.file,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanScopeIncludeDetails {
    file: String,
    #[serde(default)]
    symbols: Vec<String>,
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
    #[serde(default)]
    confidence: Option<GoalPlanConfidence>,
    scope: GoalPlanScope,
    #[serde(default)]
    steps: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanScope {
    #[serde(default)]
    include: Vec<GoalPlanScopeInclude>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GoalPlanTestPlan {
    selection_mode: GoalPlanSelectionMode,
    #[serde(default)]
    confidence: Option<GoalPlanConfidence>,
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
struct JsonTaskTestSelectOutput {
    goal_id: String,
    goal_title: String,
    selection_mode: String,
    commands: Vec<JsonTaskTestSelectCommand>,
    escalation: JsonTaskTestSelectEscalation,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskTestSelectCommand {
    language: String,
    command: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct JsonTaskTestSelectEscalation {
    level: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanOutput {
    version: u32,
    kind: String,
    request_path: String,
    request: String,
    classification: String,
    source: JsonTaskPlanSource,
    goal: JsonTaskPlanGoal,
    spec_mapping: JsonTaskPlanSpecMapping,
    implementation_plan: JsonTaskPlanImplementationPlan,
    test_plan: JsonTaskPlanTestPlan,
    coverage: JsonTaskPlanCoverage,
    completion: JsonTaskPlanCompletion,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanSourceEvidence {
    changed_files: Vec<String>,
    traced_requirements: Vec<String>,
    traced_features: Vec<String>,
    traced_policies: Vec<String>,
    traced_philosophies: Vec<String>,
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
struct JsonTaskPlanSource {
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_artifact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    classification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<String>,
    confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<JsonTaskPlanSourceEvidence>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanGoal {
    id: String,
    title: String,
    statement: String,
    non_goals: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    inferred: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanSpecMapping {
    persistent_items: JsonTaskPlanPersistentItems,
    spec_updates_required: bool,
    spec_update_reasons: Vec<String>,
}

#[derive(Debug, Serialize, Default)]
struct JsonTaskPlanPersistentItems {
    philosophies: Vec<JsonTaskPlanItem>,
    policies: Vec<JsonTaskPlanItem>,
    requirements: Vec<JsonTaskPlanItem>,
    features: Vec<JsonTaskPlanItem>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanItem {
    id: String,
    title: String,
    document_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanImplementationPlan {
    confidence: String,
    scope: JsonTaskPlanScope,
}

#[derive(Debug, Serialize, Default)]
struct JsonTaskPlanScope {
    include: Vec<JsonTaskPlanScopeEntry>,
    exclude: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanScopeEntry {
    file: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    symbols: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanTestPlan {
    selection_mode: String,
    confidence: String,
    required_tests: BTreeMap<String, Vec<JsonTaskPlanScopeEntry>>,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanCoverage {
    mode: String,
    threshold: u8,
}

#[derive(Debug, Serialize)]
struct JsonTaskPlanCompletion {
    must_pass: Vec<String>,
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
struct DiffInferenceOutcome {
    scope: ScopeOutcome,
    scope_entries: Vec<JsonTaskPlanScopeEntry>,
    source: JsonTaskPlanSourceEvidence,
    confidence: &'static str,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct TaskTestSelectionPlan {
    goal_id: String,
    goal_title: String,
    selection_mode: String,
    commands: Vec<TaskTestSelectionCommand>,
    escalation: TaskTestSelectionEscalation,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct TaskTestSelectionCommand {
    language: String,
    command: String,
    reason: String,
}

#[derive(Debug)]
struct TaskTestSelectionEscalation {
    level: String,
    reason: String,
}

#[derive(Debug, Default)]
struct TaskTestSelectionEntry {
    symbols: BTreeSet<String>,
    reasons: BTreeSet<String>,
    whole_file: bool,
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
        TaskCommands::Plan(plan) => run_task_plan_command(plan),
        TaskCommands::TestSelect(select) => run_task_test_select_command(select),
        TaskCommands::Infer(infer) => run_task_infer_command(infer),
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

pub fn run_task_plan_command(args: &TaskPlanArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let request_artifact = load_request_artifact(&args.request)?;
    let explicit_ids = request_artifact.explicit_ids();
    let scope_outcome = scope_request(&workspace, &request_artifact)?;
    let resolved_output = args
        .output
        .as_ref()
        .map(|output| resolve_task_plan_output_path(&workspace.root, output));
    let plan = build_goal_plan(
        &workspace,
        &scope_outcome,
        &explicit_ids,
        &args.request,
        args.output.as_deref(),
    )?;
    let rendered = render_goal_plan_output(
        "request",
        &args.request.display().to_string(),
        &plan,
        args.format,
    )?;

    if let Some(_output) = &args.output {
        let resolved_output = resolved_output.expect("output path should be resolved");
        if resolved_output.starts_with(&workspace.spec_root) {
            eprintln!(
                "warning: task plan output `{}` is inside spec.root `{}`; the plan is intended to stay outside the persistent spec tree",
                resolved_output.display(),
                workspace.spec_root.display()
            );
        }
        if let Some(parent) = resolved_output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved_output, rendered)?;
        println!("wrote goal plan to {}", resolved_output.display());
    } else {
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
    }

    Ok(0)
}

pub fn run_task_test_select_command(args: &TaskTestSelectArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let artifact = load_goal_plan_artifact(&args.plan)?;
    let selection = build_task_test_selection(&workspace, &artifact)?;

    match args.format {
        OutputFormat::Text => print_task_test_selection_text_output(&args.plan, &selection),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonTaskTestSelectOutput {
                goal_id: selection.goal_id,
                goal_title: selection.goal_title,
                selection_mode: selection.selection_mode,
                commands: selection
                    .commands
                    .into_iter()
                    .map(|command| JsonTaskTestSelectCommand {
                        language: command.language,
                        command: command.command,
                        reason: command.reason,
                    })
                    .collect(),
                escalation: JsonTaskTestSelectEscalation {
                    level: selection.escalation.level,
                    reason: selection.escalation.reason,
                },
                warnings: selection.warnings,
            })
            .expect("serializing task test selection output to JSON should succeed")
        ),
    }

    Ok(0)
}

pub fn run_task_infer_command(args: &TaskInferArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let range = args.range.trim();
    if range.is_empty() {
        bail!("--range must not be empty");
    }

    let changed_files = resolve_git_range_changed_files(&workspace.root, range)?;
    if changed_files.is_empty() {
        bail!("--range `{range}` does not include any changed files");
    }
    let plan =
        build_diff_inferred_goal_plan(&workspace, range, &changed_files, args.output.as_deref())?;
    let rendered = render_goal_plan_output("range", range, &plan, args.format)?;

    if let Some(output) = &args.output {
        let resolved_output = resolve_task_plan_output_path(&workspace.root, output);
        if resolved_output.starts_with(&workspace.spec_root) {
            bail!(
                "inferred Goal Plan output `{}` must stay outside the persistent spec tree `{}`",
                resolved_output.display(),
                workspace.spec_root.display()
            );
        }
        if let Some(parent) = resolved_output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved_output, rendered)?;
        println!("wrote goal plan to {}", resolved_output.display());
    } else {
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
    }

    Ok(0)
}

fn build_goal_plan(
    workspace: &crate::workspace::Workspace,
    outcome: &ScopeOutcome,
    explicit_ids: &[String],
    request_path: &Path,
    output_path: Option<&Path>,
) -> Result<JsonTaskPlanOutput> {
    let lookup = WorkspaceLookup::new(workspace);
    let persistent_items = collect_task_plan_persistent_items(&lookup, outcome)?;
    let spec_update_reasons = determine_spec_update_reasons(outcome, &persistent_items);
    let spec_updates_required = !spec_update_reasons.is_empty();
    let scope_include = collect_task_plan_scope_entries(
        workspace,
        &lookup,
        &persistent_items,
        outcome,
        explicit_ids,
    )?;
    let test_plan = collect_task_plan_tests(&lookup, &persistent_items, outcome)?;

    let source_confidence = if outcome.classification.explicit_items.is_empty()
        && outcome.classification.related_items.is_empty()
    {
        "low"
    } else if outcome.classification.explicit_items.is_empty() {
        "medium"
    } else {
        "high"
    };
    let implementation_confidence = task_plan_confidence(scope_include.len(), outcome);

    Ok(JsonTaskPlanOutput {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: request_path.display().to_string(),
        request: outcome.classification.request.clone(),
        classification: outcome.classification.classification.label().to_string(),
        source: JsonTaskPlanSource {
            mode: "request_driven".to_string(),
            request_artifact: Some(request_path.display().to_string()),
            classification: Some(outcome.classification.classification.label().to_string()),
            range: None,
            confidence: source_confidence.to_string(),
            evidence: None,
        },
        goal: JsonTaskPlanGoal {
            id: "GOAL-TASK-PLAN-001".to_string(),
            title: "Generate request-driven Goal Plans".to_string(),
            statement: format!(
                "Turn the scoped request into a temporary implementation plan linked to the current spec graph: {}",
                summarize_request(&outcome.classification.request)
            ),
            non_goals: vec![
                "Do not modify persistent spec files.".to_string(),
                "Do not add a fifth persistent spec layer.".to_string(),
                "Do not skip validation or coverage checks.".to_string(),
            ],
            inferred: false,
        },
        spec_mapping: JsonTaskPlanSpecMapping {
            persistent_items,
            spec_updates_required,
            spec_update_reasons,
        },
        implementation_plan: JsonTaskPlanImplementationPlan {
            confidence: implementation_confidence.to_string(),
            scope: JsonTaskPlanScope {
                include: scope_include,
                exclude: vec!["docs/generated/**".to_string(), "target/**".to_string()],
            },
        },
        test_plan,
        coverage: JsonTaskPlanCoverage {
            mode: "changed_lines".to_string(),
            threshold: 100,
        },
        completion: JsonTaskPlanCompletion {
            must_pass: task_plan_completion_checks(output_path),
        },
        warnings: collect_task_plan_warnings(outcome),
    })
}

fn task_plan_completion_checks(output_path: Option<&Path>) -> Vec<String> {
    let plan_path = output_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".syu/tasks/current.yaml".to_string());

    vec![
        format!("syu task check {plan_path} --range origin/main...HEAD"),
        "syu validate .".to_string(),
    ]
}

fn inferred_goal_plan_completion_checks(range: &str, output_path: Option<&Path>) -> Vec<String> {
    let plan_path = output_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "target/syu/inferred-goal.yaml".to_string());

    vec![
        format!("syu task check {plan_path} --range {range}"),
        "syu validate .".to_string(),
    ]
}

fn build_task_test_selection(
    workspace: &crate::workspace::Workspace,
    artifact: &GoalPlanArtifact,
) -> Result<TaskTestSelectionPlan> {
    let lookup = WorkspaceLookup::new(workspace);
    let mut selected = BTreeMap::<String, BTreeMap<String, TaskTestSelectionEntry>>::new();
    let mut warnings = Vec::new();

    collect_goal_plan_test_references(
        &lookup,
        "required by goal plan",
        &artifact.test_plan.required_tests,
        &mut selected,
    )?;
    collect_goal_plan_test_references(
        &lookup,
        "suggested by goal plan",
        &artifact.test_plan.suggested_tests,
        &mut selected,
    )?;
    collect_linked_requirement_tests(&lookup, artifact, &mut selected)?;

    let selection_mode = goal_plan_selection_mode_label(artifact.test_plan.selection_mode);
    let selected_test_count = count_task_test_selection_entries(&selected);
    let medium_confidence = matches!(artifact.source.confidence, Some(GoalPlanConfidence::Medium));
    let shared_utilities_changed = goal_plan_mentions_shared_utilities(artifact);
    let scope_ambiguous = goal_plan_scope_is_ambiguous(&artifact.implementation_plan.scope);

    let escalation = if selected_test_count == 0 {
        warnings.push(
            "Goal Plan does not declare required or suggested tests, so the selection falls back to the full Rust test suite.".to_string(),
        );
        TaskTestSelectionEscalation {
            level: "full".to_string(),
            reason: "Goal Plan does not declare required or suggested tests".to_string(),
        }
    } else if matches!(
        artifact.test_plan.selection_mode,
        GoalPlanSelectionMode::Full
    ) || scope_ambiguous
    {
        if scope_ambiguous {
            warnings.push(
                "Goal Plan scope is ambiguous, so the selection falls back to the full Rust test suite.".to_string(),
            );
        }
        TaskTestSelectionEscalation {
            level: "full".to_string(),
            reason: if matches!(
                artifact.test_plan.selection_mode,
                GoalPlanSelectionMode::Full
            ) {
                "Goal Plan requests full test selection".to_string()
            } else {
                "Goal Plan scope is ambiguous".to_string()
            },
        }
    } else if matches!(
        artifact.test_plan.selection_mode,
        GoalPlanSelectionMode::Affected
    ) || medium_confidence
        || shared_utilities_changed
    {
        if medium_confidence {
            warnings.push(
                "Goal Plan source confidence is medium, so the selection broadens to file-level Rust test binaries.".to_string(),
            );
        }
        if shared_utilities_changed {
            warnings.push(
                "Goal Plan evidence touches shared utility files, so the selection broadens to file-level Rust test binaries.".to_string(),
            );
        }
        TaskTestSelectionEscalation {
            level: "affected".to_string(),
            reason: join_task_test_selection_reasons(&[
                matches!(
                    artifact.test_plan.selection_mode,
                    GoalPlanSelectionMode::Affected
                )
                .then_some("Goal Plan already requests affected test selection"),
                medium_confidence.then_some("Goal Plan source confidence is medium"),
                shared_utilities_changed
                    .then_some("Goal Plan evidence touches shared utility files"),
            ]),
        }
    } else {
        TaskTestSelectionEscalation {
            level: "goal".to_string(),
            reason: "Goal Plan supplies explicit test declarations".to_string(),
        }
    };

    let commands = if escalation.level == "full" {
        vec![TaskTestSelectionCommand {
            language: "rust".to_string(),
            command: "cargo test".to_string(),
            reason: escalation.reason.clone(),
        }]
    } else if escalation.level == "affected" {
        build_task_test_selection_commands(&selected, true)
    } else {
        build_task_test_selection_commands(&selected, false)
    };

    Ok(TaskTestSelectionPlan {
        goal_id: artifact.goal.id.clone(),
        goal_title: artifact.goal.title.clone(),
        selection_mode: selection_mode.to_string(),
        commands,
        escalation,
        warnings,
    })
}

fn collect_goal_plan_test_references(
    _lookup: &WorkspaceLookup<'_>,
    source_label: &str,
    tests: &BTreeMap<String, Vec<crate::model::TraceReference>>,
    selected: &mut BTreeMap<String, BTreeMap<String, TaskTestSelectionEntry>>,
) -> Result<()> {
    for (language, references) in tests {
        validate_goal_test_language(language)?;
        for reference in references {
            add_task_test_selection_reference(
                selected,
                language,
                reference,
                source_label.to_string(),
            )?;
        }
    }

    Ok(())
}

fn collect_linked_requirement_tests(
    lookup: &WorkspaceLookup<'_>,
    artifact: &GoalPlanArtifact,
    selected: &mut BTreeMap<String, BTreeMap<String, TaskTestSelectionEntry>>,
) -> Result<()> {
    for requirement in artifact
        .spec_mapping
        .persistent_items
        .requirements
        .iter()
        .filter_map(|item| lookup.requirement(item.id()))
    {
        let label = format!("declared by linked requirement {}", requirement.id);
        collect_goal_plan_test_references(lookup, &label, &requirement.tests, selected)?;
    }

    for feature in artifact
        .spec_mapping
        .persistent_items
        .features
        .iter()
        .filter_map(|item| lookup.feature(item.id()))
    {
        for requirement_id in &feature.linked_requirements {
            if let Some(requirement) = lookup.requirement(requirement_id) {
                let label = format!(
                    "declared by linked feature {} via requirement {}",
                    feature.id, requirement.id
                );
                collect_goal_plan_test_references(lookup, &label, &requirement.tests, selected)?;
            }
        }
    }

    Ok(())
}

fn add_task_test_selection_reference(
    selected: &mut BTreeMap<String, BTreeMap<String, TaskTestSelectionEntry>>,
    language: &str,
    reference: &crate::model::TraceReference,
    reason: String,
) -> Result<()> {
    let file = normalize_relative_path(&reference.file)
        .display()
        .to_string();
    let entry = selected
        .entry(language.to_string())
        .or_default()
        .entry(file.clone())
        .or_default();
    entry.reasons.insert(reason);

    if reference.symbols.iter().any(|symbol| symbol.trim() == "*") {
        entry.whole_file = true;
        entry.symbols.clear();
        return Ok(());
    }

    let mut has_symbol = false;
    for symbol in &reference.symbols {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            entry.symbols.insert(symbol.to_string());
            has_symbol = true;
        }
    }

    if !has_symbol {
        bail!("Goal Plan test selection for `{file}` must declare at least one symbol or `*`");
    }

    Ok(())
}

fn validate_goal_test_language(language: &str) -> Result<()> {
    if language.eq_ignore_ascii_case("rust") {
        return Ok(());
    }

    if adapter_for_language(language).is_some() {
        return Ok(());
    }

    bail!("unknown test language adapter `{language}`")
}

fn count_task_test_selection_entries(
    selected: &BTreeMap<String, BTreeMap<String, TaskTestSelectionEntry>>,
) -> usize {
    selected
        .values()
        .map(|entries| {
            entries
                .values()
                .map(|entry| {
                    if entry.whole_file {
                        1
                    } else {
                        entry.symbols.len()
                    }
                })
                .sum::<usize>()
        })
        .sum()
}

fn build_task_test_selection_commands(
    selected: &BTreeMap<String, BTreeMap<String, TaskTestSelectionEntry>>,
    broaden_to_file: bool,
) -> Vec<TaskTestSelectionCommand> {
    let mut commands = Vec::new();

    for (language, files) in selected {
        if !language.eq_ignore_ascii_case("rust") {
            continue;
        }
        for (file, entry) in files {
            let reason = format_task_test_selection_reason(&entry.reasons);
            let target = rust_test_target_name(file);
            if broaden_to_file || entry.whole_file {
                commands.push(TaskTestSelectionCommand {
                    language: language.clone(),
                    command: format!("cargo test --test {target}"),
                    reason: if broaden_to_file {
                        format!("{reason}; broadened to file-level Rust test binary")
                    } else {
                        reason
                    },
                });
                continue;
            }

            for symbol in &entry.symbols {
                commands.push(TaskTestSelectionCommand {
                    language: language.clone(),
                    command: format!("cargo test --test {target} {symbol}"),
                    reason: reason.clone(),
                });
            }
        }
    }

    commands.sort_by(|left, right| left.command.cmp(&right.command));
    commands
}

fn format_task_test_selection_reason(reasons: &BTreeSet<String>) -> String {
    if reasons.is_empty() {
        return "Goal Plan declares this test".to_string();
    }

    reasons.iter().cloned().collect::<Vec<_>>().join("; ")
}

fn join_task_test_selection_reasons(reasons: &[Option<&str>]) -> String {
    let mut values = Vec::new();
    for reason in reasons.iter().copied().flatten() {
        values.push(reason.to_string());
    }

    if values.is_empty() {
        "Goal Plan declares explicit test selection".to_string()
    } else {
        values.join("; ")
    }
}

fn goal_plan_selection_mode_label(mode: GoalPlanSelectionMode) -> &'static str {
    match mode {
        GoalPlanSelectionMode::Minimal => "minimal",
        GoalPlanSelectionMode::Affected => "affected",
        GoalPlanSelectionMode::Full => "full",
    }
}

fn rust_test_target_name(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("test")
        .to_string()
}

fn goal_plan_mentions_shared_utilities(artifact: &GoalPlanArtifact) -> bool {
    artifact
        .source
        .evidence
        .as_ref()
        .map(|evidence| {
            evidence.changed_files.iter().any(|file| {
                let lower = file.to_ascii_lowercase();
                lower.contains("shared")
                    || lower.contains("util")
                    || lower.contains("common")
                    || lower.contains("helper")
            })
        })
        .unwrap_or(false)
}

fn goal_plan_scope_is_ambiguous(scope: &GoalPlanScope) -> bool {
    if scope.include.is_empty() {
        return true;
    }

    scope.include.iter().any(|pattern| {
        let pattern = pattern.pattern();
        let trimmed = pattern.trim();
        trimmed == "*"
            || trimmed == "**"
            || trimmed.contains("**")
            || trimmed.ends_with('/')
            || trimmed.contains('{')
            || scope_pattern_has_glob_metacharacters(trimmed)
    })
}

fn scope_pattern_has_glob_metacharacters(pattern: &str) -> bool {
    pattern.chars().any(|c| matches!(c, '*' | '?' | '['))
}

fn print_task_test_selection_text_output(plan_path: &Path, selection: &TaskTestSelectionPlan) {
    println!("goal plan: {}", plan_path.display());
    println!("goal id: {}", selection.goal_id);
    println!("goal title: {}", selection.goal_title);
    println!("selection mode: {}", selection.selection_mode);
    println!("escalation: {}", selection.escalation.level);
    println!("  reason: {}", selection.escalation.reason);
    println!("commands:");
    if selection.commands.is_empty() {
        println!("- none");
    } else {
        for command in &selection.commands {
            println!("- {}: {}", command.language, command.command);
            println!("  reason: {}", command.reason);
        }
    }
    if !selection.warnings.is_empty() {
        println!("warnings:");
        for warning in &selection.warnings {
            println!("- {warning}");
        }
    }
}

fn render_goal_plan_output(
    label_name: &str,
    label_value: &str,
    plan: &JsonTaskPlanOutput,
    format: TaskPlanFormat,
) -> Result<String> {
    let output = match format {
        TaskPlanFormat::Text => render_goal_plan_text_output(label_name, label_value, plan),
        TaskPlanFormat::Yaml => serde_yaml::to_string(plan)
            .expect("serializing task plan output to YAML should succeed"),
        TaskPlanFormat::Json => serde_json::to_string_pretty(plan)
            .expect("serializing task plan output to JSON should succeed"),
    };

    Ok(output)
}

fn render_goal_plan_text_output(
    label_name: &str,
    label_value: &str,
    plan: &JsonTaskPlanOutput,
) -> String {
    let mut output = String::new();
    use std::fmt::Write as _;

    writeln!(&mut output, "{label_name}: {label_value}").expect("write to string");
    writeln!(&mut output, "version: {}", plan.version).expect("write to string");
    writeln!(&mut output, "kind: {}", plan.kind).expect("write to string");
    writeln!(&mut output, "source mode: {}", plan.source.mode).expect("write to string");
    writeln!(&mut output, "classification: {}", plan.classification).expect("write to string");
    writeln!(&mut output, "source confidence: {}", plan.source.confidence)
        .expect("write to string");
    if let Some(range) = &plan.source.range {
        writeln!(&mut output, "source range: {range}").expect("write to string");
    }
    if let Some(evidence) = &plan.source.evidence {
        writeln!(&mut output, "source evidence:").expect("write to string");
        writeln!(&mut output, "  changed files:").expect("write to string");
        if evidence.changed_files.is_empty() {
            writeln!(&mut output, "    - none").expect("write to string");
        } else {
            for file in &evidence.changed_files {
                writeln!(&mut output, "    - {file}").expect("write to string");
            }
        }
        print_source_evidence_section(
            &mut output,
            "traced requirements",
            &evidence.traced_requirements,
        );
        print_source_evidence_section(&mut output, "traced features", &evidence.traced_features);
        print_source_evidence_section(&mut output, "traced policies", &evidence.traced_policies);
        print_source_evidence_section(
            &mut output,
            "traced philosophies",
            &evidence.traced_philosophies,
        );
    }
    writeln!(&mut output).expect("write to string");
    writeln!(&mut output, "goal:").expect("write to string");
    writeln!(&mut output, "  id: {}", plan.goal.id).expect("write to string");
    writeln!(&mut output, "  title: {}", plan.goal.title).expect("write to string");
    writeln!(&mut output, "  statement: {}", plan.goal.statement).expect("write to string");
    writeln!(
        &mut output,
        "  inferred: {}",
        if plan.goal.inferred { "yes" } else { "no" }
    )
    .expect("write to string");
    writeln!(&mut output, "  non-goals:").expect("write to string");
    for item in &plan.goal.non_goals {
        writeln!(&mut output, "    - {item}").expect("write to string");
    }
    writeln!(&mut output).expect("write to string");
    writeln!(&mut output, "persistent spec mapping:").expect("write to string");
    print_plan_item_section(
        &mut output,
        "philosophies",
        &plan.spec_mapping.persistent_items.philosophies,
    );
    print_plan_item_section(
        &mut output,
        "policies",
        &plan.spec_mapping.persistent_items.policies,
    );
    print_plan_item_section(
        &mut output,
        "requirements",
        &plan.spec_mapping.persistent_items.requirements,
    );
    print_plan_item_section(
        &mut output,
        "features",
        &plan.spec_mapping.persistent_items.features,
    );
    writeln!(
        &mut output,
        "  spec updates required: {}",
        if plan.spec_mapping.spec_updates_required {
            "yes"
        } else {
            "no"
        }
    )
    .expect("write to string");
    if !plan.spec_mapping.spec_update_reasons.is_empty() {
        writeln!(&mut output, "  spec update reasons:").expect("write to string");
        for reason in &plan.spec_mapping.spec_update_reasons {
            writeln!(&mut output, "    - {reason}").expect("write to string");
        }
    }
    writeln!(&mut output).expect("write to string");
    writeln!(&mut output, "implementation plan:").expect("write to string");
    writeln!(
        &mut output,
        "  confidence: {}",
        plan.implementation_plan.confidence
    )
    .expect("write to string");
    writeln!(&mut output, "  include:").expect("write to string");
    if plan.implementation_plan.scope.include.is_empty() {
        writeln!(&mut output, "    - none").expect("write to string");
    } else {
        for entry in &plan.implementation_plan.scope.include {
            writeln!(&mut output, "    - {}", render_scope_entry(entry)).expect("write to string");
        }
    }
    writeln!(&mut output, "  exclude:").expect("write to string");
    for item in &plan.implementation_plan.scope.exclude {
        writeln!(&mut output, "    - {item}").expect("write to string");
    }
    writeln!(&mut output).expect("write to string");
    writeln!(&mut output, "test plan:").expect("write to string");
    writeln!(
        &mut output,
        "  selection mode: {}",
        plan.test_plan.selection_mode
    )
    .expect("write to string");
    writeln!(&mut output, "  confidence: {}", plan.test_plan.confidence).expect("write to string");
    if plan.test_plan.required_tests.is_empty() {
        writeln!(&mut output, "  required tests:").expect("write to string");
        writeln!(&mut output, "    - none").expect("write to string");
    } else {
        writeln!(&mut output, "  required tests:").expect("write to string");
        for (language, entries) in &plan.test_plan.required_tests {
            writeln!(&mut output, "    {language}:").expect("write to string");
            for entry in entries {
                writeln!(&mut output, "      - {}", render_scope_entry(entry))
                    .expect("write to string");
            }
        }
    }
    writeln!(&mut output).expect("write to string");
    writeln!(
        &mut output,
        "coverage: {} (threshold {})",
        plan.coverage.mode, plan.coverage.threshold
    )
    .expect("write to string");
    writeln!(&mut output).expect("write to string");
    writeln!(&mut output, "completion checks:").expect("write to string");
    for item in &plan.completion.must_pass {
        writeln!(&mut output, "- {item}").expect("write to string");
    }
    if !plan.warnings.is_empty() {
        writeln!(&mut output).expect("write to string");
        writeln!(&mut output, "warnings:").expect("write to string");
        for warning in &plan.warnings {
            writeln!(&mut output, "- {warning}").expect("write to string");
        }
    }

    output
}

fn print_source_evidence_section(output: &mut String, heading: &str, items: &[String]) {
    use std::fmt::Write as _;

    writeln!(output, "  {heading}:").expect("write to string");
    if items.is_empty() {
        writeln!(output, "    - none").expect("write to string");
        return;
    }

    for item in items {
        writeln!(output, "    - {item}").expect("write to string");
    }
}

fn build_diff_inferred_goal_plan(
    workspace: &crate::workspace::Workspace,
    range: &str,
    changed_files: &[PathBuf],
    output_path: Option<&Path>,
) -> Result<JsonTaskPlanOutput> {
    let lookup = WorkspaceLookup::new(workspace);
    let inference = infer_diff_plan(workspace, &lookup, range, changed_files)?;
    let persistent_items = collect_task_plan_persistent_items(&lookup, &inference.scope)?;
    let spec_update_reasons = determine_spec_update_reasons(&inference.scope, &persistent_items);
    let spec_updates_required = !spec_update_reasons.is_empty();
    let test_plan = collect_task_plan_tests(&lookup, &persistent_items, &inference.scope)?;
    let mut test_plan = test_plan;
    test_plan.selection_mode = "affected".to_string();
    let goal_title = build_inferred_goal_title(&inference.scope, changed_files, &lookup);
    let goal_statement = build_inferred_goal_statement(
        &inference.scope,
        changed_files,
        &persistent_items,
        inference.confidence,
    );

    Ok(JsonTaskPlanOutput {
        version: GOAL_PLAN_VERSION,
        kind: "syu.goal_plan".to_string(),
        request_path: range.to_string(),
        request: format!("git diff {range}"),
        classification: inference
            .scope
            .classification
            .classification
            .label()
            .to_string(),
        source: JsonTaskPlanSource {
            mode: "diff_inferred".to_string(),
            request_artifact: None,
            classification: None,
            range: Some(range.to_string()),
            confidence: inference.confidence.to_string(),
            evidence: Some(inference.source),
        },
        goal: JsonTaskPlanGoal {
            id: "GOAL-INFERRED-001".to_string(),
            title: goal_title,
            statement: goal_statement,
            non_goals: build_inferred_goal_non_goals(inference.confidence),
            inferred: true,
        },
        spec_mapping: JsonTaskPlanSpecMapping {
            persistent_items,
            spec_updates_required,
            spec_update_reasons,
        },
        implementation_plan: JsonTaskPlanImplementationPlan {
            confidence: inference.confidence.to_string(),
            scope: JsonTaskPlanScope {
                include: inference.scope_entries,
                exclude: vec!["docs/generated/**".to_string(), "target/**".to_string()],
            },
        },
        test_plan,
        coverage: JsonTaskPlanCoverage {
            mode: "changed_lines".to_string(),
            threshold: 100,
        },
        completion: JsonTaskPlanCompletion {
            must_pass: inferred_goal_plan_completion_checks(range, output_path),
        },
        warnings: inference.warnings,
    })
}

fn infer_diff_plan(
    workspace: &crate::workspace::Workspace,
    lookup: &WorkspaceLookup<'_>,
    range: &str,
    changed_files: &[PathBuf],
) -> Result<DiffInferenceOutcome> {
    let mut changed_file_strings = Vec::new();
    let mut scope_entries = Vec::new();
    let mut direct_items = BTreeMap::<String, SearchResult>::new();
    let mut unowned_files = Vec::new();
    let mut ambiguous_files = Vec::new();
    let mut spec_files = Vec::new();

    for file in changed_files {
        let file_label = path_label(file);
        changed_file_strings.push(file_label.clone());

        let matches = collect_inferred_file_matches(workspace, lookup, file)?;
        if matches.is_spec_file {
            spec_files.push(file_label.clone());
        }
        if matches.is_unowned {
            unowned_files.push(file_label.clone());
        }
        if matches.is_ambiguous {
            ambiguous_files.push(file_label.clone());
        }

        scope_entries.push(matches.scope_entry);
        for item in matches.direct_items {
            direct_items.insert(item.id.clone(), item);
        }
    }

    changed_file_strings.sort();
    changed_file_strings.dedup();
    scope_entries.sort_by(|left, right| left.file.cmp(&right.file));
    scope_entries.dedup_by(|left, right| left.file == right.file && left.symbols == right.symbols);
    unowned_files.sort();
    unowned_files.dedup();
    ambiguous_files.sort();
    ambiguous_files.dedup();
    spec_files.sort();
    spec_files.dedup();

    let direct_items = direct_items.into_values().collect::<Vec<_>>();
    let related_items = collect_related_inference_items(lookup, &direct_items);
    let related_and_direct = merge_inference_items(&direct_items, &related_items);
    let classification = ClassificationOutcome {
        classification: infer_requirement_action(&direct_items, &related_items),
        reasons: build_inference_reasons(
            &changed_file_strings,
            &unowned_files,
            &ambiguous_files,
            &spec_files,
            &direct_items,
            &related_items,
        ),
        explicit_items: direct_items.clone(),
        related_items: related_items.clone(),
        request: format!("git diff {range}"),
        context: RequestArtifactContext::default(),
    };

    let requirements =
        collect_scoped_results(&direct_items, &related_and_direct, "requirement", 50);
    let policies = collect_scoped_results(&direct_items, &related_and_direct, "policy", 50);
    let philosophies = collect_scoped_results(&direct_items, &related_and_direct, "philosophy", 50);
    let features = collect_feature_candidates(
        lookup,
        &direct_items,
        &related_and_direct,
        classification.classification,
        50,
    );

    let scope = ScopeOutcome {
        classification,
        signals: ScopeSignals {
            policy_discussion: !policies.is_empty(),
            philosophy_discussion: !philosophies.is_empty(),
            planned_feature_updates: features.iter().any(|feature| feature.planned_state_update)
                || !spec_files.is_empty(),
        },
        requirements,
        features,
        policies,
        philosophies,
        notes: build_inference_notes(&unowned_files, &ambiguous_files, &spec_files),
    };

    let confidence = confidence_for_diff_inference(
        &changed_file_strings,
        &unowned_files,
        &ambiguous_files,
        &spec_files,
        &scope,
    );

    let warnings =
        build_inference_warnings(confidence, &unowned_files, &ambiguous_files, &spec_files);
    let source = JsonTaskPlanSourceEvidence {
        changed_files: changed_file_strings.clone(),
        traced_requirements: collect_ids_by_kind(&scope.requirements),
        traced_features: collect_feature_ids(&scope.features),
        traced_policies: collect_ids_by_kind(&scope.policies),
        traced_philosophies: collect_ids_by_kind(&scope.philosophies),
    };

    Ok(DiffInferenceOutcome {
        scope,
        source,
        confidence,
        warnings,
        scope_entries,
    })
}

#[derive(Debug)]
struct InferredFileMatches {
    direct_items: Vec<SearchResult>,
    scope_entry: JsonTaskPlanScopeEntry,
    is_spec_file: bool,
    is_unowned: bool,
    is_ambiguous: bool,
}

#[derive(Debug)]
struct TracedItemMatch {
    item: SearchResult,
    symbols: BTreeSet<String>,
}

fn collect_inferred_file_matches(
    workspace: &crate::workspace::Workspace,
    lookup: &WorkspaceLookup<'_>,
    file: &Path,
) -> Result<InferredFileMatches> {
    let file_label = path_label(file);
    let mut direct_items = BTreeMap::<String, SearchResult>::new();
    let mut symbols = BTreeSet::<String>::new();

    for item in collect_spec_items_for_path(lookup, &file_label)? {
        direct_items.insert(item.id.clone(), item);
    }

    for traced in collect_traced_items_for_path(workspace, file) {
        symbols.extend(traced.symbols.iter().cloned());
        direct_items.insert(traced.item.id.clone(), traced.item);
    }

    let is_spec_file = matches!(
        file_label.as_str(),
        label if label.starts_with("docs/syu/philosophy/")
            || label.starts_with("docs/syu/policies/")
            || label.starts_with("docs/syu/requirements/")
            || label.starts_with("docs/syu/features/")
    );
    let direct_count = direct_items.len();
    let is_unowned = direct_count == 0;
    let is_ambiguous = direct_count > 1 && !is_shared_utility_path(file);

    Ok(InferredFileMatches {
        direct_items: direct_items.into_values().collect(),
        scope_entry: JsonTaskPlanScopeEntry {
            file: file_label,
            symbols: symbols.into_iter().collect(),
        },
        is_spec_file,
        is_unowned,
        is_ambiguous,
    })
}

fn collect_spec_items_for_path(
    lookup: &WorkspaceLookup<'_>,
    file_label: &str,
) -> Result<Vec<SearchResult>> {
    let mut items = Vec::new();
    for kind in [
        LookupKind::Philosophy,
        LookupKind::Policy,
        LookupKind::Requirement,
        LookupKind::Feature,
    ] {
        for item in lookup.entries_with_document_paths(kind)? {
            if item.document_path.as_deref() == Some(file_label) {
                items.push(SearchResult {
                    id: item.id,
                    kind: kind.label(),
                    title: item.title,
                });
            }
        }
    }

    Ok(items)
}

fn collect_traced_items_for_path(
    workspace: &crate::workspace::Workspace,
    file: &Path,
) -> Vec<TracedItemMatch> {
    let normalized_file = normalize_relative_path(file);
    let mut items = BTreeMap::<String, TracedItemMatch>::new();

    for requirement in &workspace.requirements {
        for references in requirement.tests.values() {
            for reference in references {
                if normalize_relative_path(&reference.file) != normalized_file {
                    continue;
                }
                let entry =
                    items
                        .entry(requirement.id.clone())
                        .or_insert_with(|| TracedItemMatch {
                            item: SearchResult {
                                id: requirement.id.clone(),
                                kind: "requirement",
                                title: requirement.title.clone(),
                            },
                            symbols: BTreeSet::new(),
                        });
                entry.symbols.extend(reference.symbols.iter().cloned());
            }
        }
    }

    for feature in &workspace.features {
        for references in feature.implementations.values() {
            for reference in references {
                if normalize_relative_path(&reference.file) != normalized_file {
                    continue;
                }
                let entry = items
                    .entry(feature.id.clone())
                    .or_insert_with(|| TracedItemMatch {
                        item: SearchResult {
                            id: feature.id.clone(),
                            kind: "feature",
                            title: feature.title.clone(),
                        },
                        symbols: BTreeSet::new(),
                    });
                entry.symbols.extend(reference.symbols.iter().cloned());
            }
        }
    }

    items.into_values().collect()
}

fn collect_related_inference_items(
    lookup: &WorkspaceLookup<'_>,
    direct_items: &[SearchResult],
) -> Vec<SearchResult> {
    let mut requirements = BTreeMap::<String, SearchResult>::new();
    let mut features = BTreeMap::<String, SearchResult>::new();
    let mut policies = BTreeMap::<String, SearchResult>::new();
    let mut philosophies = BTreeMap::<String, SearchResult>::new();

    for item in direct_items {
        match item.kind {
            "requirement" => {
                insert_inference_item(&mut requirements, item);
                if let Some(requirement) = lookup.requirement(&item.id) {
                    for feature_id in &requirement.linked_features {
                        insert_inference_lookup_item(
                            &mut features,
                            lookup,
                            LookupKind::Feature,
                            feature_id,
                        );
                    }
                    collect_inference_requirement_context(
                        lookup,
                        requirement,
                        &mut policies,
                        &mut philosophies,
                    );
                }
            }
            "feature" => {
                insert_inference_item(&mut features, item);
                if let Some(feature) = lookup.feature(&item.id) {
                    for requirement_id in &feature.linked_requirements {
                        insert_inference_lookup_item(
                            &mut requirements,
                            lookup,
                            LookupKind::Requirement,
                            requirement_id,
                        );
                        if let Some(requirement) = lookup.requirement(requirement_id) {
                            collect_inference_requirement_context(
                                lookup,
                                requirement,
                                &mut policies,
                                &mut philosophies,
                            );
                        }
                    }
                }
            }
            "policy" => {
                insert_inference_item(&mut policies, item);
                if let Some(policy) = lookup.policy(&item.id) {
                    for philosophy_id in &policy.linked_philosophies {
                        insert_inference_lookup_item(
                            &mut philosophies,
                            lookup,
                            LookupKind::Philosophy,
                            philosophy_id,
                        );
                    }
                }
            }
            "philosophy" => {
                insert_inference_item(&mut philosophies, item);
            }
            _ => {}
        }
    }

    let mut items = requirements
        .into_values()
        .chain(features.into_values())
        .chain(policies.into_values())
        .chain(philosophies.into_values())
        .collect::<Vec<_>>();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

fn insert_inference_item(map: &mut BTreeMap<String, SearchResult>, item: &SearchResult) {
    map.entry(item.id.clone()).or_insert_with(|| item.clone());
}

fn insert_inference_lookup_item(
    map: &mut BTreeMap<String, SearchResult>,
    lookup: &WorkspaceLookup<'_>,
    kind: LookupKind,
    id: &str,
) {
    if let Some(title) = lookup.title_for(kind, id) {
        map.entry(id.to_string()).or_insert_with(|| SearchResult {
            id: id.to_string(),
            kind: kind.label(),
            title: title.to_string(),
        });
    }
}

fn collect_inference_requirement_context(
    lookup: &WorkspaceLookup<'_>,
    requirement: &crate::model::Requirement,
    policies: &mut BTreeMap<String, SearchResult>,
    philosophies: &mut BTreeMap<String, SearchResult>,
) {
    for policy_id in &requirement.linked_policies {
        insert_inference_lookup_item(policies, lookup, LookupKind::Policy, policy_id);
        if let Some(policy) = lookup.policy(policy_id) {
            for philosophy_id in &policy.linked_philosophies {
                insert_inference_lookup_item(
                    philosophies,
                    lookup,
                    LookupKind::Philosophy,
                    philosophy_id,
                );
            }
        }
    }
}

fn merge_inference_items(left: &[SearchResult], right: &[SearchResult]) -> Vec<SearchResult> {
    let mut items = BTreeMap::<String, SearchResult>::new();
    for item in left.iter().chain(right.iter()) {
        items.insert(item.id.clone(), item.clone());
    }
    items.into_values().collect()
}

fn infer_requirement_action(
    direct_items: &[SearchResult],
    related_items: &[SearchResult],
) -> RequirementAction {
    if direct_items.is_empty() && related_items.is_empty() {
        RequirementAction::Create
    } else {
        RequirementAction::Change
    }
}

fn build_inferred_goal_title(
    scope: &ScopeOutcome,
    changed_files: &[PathBuf],
    lookup: &WorkspaceLookup<'_>,
) -> String {
    if let Some(feature) = scope.features.first() {
        return format!("Extend {}", feature.title);
    }
    if let Some(requirement) = scope.requirements.first() {
        return format!("Update {}", requirement.title);
    }
    if let Some(policy) = scope.policies.first() {
        return format!("Update {}", policy.title);
    }
    if let Some(philosophy) = scope.philosophies.first() {
        return format!("Update {}", philosophy.title);
    }
    if let Some(file) = changed_files.first() {
        let file_label = path_label(file);
        if let Ok(Some(title)) = infer_title_from_file_path(lookup, &file_label) {
            return format!("Update {}", title);
        }
        return format!("Review diff for {}", file_label);
    }

    "Review inferred diff".to_string()
}

fn infer_title_from_file_path(
    lookup: &WorkspaceLookup<'_>,
    file_label: &str,
) -> Result<Option<String>> {
    for kind in [
        LookupKind::Philosophy,
        LookupKind::Policy,
        LookupKind::Requirement,
        LookupKind::Feature,
    ] {
        for item in lookup.entries_with_document_paths(kind)? {
            if item.document_path.as_deref() == Some(file_label) {
                return Ok(Some(item.title));
            }
        }
    }

    Ok(None)
}

fn build_inferred_goal_statement(
    scope: &ScopeOutcome,
    changed_files: &[PathBuf],
    persistent_items: &JsonTaskPlanPersistentItems,
    confidence: &'static str,
) -> String {
    let file_summary = changed_files
        .iter()
        .map(|path| path_label(path.as_path()))
        .take(5)
        .collect::<Vec<_>>()
        .join(", ");
    let item_summary = summarize_persistent_item_ids(persistent_items);

    if confidence == "low" {
        format!(
            "The diff appears to touch {} but the ownership evidence is weak enough that the plan should be reviewed before it is treated as certain.",
            if item_summary.is_empty() {
                file_summary
            } else {
                format!("{item_summary} and {file_summary}")
            }
        )
    } else {
        format!(
            "The diff appears to update {}{}.",
            if item_summary.is_empty() {
                file_summary
            } else {
                item_summary
            },
            if scope.notes.is_empty() {
                String::new()
            } else {
                format!(" with {}", scope.notes.join("; "))
            }
        )
    }
}

fn summarize_persistent_item_ids(items: &JsonTaskPlanPersistentItems) -> String {
    let mut ids = Vec::new();
    ids.extend(items.philosophies.iter().map(|item| item.id.as_str()));
    ids.extend(items.policies.iter().map(|item| item.id.as_str()));
    ids.extend(items.requirements.iter().map(|item| item.id.as_str()));
    ids.extend(items.features.iter().map(|item| item.id.as_str()));
    ids.join(", ")
}

fn build_inferred_goal_non_goals(confidence: &'static str) -> Vec<String> {
    let mut non_goals = vec![
        "Do not modify persistent spec files.".to_string(),
        "Do not promote the inferred goal plan into a permanent spec layer.".to_string(),
    ];
    if confidence == "low" {
        non_goals.push(
            "Do not treat ambiguous or unowned files as settled scope without review.".to_string(),
        );
    }
    non_goals
}

fn build_inference_reasons(
    changed_files: &[String],
    unowned_files: &[String],
    ambiguous_files: &[String],
    spec_files: &[String],
    direct_items: &[SearchResult],
    related_items: &[SearchResult],
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !direct_items.is_empty() {
        reasons.push(format!(
            "direct trace ownership found for {}",
            direct_items
                .iter()
                .map(|item| format!("{} {}", item.kind, item.id))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !related_items.is_empty() {
        reasons.push(format!(
            "related graph context expands to {}",
            related_items
                .iter()
                .map(|item| format!("{} {}", item.kind, item.id))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !unowned_files.is_empty() {
        reasons.push(format!(
            "unowned files reduce confidence: {}",
            unowned_files.join(", ")
        ));
    }
    if !ambiguous_files.is_empty() {
        reasons.push(format!(
            "ambiguous ownership reduces confidence: {}",
            ambiguous_files.join(", ")
        ));
    }
    if !spec_files.is_empty() {
        reasons.push(format!(
            "spec files changed directly: {}",
            spec_files.join(", ")
        ));
    }
    if reasons.is_empty() {
        reasons.push(format!(
            "changed files were analyzed from {}",
            changed_files.join(", ")
        ));
    }

    reasons
}

fn build_inference_notes(
    unowned_files: &[String],
    ambiguous_files: &[String],
    spec_files: &[String],
) -> Vec<String> {
    let mut notes = Vec::new();
    if !unowned_files.is_empty() {
        notes.push(format!(
            "no trace ownership was found for {}",
            unowned_files.join(", ")
        ));
    }
    if !ambiguous_files.is_empty() {
        notes.push(format!(
            "ambiguous ownership needs review for {}",
            ambiguous_files.join(", ")
        ));
    }
    if !spec_files.is_empty() {
        notes.push(format!(
            "spec files were included in the diff: {}",
            spec_files.join(", ")
        ));
    }
    notes
}

fn build_inference_warnings(
    confidence: &'static str,
    unowned_files: &[String],
    ambiguous_files: &[String],
    spec_files: &[String],
) -> Vec<String> {
    let mut warnings = Vec::new();
    if confidence == "low" {
        if !unowned_files.is_empty() {
            warnings.push(format!(
                "Low confidence: no trace ownership was found for {}.",
                unowned_files.join(", ")
            ));
        }
        if !ambiguous_files.is_empty() {
            warnings.push(format!(
                "Low confidence: ambiguous ownership remains for {}.",
                ambiguous_files.join(", ")
            ));
        }
    } else if confidence == "medium" {
        if !spec_files.is_empty() {
            warnings.push(format!(
                "Medium confidence: spec files changed directly in {}.",
                spec_files.join(", ")
            ));
        }
        if !ambiguous_files.is_empty() {
            warnings.push(format!(
                "Medium confidence: shared or ambiguous ownership was inferred for {}.",
                ambiguous_files.join(", ")
            ));
        }
    }
    warnings
}

fn confidence_for_diff_inference(
    changed_files: &[String],
    unowned_files: &[String],
    ambiguous_files: &[String],
    spec_files: &[String],
    scope: &ScopeOutcome,
) -> &'static str {
    if !unowned_files.is_empty() {
        return "low";
    }
    if !spec_files.is_empty()
        || changed_files.len() > 1
        || changed_files
            .iter()
            .any(|file| is_shared_utility_path(Path::new(file)))
    {
        return "medium";
    }
    if !ambiguous_files.is_empty()
        && !scope
            .features
            .iter()
            .any(|feature| feature.status.eq_ignore_ascii_case("planned"))
    {
        return "low";
    }
    "high"
}

fn is_shared_utility_path(path: &Path) -> bool {
    let rendered = path_label(path).to_lowercase();
    rendered.contains("shared")
        || rendered.contains("common")
        || rendered.contains("util")
        || rendered.contains("helper")
        || rendered.contains("generated")
}

fn collect_ids_by_kind(items: &[SearchResult]) -> Vec<String> {
    let mut ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn collect_feature_ids(items: &[ScopeFeatureCandidate]) -> Vec<String> {
    let mut ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn print_plan_item_section(output: &mut String, heading: &str, items: &[JsonTaskPlanItem]) {
    use std::fmt::Write as _;

    writeln!(output, "  {heading}:").expect("write to string");
    if items.is_empty() {
        writeln!(output, "    - none").expect("write to string");
        return;
    }

    for item in items {
        let document_path = item
            .document_path
            .as_deref()
            .map(|path| format!(" ({path})"))
            .unwrap_or_default();
        writeln!(output, "    - {} {}{}", item.id, item.title, document_path)
            .expect("write to string");
    }
}

fn render_scope_entry(entry: &JsonTaskPlanScopeEntry) -> String {
    if entry.symbols.is_empty() {
        entry.file.clone()
    } else {
        format!("{} [{}]", entry.file, entry.symbols.join(", "))
    }
}

fn resolve_task_plan_output_path(workspace_root: &Path, output: &Path) -> PathBuf {
    if output.is_absolute() {
        return output.to_path_buf();
    }

    workspace_root.join(normalize_relative_path(output))
}

fn collect_task_plan_persistent_items(
    lookup: &WorkspaceLookup<'_>,
    outcome: &ScopeOutcome,
) -> Result<JsonTaskPlanPersistentItems> {
    let mut items = JsonTaskPlanPersistentItems::default();
    let mut seen = BTreeSet::new();

    for result in outcome
        .requirements
        .iter()
        .chain(outcome.policies.iter())
        .chain(outcome.philosophies.iter())
    {
        if !seen.insert(result.id.clone()) {
            continue;
        }

        let item = JsonTaskPlanItem {
            id: result.id.clone(),
            title: result.title.clone(),
            document_path: lookup.document_path_for_id(&result.id)?,
        };
        match result.kind {
            "philosophy" => items.philosophies.push(item),
            "policy" => items.policies.push(item),
            "requirement" => items.requirements.push(item),
            _ => {}
        }
    }

    for feature in &outcome.features {
        let result = SearchResult {
            id: feature.id.clone(),
            kind: "feature",
            title: feature.title.clone(),
        };
        if !seen.insert(result.id.clone()) {
            continue;
        }

        let item = JsonTaskPlanItem {
            id: result.id.clone(),
            title: result.title.clone(),
            document_path: lookup.document_path_for_id(&result.id)?,
        };
        match result.kind {
            "philosophy" => items.philosophies.push(item),
            "policy" => items.policies.push(item),
            "requirement" => items.requirements.push(item),
            "feature" => items.features.push(item),
            _ => {}
        }
    }

    items.philosophies.sort_by(|a, b| a.id.cmp(&b.id));
    items.policies.sort_by(|a, b| a.id.cmp(&b.id));
    items.requirements.sort_by(|a, b| a.id.cmp(&b.id));
    items.features.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(items)
}

fn determine_spec_update_reasons(
    outcome: &ScopeOutcome,
    persistent_items: &JsonTaskPlanPersistentItems,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if persistent_items.requirements.is_empty() {
        reasons.push(
            "No durable requirement was identified, so a new or expanded requirement is likely needed before implementation."
                .to_string(),
        );
    }
    if persistent_items.features.is_empty() {
        reasons.push(
            "No feature anchor was identified, so the plan is still provisional and may need a new feature definition."
                .to_string(),
        );
    }
    if outcome.classification.explicit_items.is_empty() {
        reasons.push(
            "The request does not name concrete spec IDs, so the mapping is inferred from the current graph and request text."
                .to_string(),
        );
    }
    if outcome.classification.related_items.is_empty() {
        reasons.push(
            "No close graph matches were found, which is a strong signal that spec updates are likely required."
                .to_string(),
        );
    }
    if outcome.signals.planned_feature_updates {
        reasons.push(
            "The scoped feature candidates suggest planned-state updates may be required before implementation."
                .to_string(),
        );
    }

    reasons
}

fn collect_task_plan_scope_entries(
    workspace: &crate::workspace::Workspace,
    lookup: &WorkspaceLookup<'_>,
    persistent_items: &JsonTaskPlanPersistentItems,
    outcome: &ScopeOutcome,
    explicit_ids: &[String],
) -> Result<Vec<JsonTaskPlanScopeEntry>> {
    let mut files = BTreeMap::<String, BTreeSet<String>>::new();

    let feature_ids = persistent_items
        .features
        .iter()
        .map(|item| item.id.as_str())
        .chain(
            explicit_ids
                .iter()
                .map(|id| id.as_str())
                .filter(|id| id.starts_with("FEAT-")),
        );

    for feature_id in feature_ids {
        collect_feature_scope_entries(workspace, feature_id, &mut files);
    }

    for requirement in persistent_items
        .requirements
        .iter()
        .filter_map(|item| lookup.requirement(&item.id))
    {
        for feature_id in &requirement.linked_features {
            collect_feature_scope_entries(workspace, feature_id, &mut files);
        }
    }

    if files.is_empty() && !outcome.features.is_empty() {
        for feature in &outcome.features {
            collect_feature_scope_entries(workspace, &feature.id, &mut files);
        }
    }

    let mut entries = files
        .into_iter()
        .map(|(file, symbols)| JsonTaskPlanScopeEntry {
            file,
            symbols: symbols.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.file.cmp(&b.file));
    entries.dedup_by(|a, b| a.file == b.file && a.symbols == b.symbols);
    Ok(entries)
}

fn collect_feature_scope_entries(
    workspace: &crate::workspace::Workspace,
    feature_id: &str,
    files: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let Some(feature) = workspace.features.iter().find(|item| item.id == feature_id) else {
        return;
    };

    for references in feature.implementations.values() {
        for reference in references {
            let file = normalize_relative_path(&reference.file)
                .display()
                .to_string();
            let symbols = files.entry(file).or_default();
            for symbol in &reference.symbols {
                symbols.insert(symbol.clone());
            }
        }
    }
}

fn collect_task_plan_tests(
    lookup: &WorkspaceLookup<'_>,
    persistent_items: &JsonTaskPlanPersistentItems,
    outcome: &ScopeOutcome,
) -> Result<JsonTaskPlanTestPlan> {
    let mut required_tests = BTreeMap::<String, Vec<JsonTaskPlanScopeEntry>>::new();
    let mut seen = BTreeSet::new();

    for requirement in persistent_items
        .requirements
        .iter()
        .filter_map(|item| lookup.requirement(&item.id))
    {
        for (language, tests) in &requirement.tests {
            let entries = required_tests.entry(language.clone()).or_default();
            for reference in tests {
                let file = normalize_relative_path(&reference.file)
                    .display()
                    .to_string();
                let key = format!("{language}:{file}:{}", reference.symbols.join(","));
                if !seen.insert(key) {
                    continue;
                }
                entries.push(JsonTaskPlanScopeEntry {
                    file,
                    symbols: reference.symbols.clone(),
                });
            }
        }
    }

    for feature in persistent_items
        .features
        .iter()
        .filter_map(|item| lookup.feature(&item.id))
    {
        for requirement_id in &feature.linked_requirements {
            if let Some(requirement) = lookup.requirement(requirement_id) {
                for (language, tests) in &requirement.tests {
                    let entries = required_tests.entry(language.clone()).or_default();
                    for reference in tests {
                        let file = normalize_relative_path(&reference.file)
                            .display()
                            .to_string();
                        let key = format!("{language}:{file}:{}", reference.symbols.join(","));
                        if !seen.insert(key) {
                            continue;
                        }
                        entries.push(JsonTaskPlanScopeEntry {
                            file,
                            symbols: reference.symbols.clone(),
                        });
                    }
                }
            }
        }
    }

    for entries in required_tests.values_mut() {
        entries.sort_by(|a, b| a.file.cmp(&b.file));
        entries.dedup_by(|a, b| a.file == b.file && a.symbols == b.symbols);
    }

    let selection_mode = if required_tests.is_empty() {
        "minimal"
    } else {
        "linked-declarations"
    };
    let confidence = task_plan_confidence_for_tests(required_tests.len(), outcome);

    Ok(JsonTaskPlanTestPlan {
        selection_mode: selection_mode.to_string(),
        confidence: confidence.to_string(),
        required_tests,
    })
}

fn task_plan_confidence(scope_include_count: usize, outcome: &ScopeOutcome) -> &'static str {
    if scope_include_count == 0 || outcome.classification.related_items.is_empty() {
        "low"
    } else if outcome.classification.explicit_items.is_empty() {
        "medium"
    } else {
        "high"
    }
}

fn task_plan_confidence_for_tests(test_group_count: usize, outcome: &ScopeOutcome) -> &'static str {
    if test_group_count == 0 || outcome.classification.related_items.is_empty() {
        "low"
    } else {
        "high"
    }
}

fn collect_task_plan_warnings(outcome: &ScopeOutcome) -> Vec<String> {
    let mut warnings = Vec::new();
    if outcome.classification.explicit_items.is_empty() {
        warnings.push(
            "The plan is inferred from request text and graph matches, so review the proposed scope carefully."
                .to_string(),
        );
    }
    if outcome.classification.related_items.is_empty() {
        warnings.push(
            "No close graph matches were found, so the plan may need a new requirement or feature before implementation."
                .to_string(),
        );
    }
    if outcome.features.is_empty() {
        warnings.push(
            "No implementation scope could be inferred from feature candidates, so the plan is intentionally conservative."
                .to_string(),
        );
    }
    warnings
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

        if !artifact.goal.inferred {
            issues.push(Issue::warning(
                "GOAL-TASK-PLAN-012",
                "goal.inferred",
                None,
                "diff-inferred plan goal does not mark itself as inferred",
                Some(
                    "Set goal.inferred to true so readers can tell the plan was derived from a diff."
                        .to_string(),
                ),
            ));
        }

        match artifact.source.evidence.as_ref() {
            Some(evidence) if evidence.changed_files.is_empty() => issues.push(Issue::error(
                "GOAL-TASK-PLAN-013",
                "source.evidence.changed_files",
                None,
                "diff-inferred plan does not record any changed files",
                Some(
                    "Add the diff's changed files to source.evidence.changed_files so the inferred plan is explainable."
                        .to_string(),
                ),
            )),
            None => issues.push(Issue::error(
                "GOAL-TASK-PLAN-013",
                "source.evidence",
                None,
                "diff-inferred plan does not record evidence",
                Some(
                    "Add source.evidence with changed files and traced IDs so reviewers can validate the inference."
                        .to_string(),
                ),
            )),
            _ => {}
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

    let include_patterns = artifact
        .implementation_plan
        .scope
        .include
        .iter()
        .map(GoalPlanScopeInclude::pattern)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let include_matcher = build_globset(&include_patterns)?;
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
        let id = id.id();
        if lookup.find(id).is_none() {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-002",
                "spec_mapping.persistent_items",
                Some(id.to_string()),
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
    let location = reference.file.display().to_string();
    let test_path = match resolve_goal_plan_test_path(workspace, &reference.file, issues)? {
        Some(path) => path,
        None => return Ok(()),
    };
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
        let symbol = symbol.trim();
        if symbol == "*" {
            continue;
        }
        if symbol.is_empty() {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-003",
                "test_plan.required_tests",
                Some(location.clone()),
                "required test symbol is empty",
                Some("Specify a test function, method, or symbol name.".to_string()),
            ));
            continue;
        }
        if !goal_plan_required_test_symbol_exists(language, &contents, symbol) {
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

fn resolve_goal_plan_test_path(
    workspace: &crate::workspace::Workspace,
    reference_file: &Path,
    issues: &mut Vec<Issue>,
) -> Result<Option<PathBuf>> {
    let candidate = if reference_file.is_absolute() {
        reference_file.to_path_buf()
    } else {
        workspace.root.join(reference_file)
    };

    let resolved = match fs::canonicalize(&candidate) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            issues.push(Issue::error(
                "GOAL-TASK-PLAN-003",
                "test_plan.required_tests",
                Some(reference_file.display().to_string()),
                "required test file is missing",
                Some(
                    "Create the referenced test file or update the Goal Plan to point at an existing repository test."
                        .to_string(),
                ),
            ));
            return Ok(None);
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to resolve required test file `{}`",
                    reference_file.display()
                )
            });
        }
    };

    if !resolved.starts_with(&workspace.root) {
        issues.push(Issue::error(
            "GOAL-TASK-PLAN-003",
            "test_plan.required_tests",
            Some(reference_file.display().to_string()),
            "required test file must stay within the workspace",
            Some("Point required tests at a repository file under the workspace root.".to_string()),
        ));
        return Ok(None);
    }

    Ok(Some(resolved))
}

fn goal_plan_required_test_symbol_exists(language: &str, contents: &str, symbol: &str) -> bool {
    adapter_for_language(language)
        .map(|adapter| {
            let mut patterns = adapter.patterns(symbol);
            if patterns.len() > 1 {
                patterns.pop();
            }
            patterns
                .into_iter()
                .filter_map(|pattern| Regex::new(&pattern).ok())
                .any(|regex| regex.is_match(contents))
        })
        .unwrap_or_else(|| contents.contains(symbol))
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
        OutputFormat, TaskArgs, TaskCheckArgs, TaskClassifyArgs, TaskCommands, TaskPlanArgs,
        TaskPlanFormat, TaskScaffoldArgs, TaskScopeArgs, TaskTestSelectArgs,
    };

    use super::{
        ClassificationOutcome, GoalPlanSourceMode, RequirementAction, SearchResult,
        WorkspaceLookup, build_goal_plan, build_scaffold_plan, classify_request,
        collect_feature_candidates, load_goal_plan_artifact, load_request_artifact,
        render_goal_plan_output, resolve_task_plan_output_path, run_task_command, scope_request,
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
            "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n      - REQ-CORE-029\n      - REQ-CORE-030\n      - REQ-CORE-031\n",
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
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-032\n    title: Validate temporary Goal Plans against the current spec graph and git range\n    description: The task check command should validate Goal Plan conformance against changed files, linked spec IDs, required tests, and completion commands.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-005\n    tests:\n      rust:\n        - file: tests/task_command.rs\n          symbols:\n            - task_check_reports_pass_fail_results_for_goal_plans\n",
        )
        .expect("check requirement doc");
        fs::write(
            root.join("docs/syu/features/features.yaml"),
            "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n  - kind: task\n    file: core/scaffold.yaml\n  - kind: task\n    file: core/scope.yaml\n  - kind: task\n    file: core/plan.yaml\n",
        )
        .expect("feature registry");
        fs::write(
            root.join("docs/syu/features/core/task.yaml"),
            "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify planned request artifacts into create, change, or delete decisions using the current spec graph and a brief explanation.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskClassifyArgs\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: planned\n    linked_requirements:\n      - REQ-CORE-030\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scope_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskScopeArgs\n  - id: FEAT-TASK-005\n    title: Goal Plan conformance checking\n    summary: Validate temporary Goal Plan artifacts against changed files, linked spec IDs, required tests, and declared completion commands before review.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-032\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_check_command\n            - load_goal_plan_artifact\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskCheckArgs\n        - file: src/lib.rs\n          symbols:\n            - dispatch\n            - run_dispatch\n",
        )
        .expect("feature doc");
        fs::write(
            root.join("docs/syu/requirements/core/plan.yaml"),
            "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-031\n    title: Generate temporary Goal Plans from scoped requests\n    description: The task plan command should turn a scoped request artifact into a temporary Goal Plan while keeping persistent spec files untouched.\n    priority: medium\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-004\n    tests: {}\n",
        )
        .expect("plan requirement doc");
        fs::write(
            root.join("docs/syu/features/core/plan.yaml"),
            "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-004\n    title: Goal Plan generation\n    summary: Turn scoped request artifacts into temporary Goal Plans with implementation, test, coverage, and completion sections outside the persistent spec tree.\n    status: planned\n    linked_requirements:\n      - REQ-CORE-031\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_plan_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskPlanArgs\n",
        )
        .expect("plan feature doc");
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
            "FEAT-TASK-006"
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
        let _ = TaskCommands::Plan(TaskPlanArgs {
            request: PathBuf::from("request.yaml"),
            workspace: PathBuf::from("."),
            output: None,
            format: TaskPlanFormat::Text,
        });
        let _ = TaskCommands::TestSelect(TaskTestSelectArgs {
            plan: PathBuf::from("goal-plan.yaml"),
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
    fn goal_plan_builder_renders_outputs_and_writes_files() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(
            &request,
            "Generate a plan for the current request-driven workflow.",
            &["PHIL-001", "POL-001", "REQ-CORE-030", "FEAT-TASK-003"],
        );

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let scope_outcome = scope_request(&workspace, &artifact).expect("scope");
        let explicit_ids = artifact.explicit_ids();
        let plan = build_goal_plan(&workspace, &scope_outcome, &explicit_ids, &request, None)
            .expect("goal plan");

        assert_eq!(plan.kind, "syu.goal_plan");
        assert_eq!(plan.source.mode, "request_driven");
        assert_eq!(plan.source.confidence, "high");
        assert_eq!(plan.coverage.threshold, 100);
        assert!(plan.spec_mapping.spec_updates_required);
        assert!(!plan.spec_mapping.spec_update_reasons.is_empty());
        assert!(
            plan.implementation_plan
                .scope
                .include
                .iter()
                .any(|entry| entry.file.contains("src/command/task.rs"))
        );

        let text = render_goal_plan_output(
            "request",
            &request.display().to_string(),
            &plan,
            TaskPlanFormat::Text,
        )
        .expect("text render");
        assert!(text.contains("kind: syu.goal_plan"));
        assert!(text.contains("goal:"));
        assert!(text.contains("implementation plan:"));
        assert!(text.contains("test plan:"));
        assert!(text.contains("coverage: changed_lines (threshold 100)"));
        assert!(text.contains("syu task check .syu/tasks/current.yaml --range origin/main...HEAD"));

        let yaml = render_goal_plan_output(
            "request",
            &request.display().to_string(),
            &plan,
            TaskPlanFormat::Yaml,
        )
        .expect("yaml render");
        assert!(yaml.contains("kind: syu.goal_plan"));

        let json = render_goal_plan_output(
            "request",
            &request.display().to_string(),
            &plan,
            TaskPlanFormat::Json,
        )
        .expect("json render");
        assert!(json.contains("\"kind\": \"syu.goal_plan\""));

        let output_path = tempdir.path().join(".syu/tasks/current.yaml");
        let resolved = resolve_task_plan_output_path(tempdir.path(), &output_path);
        assert_eq!(resolved, output_path);
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).expect("create output dir");
        }
        fs::write(&resolved, text).expect("write plan");
        assert!(resolved.exists());

        let custom_output = tempdir.path().join("docs/syu/plans/current.yaml");
        let custom_plan = build_goal_plan(
            &workspace,
            &scope_outcome,
            &explicit_ids,
            &request,
            Some(custom_output.as_path()),
        )
        .expect("goal plan");
        assert!(
            custom_plan
                .completion
                .must_pass
                .iter()
                .any(|check| check.contains(&format!(
                    "syu task check {}",
                    custom_output.display()
                )))
        );
    }

    #[test]
    fn goal_plan_builder_marks_low_confidence_and_warns_when_scope_is_sparse() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let request = tempdir.path().join("request.yaml");
        write_request_artifact(&request, "Introduce a brand new planning path.", &[]);

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_request_artifact(&request).expect("request");
        let scope_outcome = scope_request(&workspace, &artifact).expect("scope");
        let explicit_ids = artifact.explicit_ids();
        let plan = build_goal_plan(&workspace, &scope_outcome, &explicit_ids, &request, None)
            .expect("goal plan");

        assert_eq!(plan.source.confidence, "low");
        assert_eq!(plan.implementation_plan.confidence, "low");
        assert_eq!(plan.test_plan.confidence, "low");
        assert!(plan.spec_mapping.persistent_items.requirements.is_empty());
        assert!(plan.spec_mapping.persistent_items.features.is_empty());
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("inferred from request text"))
        );
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("No close graph matches were found"))
        );
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("No implementation scope could be inferred"))
        );
    }

    #[test]
    fn task_test_select_rejects_empty_symbol_lists() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let path = tempdir.path().join("goal-plan.yaml");
        fs::write(
            &path,
            "version: 1\nkind: syu.goal_plan\nsource:\n  mode: request_driven\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\nimplementation_plan:\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: src/command/task.rs\n        symbols: []\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n",
        )
        .expect("goal plan");

        let workspace = crate::workspace::load_workspace(tempdir.path()).expect("workspace");
        let artifact = load_goal_plan_artifact(&path).expect("goal plan should load");
        let error =
            super::build_task_test_selection(&workspace, &artifact).expect_err("empty symbols");

        assert!(
            error
                .to_string()
                .contains("must declare at least one symbol or `*`")
        );
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
