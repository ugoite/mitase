use super::{HelpTopic, Locale, UiCopy};
use crate::WorkbenchPane;

pub struct English;

pub static EN: English = English;

impl UiCopy for English {
    fn app_title(&self) -> &'static str {
        "Syu Workbench"
    }

    fn app_tagline(&self) -> &'static str {
        "command-palette-first workspace"
    }

    fn workspace_label(&self) -> &'static str {
        "workspace"
    }

    fn branch_label(&self) -> &'static str {
        "branch"
    }

    fn health_label(&self) -> &'static str {
        "health"
    }

    fn actions_label(&self) -> &'static str {
        "suggestions"
    }

    fn palette_placeholder(&self) -> &'static str {
        "Type a command"
    }

    fn palette_hint(&self) -> &'static str {
        "Results appear as you type"
    }

    fn sidebar_title(&self) -> &'static str {
        "navigation"
    }

    fn help_label(&self) -> &'static str {
        "help"
    }

    fn language_label(&self) -> &'static str {
        "language"
    }

    fn language_name(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => "EN",
            Locale::Ja => "日本語",
        }
    }

    fn pane_title(&self, pane: WorkbenchPane) -> &'static str {
        match pane {
            WorkbenchPane::Items => "Items",
            WorkbenchPane::Diagnostics => "Diagnostics",
            WorkbenchPane::Pulse => "Work",
            WorkbenchPane::Commands => "Command palette",
            WorkbenchPane::Goals => "Goal plan",
            WorkbenchPane::Request => "Request intake",
            WorkbenchPane::Branch => "Scope",
            WorkbenchPane::Assignment => "Assignment",
            WorkbenchPane::Graph => "Spec graph",
            WorkbenchPane::Evidence => "Evidence",
        }
    }

    fn pane_summary(&self, pane: WorkbenchPane) -> &'static str {
        match pane {
            WorkbenchPane::Items => "browse and edit the persistent specification",
            WorkbenchPane::Diagnostics => "refresh workspace and goal checks",
            WorkbenchPane::Pulse => "requests, goals, assignment, and evidence",
            WorkbenchPane::Commands => "the top box that launches actions",
            WorkbenchPane::Goals => "the current goal and the plan behind it",
            WorkbenchPane::Request => "the request you are classifying",
            WorkbenchPane::Branch => "branch scope and specification impact",
            WorkbenchPane::Assignment => "who owns the handoff",
            WorkbenchPane::Graph => "how specs, code, and tests connect",
            WorkbenchPane::Evidence => "what happened most recently",
        }
    }

    fn help_title(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Items => "Items",
            HelpTopic::Diagnostics => "Diagnostics",
            HelpTopic::Palette => "Command palette",
            HelpTopic::Sidebar => "Sidebar navigation",
            HelpTopic::Pulse => "Workspace pulse",
            HelpTopic::Goals => "Goal plan",
            HelpTopic::Request => "Request intake",
            HelpTopic::Branch => "Branch",
            HelpTopic::Assignment => "Assignment",
            HelpTopic::Graph => "Spec graph",
            HelpTopic::Evidence => "Evidence",
        }
    }

    fn help_body(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Items => {
                "Browse the layered file tree, follow links, and manage specification items."
            }
            HelpTopic::Diagnostics => {
                "Run workspace and goal checks, then jump from findings to affected items."
            }
            HelpTopic::Palette => "Focus the top box, type a few letters, then choose a result.",
            HelpTopic::Sidebar => "Use the left sidebar to switch views.",
            HelpTopic::Pulse => "Shows the workspace, branch, and one action to open together.",
            HelpTopic::Goals => "Shows the current goal and the plan behind it.",
            HelpTopic::Request => {
                "Shows the request text, its classification, and the scope it opens."
            }
            HelpTopic::Branch => "Shows the branch range, changed files, and impact.",
            HelpTopic::Assignment => "Shows who owns the handoff and how it runs.",
            HelpTopic::Graph => "Shows how specs, files, and tests connect.",
            HelpTopic::Evidence => "Shows the newest events and outputs first.",
        }
    }

    fn command_surface_body(&self) -> &'static str {
        "Focus the top box, type to filter, and pick a result."
    }

    fn run_label(&self) -> &'static str {
        "Run"
    }

    fn running_label(&self) -> &'static str {
        "Running..."
    }
}
