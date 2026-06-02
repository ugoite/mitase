use crate::i18n::{HelpTopic, Locale, UiCopy, copy};
use std::collections::BTreeMap;
use syu_task_model::{
    GoalPlanArtifact, GoalPlanCompletion, GoalPlanConfidence, GoalPlanCoverage,
    GoalPlanCoverageMode, GoalPlanGoal, GoalPlanImplementationPlan, GoalPlanPersistentItem,
    GoalPlanPersistentItems, GoalPlanScope, GoalPlanScopeInclude, GoalPlanSource,
    GoalPlanSpecMapping, GoalPlanSpecUpdates, GoalPlanTestPlan, RequestArtifact,
    RequestArtifactContext, RequestClassification, ScaffoldAction, ScaffoldPlan, ScaffoldUpdate,
    ScaffoldUpdateKind, ScopeFeatureCandidate, ScopeOutcome, ScopeSignals, SearchResult,
    TaskTestSelectionCommand, TaskTestSelectionEscalation, TaskTestSelectionPlan,
};
use syu_workbench::{
    ActiveGoalState, ActiveRequestState, AffectedSpecItem, BranchScopeEvidence, BranchScopeReport,
    ChangedFileReport, EvidenceCommand, EvidenceEntry, EvidenceRecord, EvidenceSeverity,
    EvidenceSource, EvidenceStatus, EvidenceSubject, GoalListState, OutOfScopeChange,
    OwnershipStatus, WorkbenchAction, WorkbenchActionAvailability, WorkbenchActionId,
    WorkbenchActionMutability, WorkbenchActionRegistry, WorkbenchApiPayload, WorkbenchEvidenceKind,
    WorkbenchState,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPaletteEntry {
    pub action: WorkbenchAction,
    pub availability: WorkbenchActionAvailability,
    pub disabled_reason: Option<String>,
    pub matched_query: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchActionRunPreview {
    pub action_id: WorkbenchActionId,
    pub title: String,
    pub result_summary: String,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliCommandEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub invocation: &'static str,
    pub requires_input: bool,
    pub mutates_files: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliCommandPreview {
    pub id: String,
    pub title: String,
    pub invocation: String,
    pub result_summary: String,
    pub evidence_summary: String,
    pub requires_input: bool,
    pub mutates_files: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePulseSummary {
    pub workspace: String,
    pub branch: String,
    pub health: String,
    pub available_actions: usize,
    pub recent_evidence: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchUiState {
    pub payload: WorkbenchApiPayload,
    pub command_palette_open: bool,
    pub command_query: String,
    pub selected_action_id: Option<WorkbenchActionId>,
    pub selected_cli_command_id: Option<String>,
    pub preview: Option<WorkbenchActionRunPreview>,
    pub cli_preview: Option<CliCommandPreview>,
    pub locale: Locale,
    pub help_topic: Option<HelpTopic>,
}

impl WorkbenchUiState {
    pub fn from_state(state: WorkbenchState) -> Self {
        Self {
            payload: WorkbenchApiPayload::new(state),
            command_palette_open: true,
            command_query: String::new(),
            selected_action_id: None,
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            locale: Locale::En,
            help_topic: None,
        }
    }

    pub fn with_registry(state: WorkbenchState, registry: WorkbenchActionRegistry) -> Self {
        let availability = registry.availability(&state);
        Self {
            payload: WorkbenchApiPayload {
                state,
                actions: registry.actions().to_vec(),
                availability,
            },
            command_palette_open: true,
            command_query: String::new(),
            selected_action_id: None,
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            locale: Locale::En,
            help_topic: None,
        }
    }

    pub fn open_command_palette(&mut self) {
        self.command_palette_open = true;
    }

    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.command_query = query.into();
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub fn set_help_topic(&mut self, help_topic: Option<HelpTopic>) {
        self.help_topic = help_topic;
    }

    pub fn copy(&self) -> &'static dyn UiCopy {
        copy(self.locale)
    }

    pub fn select_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        self.selected_action_id = Some(action_id);
        self.selected_cli_command_id = None;
        self.cli_preview = None;
        let preview = self.action_preview(action_id);
        self.preview = preview.clone();
        preview
    }

    pub fn select_cli_command(
        &mut self,
        command_id: impl Into<String>,
    ) -> Option<CliCommandPreview> {
        let command_id = command_id.into();
        self.selected_action_id = None;
        self.preview = None;
        self.selected_cli_command_id = Some(command_id.clone());
        let preview = self.cli_command_preview(&command_id);
        self.cli_preview = preview.clone();
        preview
    }

    pub fn visible_actions(&self) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut actions = self
            .payload
            .actions
            .iter()
            .cloned()
            .zip(self.payload.availability.iter().cloned())
            .filter_map(|(action, availability)| {
                let haystack = format!(
                    "{} {} {}",
                    action.id.label(),
                    action.title.to_lowercase(),
                    action.description.to_lowercase()
                );
                let matched_query = query.is_empty() || haystack.contains(&query);
                matched_query.then(|| CommandPaletteEntry {
                    disabled_reason: (!availability.available)
                        .then(|| availability_reason(&availability)),
                    action,
                    availability,
                    matched_query,
                })
            })
            .collect::<Vec<_>>();
        actions.sort_by_key(|entry| {
            (
                !entry.availability.available,
                !entry.action.id.label().contains(query.as_str()),
                entry.action.title.clone(),
            )
        });
        actions
    }

    pub fn suggested_actions(&self, limit: usize) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut actions = self.visible_actions();
        if query.is_empty() {
            actions
                .sort_by_key(|entry| (!entry.availability.available, entry.action.title.clone()));
        }
        actions.into_iter().take(limit).collect()
    }

    pub fn visible_cli_commands(&self) -> Vec<CliCommandEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut commands = cli_command_catalog()
            .iter()
            .copied()
            .filter(|command| {
                let haystack = format!(
                    "{} {} {} {}",
                    command.id, command.title, command.description, command.invocation
                )
                .to_lowercase();
                query.is_empty() || haystack.contains(&query)
            })
            .collect::<Vec<_>>();
        commands.sort_by_key(|command| {
            (
                command.mutates_files,
                !command.id.contains(query.as_str()),
                command.title,
            )
        });
        commands
    }

    pub fn cli_command_preview(&self, command_id: &str) -> Option<CliCommandPreview> {
        let command = cli_command_catalog()
            .iter()
            .find(|command| command.id == command_id)?;
        let result_summary = if command.requires_input {
            format!(
                "{} needs a selector or file before it can run.",
                command.invocation
            )
        } else if command.mutates_files {
            format!(
                "{} is available from the palette with confirmation.",
                command.invocation
            )
        } else {
            format!("{} is ready for this workspace.", command.invocation)
        };
        let evidence_summary = if command.mutates_files {
            "writes files".to_string()
        } else if command.requires_input {
            "input required".to_string()
        } else {
            "read-only".to_string()
        };
        Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary,
            evidence_summary,
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        })
    }

    pub fn action_preview(
        &self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        let action = self
            .payload
            .actions
            .iter()
            .find(|candidate| candidate.id == action_id)?;
        if action.mutability != WorkbenchActionMutability::ReadOnly {
            return None;
        }

        Some(WorkbenchActionRunPreview {
            action_id,
            title: action.title.clone(),
            result_summary: format!("Preview opened for {}", action.title),
            evidence_summary: "Ready to review".to_string(),
        })
    }

    pub fn run_read_only_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        let preview = self.action_preview(action_id)?;
        self.selected_action_id = Some(action_id);
        self.selected_cli_command_id = None;
        self.cli_preview = None;
        self.preview = Some(preview.clone());
        Some(preview)
    }

    pub fn selected_action(&self) -> Option<&WorkbenchAction> {
        let selected = self.selected_action_id?;
        self.payload
            .actions
            .iter()
            .find(|action| action.id == selected)
    }

    pub fn disabled_reason(&self, action_id: WorkbenchActionId) -> Option<String> {
        self.payload
            .availability
            .iter()
            .find(|availability| availability.id == action_id)
            .and_then(|availability| {
                (!availability.available).then(|| availability_reason(availability))
            })
    }

    pub fn pulse_summary(&self) -> WorkspacePulseSummary {
        let workspace = self
            .payload
            .state
            .workspace
            .as_ref()
            .map(|workspace| workspace.workspace_root.display().to_string())
            .unwrap_or_else(|| "workspace not loaded".to_string());
        let branch = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.branch.clone())
            .unwrap_or_else(|| "no branch loaded".to_string());
        let health = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.validation_summary.clone())
            .unwrap_or_else(|| "health pending".to_string());
        let available_actions = self
            .payload
            .availability
            .iter()
            .filter(|availability| availability.available)
            .count();
        let recent_evidence = self
            .payload
            .state
            .evidence_timeline
            .entries
            .last()
            .map(evidence_summary)
            .unwrap_or_else(|| "no evidence yet".to_string());
        let next_action = self
            .visible_actions()
            .into_iter()
            .find(|entry| entry.availability.available)
            .map(|entry| entry.action.title)
            .unwrap_or_else(|| "nothing to open".to_string());

        WorkspacePulseSummary {
            workspace,
            branch,
            health,
            available_actions,
            recent_evidence,
            next_action,
        }
    }
}

