use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

type ProfileParts = (
    &'static [WorkSurface],
    &'static [WorkSurface],
    &'static [WorkSurface],
    &'static [&'static str],
    bool,
    bool,
);

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
pub enum ImpactRole {
    Seed,
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkConstraints {
    #[serde(default)]
    pub forbidden_surfaces: BTreeSet<WorkSurface>,
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
pub struct ImpactedItem {
    pub id: String,
    pub surface: WorkSurface,
    pub role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactedEdge {
    pub from: String,
    pub to: String,
    pub role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryImpact {
    pub path: String,
    pub role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestImpact {
    pub reference: String,
    pub role: ImpactRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticImpact {
    pub rule: String,
    pub role: ImpactRole,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkMutation {
    SpecItem {
        id: String,
        operation: WorkOperation,
    },
    SpecEdge {
        from: String,
        to: String,
        operation: WorkOperation,
    },
    Trace {
        reference: String,
        operation: WorkOperation,
    },
    Repository {
        path: String,
        operation: WorkOperation,
    },
    Config {
        path: String,
        operation: WorkOperation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkVerification {
    pub required_surfaces: BTreeSet<WorkSurface>,
    pub completion_commands: Vec<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkKindProfile {
    pub kind: WorkKind,
    pub direct_surfaces: BTreeSet<WorkSurface>,
    pub required_surfaces: BTreeSet<WorkSurface>,
    pub forbidden_surfaces: BTreeSet<WorkSurface>,
    pub default_completion: Vec<String>,
    pub cargo_test_fallback: bool,
    pub mutation_forbidden: bool,
}

pub fn resolve_work_intent(
    request: &str,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
    seeds: Vec<WorkSeed>,
) -> WorkIntent {
    let text = request.to_lowercase();
    let mode = explicit_mode.unwrap_or_else(|| {
        if contains_any(
            &text,
            &["review", "audit", "investigate", "調査", "レビュー", "分析"],
        ) {
            WorkMode::ReviewOnly
        } else {
            WorkMode::PlanAndExecute
        }
    });
    let kind = explicit_kind.unwrap_or_else(|| infer_kind(&text, mode));
    let operation = explicit_operation.unwrap_or_else(|| infer_operation(&text, kind));
    let requested_surfaces = infer_surfaces(&text, &seeds);
    let mut constraints = WorkConstraints::default();
    constraints
        .forbidden_surfaces
        .extend(work_kind_profile(kind).forbidden_surfaces);
    WorkIntent {
        kind,
        operation,
        mode,
        seeds,
        requested_surfaces,
        constraints,
    }
}

pub fn work_kind_profile(kind: WorkKind) -> WorkKindProfile {
    use WorkSurface as S;
    let (direct, required, forbidden, completion, fallback, mutation_forbidden): ProfileParts =
        match kind {
            WorkKind::Deliver => (
                &[S::Implementation, S::Test],
                &[S::Implementation],
                &[],
                &["syu validate .", "syu task check"],
                true,
                false,
            ),
            WorkKind::Specify => (
                &[S::Requirement, S::Feature],
                &[S::Requirement],
                &[],
                &["syu validate ."],
                false,
                false,
            ),
            WorkKind::Govern => (
                &[S::Philosophy, S::Policy],
                &[S::Policy],
                &[S::Implementation],
                &["syu validate .", "syu check graph"],
                false,
                false,
            ),
            WorkKind::Restructure => (
                &[
                    S::Philosophy,
                    S::Policy,
                    S::Requirement,
                    S::Feature,
                    S::Trace,
                ],
                &[S::Trace],
                &[],
                &["syu validate .", "syu check graph"],
                false,
                false,
            ),
            WorkKind::Verify => (
                &[S::Test, S::Trace],
                &[S::Test],
                &[],
                &["syu validate .", "syu check coverage"],
                false,
                false,
            ),
            WorkKind::Repair => (
                &[S::Trace, S::Config, S::GeneratedArtifact],
                &[],
                &[S::Implementation],
                &["syu validate ."],
                false,
                false,
            ),
            WorkKind::Maintain => (
                &[S::Tooling, S::Config, S::Implementation],
                &[S::Tooling],
                &[S::Philosophy, S::Policy, S::Requirement, S::Feature],
                &["syu validate ."],
                true,
                false,
            ),
            WorkKind::Retire => (
                &[
                    S::Requirement,
                    S::Feature,
                    S::Implementation,
                    S::Test,
                    S::Trace,
                ],
                &[S::Trace],
                &[],
                &["syu validate .", "syu check graph"],
                true,
                false,
            ),
            WorkKind::Review => (
                &[],
                &[],
                &[
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
                ],
                &[],
                false,
                true,
            ),
            WorkKind::Adopt => (
                &[S::Config, S::Documentation, S::Trace],
                &[S::Config],
                &[],
                &["syu validate ."],
                false,
                false,
            ),
        };
    WorkKindProfile {
        kind,
        direct_surfaces: direct.iter().copied().collect(),
        required_surfaces: required.iter().copied().collect(),
        forbidden_surfaces: forbidden.iter().copied().collect(),
        default_completion: completion
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        cargo_test_fallback: fallback,
        mutation_forbidden,
    }
}

fn infer_kind(text: &str, mode: WorkMode) -> WorkKind {
    if mode == WorkMode::ReviewOnly {
        return WorkKind::Review;
    }
    let table = [
        (
            WorkKind::Deliver,
            &["implement", "bug fix", "実装", "バグ修正"] as &[&str],
        ),
        (
            WorkKind::Retire,
            &["retire", "deprecat", "supersede", "廃止", "削除"],
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
                "trace",
                "reciprocal",
                "registry",
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
    table
        .into_iter()
        .find(|(_, words)| contains_any(text, words))
        .map(|(kind, _)| kind)
        .unwrap_or(WorkKind::Deliver)
}

fn infer_operation(text: &str, kind: WorkKind) -> WorkOperation {
    let table = [
        (WorkOperation::Supersede, &["supersede", "置換"] as &[&str]),
        (WorkOperation::Rename, &["rename", "名前変更"]),
        (WorkOperation::Relink, &["relink", "リンク修復"]),
        (WorkOperation::Split, &["split", "分割"]),
        (WorkOperation::Merge, &["merge", "統合"]),
        (WorkOperation::Promote, &["promote", "昇格"]),
        (WorkOperation::Demote, &["demote", "降格"]),
        (WorkOperation::Move, &["move", "移動"]),
        (WorkOperation::Delete, &["delete", "remove", "削除", "廃止"]),
        (
            WorkOperation::Create,
            &["create", "add", "new", "追加", "新規"],
        ),
    ];
    table
        .into_iter()
        .find(|(_, words)| contains_any(text, words))
        .map(|(op, _)| op)
        .unwrap_or(if kind == WorkKind::Review {
            WorkOperation::Validate
        } else {
            WorkOperation::Modify
        })
}

fn infer_surfaces(text: &str, seeds: &[WorkSeed]) -> BTreeSet<WorkSurface> {
    use WorkSurface as S;
    let mut surfaces: BTreeSet<_> = seeds.iter().map(|seed| seed.surface).collect();
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
        if contains_any(text, words) {
            surfaces.insert(surface);
        }
    }
    surfaces
}

fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_kind_wins_and_axes_stay_independent() {
        let intent = resolve_work_intent(
            "review and split FEAT-1",
            Some(WorkKind::Restructure),
            None,
            Some(WorkMode::PlanOnly),
            vec![],
        );
        assert_eq!(intent.kind, WorkKind::Restructure);
        assert_eq!(intent.operation, WorkOperation::Split);
        assert_eq!(intent.mode, WorkMode::PlanOnly);
    }

    #[test]
    fn review_profile_forbids_mutation_without_test_fallback() {
        let profile = work_kind_profile(WorkKind::Review);
        assert!(profile.mutation_forbidden);
        assert!(!profile.cargo_test_fallback);
        assert!(profile.default_completion.is_empty());
    }

    #[test]
    fn classifies_representative_requests() {
        let cases = [
            ("implement the existing feature", WorkKind::Deliver),
            ("change the security policy", WorkKind::Govern),
            ("split FEAT-1", WorkKind::Restructure),
            ("add a coverage test", WorkKind::Verify),
            ("repair a broken trace", WorkKind::Repair),
            ("update a dependency", WorkKind::Maintain),
            ("retire the old feature", WorkKind::Retire),
            ("audit the branch diff", WorkKind::Review),
            ("bootstrap syu", WorkKind::Adopt),
        ];
        for (request, expected) in cases {
            assert_eq!(
                resolve_work_intent(request, None, None, None, vec![]).kind,
                expected,
                "{request}"
            );
        }
    }
}
