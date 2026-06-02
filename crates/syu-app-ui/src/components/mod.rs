mod primitives;
mod shell;

pub use primitives::{
    AgentEvidenceView, Button, CommandItem, CommandOutputView, DetailDrawer, EmptyState,
    EvidenceBadge, EvidenceDetailDrawer, EvidenceLogRow, EvidenceRecordCard, GoalCard, IconButton,
    ManualDecisionEvidenceView, Panel, ScopeChip, ScopeEvidenceView, StatusDot, TestEvidenceView,
    ValidationEvidenceView,
};
pub use shell::WorkbenchPane;
pub use shell::{
    AffectedSpecPanel, AgentRunPanel, AppShell, AssignGoalDialog, AssigneeSelector,
    AssignmentConstraintPanel, AssignmentEvidencePanel, AssignmentPromptPreview, BranchScopeLens,
    ChangedFilesPanel, CommandPalette, EvidencePanel, EvidenceTimeline, GoalCanvas,
    GoalDependencyView, GoalPlanCanvas, GoalPlanExportPanel, GoalRail, GoalScopePanel,
    GoalTestPlanPanel, GraphEdge, GraphNode, HumanAssignmentPanel, ImpactSummaryPanel,
    OutOfScopePanel, OwnershipBadge, OwnershipPanel, RequestClassificationPanel,
    RequestContextEditor, RequestIntakeCanvas, RequestScopePanel, ScaffoldPreviewPanel,
    ScopeGuardPreview, ScopeLegend, SpecImpactGraph, StatusBar, SuggestedGoalSplitPanel,
    TestRecommendationPanel, WorkspacePulse,
};
