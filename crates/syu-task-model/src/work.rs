use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use syu_domain::TraceReference;

use crate::{
    GoalPlanArtifact, GoalPlanCompletion, GoalPlanConfidence, GoalPlanCoverage,
    GoalPlanCoverageMode, GoalPlanGoal, GoalPlanImplementationPlan, GoalPlanPersistentItem,
    GoalPlanPersistentItems, GoalPlanScope, GoalPlanScopeInclude, GoalPlanSelectionMode,
    GoalPlanSource, GoalPlanSourceEvidence, GoalPlanSourceMode, GoalPlanSpecMapping,
    GoalPlanTestPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    Deliver,
    Specify,
    Govern,
    Restructure,
    Verify,
    Repair,
    Maintain,
    Retire,
    Review,
    Adopt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOperation {
    Create,
    Modify,
    Delete,
    Rename,
    Move,
    Relink,
    Split,
    Merge,
    Promote,
    Demote,
    Supersede,
    Validate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkSurface {
    Philosophy,
    Policy,
    Requirement,
    Feature,
    Implementation,
    Test,
    Trace,
    Config,
    Documentation,
    Tooling,
    GeneratedArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkMode {
    PlanAndExecute,
    PlanOnly,
    ReviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    Seed,
    Inferred,
    SearchCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactRole {
    DirectChange,
    Context,
    FollowUp,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkSeed {
    pub id: String,
    pub surface: WorkSurface,
    pub source_role: SourceRole,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkConstraints {
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target_surface: Option<WorkSurface>,
    #[serde(default)]
    pub destination_document: Option<String>,
    #[serde(default)]
    pub related_item_ids: Vec<String>,
    #[serde(default)]
    pub remove_edges: Vec<SpecEdge>,
    #[serde(default)]
    pub add_edges: Vec<SpecEdge>,
    #[serde(default)]
    pub redistribution: Vec<RelationshipMove>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkIntent {
    pub kind: WorkKind,
    pub operation: WorkOperation,
    pub mode: WorkMode,
    #[serde(default)]
    pub seeds: Vec<WorkSeed>,
    #[serde(default)]
    pub requested_surfaces: BTreeSet<WorkSurface>,
    #[serde(default)]
    pub constraints: WorkConstraints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceTarget {
    pub language: String,
    pub file: String,
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkGraphNode {
    pub id: String,
    pub surface: WorkSurface,
    #[serde(default)]
    pub document_path: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub linked_ids: Vec<String>,
    #[serde(default)]
    pub implementations: Vec<TraceTarget>,
    #[serde(default)]
    pub tests: Vec<TraceTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryChange {
    pub path: String,
    #[serde(default)]
    pub owner_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkDiagnostic {
    pub rule: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPlanningInput {
    pub request: String,
    #[serde(default)]
    pub explicit_kind: Option<WorkKind>,
    #[serde(default)]
    pub explicit_operation: Option<WorkOperation>,
    #[serde(default)]
    pub explicit_mode: Option<WorkMode>,
    #[serde(default)]
    pub seeds: Vec<WorkSeed>,
    #[serde(default)]
    pub search_candidates: Vec<WorkSeed>,
    #[serde(default)]
    pub nodes: Vec<WorkGraphNode>,
    #[serde(default)]
    pub repository_changes: Vec<RepositoryChange>,
    #[serde(default)]
    pub diagnostics: Vec<WorkDiagnostic>,
    #[serde(default)]
    pub constraints: WorkConstraints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactedItem {
    pub id: String,
    pub surface: WorkSurface,
    pub source_role: SourceRole,
    pub impact_role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactedEdge {
    pub from: String,
    pub to: String,
    pub impact_role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryImpact {
    pub path: String,
    pub surface: WorkSurface,
    pub impact_role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestImpact {
    pub target: TraceTarget,
    pub impact_role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticImpact {
    pub rule: String,
    pub subject: String,
    pub impact_role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalSplitSuggestion {
    pub reason: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkImpact {
    #[serde(default)]
    pub items: Vec<ImpactedItem>,
    #[serde(default)]
    pub edges: Vec<ImpactedEdge>,
    #[serde(default)]
    pub repository: Vec<RepositoryImpact>,
    #[serde(default)]
    pub tests: Vec<TestImpact>,
    #[serde(default)]
    pub diagnostics: Vec<DiagnosticImpact>,
    #[serde(default)]
    pub split_suggestions: Vec<GoalSplitSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecItemDraft {
    pub id: String,
    pub surface: WorkSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecItemPatch {
    #[serde(default)]
    pub requested_surfaces: BTreeSet<WorkSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipMove {
    pub relationship: SpecEdge,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkMutation {
    CreateItem {
        draft: SpecItemDraft,
    },
    ModifyItem {
        id: String,
        patch: SpecItemPatch,
    },
    DeleteItem {
        id: String,
    },
    RenameItem {
        from: String,
        to: String,
    },
    MoveItem {
        id: String,
        destination_document: String,
    },
    Relink {
        remove: Vec<SpecEdge>,
        add: Vec<SpecEdge>,
    },
    SplitItem {
        source: String,
        targets: Vec<SpecItemDraft>,
        redistribution: Vec<RelationshipMove>,
    },
    MergeItems {
        sources: Vec<String>,
        target: String,
    },
    ChangeItemKind {
        id: String,
        target_surface: WorkSurface,
        relink: Vec<SpecEdge>,
    },
    Supersede {
        old: String,
        replacement: String,
    },
    UpdateTrace {
        target: TraceTarget,
    },
    RepairDiagnostic {
        rule: String,
        subject: String,
    },
    ModifyRepository {
        path: String,
    },
    CreateRepository {
        path: String,
    },
    DeleteRepository {
        path: String,
    },
    MoveRepository {
        path: String,
        destination: String,
    },
    ModifyConfig {
        path: String,
    },
    CreateConfig {
        path: String,
    },
    MoveConfig {
        path: String,
        destination: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationGenre {
    Workspace,
    Graph,
    Delivery,
    Trace,
    Coverage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompletionCheck {
    Validate { genres: BTreeSet<ValidationGenre> },
    GoalPlanCheck { plan_path: String, range: String },
    TestSelection { plan_path: String },
    Command { argv: Vec<String> },
}

impl CompletionCheck {
    pub fn render(&self) -> String {
        match self {
            Self::Validate { genres } if genres.is_empty() => "syu validate .".to_string(),
            Self::Validate { genres } => genres
                .iter()
                .map(|genre| format!("syu check . --genre {}", genre.label()))
                .collect::<Vec<_>>()
                .join(" && "),
            Self::GoalPlanCheck { plan_path, range } => {
                format!("syu task check {plan_path} --range {range}")
            }
            Self::TestSelection { plan_path } => format!("syu task test-select {plan_path}"),
            Self::Command { argv } => argv.join(" "),
        }
    }
}

impl ValidationGenre {
    fn label(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Graph => "graph",
            Self::Delivery => "delivery",
            Self::Trace => "trace",
            Self::Coverage => "coverage",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRequirement {
    #[serde(default)]
    pub all_of: BTreeSet<WorkSurface>,
    #[serde(default)]
    pub any_of: Vec<BTreeSet<WorkSurface>>,
    #[serde(default)]
    pub forbidden_mutations: BTreeSet<WorkSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkKindProfile {
    pub kind: WorkKind,
    pub surface_requirement: SurfaceRequirement,
    pub completion: Vec<CompletionCheck>,
    pub cargo_test_fallback: bool,
    pub mutation_forbidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkVerification {
    pub contract: SurfaceRequirement,
    pub completion: Vec<CompletionCheck>,
    pub cargo_test_fallback: bool,
    pub mutation_forbidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPlan {
    pub intent: WorkIntent,
    pub impact: WorkImpact,
    pub mutations: Vec<WorkMutation>,
    pub verification: WorkVerification,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalPlanConversionContext {
    pub goal_id: String,
    pub source_mode: GoalPlanSourceMode,
    pub source_path: Option<String>,
    pub plan_output_path: String,
    pub range: Option<String>,
    pub confidence: GoalPlanConfidence,
}

impl Default for GoalPlanConversionContext {
    fn default() -> Self {
        Self {
            goal_id: "GOAL-WORK-001".to_string(),
            source_mode: GoalPlanSourceMode::RequestDriven,
            source_path: None,
            plan_output_path: ".syu/tasks/current.yaml".to_string(),
            range: None,
            confidence: GoalPlanConfidence::High,
        }
    }
}

pub fn goal_plan_from_work_plan(
    request: &str,
    plan: &WorkPlan,
    context: &GoalPlanConversionContext,
) -> GoalPlanArtifact {
    let mut items = GoalPlanPersistentItems::default();
    for item in &plan.impact.items {
        let target = match item.surface {
            WorkSurface::Philosophy => &mut items.philosophies,
            WorkSurface::Policy => &mut items.policies,
            WorkSurface::Requirement => &mut items.requirements,
            WorkSurface::Feature => &mut items.features,
            _ => continue,
        };
        target.push(GoalPlanPersistentItem::Id(item.id.clone()));
    }
    let include = plan
        .impact
        .repository
        .iter()
        .filter(|impact| impact.impact_role == ImpactRole::DirectChange)
        .map(|impact| GoalPlanScopeInclude::Pattern(impact.path.clone()))
        .collect();
    let completion = plan
        .verification
        .completion
        .iter()
        .map(|check| match check {
            CompletionCheck::GoalPlanCheck { range, .. } => CompletionCheck::GoalPlanCheck {
                plan_path: context.plan_output_path.clone(),
                range: context.range.clone().unwrap_or_else(|| range.clone()),
            }
            .render(),
            CompletionCheck::TestSelection { .. } => CompletionCheck::TestSelection {
                plan_path: context.plan_output_path.clone(),
            }
            .render(),
            other => other.render(),
        })
        .collect();
    let mut required_tests = BTreeMap::<String, Vec<TraceReference>>::new();
    for impact in &plan.impact.tests {
        if impact.impact_role == ImpactRole::DirectChange {
            required_tests
                .entry(impact.target.language.clone())
                .or_default()
                .push(TraceReference {
                    file: PathBuf::from(&impact.target.file),
                    symbols: impact.target.symbols.clone(),
                    doc_contains: Vec::new(),
                    method: None,
                    path: None,
                });
        }
    }
    let warnings = plan
        .impact
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.rule, diagnostic.reason))
        .collect();
    GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: context.source_path.clone(),
        request: Some(request.to_string()),
        classification: Some(format!("{:?}", plan.intent.kind).to_lowercase()),
        work: Some(plan.clone()),
        source: GoalPlanSource {
            mode: context.source_mode,
            request_artifact: context.source_path.clone(),
            classification: Some(format!("{:?}", plan.intent.kind).to_lowercase()),
            range: context.range.clone(),
            confidence: Some(if plan.executable {
                context.confidence
            } else {
                GoalPlanConfidence::Low
            }),
            evidence: Some(GoalPlanSourceEvidence::default()),
        },
        goal: GoalPlanGoal {
            id: context.goal_id.clone(),
            title: format!("Plan {:?} work", plan.intent.kind),
            statement: request.to_string(),
            non_goals: vec!["Do not create a fifth persistent spec layer".to_string()],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items: items,
            spec_updates: Default::default(),
            spec_updates_required: plan.mutations.iter().any(|mutation| {
                !matches!(
                    mutation,
                    WorkMutation::ModifyRepository { .. }
                        | WorkMutation::CreateRepository { .. }
                        | WorkMutation::DeleteRepository { .. }
                        | WorkMutation::MoveRepository { .. }
                        | WorkMutation::UpdateTrace { .. }
                        | WorkMutation::RepairDiagnostic { .. }
                )
            }),
            spec_update_reasons: plan
                .mutations
                .iter()
                .map(|mutation| format!("typed mutation: {mutation:?}"))
                .collect(),
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(if plan.executable {
                context.confidence
            } else {
                GoalPlanConfidence::Low
            }),
            scope: GoalPlanScope {
                include,
                exclude: vec!["docs/generated/**".to_string(), "target/**".to_string()],
            },
            steps: plan
                .mutations
                .iter()
                .map(|mutation| format!("Apply {mutation:?}"))
                .collect(),
        },
        test_plan: GoalPlanTestPlan {
            selection_mode: GoalPlanSelectionMode::Affected,
            confidence: Some(context.confidence),
            required_tests,
            suggested_tests: BTreeMap::new(),
        },
        coverage: GoalPlanCoverage {
            mode: GoalPlanCoverageMode::ChangedLines,
            threshold: 100,
            include: plan
                .impact
                .repository
                .iter()
                .filter(|impact| impact.impact_role == ImpactRole::DirectChange)
                .map(|impact| impact.path.clone())
                .collect(),
            exclude: Vec::new(),
        },
        completion: GoalPlanCompletion {
            must_pass: completion,
        },
        warnings,
    }
}

pub fn resolve_work_intent(
    request: &str,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
    seeds: Vec<WorkSeed>,
) -> WorkIntent {
    let mode = explicit_mode.unwrap_or_else(|| {
        if score(
            request,
            &["review", "audit", "investigate", "調査", "レビュー", "分析"],
        ) > 0
        {
            WorkMode::ReviewOnly
        } else {
            WorkMode::PlanAndExecute
        }
    });
    let kind = explicit_kind.unwrap_or_else(|| infer_kind(request, mode));
    let operation = explicit_operation.unwrap_or_else(|| infer_operation(request, kind));
    WorkIntent {
        kind,
        operation,
        mode,
        requested_surfaces: infer_surfaces(request, &seeds),
        seeds,
        constraints: WorkConstraints::default(),
    }
}

pub fn work_request_text(request: &crate::RequestArtifact) -> String {
    std::iter::once(request.request.as_str())
        .chain(request.context.affected_area.as_deref())
        .chain(
            request
                .context
                .repository_constraints
                .iter()
                .map(String::as_str),
        )
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn work_kind_profile(kind: WorkKind) -> WorkKindProfile {
    use WorkSurface as S;
    let any = |values: &[S]| vec![values.iter().copied().collect()];
    let mut contract = SurfaceRequirement::default();
    let (completion, fallback, forbidden) = match kind {
        WorkKind::Deliver => {
            contract.all_of.insert(S::Implementation);
            (
                vec![
                    validate(&[ValidationGenre::Delivery, ValidationGenre::Trace]),
                    goal_check(),
                ],
                true,
                false,
            )
        }
        WorkKind::Specify => {
            contract.any_of = any(&[S::Requirement, S::Feature]);
            (
                vec![validate(&[
                    ValidationGenre::Graph,
                    ValidationGenre::Delivery,
                ])],
                false,
                false,
            )
        }
        WorkKind::Govern => {
            contract.any_of = any(&[S::Philosophy, S::Policy]);
            (vec![validate(&[ValidationGenre::Graph])], false, false)
        }
        WorkKind::Restructure => {
            contract.any_of = any(&[
                S::Philosophy,
                S::Policy,
                S::Requirement,
                S::Feature,
                S::Trace,
            ]);
            (
                vec![validate(&[ValidationGenre::Graph, ValidationGenre::Trace])],
                false,
                false,
            )
        }
        WorkKind::Verify => {
            contract.any_of = any(&[S::Test, S::Trace]);
            (
                vec![validate(&[
                    ValidationGenre::Coverage,
                    ValidationGenre::Trace,
                ])],
                false,
                false,
            )
        }
        WorkKind::Repair => {
            contract.any_of = any(&[S::Trace, S::Config, S::GeneratedArtifact]);
            contract.forbidden_mutations.insert(S::Implementation);
            (
                vec![validate(&[ValidationGenre::Graph, ValidationGenre::Trace])],
                false,
                false,
            )
        }
        WorkKind::Maintain => {
            contract.any_of = any(&[S::Tooling, S::Config, S::Implementation]);
            contract.forbidden_mutations.extend([
                S::Philosophy,
                S::Policy,
                S::Requirement,
                S::Feature,
            ]);
            (
                vec![validate(&[
                    ValidationGenre::Workspace,
                    ValidationGenre::Delivery,
                ])],
                true,
                false,
            )
        }
        WorkKind::Retire => {
            contract.any_of = any(&[S::Requirement, S::Feature, S::Implementation]);
            (
                vec![validate(&[
                    ValidationGenre::Graph,
                    ValidationGenre::Trace,
                    ValidationGenre::Delivery,
                ])],
                true,
                false,
            )
        }
        WorkKind::Review => {
            contract.forbidden_mutations.extend(all_surfaces());
            (Vec::new(), false, true)
        }
        WorkKind::Adopt => {
            contract.any_of = any(&[S::Config, S::Documentation, S::Trace]);
            (
                vec![validate(&[
                    ValidationGenre::Workspace,
                    ValidationGenre::Graph,
                ])],
                false,
                false,
            )
        }
    };
    WorkKindProfile {
        kind,
        surface_requirement: contract,
        completion,
        cargo_test_fallback: fallback,
        mutation_forbidden: forbidden,
    }
}

fn validate(genres: &[ValidationGenre]) -> CompletionCheck {
    CompletionCheck::Validate {
        genres: genres.iter().copied().collect(),
    }
}
fn goal_check() -> CompletionCheck {
    CompletionCheck::GoalPlanCheck {
        plan_path: ".syu/tasks/current.yaml".to_string(),
        range: "origin/main...HEAD".to_string(),
    }
}

pub fn plan_work(mut input: WorkPlanningInput) -> WorkPlan {
    input.seeds.sort_by(|a, b| a.id.cmp(&b.id));
    input.seeds.dedup_by(|a, b| a.id == b.id);
    input.search_candidates.sort_by(|a, b| a.id.cmp(&b.id));
    input.search_candidates.dedup_by(|a, b| a.id == b.id);
    if input.seeds.is_empty()
        && let Some(candidate) = input.search_candidates.first().cloned()
    {
        input.seeds.push(WorkSeed {
            source_role: SourceRole::Inferred,
            ..candidate
        });
    }
    let mut intent = resolve_work_intent(
        &input.request,
        input.explicit_kind,
        input.explicit_operation,
        input.explicit_mode,
        input.seeds.clone(),
    );
    intent.constraints = input.constraints.clone();
    let profile = work_kind_profile(intent.kind);
    let nodes: BTreeMap<_, _> = input
        .nodes
        .into_iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut impact = WorkImpact::default();
    let mut visited = BTreeMap::<String, TraversalVisit>::new();
    let mut queue = VecDeque::new();
    for seed in &intent.seeds {
        if let Some(node) = nodes.get(&seed.id) {
            if node.surface != seed.surface {
                blocker(
                    &mut impact,
                    "WORK_SEED_KIND_MISMATCH",
                    &seed.id,
                    "seed prefix does not match the workspace Item kind",
                );
                continue;
            }
            if node
                .status
                .as_deref()
                .is_some_and(|status| matches!(status, "retired" | "superseded"))
                && intent.kind != WorkKind::Retire
            {
                blocker(
                    &mut impact,
                    "WORK_RETIRED_SEED",
                    &seed.id,
                    "retired or superseded Items require Retire work",
                );
                continue;
            }
            let state = TraversalVisit {
                source_role: seed.source_role,
                seed_surface: seed.surface,
                arrived_from: None,
                depth: 0,
                branch_root: seed.id.clone(),
            };
            visited.insert(seed.id.clone(), state.clone());
            queue.push_back((seed.id.clone(), state));
        } else if intent.operation != WorkOperation::Create {
            blocker(
                &mut impact,
                "WORK_UNKNOWN_SEED",
                &seed.id,
                "seed ID does not exist in the workspace",
            );
        } else {
            impact.items.push(ImpactedItem {
                id: seed.id.clone(),
                surface: seed.surface,
                source_role: seed.source_role,
                impact_role: ImpactRole::DirectChange,
                reason: "new explicit Item seed".to_string(),
            });
        }
    }
    for candidate in &input.search_candidates {
        if nodes.contains_key(&candidate.id) {
            let entry = visited
                .entry(candidate.id.clone())
                .or_insert(TraversalVisit {
                    source_role: SourceRole::SearchCandidate,
                    seed_surface: candidate.surface,
                    arrived_from: None,
                    depth: 0,
                    branch_root: candidate.id.clone(),
                });
            if entry.source_role == SourceRole::SearchCandidate {
                queue.push_back((candidate.id.clone(), entry.clone()));
            }
        }
    }
    while let Some((id, state)) = queue.pop_front() {
        let Some(node) = nodes.get(&id) else { continue };
        for linked in neighbors(intent.kind, &state, node, &nodes) {
            if !visited.contains_key(&linked) {
                let next = TraversalVisit {
                    source_role: SourceRole::Inferred,
                    seed_surface: state.seed_surface,
                    arrived_from: Some(node.surface),
                    depth: state.depth + 1,
                    branch_root: state.branch_root.clone(),
                };
                visited.insert(linked.clone(), next.clone());
                queue.push_back((linked.clone(), next));
            }
            impact.edges.push(ImpactedEdge {
                from: id.clone(),
                to: linked,
                impact_role: edge_role(intent.kind),
                reason: "expanded by the WorkKind traversal policy".to_string(),
            });
        }
    }
    for (id, visit) in &visited {
        let node = &nodes[id];
        let impact_role = item_role(intent.kind, node.surface, visit);
        impact.items.push(ImpactedItem {
            id: id.clone(),
            surface: node.surface,
            source_role: visit.source_role,
            impact_role,
            reason: role_reason(intent.kind, impact_role).to_string(),
        });
        add_owned_impacts(intent.kind, node, visit, &mut impact);
    }
    for change in input.repository_changes {
        impact.repository.push(RepositoryImpact {
            path: change.path,
            surface: repository_surface(intent.kind),
            impact_role: if intent.kind == WorkKind::Review {
                ImpactRole::Context
            } else {
                ImpactRole::DirectChange
            },
            reason: "changed repository path".to_string(),
        });
    }
    for diagnostic in input.diagnostics {
        impact.diagnostics.push(DiagnosticImpact {
            rule: diagnostic.rule,
            subject: diagnostic.subject,
            impact_role: if intent.kind == WorkKind::Review {
                ImpactRole::Context
            } else {
                ImpactRole::DirectChange
            },
            reason: diagnostic.message,
        });
    }
    validate_operation_compatibility(&intent, &mut impact);
    validate_operation_payload(&intent, &mut impact);
    let mut mutations = build_mutations(&intent, &impact);
    if profile.mutation_forbidden || intent.mode == WorkMode::ReviewOnly {
        mutations.clear();
    }
    validate_contract(&profile.surface_requirement, &mut impact, &mut mutations);
    sort_impact(&mut impact);
    let executable = !impact
        .diagnostics
        .iter()
        .any(|item| item.impact_role == ImpactRole::Blocker);
    WorkPlan {
        intent,
        impact,
        mutations,
        verification: WorkVerification {
            contract: profile.surface_requirement,
            completion: profile.completion,
            cargo_test_fallback: profile.cargo_test_fallback,
            mutation_forbidden: profile.mutation_forbidden,
        },
        executable,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraversalVisit {
    source_role: SourceRole,
    seed_surface: WorkSurface,
    arrived_from: Option<WorkSurface>,
    depth: usize,
    branch_root: String,
}

fn validate_operation_payload(intent: &WorkIntent, impact: &mut WorkImpact) {
    let missing = match intent.operation {
        WorkOperation::Rename | WorkOperation::Supersede
            if intent.constraints.target_id.is_none() =>
        {
            Some("target_id")
        }
        WorkOperation::Move if intent.constraints.destination_document.is_none() => {
            Some("destination_document")
        }
        WorkOperation::Split
            if intent.constraints.related_item_ids.is_empty()
                || intent.constraints.redistribution.is_empty() =>
        {
            Some("related_item_ids and redistribution")
        }
        WorkOperation::Relink
            if intent.constraints.remove_edges.is_empty()
                && intent.constraints.add_edges.is_empty() =>
        {
            Some("remove_edges or add_edges")
        }
        WorkOperation::Merge
            if intent.constraints.target_id.is_none()
                || intent.constraints.related_item_ids.is_empty() =>
        {
            Some("target_id and related_item_ids")
        }
        WorkOperation::Promote | WorkOperation::Demote
            if intent.constraints.target_surface.is_none() =>
        {
            Some("target surface")
        }
        _ => None,
    };
    if let Some(field) = missing {
        blocker(
            impact,
            "WORK_OPERATION_PAYLOAD",
            field,
            "operation requires additional typed payload",
        );
    }
}

fn validate_operation_compatibility(intent: &WorkIntent, impact: &mut WorkImpact) {
    use WorkKind as K;
    use WorkOperation as O;
    let allowed = match intent.kind {
        K::Deliver | K::Specify | K::Verify | K::Maintain => {
            matches!(intent.operation, O::Create | O::Modify | O::Delete)
        }
        K::Govern => matches!(intent.operation, O::Create | O::Modify | O::Supersede),
        K::Restructure => matches!(
            intent.operation,
            O::Rename | O::Move | O::Relink | O::Split | O::Merge | O::Promote | O::Demote
        ),
        K::Repair => matches!(intent.operation, O::Modify | O::Relink | O::Validate),
        K::Retire => matches!(intent.operation, O::Delete | O::Supersede),
        K::Review => intent.operation == O::Validate,
        K::Adopt => matches!(intent.operation, O::Create | O::Modify | O::Move),
    };
    if !allowed {
        blocker(
            impact,
            "WORK_OPERATION_INCOMPATIBLE",
            &format!("{:?}+{:?}", intent.kind, intent.operation),
            "WorkKind does not support the requested operation",
        );
    }
}

fn neighbors(
    kind: WorkKind,
    state: &TraversalVisit,
    node: &WorkGraphNode,
    nodes: &BTreeMap<String, WorkGraphNode>,
) -> Vec<String> {
    let mut result = node
        .linked_ids
        .iter()
        .filter(|id| {
            nodes
                .get(*id)
                .is_some_and(|target| traversal_allows(kind, state, node, target))
        })
        .cloned()
        .collect::<Vec<_>>();
    for candidate in nodes.values() {
        if candidate.linked_ids.contains(&node.id) && traversal_allows(kind, state, node, candidate)
        {
            result.push(candidate.id.clone());
        }
    }
    result.sort();
    result.dedup();
    result
}

fn item_role(kind: WorkKind, surface: WorkSurface, visit: &TraversalVisit) -> ImpactRole {
    if visit.source_role == SourceRole::SearchCandidate {
        return ImpactRole::Context;
    }
    match kind {
        WorkKind::Specify if visit.source_role == SourceRole::Seed => ImpactRole::DirectChange,
        WorkKind::Specify
            if visit.source_role == SourceRole::Inferred && surface == visit.seed_surface =>
        {
            ImpactRole::DirectChange
        }
        WorkKind::Specify
            if visit.seed_surface == WorkSurface::Requirement
                && surface == WorkSurface::Feature =>
        {
            ImpactRole::FollowUp
        }
        WorkKind::Govern if matches!(surface, WorkSurface::Philosophy | WorkSurface::Policy) => {
            ImpactRole::DirectChange
        }
        WorkKind::Govern => ImpactRole::FollowUp,
        WorkKind::Restructure | WorkKind::Retire if visit.source_role == SourceRole::Seed => {
            ImpactRole::DirectChange
        }
        WorkKind::Review
        | WorkKind::Deliver
        | WorkKind::Verify
        | WorkKind::Repair
        | WorkKind::Maintain
        | WorkKind::Adopt => ImpactRole::Context,
        _ => ImpactRole::Context,
    }
}

fn add_owned_impacts(
    kind: WorkKind,
    node: &WorkGraphNode,
    visit: &TraversalVisit,
    impact: &mut WorkImpact,
) {
    let repository_role = owned_repository_role(kind, visit, node.surface);
    for target in &node.implementations {
        impact.repository.push(RepositoryImpact {
            path: target.file.clone(),
            surface: WorkSurface::Implementation,
            impact_role: repository_role,
            reason: format!("owned by {}", node.id),
        });
    }
    let test_role = owned_test_role(kind, visit, node.surface);
    for target in &node.tests {
        impact.tests.push(TestImpact {
            target: target.clone(),
            impact_role: test_role,
            reason: format!("declared by {}", node.id),
        });
    }
}

fn build_mutations(intent: &WorkIntent, impact: &WorkImpact) -> Vec<WorkMutation> {
    let direct = impact
        .items
        .iter()
        .filter(|item| item.impact_role == ImpactRole::DirectChange)
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    if intent.kind == WorkKind::Deliver {
        out.extend(
            impact
                .repository
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .filter_map(|item| repository_mutation(intent.operation, &item.path, None)),
        );
        out.extend(
            impact
                .tests
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .map(|item| WorkMutation::UpdateTrace {
                    target: item.target.clone(),
                }),
        );
        return out;
    }
    if intent.kind == WorkKind::Verify {
        out.extend(
            impact
                .tests
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .filter_map(|item| repository_mutation(intent.operation, &item.target.file, None)),
        );
        return out;
    }
    if intent.kind == WorkKind::Repair {
        if intent.operation == WorkOperation::Relink {
            out.push(WorkMutation::Relink {
                remove: intent.constraints.remove_edges.clone(),
                add: intent.constraints.add_edges.clone(),
            });
            return out;
        }
        if intent.operation == WorkOperation::Validate {
            return out;
        }
        out.extend(
            impact
                .diagnostics
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .map(|item| WorkMutation::RepairDiagnostic {
                    rule: item.rule.clone(),
                    subject: item.subject.clone(),
                }),
        );
        return out;
    }
    if intent.kind == WorkKind::Maintain {
        out.extend(
            impact
                .repository
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .filter_map(|item| repository_mutation(intent.operation, &item.path, None)),
        );
        return out;
    }
    if intent.kind == WorkKind::Adopt {
        out.extend(
            impact
                .repository
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .filter_map(|item| match intent.operation {
                    WorkOperation::Create => Some(WorkMutation::CreateConfig {
                        path: item.path.clone(),
                    }),
                    WorkOperation::Modify => Some(WorkMutation::ModifyConfig {
                        path: item.path.clone(),
                    }),
                    WorkOperation::Move => {
                        intent
                            .constraints
                            .destination_document
                            .as_ref()
                            .map(|destination| WorkMutation::MoveConfig {
                                path: item.path.clone(),
                                destination: destination.clone(),
                            })
                    }
                    _ => None,
                }),
        );
        return out;
    }
    for item in direct {
        match intent.operation {
            WorkOperation::Create => out.push(WorkMutation::CreateItem {
                draft: SpecItemDraft {
                    id: item.id.clone(),
                    surface: item.surface,
                },
            }),
            WorkOperation::Modify => out.push(WorkMutation::ModifyItem {
                id: item.id.clone(),
                patch: SpecItemPatch {
                    requested_surfaces: intent.requested_surfaces.clone(),
                },
            }),
            WorkOperation::Delete => out.push(WorkMutation::DeleteItem {
                id: item.id.clone(),
            }),
            WorkOperation::Rename => {
                if let Some(to) = &intent.constraints.target_id {
                    out.push(WorkMutation::RenameItem {
                        from: item.id.clone(),
                        to: to.clone(),
                    });
                }
            }
            WorkOperation::Move => {
                if let Some(destination) = &intent.constraints.destination_document {
                    out.push(WorkMutation::MoveItem {
                        id: item.id.clone(),
                        destination_document: destination.clone(),
                    });
                }
            }
            WorkOperation::Relink => out.push(WorkMutation::Relink {
                remove: intent.constraints.remove_edges.clone(),
                add: intent.constraints.add_edges.clone(),
            }),
            WorkOperation::Split => {
                let targets = intent
                    .constraints
                    .related_item_ids
                    .iter()
                    .filter_map(|id| {
                        surface_from_id(id).map(|surface| SpecItemDraft {
                            id: id.clone(),
                            surface,
                        })
                    })
                    .collect::<Vec<_>>();
                out.push(WorkMutation::SplitItem {
                    source: item.id.clone(),
                    targets,
                    redistribution: intent.constraints.redistribution.clone(),
                });
            }
            WorkOperation::Merge => out.push(WorkMutation::MergeItems {
                sources: std::iter::once(item.id.clone())
                    .chain(intent.constraints.related_item_ids.clone())
                    .collect(),
                target: intent
                    .constraints
                    .target_id
                    .clone()
                    .unwrap_or_else(|| item.id.clone()),
            }),
            WorkOperation::Promote | WorkOperation::Demote => {
                if let Some(surface) = intent.constraints.target_surface {
                    out.push(WorkMutation::ChangeItemKind {
                        id: item.id.clone(),
                        target_surface: surface,
                        relink: Vec::new(),
                    });
                }
            }
            WorkOperation::Supersede => {
                if let Some(replacement) = &intent.constraints.target_id {
                    out.push(WorkMutation::Supersede {
                        old: item.id.clone(),
                        replacement: replacement.clone(),
                    });
                }
            }
            WorkOperation::Validate => {}
        }
    }
    out
}

fn traversal_allows(
    kind: WorkKind,
    state: &TraversalVisit,
    node: &WorkGraphNode,
    target: &WorkGraphNode,
) -> bool {
    use WorkKind as K;
    use WorkSurface as S;

    match kind {
        K::Deliver => matches!(
            (
                state.seed_surface,
                node.surface,
                target.surface,
                state.depth,
            ),
            (S::Requirement, S::Requirement, S::Feature, 0)
                | (S::Requirement, S::Requirement, S::Policy, _)
                | (S::Requirement, S::Policy, S::Philosophy, _)
                | (S::Feature, S::Feature, S::Requirement, 0)
                | (S::Feature, S::Requirement, S::Policy, _)
                | (S::Feature, S::Policy, S::Philosophy, _)
                | (S::Policy, S::Policy, S::Philosophy, _)
                | (S::Policy, S::Policy, S::Requirement, _)
                | (S::Policy, S::Requirement, S::Feature, 0)
        ),
        K::Specify => matches!(
            (
                state.seed_surface,
                node.surface,
                target.surface,
                state.depth,
            ),
            (S::Requirement, S::Requirement, S::Feature, 0)
                | (S::Requirement, S::Requirement, S::Policy, _)
                | (S::Requirement, S::Policy, S::Philosophy, _)
                | (S::Feature, S::Feature, S::Requirement, 0)
                | (S::Feature, S::Requirement, S::Policy, _)
                | (S::Feature, S::Policy, S::Philosophy, _)
        ),
        K::Govern => matches!(
            (node.surface, target.surface),
            (S::Philosophy, S::Policy) | (S::Policy, S::Requirement) | (S::Requirement, S::Feature)
        ),
        K::Restructure | K::Retire => state.arrived_from != Some(target.surface),
        _ => matches!(
            (node.surface, target.surface),
            (S::Feature, S::Requirement) | (S::Requirement, S::Policy) | (S::Policy, S::Philosophy)
        ),
    }
}

fn owned_repository_role(
    kind: WorkKind,
    visit: &TraversalVisit,
    surface: WorkSurface,
) -> ImpactRole {
    if visit.source_role == SourceRole::SearchCandidate {
        return ImpactRole::Context;
    }
    if matches!(kind, WorkKind::Deliver | WorkKind::Retire) && surface == WorkSurface::Feature {
        ImpactRole::DirectChange
    } else {
        ImpactRole::Context
    }
}

fn owned_test_role(kind: WorkKind, visit: &TraversalVisit, surface: WorkSurface) -> ImpactRole {
    if visit.source_role == SourceRole::SearchCandidate {
        return ImpactRole::Context;
    }
    if matches!(
        kind,
        WorkKind::Deliver | WorkKind::Verify | WorkKind::Retire
    ) && surface == WorkSurface::Requirement
    {
        ImpactRole::DirectChange
    } else {
        ImpactRole::Context
    }
}

fn repository_mutation(
    operation: WorkOperation,
    path: &str,
    destination: Option<&str>,
) -> Option<WorkMutation> {
    match operation {
        WorkOperation::Create => Some(WorkMutation::CreateRepository {
            path: path.to_string(),
        }),
        WorkOperation::Modify => Some(WorkMutation::ModifyRepository {
            path: path.to_string(),
        }),
        WorkOperation::Delete => Some(WorkMutation::DeleteRepository {
            path: path.to_string(),
        }),
        WorkOperation::Move => destination.map(|destination| WorkMutation::MoveRepository {
            path: path.to_string(),
            destination: destination.to_string(),
        }),
        _ => None,
    }
}

fn validate_contract(
    contract: &SurfaceRequirement,
    impact: &mut WorkImpact,
    mutations: &mut Vec<WorkMutation>,
) {
    let present = direct_surfaces(impact);
    for surface in contract.all_of.difference(&present) {
        blocker(
            impact,
            "WORK_REQUIRED_SURFACE",
            &format!("{surface:?}"),
            "required direct-change surface is missing",
        );
    }
    for group in &contract.any_of {
        if group.is_disjoint(&present) {
            blocker(
                impact,
                "WORK_REQUIRED_SURFACE",
                "work contract",
                "none of the required alternative direct-change surfaces are present",
            );
        }
    }
    let forbidden = mutation_surfaces(mutations);
    for surface in contract.forbidden_mutations.intersection(&forbidden) {
        blocker(
            impact,
            "WORK_FORBIDDEN_MUTATION",
            &format!("{surface:?}"),
            "WorkKind forbids mutation on this surface",
        );
    }
    if impact
        .diagnostics
        .iter()
        .any(|item| item.impact_role == ImpactRole::Blocker)
    {
        mutations.clear();
    }
}

fn direct_surfaces(impact: &WorkImpact) -> BTreeSet<WorkSurface> {
    impact
        .items
        .iter()
        .filter(|item| item.impact_role == ImpactRole::DirectChange)
        .map(|item| item.surface)
        .chain(
            impact
                .repository
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .map(|item| item.surface),
        )
        .chain(
            impact
                .tests
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .map(|_| WorkSurface::Test),
        )
        .chain(
            impact
                .diagnostics
                .iter()
                .filter(|item| item.impact_role == ImpactRole::DirectChange)
                .map(|_| WorkSurface::Trace),
        )
        .collect()
}

fn mutation_surfaces(mutations: &[WorkMutation]) -> BTreeSet<WorkSurface> {
    mutations
        .iter()
        .filter_map(|mutation| match mutation {
            WorkMutation::CreateItem { draft } => Some(draft.surface),
            WorkMutation::ModifyItem { id, .. }
            | WorkMutation::DeleteItem { id }
            | WorkMutation::RenameItem { from: id, .. }
            | WorkMutation::MoveItem { id, .. }
            | WorkMutation::SplitItem { source: id, .. }
            | WorkMutation::ChangeItemKind { id, .. }
            | WorkMutation::Supersede { old: id, .. } => surface_from_id(id),
            WorkMutation::ModifyRepository { .. }
            | WorkMutation::CreateRepository { .. }
            | WorkMutation::DeleteRepository { .. }
            | WorkMutation::MoveRepository { .. } => Some(WorkSurface::Implementation),
            WorkMutation::UpdateTrace { .. } | WorkMutation::Relink { .. } => {
                Some(WorkSurface::Trace)
            }
            WorkMutation::RepairDiagnostic { .. } => Some(WorkSurface::Trace),
            WorkMutation::ModifyConfig { .. }
            | WorkMutation::CreateConfig { .. }
            | WorkMutation::MoveConfig { .. } => Some(WorkSurface::Config),
            WorkMutation::MergeItems { target, .. } => surface_from_id(target),
        })
        .collect()
}

fn blocker(impact: &mut WorkImpact, rule: &str, subject: &str, reason: &str) {
    impact.diagnostics.push(DiagnosticImpact {
        rule: rule.to_string(),
        subject: subject.to_string(),
        impact_role: ImpactRole::Blocker,
        reason: reason.to_string(),
    });
}
fn edge_role(kind: WorkKind) -> ImpactRole {
    if matches!(kind, WorkKind::Restructure | WorkKind::Retire) {
        ImpactRole::DirectChange
    } else {
        ImpactRole::Context
    }
}
fn role_reason(kind: WorkKind, role: ImpactRole) -> &'static str {
    match role {
        ImpactRole::DirectChange => "selected as a direct change by the WorkKind profile",
        ImpactRole::Context => "required context for the selected work",
        ImpactRole::FollowUp if kind == WorkKind::Govern => {
            "downstream contract affected by governance work"
        }
        ImpactRole::FollowUp => "linked work should be handled as a follow-up",
        ImpactRole::Blocker => "work cannot proceed",
    }
}
fn repository_surface(kind: WorkKind) -> WorkSurface {
    match kind {
        WorkKind::Adopt => WorkSurface::Config,
        WorkKind::Maintain => WorkSurface::Tooling,
        _ => WorkSurface::Implementation,
    }
}
fn surface_from_id(id: &str) -> Option<WorkSurface> {
    if id.starts_with("PHIL-") {
        Some(WorkSurface::Philosophy)
    } else if id.starts_with("POL-") {
        Some(WorkSurface::Policy)
    } else if id.starts_with("REQ-") {
        Some(WorkSurface::Requirement)
    } else if id.starts_with("FEAT-") {
        Some(WorkSurface::Feature)
    } else {
        None
    }
}
fn all_surfaces() -> BTreeSet<WorkSurface> {
    use WorkSurface as S;
    [
        S::Philosophy,
        S::Policy,
        S::Requirement,
        S::Feature,
        S::Implementation,
        S::Test,
        S::Trace,
        S::Config,
        S::Documentation,
        S::Tooling,
        S::GeneratedArtifact,
    ]
    .into_iter()
    .collect()
}
fn sort_impact(impact: &mut WorkImpact) {
    impact.items.sort_by(|a, b| a.id.cmp(&b.id));
    impact
        .edges
        .sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    impact.repository.sort_by(|a, b| a.path.cmp(&b.path));
    impact
        .repository
        .dedup_by(|a, b| a.path == b.path && a.impact_role == b.impact_role);
    impact.tests.sort_by(|a, b| {
        (&a.target.file, &a.target.symbols).cmp(&(&b.target.file, &b.target.symbols))
    });
    impact
        .tests
        .dedup_by(|a, b| a.target == b.target && a.impact_role == b.impact_role);
    impact
        .diagnostics
        .sort_by(|a, b| (&a.rule, &a.subject).cmp(&(&b.rule, &b.subject)));
}

fn infer_kind(request: &str, mode: WorkMode) -> WorkKind {
    if mode == WorkMode::ReviewOnly {
        return WorkKind::Review;
    }
    let candidates = [
        (
            WorkKind::Deliver,
            &["implement", "bug fix", "実装", "バグ修正"] as &[&str],
        ),
        (
            WorkKind::Retire,
            &["retire", "deprecate", "supersede", "remove old", "廃止"],
        ),
        (
            WorkKind::Restructure,
            &[
                "rename",
                "move",
                "relink",
                "split",
                "merge",
                "promote",
                "demote",
                "名前変更",
                "移動",
                "分割",
                "統合",
            ],
        ),
        (
            WorkKind::Govern,
            &["policy", "philosophy", "govern", "方針", "原則"],
        ),
        (
            WorkKind::Verify,
            &[
                "coverage",
                "quality gate",
                "test only",
                "検証",
                "テスト追加",
            ],
        ),
        (
            WorkKind::Repair,
            &[
                "broken trace",
                "reciprocal",
                "registry inconsistency",
                "diagnostic",
                "整合性",
                "修復",
            ],
        ),
        (
            WorkKind::Adopt,
            &["bootstrap", "onboard", "migrate to syu", "導入", "初期化"],
        ),
        (
            WorkKind::Maintain,
            &["dependency", "toolchain", "ci", "refactor", "保守", "依存"],
        ),
        (
            WorkKind::Specify,
            &[
                "requirement",
                "feature spec",
                "acceptance criteria",
                "要件",
                "仕様",
            ],
        ),
    ];
    candidates
        .into_iter()
        .max_by_key(|(_, words)| score(request, words))
        .filter(|(_, words)| score(request, words) > 0)
        .map(|(kind, _)| kind)
        .unwrap_or(WorkKind::Deliver)
}

fn infer_operation(request: &str, kind: WorkKind) -> WorkOperation {
    let candidates = [
        (
            WorkOperation::Supersede,
            &["supersede", "retire", "deprecate", "置換", "廃止"] as &[&str],
        ),
        (WorkOperation::Rename, &["rename", "名前変更"]),
        (WorkOperation::Relink, &["relink", "リンク修復"]),
        (WorkOperation::Split, &["split", "分割"]),
        (WorkOperation::Merge, &["merge", "統合"]),
        (WorkOperation::Promote, &["promote", "昇格"]),
        (WorkOperation::Demote, &["demote", "降格"]),
        (WorkOperation::Move, &["move", "移動"]),
        (WorkOperation::Delete, &["delete", "remove", "削除"]),
        (
            WorkOperation::Create,
            &[
                "create",
                "add",
                "new",
                "bootstrap",
                "onboard",
                "追加",
                "新規",
                "導入",
                "初期化",
            ],
        ),
    ];
    candidates
        .into_iter()
        .max_by_key(|(_, words)| score(request, words))
        .filter(|(_, words)| score(request, words) > 0)
        .map(|(operation, _)| operation)
        .unwrap_or(if kind == WorkKind::Review {
            WorkOperation::Validate
        } else {
            WorkOperation::Modify
        })
}

fn infer_surfaces(request: &str, seeds: &[WorkSeed]) -> BTreeSet<WorkSurface> {
    use WorkSurface as S;
    let mut out = seeds
        .iter()
        .map(|seed| seed.surface)
        .collect::<BTreeSet<_>>();
    for (surface, words) in [
        (S::Philosophy, &["philosophy", "原則"] as &[&str]),
        (S::Policy, &["policy", "方針"]),
        (S::Requirement, &["requirement", "要件"]),
        (S::Feature, &["feature", "機能"]),
        (S::Implementation, &["implementation", "code", "実装"]),
        (S::Test, &["test", "coverage", "テスト"]),
        (S::Trace, &["trace", "traceability", "トレース"]),
        (S::Config, &["config", "設定"]),
        (S::Documentation, &["documentation", "docs", "文書"]),
        (S::Tooling, &["tooling", "toolchain", "ci", "ツール"]),
        (S::GeneratedArtifact, &["generated", "生成物"]),
    ] {
        if score(request, words) > 0 {
            out.insert(surface);
        }
    }
    out
}

fn score(text: &str, keywords: &[&str]) -> usize {
    let lower = text.to_lowercase();
    let tokens = lower
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>();
    keywords
        .iter()
        .map(|keyword| {
            let key = keyword.to_lowercase();
            if key.chars().any(char::is_whitespace) {
                if lower.contains(&key) { 4 } else { 0 }
            } else if key.is_ascii() {
                if tokens.contains(key.as_str()) { 3 } else { 0 }
            } else if lower.contains(&key) {
                3
            } else {
                0
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seed(id: &str, surface: WorkSurface) -> WorkSeed {
        WorkSeed {
            id: id.to_string(),
            surface,
            source_role: SourceRole::Seed,
        }
    }
    fn node(id: &str, surface: WorkSurface, links: &[&str]) -> WorkGraphNode {
        WorkGraphNode {
            id: id.to_string(),
            surface,
            document_path: None,
            status: None,
            linked_ids: links.iter().map(|s| s.to_string()).collect(),
            implementations: Vec::new(),
            tests: Vec::new(),
        }
    }

    #[test]
    fn classifier_respects_tokens_and_infers_all_axes() {
        assert_eq!(
            resolve_work_intent("change a specific requirement", None, None, None, vec![]).kind,
            WorkKind::Specify
        );
        let retire = resolve_work_intent("retire old feature", None, None, None, vec![]);
        assert_eq!(
            (retire.kind, retire.operation),
            (WorkKind::Retire, WorkOperation::Supersede)
        );
        let adopt = resolve_work_intent("bootstrap syu", None, None, None, vec![]);
        assert_eq!(
            (adopt.kind, adopt.operation),
            (WorkKind::Adopt, WorkOperation::Create)
        );
    }
    #[test]
    fn govern_traverses_philosophy_to_features_with_queue() {
        let nodes = vec![
            node("PHIL-1", WorkSurface::Philosophy, &["POL-1"]),
            node("POL-1", WorkSurface::Policy, &["REQ-1"]),
            node("REQ-1", WorkSurface::Requirement, &["FEAT-1"]),
            node("FEAT-1", WorkSurface::Feature, &[]),
        ];
        let input = WorkPlanningInput {
            request: "govern philosophy".into(),
            seeds: vec![seed("PHIL-1", WorkSurface::Philosophy)],
            nodes: nodes.clone(),
            ..Default::default()
        };
        let plan = plan_work(input);
        assert!(plan.executable);
        assert_eq!(plan.impact.items.len(), 4);
        assert!(
            plan.impact
                .items
                .iter()
                .any(|i| i.id == "FEAT-1" && i.impact_role == ImpactRole::FollowUp)
        );
        let policy_plan = plan_work(WorkPlanningInput {
            request: "govern policy".into(),
            seeds: vec![seed("POL-1", WorkSurface::Policy)],
            nodes,
            ..Default::default()
        });
        assert!(policy_plan.executable);
        assert!(
            policy_plan
                .impact
                .items
                .iter()
                .any(|item| item.id == "FEAT-1")
        );
    }
    #[test]
    fn deliver_changes_owned_code_and_tests_not_feature_spec() {
        let mut feature = node("FEAT-1", WorkSurface::Feature, &["REQ-1"]);
        feature.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/lib.rs".into(),
            symbols: vec!["run".into()],
        });
        let mut req = node("REQ-1", WorkSurface::Requirement, &[]);
        req.tests.push(TraceTarget {
            language: "rust".into(),
            file: "tests/run.rs".into(),
            symbols: vec!["runs".into()],
        });
        let plan = plan_work(WorkPlanningInput {
            request: "implement FEAT-1".into(),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![feature, req],
            ..Default::default()
        });
        assert!(plan.executable);
        assert!(
            plan.impact
                .items
                .iter()
                .all(|i| i.impact_role == ImpactRole::Context)
        );
        assert!(
            plan.mutations
                .iter()
                .any(|m| matches!(m,WorkMutation::ModifyRepository{path} if path=="src/lib.rs"))
        );
        assert!(
            !plan
                .mutations
                .iter()
                .any(|m| matches!(m, WorkMutation::ModifyItem { .. }))
        );
    }

    #[test]
    fn deliver_requirement_seed_reaches_feature_without_siblings() {
        let mut requirement = node("REQ-1", WorkSurface::Requirement, &["FEAT-A", "POL-1"]);
        requirement.tests.push(TraceTarget {
            language: "rust".into(),
            file: "tests/req.rs".into(),
            symbols: vec!["req_behavior".into()],
        });
        let mut feature_a = node("FEAT-A", WorkSurface::Feature, &["REQ-1"]);
        feature_a.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/a.rs".into(),
            symbols: vec!["run_a".into()],
        });
        let mut feature_b = node("FEAT-B", WorkSurface::Feature, &["REQ-2"]);
        feature_b.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/b.rs".into(),
            symbols: vec!["run_b".into()],
        });
        let plan = plan_work(WorkPlanningInput {
            request: "deliver REQ-1".into(),
            explicit_kind: Some(WorkKind::Deliver),
            seeds: vec![seed("REQ-1", WorkSurface::Requirement)],
            nodes: vec![
                requirement,
                feature_a,
                feature_b,
                node("POL-1", WorkSurface::Policy, &["PHIL-1"]),
                node("PHIL-1", WorkSurface::Philosophy, &[]),
            ],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(
            plan.impact
                .repository
                .iter()
                .any(|item| item.path == "src/a.rs" && item.impact_role == ImpactRole::DirectChange)
        );
        assert!(
            !plan
                .impact
                .repository
                .iter()
                .any(|item| item.path == "src/b.rs" && item.impact_role == ImpactRole::DirectChange)
        );
    }

    #[test]
    fn specify_feature_seed_excludes_sibling_features() {
        let plan = plan_work(WorkPlanningInput {
            request: "specify FEAT-A".into(),
            explicit_kind: Some(WorkKind::Specify),
            seeds: vec![seed("FEAT-A", WorkSurface::Feature)],
            nodes: vec![
                node("FEAT-A", WorkSurface::Feature, &["REQ-1"]),
                node(
                    "REQ-1",
                    WorkSurface::Requirement,
                    &["FEAT-A", "FEAT-B", "POL-1"],
                ),
                node("FEAT-B", WorkSurface::Feature, &["REQ-1"]),
                node("POL-1", WorkSurface::Policy, &[]),
            ],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(
            plan.impact
                .items
                .iter()
                .any(|item| item.id == "FEAT-A" && item.impact_role == ImpactRole::DirectChange)
        );
        assert!(!plan.impact.items.iter().any(|item| item.id == "FEAT-B"));
    }

    #[test]
    fn specify_requirement_seed_marks_linked_features_follow_up() {
        let plan = plan_work(WorkPlanningInput {
            request: "specify REQ-1".into(),
            explicit_kind: Some(WorkKind::Specify),
            seeds: vec![seed("REQ-1", WorkSurface::Requirement)],
            nodes: vec![
                node("REQ-1", WorkSurface::Requirement, &["FEAT-A", "POL-1"]),
                node("FEAT-A", WorkSurface::Feature, &["REQ-1"]),
                node("POL-1", WorkSurface::Policy, &[]),
            ],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(
            plan.impact
                .items
                .iter()
                .any(|item| item.id == "REQ-1" && item.impact_role == ImpactRole::DirectChange)
        );
        assert!(
            plan.impact
                .items
                .iter()
                .any(|item| item.id == "FEAT-A" && item.impact_role == ImpactRole::FollowUp)
        );
    }

    #[test]
    fn deliver_delete_maps_to_repository_delete() {
        let mut feature = node("FEAT-1", WorkSurface::Feature, &[]);
        feature.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/delete_me.rs".into(),
            symbols: vec!["delete_me".into()],
        });
        let plan = plan_work(WorkPlanningInput {
            request: "remove FEAT-1".into(),
            explicit_kind: Some(WorkKind::Deliver),
            explicit_operation: Some(WorkOperation::Delete),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![feature],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(plan.mutations.iter().any(
            |mutation| matches!(mutation, WorkMutation::DeleteRepository { path } if path == "src/delete_me.rs")
        ));
    }

    #[test]
    fn verify_create_maps_to_repository_create() {
        let mut requirement = node("REQ-1", WorkSurface::Requirement, &[]);
        requirement.tests.push(TraceTarget {
            language: "rust".into(),
            file: "tests/new_case.rs".into(),
            symbols: vec!["new_case".into()],
        });
        let plan = plan_work(WorkPlanningInput {
            request: "add tests for REQ-1".into(),
            explicit_kind: Some(WorkKind::Verify),
            explicit_operation: Some(WorkOperation::Create),
            seeds: vec![seed("REQ-1", WorkSurface::Requirement)],
            nodes: vec![requirement],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(plan.mutations.iter().any(
            |mutation| matches!(mutation, WorkMutation::CreateRepository { path } if path == "tests/new_case.rs")
        ));
    }

    #[test]
    fn relink_requires_exact_edge_payload() {
        let plan = plan_work(WorkPlanningInput {
            request: "relink FEAT-1".into(),
            explicit_kind: Some(WorkKind::Restructure),
            explicit_operation: Some(WorkOperation::Relink),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![node("FEAT-1", WorkSurface::Feature, &["REQ-1"])],
            constraints: WorkConstraints {
                remove_edges: vec![SpecEdge {
                    from: "FEAT-1".into(),
                    to: "REQ-OLD".into(),
                }],
                add_edges: vec![SpecEdge {
                    from: "FEAT-1".into(),
                    to: "REQ-NEW".into(),
                }],
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert!(plan.mutations.iter().any(|mutation| matches!(
            mutation,
            WorkMutation::Relink { remove, add }
                if remove == &vec![SpecEdge { from: "FEAT-1".into(), to: "REQ-OLD".into() }]
                    && add == &vec![SpecEdge { from: "FEAT-1".into(), to: "REQ-NEW".into() }]
        )));
    }

    #[test]
    fn split_without_redistribution_is_blocked() {
        let plan = plan_work(WorkPlanningInput {
            request: "split FEAT-1".into(),
            explicit_kind: Some(WorkKind::Restructure),
            explicit_operation: Some(WorkOperation::Split),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![node("FEAT-1", WorkSurface::Feature, &["REQ-1"])],
            constraints: WorkConstraints {
                related_item_ids: vec!["FEAT-1A".into(), "FEAT-1B".into()],
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(!plan.executable);
        assert!(plan.impact.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "WORK_OPERATION_PAYLOAD"
                && diagnostic.subject == "related_item_ids and redistribution"
        }));
    }

    #[test]
    fn natural_language_request_promotes_best_search_candidate() {
        let mut feature = node("FEAT-TRACE-001", WorkSurface::Feature, &["REQ-TRACE-001"]);
        feature.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/trace.rs".into(),
            symbols: vec!["lookup".into()],
        });
        let plan = plan_work(WorkPlanningInput {
            request: "implement a simpler trace lookup command".into(),
            explicit_kind: Some(WorkKind::Deliver),
            search_candidates: vec![WorkSeed {
                id: "FEAT-TRACE-001".into(),
                surface: WorkSurface::Feature,
                source_role: SourceRole::SearchCandidate,
            }],
            nodes: vec![
                feature,
                node("REQ-TRACE-001", WorkSurface::Requirement, &[]),
            ],
            ..Default::default()
        });
        assert!(plan.executable, "{:?}", plan.impact.diagnostics);
        assert_eq!(plan.intent.seeds.len(), 1);
        assert_eq!(plan.intent.seeds[0].source_role, SourceRole::Inferred);
        assert!(plan.impact.repository.iter().any(
            |item| item.path == "src/trace.rs" && item.impact_role == ImpactRole::DirectChange
        ));
    }
    #[test]
    fn unknown_seed_and_missing_contract_are_blockers() {
        let plan = plan_work(WorkPlanningInput {
            request: "implement REQ-MISSING".into(),
            seeds: vec![seed("REQ-MISSING", WorkSurface::Requirement)],
            ..Default::default()
        });
        assert!(!plan.executable);
        assert!(
            plan.impact
                .diagnostics
                .iter()
                .all(|d| d.impact_role == ImpactRole::Blocker)
        );
    }
    #[test]
    fn completion_commands_render_valid_cli_shapes() {
        for kind in [
            WorkKind::Deliver,
            WorkKind::Specify,
            WorkKind::Govern,
            WorkKind::Restructure,
            WorkKind::Verify,
            WorkKind::Repair,
            WorkKind::Maintain,
            WorkKind::Retire,
            WorkKind::Review,
            WorkKind::Adopt,
        ] {
            for check in work_kind_profile(kind).completion {
                let command = check.render();
                assert!(!command.contains("syu check graph"));
                assert!(!command.contains("syu task check\n"));
            }
        }
    }
    #[test]
    fn serialization_is_deterministic() {
        let input = WorkPlanningInput {
            request: "review".into(),
            explicit_kind: Some(WorkKind::Review),
            nodes: vec![
                node("REQ-B", WorkSurface::Requirement, &[]),
                node("REQ-A", WorkSurface::Requirement, &[]),
            ],
            search_candidates: vec![
                seed("REQ-B", WorkSurface::Requirement),
                seed("REQ-A", WorkSurface::Requirement),
            ],
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&plan_work(input.clone())).unwrap(),
            serde_json::to_string(&plan_work(input)).unwrap()
        );
    }

    #[test]
    fn every_work_kind_applies_its_surface_contract() {
        let cases = [
            (
                WorkKind::Specify,
                WorkPlanningInput {
                    request: "specify".into(),
                    explicit_kind: Some(WorkKind::Specify),
                    seeds: vec![seed("REQ-1", WorkSurface::Requirement)],
                    nodes: vec![node("REQ-1", WorkSurface::Requirement, &[])],
                    ..Default::default()
                },
            ),
            (
                WorkKind::Govern,
                WorkPlanningInput {
                    request: "govern".into(),
                    explicit_kind: Some(WorkKind::Govern),
                    seeds: vec![seed("POL-1", WorkSurface::Policy)],
                    nodes: vec![node("POL-1", WorkSurface::Policy, &[])],
                    ..Default::default()
                },
            ),
            (
                WorkKind::Restructure,
                WorkPlanningInput {
                    request: "restructure".into(),
                    explicit_kind: Some(WorkKind::Restructure),
                    explicit_operation: Some(WorkOperation::Rename),
                    seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
                    constraints: WorkConstraints {
                        target_id: Some("FEAT-2".into()),
                        ..Default::default()
                    },
                    nodes: vec![node("FEAT-1", WorkSurface::Feature, &[])],
                    ..Default::default()
                },
            ),
            (WorkKind::Verify, {
                let mut requirement = node("REQ-1", WorkSurface::Requirement, &[]);
                requirement.tests.push(TraceTarget {
                    language: "rust".into(),
                    file: "tests/verify.rs".into(),
                    symbols: vec!["verifies".into()],
                });
                WorkPlanningInput {
                    request: "verify".into(),
                    explicit_kind: Some(WorkKind::Verify),
                    seeds: vec![seed("REQ-1", WorkSurface::Requirement)],
                    nodes: vec![requirement],
                    ..Default::default()
                }
            }),
            (
                WorkKind::Repair,
                WorkPlanningInput {
                    request: "repair".into(),
                    explicit_kind: Some(WorkKind::Repair),
                    diagnostics: vec![WorkDiagnostic {
                        rule: "trace".into(),
                        subject: "REQ-1".into(),
                        message: "broken trace".into(),
                    }],
                    ..Default::default()
                },
            ),
            (
                WorkKind::Maintain,
                WorkPlanningInput {
                    request: "maintain".into(),
                    explicit_kind: Some(WorkKind::Maintain),
                    repository_changes: vec![RepositoryChange {
                        path: "Cargo.toml".into(),
                        owner_ids: Vec::new(),
                    }],
                    ..Default::default()
                },
            ),
            (
                WorkKind::Retire,
                WorkPlanningInput {
                    request: "retire".into(),
                    explicit_kind: Some(WorkKind::Retire),
                    explicit_operation: Some(WorkOperation::Delete),
                    seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
                    nodes: vec![node("FEAT-1", WorkSurface::Feature, &[])],
                    ..Default::default()
                },
            ),
            (
                WorkKind::Review,
                WorkPlanningInput {
                    request: "review".into(),
                    explicit_kind: Some(WorkKind::Review),
                    explicit_mode: Some(WorkMode::ReviewOnly),
                    ..Default::default()
                },
            ),
            (
                WorkKind::Adopt,
                WorkPlanningInput {
                    request: "adopt".into(),
                    explicit_kind: Some(WorkKind::Adopt),
                    repository_changes: vec![RepositoryChange {
                        path: "syu.yaml".into(),
                        owner_ids: Vec::new(),
                    }],
                    ..Default::default()
                },
            ),
        ];
        for (kind, input) in cases {
            let plan = plan_work(input);
            assert!(plan.executable, "{kind:?}: {:?}", plan.impact.diagnostics);
            if kind == WorkKind::Review {
                assert!(plan.mutations.is_empty());
            } else {
                assert!(!plan.mutations.is_empty(), "{kind:?}");
            }
        }
    }

    #[test]
    fn deliver_excludes_sibling_features() {
        let mut feature = node("FEAT-1", WorkSurface::Feature, &["REQ-1"]);
        feature.implementations.push(TraceTarget {
            language: "rust".into(),
            file: "src/one.rs".into(),
            symbols: Vec::new(),
        });
        let plan = plan_work(WorkPlanningInput {
            request: "implement FEAT-1".into(),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![
                feature,
                node("REQ-1", WorkSurface::Requirement, &["FEAT-1", "FEAT-2"]),
                node("FEAT-2", WorkSurface::Feature, &["REQ-1"]),
            ],
            ..Default::default()
        });
        assert!(!plan.impact.items.iter().any(|item| item.id == "FEAT-2"));
    }

    #[test]
    fn operation_payload_and_forbidden_surface_fail_closed() {
        let rename = plan_work(WorkPlanningInput {
            request: "rename FEAT-1".into(),
            explicit_kind: Some(WorkKind::Restructure),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![node("FEAT-1", WorkSurface::Feature, &[])],
            ..Default::default()
        });
        assert!(!rename.executable);
        assert!(rename.mutations.is_empty());

        let maintain = plan_work(WorkPlanningInput {
            request: "maintain FEAT-1".into(),
            explicit_kind: Some(WorkKind::Maintain),
            seeds: vec![seed("FEAT-1", WorkSurface::Feature)],
            nodes: vec![node("FEAT-1", WorkSurface::Feature, &[])],
            ..Default::default()
        });
        assert!(!maintain.executable);
        assert!(maintain.mutations.is_empty());
    }

    #[test]
    fn create_accepts_a_new_typed_seed() {
        let plan = plan_work(WorkPlanningInput {
            request: "create requirement".into(),
            explicit_kind: Some(WorkKind::Specify),
            explicit_operation: Some(WorkOperation::Create),
            seeds: vec![seed("REQ-NEW", WorkSurface::Requirement)],
            ..Default::default()
        });
        assert!(plan.executable);
        assert!(matches!(
            plan.mutations.as_slice(),
            [WorkMutation::CreateItem { .. }]
        ));
    }
}