pub fn cli_command_catalog() -> &'static [CliCommandEntry] {
    &[
        CliCommandEntry {
            id: "cli.browse",
            title: "Browse spec",
            description: "Explore the spec tree.",
            invocation: "syu browse . --non-interactive",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.workbench",
            title: "Open workbench",
            description: "Run the browser or desktop Workbench.",
            invocation: "syu workbench .",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.list",
            title: "List spec items",
            description: "List philosophies, policies, requirements, or features.",
            invocation: "syu list",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.show",
            title: "Show item",
            description: "Show one spec item by ID.",
            invocation: "syu show <id>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.search",
            title: "Search spec",
            description: "Search spec items by text or ID.",
            invocation: "syu search <query>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.audit",
            title: "Audit spec",
            description: "Audit the layered spec for overlap and tension.",
            invocation: "syu audit .",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.log",
            title: "Show history",
            description: "Show git history for a traced requirement or feature.",
            invocation: "syu log <id>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.explain",
            title: "Explain selector",
            description: "Explain how an ID, file, or symbol fits the spec chain.",
            invocation: "syu explain <selector>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.relate",
            title: "Relate selector",
            description: "Inspect the connected graph around an ID, path, symbol, or range.",
            invocation: "syu relate <selector>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.trace",
            title: "Trace file or range",
            description: "Resolve linked specs from a traced file, symbol, or git range.",
            invocation: "syu trace <file>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.doctor",
            title: "Run doctor",
            description: "Inspect contributor-tooling readiness.",
            invocation: "syu doctor .",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.validate",
            title: "Validate workspace",
            description: "Validate layered graph, traces, and optional fixes.",
            invocation: "syu validate .",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.report",
            title: "Render report",
            description: "Render a Markdown validation report.",
            invocation: "syu report .",
            requires_input: false,
            mutates_files: true,
        },
        CliCommandEntry {
            id: "cli.init",
            title: "Initialize workspace",
            description: "Scaffold a version-matched syu workspace.",
            invocation: "syu init .",
            requires_input: false,
            mutates_files: true,
        },
        CliCommandEntry {
            id: "cli.templates",
            title: "List templates",
            description: "List starter templates and examples.",
            invocation: "syu templates",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.completion",
            title: "Generate completion",
            description: "Generate shell completion scripts.",
            invocation: "syu completion <shell>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.classify",
            title: "Classify request",
            description: "Classify a request artifact.",
            invocation: "syu task classify <request.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.scope",
            title: "Scope request",
            description: "Map a request artifact onto the spec graph.",
            invocation: "syu task scope <request.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.scaffold",
            title: "Scaffold request",
            description: "Preview spec and file updates from a request.",
            invocation: "syu task scaffold <request.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.plan",
            title: "Plan request",
            description: "Generate a temporary Goal Plan.",
            invocation: "syu task plan <request.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.test_select",
            title: "Select task tests",
            description: "Select tests from a Goal Plan.",
            invocation: "syu task test-select <goal-plan.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.infer",
            title: "Infer task goal",
            description: "Infer a provisional Goal Plan from a git diff.",
            invocation: "syu task infer --range origin/main...HEAD",
            requires_input: false,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.task.check",
            title: "Check task goal",
            description: "Validate a Goal Plan against a git range.",
            invocation: "syu task check <goal-plan.yaml>",
            requires_input: true,
            mutates_files: false,
        },
        CliCommandEntry {
            id: "cli.add",
            title: "Add spec stub",
            description: "Scaffold a philosophy, policy, requirement, or feature stub.",
            invocation: "syu add <kind> <id>",
            requires_input: true,
            mutates_files: true,
        },
        CliCommandEntry {
            id: "cli.lsp",
            title: "Start LSP",
            description: "Start the editor integration language server.",
            invocation: "syu lsp",
            requires_input: false,
            mutates_files: false,
        },
    ]
}

