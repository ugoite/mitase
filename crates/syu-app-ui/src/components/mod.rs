mod primitives;
mod shell;

pub use primitives::{
    AgentEvidenceView, Button, CommandItem, CommandOutputView, DetailDrawer, EmptyState,
    EvidenceBadge, EvidenceDetailDrawer, EvidenceLogRow, EvidenceRecordCard, GoalCard, IconButton,
    ManualDecisionEvidenceView, Panel, ScopeChip, ScopeEvidenceView, StatusDot, TestEvidenceView,
    ValidationEvidenceView,
};
pub use shell::{
    AffectedSpecPanel, AppShell, BranchScopeLens, ChangedFilesPanel, CommandPalette, EvidencePanel,
    EvidenceTimeline, GoalCanvas, GoalDependencyView, GoalPlanCanvas, GoalPlanExportPanel,
    GoalRail, GoalScopePanel, GoalTestPlanPanel, GraphEdge, GraphNode, ImpactSummaryPanel,
    OutOfScopePanel, OwnershipBadge, OwnershipPanel, RequestClassificationPanel,
    RequestContextEditor, RequestIntakeCanvas, RequestScopePanel, ScaffoldPreviewPanel,
    ScopeLegend, SpecImpactGraph, StatusBar, SuggestedGoalSplitPanel, TestRecommendationPanel,
    WorkspacePulse,
};
