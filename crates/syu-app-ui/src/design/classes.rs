pub const APP_SHELL: &str = "min-h-screen bg-background text-foreground";
pub const PAGE_FRAME: &str = "mx-auto flex min-h-screen w-full max-w-[1600px] flex-col gap-3 p-3";
pub const CHROME_BAR: &str =
    "flex items-center justify-between gap-4 rounded-2xl border border-border bg-panel px-4 py-3";
pub const CHROME_META: &str =
    "flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.24em] text-foreground/70";
pub const CHROME_BADGE: &str = "rounded-full border border-border bg-panel-muted px-3 py-1";
pub const MAIN_GRID: &str =
    "grid flex-1 grid-cols-1 gap-3 lg:grid-cols-[18rem_minmax(0,1fr)_22rem]";
pub const PANEL: &str =
    "rounded-2xl border border-border bg-panel shadow-[0_24px_60px_rgba(0,0,0,0.24)]";
pub const PANEL_INNER: &str = "flex h-full flex-col gap-3 p-4";
pub const PANEL_MUTED: &str = "rounded-xl border border-border bg-panel-muted";
pub const SECTION_HEADER: &str = "flex items-center justify-between gap-3";
pub const SECTION_TITLE: &str =
    "text-sm font-semibold uppercase tracking-[0.22em] text-foreground/70";
pub const SECTION_BODY: &str = "space-y-3";
pub const COMMAND_ITEM: &str = "flex w-full items-start justify-between gap-3 rounded-xl border border-border bg-command px-3 py-2 text-left transition hover:bg-panel-muted";
pub const COMMAND_ITEM_ACTIVE: &str = "border-command-active bg-command-active text-background";
pub const COMMAND_ITEM_DISABLED: &str = "opacity-60";
pub const CHIP: &str = "inline-flex items-center gap-2 rounded-full border border-border bg-panel-muted px-2.5 py-1 text-xs";
pub const STATUS_DOT: &str = "inline-block h-2.5 w-2.5 rounded-full";
pub const EVIDENCE_CARD: &str = "rounded-xl border border-border bg-panel-muted p-3";
pub const EMPTY_STATE: &str = "rounded-xl border border-dashed border-border bg-panel-muted px-4 py-6 text-sm text-foreground/70";
pub const DRAWER: &str = "rounded-xl border border-border bg-panel-muted p-3";