pub fn build_demo_state() -> WorkbenchUiState {
    let request_artifact = demo_request_artifact();
    let classification = syu_workbench::ClassificationOutcome {
        classification: RequestClassification::Change,
        reasons: vec![
            "Existing Workbench request and goal artifacts are being expanded.".to_string(),
            "The request names REQ-WORKBENCH-003 and Workbench UI features.".to_string(),
        ],
        explicit_items: vec![SearchResult {
            id: "REQ-WORKBENCH-003".to_string(),
            kind: "requirement".to_string(),
            title: "Goal splitting for large change requests".to_string(),
        }],
        related_items: vec![SearchResult {
            id: "FEAT-WORKBENCH-GOAL-SPLITTER-001".to_string(),
            kind: "feature".to_string(),
            title: "Goal Splitter canvas".to_string(),
        }],
        request: request_artifact.request.clone(),
        context: request_artifact.context.clone(),
    };
    let scope = ScopeOutcome {
        classification: classification.clone(),
        signals: ScopeSignals {
            policy_discussion: false,
            philosophy_discussion: false,
            planned_feature_updates: true,
        },
        requirements: classification.explicit_items.clone(),
        features: vec![ScopeFeatureCandidate {
            id: "FEAT-WORKBENCH-REQUEST-INTAKE-001".to_string(),
            title: "Request Intake canvas".to_string(),
            status: "temporary planning artifact".to_string(),
            linked_requirements: vec!["REQ-WORKBENCH-003".to_string()],
            planned_state_update: true,
        }],
        policies: Vec::new(),
        philosophies: Vec::new(),
        notes: vec![
            "Temporary artifacts stay under .syu/workbench/requests and .syu/workbench/goals."
                .to_string(),
            "Exported Goal Plan YAML must remain compatible with syu task check.".to_string(),
        ],
    };
    let scaffold = ScaffoldPlan {
        updates: vec![
            ScaffoldUpdate {
                kind: ScaffoldUpdateKind::Feature,
                action: ScaffoldAction::Update,
                path: "docs/syu/features/workbench/goal-splitter.yaml".to_string(),
                id: Some("FEAT-WORKBENCH-GOAL-SPLITTER-001".to_string()),
                contents: "Add request intake and goal splitter UI coverage.".to_string(),
            },
            ScaffoldUpdate {
                kind: ScaffoldUpdateKind::Requirement,
                action: ScaffoldAction::Update,
                path: "docs/syu/requirements/core/workbench.yaml".to_string(),
                id: Some("REQ-WORKBENCH-003".to_string()),
                contents: "Clarify temporary Goal Plans and export compatibility.".to_string(),
            },
        ],
    };
    let goal_plan = demo_goal_plan();
    let assignment = syu_workbench::Assignment::from_goal_plan(
        &goal_plan,
        syu_workbench::Assignee::local_command("local-coder", "Local command adapter"),
        syu_workbench::AgentRunMode::DryRun,
        vec![syu_workbench::AssignmentEvidenceRequirement {
            id: "runner-output".to_string(),
            description: "stdout/stderr and prompt context from the dry-run adapter".to_string(),
            kind: WorkbenchEvidenceKind::AgentRun,
            required: true,
        }],
    );
    let mut state = WorkbenchState {
        workspace: Some(syu_workbench::WorkspaceSnapshot {
            workspace_root: std::path::PathBuf::from("/workspace/syu"),
            spec_root: std::path::PathBuf::from("/workspace/syu/docs/syu"),
            branch: Some("issue-739-request-goal-splitter".to_string()),
            validation_summary: Some("green".to_string()),
        }),
        request: Some(ActiveRequestState {
            request_path: Some(std::path::PathBuf::from(
                ".syu/workbench/requests/request-739.yaml",
            )),
            artifact: Some(request_artifact),
            classification: Some(classification),
            scope: Some(scope),
            scaffold: Some(scaffold),
        }),
        goals: GoalListState {
            active: vec![ActiveGoalState {
                goal_id: "GOAL-WB-REQUEST-001".to_string(),
                goal_plan: Some(goal_plan),
                test_selection: Some(TaskTestSelectionPlan {
                    goal_id: "GOAL-WB-REQUEST-001".to_string(),
                    goal_title: "Render request intake into Goal Plan cards".to_string(),
                    selection_mode: "minimal".to_string(),
                    commands: vec![TaskTestSelectionCommand {
                        language: "rust".to_string(),
                        command: "cargo test -p syu-app-ui request_intake_flow_renders_generated_goal_plan"
                            .to_string(),
                        reason: "covers the generated Goal Plan UI path".to_string(),
                    }],
                    escalation: TaskTestSelectionEscalation {
                        level: "affected".to_string(),
                        reason: "run workbench smoke tests after UI changes".to_string(),
                    },
                    warnings: Vec::new(),
                }),
                check_report: None,
            }],
            selected_goal_id: Some("GOAL-WB-REQUEST-001".to_string()),
        },
        confirmation: Some(syu_workbench::WorkbenchConfirmationMetadata {
            confirmed_by: "demo".to_string(),
            rationale: Some("demo fixture for request-to-goals flow".to_string()),
            scope_token: Some("request-739".to_string()),
        }),
        branch_scope: Some(syu_workbench::BranchScopeState {
            range: Some("origin/main...HEAD".to_string()),
            report: Some(demo_branch_scope_report()),
            bounded_scope: Some(syu_workbench::BoundedScope {
                range: Some("origin/main...HEAD".to_string()),
                allowed_ids: vec![
                    "REQ-WORKBENCH-004".to_string(),
                    "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
                    "FEAT-WORKBENCH-BRANCH-SCOPE-001".to_string(),
                ],
                max_files: Some(3),
            }),
            allowed_ids: vec![
                "REQ-WORKBENCH-004".to_string(),
                "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
                "FEAT-WORKBENCH-BRANCH-SCOPE-001".to_string(),
            ],
        }),
        assignment: Some(assignment),
        ..WorkbenchState::default()
    };
    state.evidence_timeline.append(
        EvidenceRecord::new(
            WorkbenchEvidenceKind::ValidationReport,
            EvidenceStatus::Pass,
            "validation passed",
            Some(EvidenceSource::Command {
                command: "syu validate".to_string(),
            }),
        )
        .with_subject(EvidenceSubject::Workspace)
        .with_severity(EvidenceSeverity::Low)
        .with_command(EvidenceCommand {
            command: "validation.run".to_string(),
            args: Vec::new(),
        }),
    );
    state.evidence_timeline.append(
        EvidenceRecord::new(
            WorkbenchEvidenceKind::BranchScopeReport,
            EvidenceStatus::Pass,
            "branch.scope connected specs, code, tests, and ownership",
            Some(EvidenceSource::Action {
                action_id: Some(WorkbenchActionId::BranchScope),
                action_label: Some("branch.scope".to_string()),
            }),
        )
        .with_action_id(WorkbenchActionId::BranchScope)
        .with_subject(EvidenceSubject::Branch)
        .with_severity(EvidenceSeverity::Low)
        .with_command(EvidenceCommand {
            command: "branch.scope".to_string(),
            args: Vec::new(),
        })
        .with_goal_id(Some("GOAL-WB-REQUEST-001".to_string())),
    );
    state.evidence_timeline.append(
        EvidenceRecord::new(
            WorkbenchEvidenceKind::TaskTestSelectionPlan,
            EvidenceStatus::Pass,
            "test selection covers the active goal",
            Some(EvidenceSource::Action {
                action_id: Some(WorkbenchActionId::GoalTestSelect),
                action_label: Some("goal.test_select".to_string()),
            }),
        )
        .with_action_id(WorkbenchActionId::GoalTestSelect)
        .with_goal_id(Some("GOAL-WB-REQUEST-001".to_string()))
        .with_subject(EvidenceSubject::Goal)
        .with_severity(EvidenceSeverity::Low)
        .with_command(EvidenceCommand {
            command: "goal.test_select".to_string(),
            args: Vec::new(),
        }),
    );
    state.evidence_timeline.append(
        EvidenceRecord::new(
            WorkbenchEvidenceKind::GoalPlanCheckReport,
            EvidenceStatus::Pass,
            "goal check passed for origin/main...HEAD",
            Some(EvidenceSource::Action {
                action_id: Some(WorkbenchActionId::GoalCheck),
                action_label: Some("goal.check".to_string()),
            }),
        )
        .with_action_id(WorkbenchActionId::GoalCheck)
        .with_goal_id(Some("GOAL-WB-REQUEST-001".to_string()))
        .with_subject(EvidenceSubject::Goal)
        .with_severity(EvidenceSeverity::Low)
        .with_command(EvidenceCommand {
            command: "goal.check".to_string(),
            args: Vec::new(),
        }),
    );
    let mut ui = WorkbenchUiState::from_state(state);
    ui.command_palette_open = true;
    ui.command_query = "goal".to_string();
    ui
}

