mod primitives;
mod shell;

pub use primitives::{
    Button, CommandItem, DetailDrawer, EmptyState, EvidenceBadge, EvidenceLogRow, GoalCard,
    IconButton, Panel, ScopeChip, StatusDot,
};
pub use shell::{
    AppShell, CommandPalette, EvidencePanel, GoalCanvas, GoalDependencyView, GoalPlanCanvas,
    GoalPlanExportPanel, GoalRail, GoalScopePanel, GoalTestPlanPanel, RequestClassificationPanel,
    RequestContextEditor, RequestIntakeCanvas, RequestScopePanel, ScaffoldPreviewPanel, StatusBar,
    WorkspacePulse,
};
