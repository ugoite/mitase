mod primitives;
mod shell;

pub use primitives::{
    Button, CommandItem, DetailDrawer, EmptyState, EvidenceBadge, EvidenceLogRow, GoalCard,
    IconButton, Panel, ScopeChip, StatusDot,
};
pub use shell::{
    AppShell, CommandPalette, EvidencePanel, GoalCanvas, GoalRail, StatusBar, WorkspacePulse,
};
