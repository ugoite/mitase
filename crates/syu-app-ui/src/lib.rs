#![forbid(unsafe_code)]

pub mod components;
pub mod design;
pub mod model;

use dioxus::prelude::{Asset, asset};

pub use components::{
    AffectedSpecPanel, AgentEvidenceView, AppShell, BranchScopeLens, ChangedFilesPanel,
    CommandOutputView, CommandPalette, EvidenceDetailDrawer, EvidencePanel, EvidenceRecordCard,
    EvidenceTimeline, GoalCanvas, GoalDependencyView, GoalPlanCanvas, GoalPlanExportPanel,
    GoalRail, GoalScopePanel, GoalTestPlanPanel, GraphEdge, GraphNode, ImpactSummaryPanel,
    ManualDecisionEvidenceView, OutOfScopePanel, OwnershipBadge, OwnershipPanel,
    RequestClassificationPanel, RequestContextEditor, RequestIntakeCanvas, RequestScopePanel,
    ScaffoldPreviewPanel, ScopeEvidenceView, ScopeLegend, SpecImpactGraph, StatusBar,
    SuggestedGoalSplitPanel, TestEvidenceView, TestRecommendationPanel, ValidationEvidenceView,
    WorkspacePulse,
};
pub use model::{
    CommandPaletteEntry, WorkbenchActionRunPreview, WorkbenchUiState, build_demo_state,
};

const _: Asset = asset!("/assets/tailwind.css");
