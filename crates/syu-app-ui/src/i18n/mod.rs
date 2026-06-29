use crate::model::{PageSection, WorkbenchPage};

mod en;
mod ja;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ja,
}

impl Locale {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "ja" => Some(Self::Ja),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTopic {
    Palette,
    Sidebar,
    Work,
    Scope,
    Items,
    Diagnostics,
    Settings,
}

impl HelpTopic {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Palette => "palette",
            Self::Sidebar => "sidebar",
            Self::Work => "work",
            Self::Scope => "scope",
            Self::Items => "items",
            Self::Diagnostics => "diagnostics",
            Self::Settings => "settings",
        }
    }
    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "palette" => Some(Self::Palette),
            "sidebar" => Some(Self::Sidebar),
            "work" => Some(Self::Work),
            "scope" => Some(Self::Scope),
            "items" => Some(Self::Items),
            "diagnostics" => Some(Self::Diagnostics),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

pub trait UiCopy: Sync {
    fn workspace_label(&self) -> &'static str;
    fn branch_label(&self) -> &'static str;
    fn health_label(&self) -> &'static str;
    fn palette_placeholder(&self) -> &'static str;
    fn sidebar_title(&self) -> &'static str;
    fn help_label(&self) -> &'static str;
    fn language_label(&self) -> &'static str;
    fn language_name(&self, locale: Locale) -> &'static str;
    fn page_title(&self, page: WorkbenchPage) -> &'static str;
    fn page_summary(&self, page: WorkbenchPage) -> &'static str;
    fn section_title(&self, section: PageSection) -> &'static str;
    fn new_work(&self) -> &'static str;
    fn search(&self) -> &'static str;
    fn run_diagnostics(&self) -> &'static str;
    fn help_body(&self, topic: HelpTopic) -> &'static str;
}

pub fn copy(locale: Locale) -> &'static dyn UiCopy {
    match locale {
        Locale::En => &en::EN,
        Locale::Ja => &ja::JA,
    }
}
