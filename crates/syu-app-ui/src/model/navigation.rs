use super::CommandCategory;
use crate::i18n::Locale;
use syu_workbench::WorkbenchActionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkbenchPage {
    #[default]
    Work,
    Scope,
    Items,
    Diagnostics,
    Settings,
}

impl WorkbenchPage {
    pub const ROLES: [Self; 4] = [Self::Work, Self::Scope, Self::Items, Self::Diagnostics];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Scope => "scope",
            Self::Items => "items",
            Self::Diagnostics => "diagnostics",
            Self::Settings => "settings",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "work" => Some(Self::Work),
            "scope" => Some(Self::Scope),
            "items" => Some(Self::Items),
            "diagnostics" => Some(Self::Diagnostics),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }

    pub const fn icon(self) -> &'static str {
        match self {
            Self::Work => "◉",
            Self::Scope => "↗",
            Self::Items => "▤",
            Self::Diagnostics => "✓",
            Self::Settings => "⚙",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSection {
    Brief,
    WorkScope,
    Delivery,
    Evidence,
    CodeTests,
    Feature,
    Requirement,
    Policy,
    Philosophy,
    Workspace,
    GoalPlan,
    Trace,
    Repository,
    General,
    SyuYaml,
    Integrations,
}

impl PageSection {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::WorkScope => "work-scope",
            Self::Delivery => "delivery",
            Self::Evidence => "evidence",
            Self::CodeTests => "code-tests",
            Self::Feature => "feature",
            Self::Requirement => "requirement",
            Self::Policy => "policy",
            Self::Philosophy => "philosophy",
            Self::Workspace => "workspace",
            Self::GoalPlan => "goal-plan",
            Self::Trace => "trace",
            Self::Repository => "repository",
            Self::General => "general",
            Self::SyuYaml => "syu-yaml",
            Self::Integrations => "integrations",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Some(match value {
            "brief" => Self::Brief,
            "work-scope" => Self::WorkScope,
            "delivery" => Self::Delivery,
            "evidence" => Self::Evidence,
            "code-tests" => Self::CodeTests,
            "feature" => Self::Feature,
            "requirement" => Self::Requirement,
            "policy" => Self::Policy,
            "philosophy" => Self::Philosophy,
            "workspace" => Self::Workspace,
            "goal-plan" => Self::GoalPlan,
            "trace" => Self::Trace,
            "repository" => Self::Repository,
            "general" => Self::General,
            "syu-yaml" => Self::SyuYaml,
            "integrations" => Self::Integrations,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusIntent {
    Search,
    Create,
    Timeline,
    DiagnosticsRun,
    ScopeSelector,
    Assignment,
    Completion,
    Configuration,
}

impl FocusIntent {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Create => "create",
            Self::Timeline => "timeline",
            Self::DiagnosticsRun => "diagnostics-run",
            Self::ScopeSelector => "scope-selector",
            Self::Assignment => "assignment",
            Self::Completion => "completion",
            Self::Configuration => "configuration",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Some(match value {
            "search" => Self::Search,
            "create" => Self::Create,
            "timeline" => Self::Timeline,
            "diagnostics-run" => Self::DiagnosticsRun,
            "scope-selector" => Self::ScopeSelector,
            "assignment" => Self::Assignment,
            "completion" => Self::Completion,
            "configuration" => Self::Configuration,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandExecution {
    NavigationOnly,
    PrepareForm,
    ReadOnlyRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandTarget {
    pub page: WorkbenchPage,
    pub section: Option<PageSection>,
    pub entity: Option<String>,
    pub anchor: &'static str,
    pub focus: FocusIntent,
    pub execution: CommandExecution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteTargetEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: CommandCategory,
    pub target: CommandTarget,
    pub disabled_reason: Option<String>,
}

pub fn target_for_command(id: &str) -> Option<CommandTarget> {
    Some(match id {
        "cli.show" | "cli.browse" | "cli.list" | "cli.search" | "cli.explain" => CommandTarget {
            page: WorkbenchPage::Items,
            section: None,
            entity: None,
            anchor: "items-search",
            focus: FocusIntent::Search,
            execution: CommandExecution::NavigationOnly,
        },
        "cli.add" => CommandTarget {
            page: WorkbenchPage::Items,
            section: None,
            entity: Some("draft".to_string()),
            anchor: "item-editor",
            focus: FocusIntent::Create,
            execution: CommandExecution::PrepareForm,
        },
        "cli.log" => CommandTarget {
            page: WorkbenchPage::Work,
            section: Some(PageSection::Evidence),
            entity: None,
            anchor: "evidence-timeline",
            focus: FocusIntent::Timeline,
            execution: CommandExecution::NavigationOnly,
        },
        "cli.validate" | "cli.doctor" | "cli.audit" | "cli.report" | "cli.task.check"
        | "diagnostics.all" => CommandTarget {
            page: WorkbenchPage::Diagnostics,
            section: Some(if id == "cli.task.check" {
                PageSection::GoalPlan
            } else {
                PageSection::Workspace
            }),
            entity: None,
            anchor: "diagnostics-run",
            focus: FocusIntent::DiagnosticsRun,
            execution: CommandExecution::ReadOnlyRefresh,
        },
        "cli.trace" | "cli.task.scope" | "cli.task.infer" | "cli.relate" => CommandTarget {
            page: WorkbenchPage::Scope,
            section: Some(PageSection::CodeTests),
            entity: None,
            anchor: "scope-selector",
            focus: FocusIntent::ScopeSelector,
            execution: CommandExecution::NavigationOnly,
        },
        "cli.task.classify" | "cli.task.plan" | "cli.task.scaffold" => CommandTarget {
            page: WorkbenchPage::Work,
            section: Some(PageSection::Brief),
            entity: Some("new".to_string()),
            anchor: "work-brief",
            focus: FocusIntent::Create,
            execution: CommandExecution::PrepareForm,
        },
        "cli.task.test_select" | "cli.completion" => CommandTarget {
            page: WorkbenchPage::Work,
            section: Some(PageSection::Delivery),
            entity: None,
            anchor: "assignment",
            focus: FocusIntent::Completion,
            execution: CommandExecution::PrepareForm,
        },
        "cli.lsp" => CommandTarget {
            page: WorkbenchPage::Settings,
            section: Some(PageSection::Integrations),
            entity: None,
            anchor: "workspace-configuration",
            focus: FocusIntent::Configuration,
            execution: CommandExecution::PrepareForm,
        },
        "cli.init" | "cli.templates" => CommandTarget {
            page: WorkbenchPage::Settings,
            section: Some(PageSection::SyuYaml),
            entity: None,
            anchor: "workspace-configuration",
            focus: FocusIntent::Configuration,
            execution: CommandExecution::PrepareForm,
        },
        _ => return None,
    })
}

pub fn target_for_action(id: WorkbenchActionId) -> CommandTarget {
    use WorkbenchActionId::*;
    match id {
        HistoryShow => target_for_command("cli.log").expect("history target"),
        ValidationRun | GoalCheck => target_for_command(if id == GoalCheck {
            "cli.task.check"
        } else {
            "cli.validate"
        })
        .expect("diagnostic target"),
        BranchScope | SpecImpact | TraceRange | RelateRange => {
            target_for_command("cli.task.scope").expect("scope target")
        }
        AssignmentCreate | AgentRun => CommandTarget {
            page: WorkbenchPage::Work,
            section: Some(PageSection::Delivery),
            entity: None,
            anchor: "assignment",
            focus: FocusIntent::Assignment,
            execution: CommandExecution::PrepareForm,
        },
        _ => CommandTarget {
            page: WorkbenchPage::Work,
            section: Some(PageSection::Brief),
            entity: None,
            anchor: "work-brief",
            focus: FocusIntent::Create,
            execution: CommandExecution::PrepareForm,
        },
    }
}

pub fn localized_target_title(locale: Locale, id: &str, fallback: &str) -> String {
    match (locale, id) {
        (Locale::Ja, "cli.log") => "この作業の実行履歴を表示".to_string(),
        (Locale::Ja, "cli.add") => "新しい Item を作成".to_string(),
        (Locale::Ja, "diagnostics.all") => "すべての診断を実行".to_string(),
        _ => fallback.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::cli_command_catalog;

    #[test]
    fn every_palette_command_has_one_page_target() {
        for command in cli_command_catalog(Locale::En) {
            let target = target_for_command(command.id)
                .unwrap_or_else(|| panic!("missing target for {}", command.id));
            assert!(!target.anchor.is_empty());
        }
    }

    #[test]
    fn legacy_page_slugs_are_rejected() {
        for slug in [
            "commands",
            "pulse",
            "goals",
            "request",
            "branch",
            "assignment",
            "graph",
            "evidence",
        ] {
            assert_eq!(WorkbenchPage::from_slug(slug), None);
        }
    }
}
