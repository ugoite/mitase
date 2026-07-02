use super::{HelpTopic, Locale, UiCopy};
use crate::model::{PageSection, WorkbenchPage};

pub struct English;
pub static EN: English = English;

impl UiCopy for English {
    fn workspace_label(&self) -> &'static str {
        "Workspace"
    }
    fn branch_label(&self) -> &'static str {
        "Branch"
    }
    fn health_label(&self) -> &'static str {
        "Health"
    }
    fn palette_placeholder(&self) -> &'static str {
        "Search commands, tasks, and Items"
    }
    fn sidebar_title(&self) -> &'static str {
        "Workbench navigation"
    }
    fn help_label(&self) -> &'static str {
        "Help"
    }
    fn language_label(&self) -> &'static str {
        "Language"
    }
    fn language_name(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => "English",
            Locale::Ja => "Japanese",
        }
    }
    fn page_title(&self, page: WorkbenchPage) -> &'static str {
        match page {
            WorkbenchPage::Work => "Work",
            WorkbenchPage::Scope => "Scope",
            WorkbenchPage::Items => "Items",
            WorkbenchPage::Diagnostics => "Diagnostics",
            WorkbenchPage::Settings => "Settings",
        }
    }
    fn page_summary(&self, page: WorkbenchPage) -> &'static str {
        match page {
            WorkbenchPage::Work => "Understand, assign, and verify implementation work",
            WorkbenchPage::Scope => "Explain the code, spec, and test boundary",
            WorkbenchPage::Items => "Browse and edit the specification source of truth",
            WorkbenchPage::Diagnostics => "Check whether scope and execution can be trusted",
            WorkbenchPage::Settings => "Configure this workspace safely",
        }
    }
    fn section_title(&self, section: PageSection) -> &'static str {
        match section {
            PageSection::Brief => "Brief",
            PageSection::WorkScope => "Scope",
            PageSection::Delivery => "Delivery",
            PageSection::Evidence => "Evidence",
            PageSection::CodeTests => "Code & Tests",
            PageSection::Feature => "Feature",
            PageSection::Requirement => "Requirement",
            PageSection::Policy => "Policy",
            PageSection::Philosophy => "Philosophy",
            PageSection::Workspace => "Workspace",
            PageSection::GoalPlan => "Goal Plan",
            PageSection::Trace => "Trace",
            PageSection::Repository => "Repository",
            PageSection::General => "General",
            PageSection::App => "App",
            PageSection::SyuYaml => "syu.yaml",
            PageSection::Integrations => "Integrations",
        }
    }
    fn new_work(&self) -> &'static str {
        "+ New Work"
    }
    fn search(&self) -> &'static str {
        "Search"
    }
    fn run_diagnostics(&self) -> &'static str {
        "Run diagnostics"
    }
    fn help_body(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Palette => {
                "Choose a result to move to its page and focus the relevant control."
            }
            HelpTopic::Sidebar => "Switch among the four stable Workbench pages.",
            HelpTopic::Work => "Read the purpose first, then review scope, delivery, and evidence.",
            HelpTopic::Scope => {
                "Review implementation slices and the evidence behind each inferred boundary."
            }
            HelpTopic::Items => "Maintain specification Items and start Item-driven work.",
            HelpTopic::Diagnostics => "Run all checks and inspect structured findings by group.",
            HelpTopic::Settings => {
                "Preview and validate workspace configuration before applying it."
            }
        }
    }
}