fn demo_branch_scope_report() -> BranchScopeReport {
    let requirement = AffectedSpecItem {
        kind: "requirement".to_string(),
        id: "REQ-WORKBENCH-004".to_string(),
        title: "Spec impact and branch scope visualization".to_string(),
        document_path: Some("docs/syu/requirements/core/workbench.yaml".to_string()),
        direct: true,
    };
    let spec_graph = AffectedSpecItem {
        kind: "feature".to_string(),
        id: "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
        title: "Spec Impact Graph".to_string(),
        document_path: Some("docs/syu/features/workbench/branch-scope.yaml".to_string()),
        direct: true,
    };
    let branch_lens = AffectedSpecItem {
        kind: "feature".to_string(),
        id: "FEAT-WORKBENCH-BRANCH-SCOPE-001".to_string(),
        title: "Branch Scope Lens".to_string(),
        document_path: Some("docs/syu/features/workbench/branch-scope.yaml".to_string()),
        direct: true,
    };

    BranchScopeReport::from_evidence(BranchScopeEvidence {
        range: "origin/main...HEAD".to_string(),
        changed_files: vec![
            ChangedFileReport {
                file: "crates/syu-app-ui/src/components/shell.rs".to_string(),
                symbols: vec!["SpecImpactGraph".to_string(), "BranchScopeLens".to_string()],
                owners: vec![spec_graph.clone(), branch_lens.clone()],
                status: OwnershipStatus::Owned,
                is_spec_file: false,
            },
            ChangedFileReport {
                file: "crates/syu-app-ui/src/design/tokens.rs".to_string(),
                symbols: vec!["SCOPE_AMBIGUOUS".to_string()],
                owners: vec![spec_graph.clone()],
                status: OwnershipStatus::Partial,
                is_spec_file: false,
            },
            ChangedFileReport {
                file: "examples/legacy-browser/index.ts".to_string(),
                symbols: Vec::new(),
                owners: Vec::new(),
                status: OwnershipStatus::Unowned,
                is_spec_file: false,
            },
        ],
        trace_ownership: Vec::new(),
        spec_items: vec![requirement, spec_graph, branch_lens],
        required_tests: vec!["tests/workbench_smoke.rs".to_string()],
        linked_tests: vec!["cargo test -p syu-code-intel branch_scope".to_string()],
        include_patterns: vec![
            "crates/syu-app-ui/src/**".to_string(),
            "crates/syu-code-intel/src/branch_scope.rs".to_string(),
        ],
        exclude_patterns: vec!["examples/legacy-browser/**".to_string()],
        allowed_ids: vec![
            "REQ-WORKBENCH-004".to_string(),
            "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
            "FEAT-WORKBENCH-BRANCH-SCOPE-001".to_string(),
        ],
        unowned_files: vec!["examples/legacy-browser/index.ts".to_string()],
        ambiguous_files: vec!["crates/syu-app-ui/src/design/tokens.rs".to_string()],
        spec_files: Vec::new(),
        out_of_scope_changes: vec![OutOfScopeChange {
            file: "examples/legacy-browser/index.ts".to_string(),
            allowed_ids: vec!["FEAT-WORKBENCH-BRANCH-SCOPE-001".to_string()],
            reason: "legacy browser surface is excluded by the Goal Plan".to_string(),
        }],
        direct_items: Vec::new(),
        related_items: Vec::new(),
        has_planned_features: true,
    })
}

