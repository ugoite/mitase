#![forbid(unsafe_code)]

pub mod components;
pub mod design;
pub mod i18n;
pub mod model;

use dioxus::prelude::{Asset, asset};

pub use components::{AppShell, CommandPalette, StatusBar, WorkbenchSidebar, WorkbenchStage};
pub use i18n::{HelpTopic, Locale};
pub use model::{FocusIntent, PageSection, WorkbenchPage, WorkbenchUiState};

const _: Asset = asset!("/assets/tailwind.css");
