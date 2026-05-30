#![forbid(unsafe_code)]

pub mod components;
pub mod design;
pub mod model;

use dioxus::prelude::{Asset, asset};

pub use components::{
    AppShell, CommandPalette, EvidencePanel, GoalCanvas, GoalRail, StatusBar, WorkspacePulse,
};
pub use model::{
    CommandPaletteEntry, WorkbenchActionRunPreview, WorkbenchUiState, build_demo_state,
};

const _: Asset = asset!("/assets/tailwind.css");
