use crate::WorkbenchPane;

mod en;
mod ja;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ja,
}

impl Locale {
    pub fn slug(self) -> &'static str {
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
    Pulse,
    Goals,
    Request,
    Branch,
    Assignment,
    Graph,
    Evidence,
}

impl HelpTopic {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Palette => "palette",
            Self::Sidebar => "sidebar",
            Self::Pulse => "pulse",
            Self::Goals => "goals",
            Self::Request => "request",
            Self::Branch => "branch",
            Self::Assignment => "assignment",
            Self::Graph => "graph",
            Self::Evidence => "evidence",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "palette" => Some(Self::Palette),
            "sidebar" => Some(Self::Sidebar),
            "pulse" => Some(Self::Pulse),
            "goals" => Some(Self::Goals),
            "request" => Some(Self::Request),
            "branch" => Some(Self::Branch),
            "assignment" => Some(Self::Assignment),
            "graph" => Some(Self::Graph),
            "evidence" => Some(Self::Evidence),
            _ => None,
        }
    }
}

pub trait UiCopy: Sync {
    fn app_title(&self) -> &'static str;
    fn app_tagline(&self) -> &'static str;
    fn workspace_label(&self) -> &'static str;
    fn branch_label(&self) -> &'static str;
    fn health_label(&self) -> &'static str;
    fn actions_label(&self) -> &'static str;
    fn palette_placeholder(&self) -> &'static str;
    fn palette_hint(&self) -> &'static str;
    fn sidebar_title(&self) -> &'static str;
    fn sidebar_toggle_open(&self) -> &'static str;
    fn sidebar_toggle_close(&self) -> &'static str;
    fn help_label(&self) -> &'static str;
    fn close_label(&self) -> &'static str;
    fn language_label(&self) -> &'static str;
    fn language_name(&self, locale: Locale) -> &'static str;
    fn pane_title(&self, pane: WorkbenchPane) -> &'static str;
    fn pane_summary(&self, pane: WorkbenchPane) -> &'static str;
    fn help_title(&self, topic: HelpTopic) -> &'static str;
    fn help_body(&self, topic: HelpTopic) -> &'static str;
    fn command_surface_body(&self) -> &'static str;
}

pub fn copy(locale: Locale) -> &'static dyn UiCopy {
    match locale {
        Locale::En => &en::EN,
        Locale::Ja => &ja::JA,
    }
}
