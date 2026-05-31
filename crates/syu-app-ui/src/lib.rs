#![forbid(unsafe_code)]

pub mod components;
pub mod design;
pub mod model;

use dioxus::prelude::{Asset, asset};

pub use components::{
    AffectedSpecPanel, AppShell, BranchScopeLens, ChangedFilesPanel, CommandPalette, EvidencePanel,
    GoalCanvas, GoalDependencyView, GoalPlanCanvas, GoalPlanExportPanel, GoalRail, GoalScopePanel,
    GoalTestPlanPanel, GraphEdge, GraphNode, ImpactSummaryPanel, OutOfScopePanel, OwnershipBadge,
    OwnershipPanel, RequestClassificationPanel, RequestContextEditor, RequestIntakeCanvas,
    RequestScopePanel, ScaffoldPreviewPanel, ScopeLegend, SpecImpactGraph, StatusBar,
    SuggestedGoalSplitPanel, TestRecommendationPanel, WorkspacePulse,
};
pub use model::{
    CommandPaletteEntry, WorkbenchActionRunPreview, WorkbenchUiState, build_demo_state,
};

const _: Asset = asset!("/assets/tailwind.css");
