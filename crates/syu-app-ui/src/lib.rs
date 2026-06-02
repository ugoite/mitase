#![forbid(unsafe_code)]

pub mod components;
pub mod design;
pub mod i18n;
pub mod model;

use dioxus::prelude::{Asset, asset};

pub use components::WorkbenchPane;
pub use components::{
    AffectedSpecPanel, AgentEvidenceView, AgentRunPanel, AppShell, AssignGoalDialog,
    AssigneeSelector, AssignmentConstraintPanel, AssignmentEvidencePanel, AssignmentPromptPreview,
    BranchScopeLens, ChangedFilesPanel, CommandOutputView, CommandPalette, EvidenceDetailDrawer,
    EvidencePanel, EvidenceRecordCard, EvidenceTimeline, GoalCanvas, GoalDependencyView,
    GoalPlanCanvas, GoalPlanExportPanel, GoalRail, GoalScopePanel, GoalTestPlanPanel, GraphEdge,
    GraphNode, HumanAssignmentPanel, ImpactSummaryPanel, ManualDecisionEvidenceView,
    OutOfScopePanel, OwnershipBadge, OwnershipPanel, RequestClassificationPanel,
    RequestContextEditor, RequestIntakeCanvas, RequestScopePanel, ScaffoldPreviewPanel,
    ScopeEvidenceView, ScopeGuardPreview, ScopeLegend, SpecImpactGraph, StatusBar,
    SuggestedGoalSplitPanel, TestEvidenceView, TestRecommendationPanel, ValidationEvidenceView,
    WorkspacePulse,
};
pub use i18n::{HelpTopic, Locale};
pub use model::{
    CommandPaletteEntry, WorkbenchActionRunPreview, WorkbenchUiState, build_demo_state,
};

const _: Asset = asset!("/assets/tailwind.css");