fn demo_request_artifact() -> RequestArtifact {
    RequestArtifact {
        version: 1,
        request: "Add a Workbench request intake flow that classifies, scopes, previews scaffold updates, generates a Goal Plan, and exports YAML.".to_string(),
        context: RequestArtifactContext {
            affected_area: Some("Dioxus Workbench UI".to_string()),
            repository_constraints: vec![
                "Reuse Workbench primitives instead of local card or badge styling.".to_string(),
                "Keep Goal Plans temporary until exported or committed.".to_string(),
            ],
            linked_ids: vec![
                "REQ-WORKBENCH-003".to_string(),
                "FEAT-WORKBENCH-REQUEST-INTAKE-001".to_string(),
            ],
        },
    }
}

fn demo_goal_plan() -> GoalPlanArtifact {
    GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: Some(".syu/workbench/requests/request-739.yaml".to_string()),
        request: Some("Add a Workbench request intake flow.".to_string()),
        classification: Some("requirement_change".to_string()),
        source: GoalPlanSource {
            request_artifact: Some(".syu/workbench/requests/request-739.yaml".to_string()),
            classification: Some("requirement_change".to_string()),
            confidence: Some(GoalPlanConfidence::High),
            ..GoalPlanSource::default()
        },
        goal: GoalPlanGoal {
            id: "GOAL-WB-REQUEST-001".to_string(),
            title: "Render request intake into Goal Plan cards".to_string(),
            statement: "Turn a scoped Workbench request into a reviewable temporary Goal Plan with explicit scope, tests, completion commands, and evidence.".to_string(),
            non_goals: vec![
                "Build a raw YAML editor".to_string(),
                "Introduce a second browser SPA or styling system".to_string(),
            ],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items: GoalPlanPersistentItems {
                requirements: vec![GoalPlanPersistentItem::Id("REQ-WORKBENCH-003".to_string())],
                features: vec![
                    GoalPlanPersistentItem::Id(
                        "FEAT-WORKBENCH-REQUEST-INTAKE-001".to_string(),
                    ),
                    GoalPlanPersistentItem::Id(
                        "FEAT-WORKBENCH-GOAL-SPLITTER-001".to_string(),
                    ),
                ],
                ..GoalPlanPersistentItems::default()
            },
            spec_updates: GoalPlanSpecUpdates {
                required: true,
                expected_updates: vec![
                    "Clarify temporary Goal Plans in REQ-WORKBENCH-003".to_string(),
                    "Register request intake and goal splitter UI coverage".to_string(),
                ],
            },
            spec_updates_required: true,
            spec_update_reasons: vec![
                "Issue 739 adds explicit request intake and Goal Splitter feature IDs.".to_string(),
            ],
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(GoalPlanConfidence::High),
            scope: GoalPlanScope {
                include: vec![
                    GoalPlanScopeInclude::Pattern("crates/syu-app-ui/src/model.rs".to_string()),
                    GoalPlanScopeInclude::Pattern(
                        "crates/syu-app-ui/src/components/shell.rs".to_string(),
                    ),
                    GoalPlanScopeInclude::Pattern("tests/workbench_smoke.rs".to_string()),
                ],
                exclude: vec![
                    "examples/browser-ui/**".to_string(),
                    "examples/legacy-browser/**".to_string(),
                    "website/**".to_string(),
                    "React, TypeScript, Vite, and Playwright surfaces".to_string(),
                ],
            },
            steps: vec![
                "Capture the plain-text request and linked context.".to_string(),
                "Render classification, scope, and scaffold preview panels.".to_string(),
                "Split the generated Goal Plan into reusable Goal cards.".to_string(),
                "Expose YAML export and next-flow actions from the same registry.".to_string(),
            ],
        },
        test_plan: GoalPlanTestPlan {
            selection_mode: syu_task_model::GoalPlanSelectionMode::Minimal,
            confidence: Some(GoalPlanConfidence::High),
            required_tests: BTreeMap::new(),
            suggested_tests: BTreeMap::new(),
        },
        coverage: GoalPlanCoverage {
            mode: GoalPlanCoverageMode::ChangedLines,
            threshold: 100,
            include: vec!["crates/syu-app-ui/src/**".to_string()],
            exclude: vec!["examples/browser-ui/**".to_string()],
        },
        completion: GoalPlanCompletion {
            must_pass: vec![
                "cargo test -p syu-app-ui".to_string(),
                "cargo test --test workbench_smoke".to_string(),
            ],
        },
        warnings: vec![
            "Temporary planning artifacts are not persistent spec content.".to_string(),
        ],
    }
}

