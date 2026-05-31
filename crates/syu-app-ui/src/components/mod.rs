mod primitives;
mod shell;

pub use primitives::{
    Button, CommandItem, DetailDrawer, EmptyState, EvidenceBadge, EvidenceLogRow, GoalCard,
    IconButton, Panel, ScopeChip, StatusDot,
};
pub use shell::{
    AffectedSpecPanel, AppShell, BranchScopeLens, ChangedFilesPanel, CommandPalette, EvidencePanel,
    GoalCanvas, GoalDependencyView, GoalPlanCanvas, GoalPlanExportPanel, GoalRail, GoalScopePanel,
    GoalTestPlanPanel, GraphEdge, GraphNode, ImpactSummaryPanel, OutOfScopePanel, OwnershipBadge,
    OwnershipPanel, RequestClassificationPanel, RequestContextEditor, RequestIntakeCanvas,
    RequestScopePanel, ScaffoldPreviewPanel, ScopeLegend, SpecImpactGraph, StatusBar,
    SuggestedGoalSplitPanel, TestRecommendationPanel, WorkspacePulse,
};