fn availability_reason(availability: &WorkbenchActionAvailability) -> String {
    let missing = availability
        .missing_state
        .iter()
        .map(|state| state.label())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        "available".to_string()
    } else {
        format!("disabled: missing {}", missing.join(", "))
    }
}

fn evidence_summary(entry: &EvidenceEntry) -> String {
    format!("{}: {}", entry.kind.label(), entry.summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syu_workbench::{WorkbenchActionId, WorkbenchActionRegistry};

    #[test]
    fn filters_actions_by_query() {
        let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
        ui.payload = WorkbenchApiPayload::new(WorkbenchState::default());
        ui.command_query = "history".to_string();

        let visible = ui.visible_actions();

        assert!(!visible.is_empty());
        assert!(
            visible
                .iter()
                .all(|entry| entry.action.id.label().contains("history")
                    || entry.action.title.to_lowercase().contains("history"))
        );
    }

    #[test]
    fn read_only_action_returns_placeholder_preview() {
        let ui = build_demo_state();

        let preview = ui.action_preview(WorkbenchActionId::HistoryShow).unwrap();

        assert!(preview.result_summary.contains("Preview opened"));
        assert_eq!(preview.evidence_summary, "Ready to review");
    }

    #[test]
    fn registry_loaded_from_server_payload() {
        let state = WorkbenchState::default();
        let payload = WorkbenchApiPayload::new(state);
        assert_eq!(
            payload.actions.len(),
            WorkbenchActionRegistry::standard().actions().len()
        );
    }

    #[test]
    fn cli_catalog_exposes_top_level_and_task_commands() {
        let ids = cli_command_catalog()
            .iter()
            .map(|command| command.id)
            .collect::<Vec<_>>();

        assert_eq!(ids.len(), 25);
        assert!(ids.contains(&"cli.validate"));
        assert!(ids.contains(&"cli.task.check"));
        assert!(ids.contains(&"cli.add"));
    }

    #[test]
    fn filters_cli_commands_by_query_and_previews_invocation() {
        let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
        ui.set_query("validate");

        let visible = ui.visible_cli_commands();
        let preview = ui.cli_command_preview("cli.validate").unwrap();

        assert!(visible.iter().any(|command| command.id == "cli.validate"));
        assert!(preview.invocation.contains("syu validate ."));
        assert_eq!(preview.evidence_summary, "read-only");
    }
}
