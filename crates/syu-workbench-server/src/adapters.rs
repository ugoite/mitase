use super::*;

pub(super) fn shared_workbench_state(state: WorkbenchState) -> shared_workbench::WorkbenchState {
    shared_workbench::WorkbenchState {
        workspace: state.workspace.map(shared_workspace_snapshot),
        request: state.request.map(shared_active_request_state),
        goals: shared_goal_list_state(state.goals),
        branch_scope: state.branch_scope.map(shared_branch_scope_state),
        evidence_timeline: shared_evidence_timeline_state(state.evidence_timeline),
        assignment: state.assignment.map(shared_assignment_state),
        job: shared_job_state(state.job),
        command_palette: shared_command_palette_state(state.command_palette),
        confirmation: state.confirmation.map(shared_confirmation_metadata),
    }
}

pub(super) fn shared_workspace_snapshot(
    snapshot: WorkspaceSnapshot,
) -> shared_workbench::WorkspaceSnapshot {
    shared_workbench::WorkspaceSnapshot {
        workspace_root: snapshot.workspace_root,
        spec_root: snapshot.spec_root,
        branch: snapshot.branch,
        validation_summary: snapshot.validation_summary,
    }
}

pub(super) fn shared_active_request_state(
    request: ActiveRequestState,
) -> shared_workbench::ActiveRequestState {
    shared_workbench::ActiveRequestState {
        request_path: request.request_path,
        artifact: request.artifact,
        classification: request.classification,
        scope: request.scope,
        scaffold: request.scaffold,
    }
}

pub(super) fn shared_active_goal_state(goal: ActiveGoalState) -> shared_workbench::ActiveGoalState {
    shared_workbench::ActiveGoalState {
        goal_id: goal.goal_id,
        goal_plan: goal.goal_plan,
        test_selection: goal.test_selection,
        check_report: goal.check_report,
    }
}

pub(super) fn shared_goal_list_state(goals: GoalListState) -> shared_workbench::GoalListState {
    shared_workbench::GoalListState {
        active: goals
            .active
            .into_iter()
            .map(shared_active_goal_state)
            .collect(),
        selected_goal_id: goals.selected_goal_id,
    }
}

pub(super) fn shared_branch_scope_state(
    report: BranchScopeReport,
) -> shared_workbench::BranchScopeState {
    shared_workbench::BranchScopeState {
        range: Some(report.range.clone()),
        bounded_scope: Some(shared_workbench::BoundedScope {
            range: Some(report.range.clone()),
            allowed_ids: report
                .spec_impact
                .affected_items
                .iter()
                .map(|item| item.id.clone())
                .collect(),
            max_files: Some(report.changed_files.len()),
        }),
        allowed_ids: report
            .spec_impact
            .affected_items
            .iter()
            .map(|item| item.id.clone())
            .collect(),
        report: Some(report),
    }
}

pub(super) fn shared_evidence_timeline_state(
    timeline: EvidenceTimelineState,
) -> shared_workbench::EvidenceTimelineState {
    shared_workbench::EvidenceTimelineState {
        entries: timeline
            .entries
            .into_iter()
            .map(shared_evidence_record)
            .collect(),
    }
}

pub(super) fn shared_evidence_record(entry: EvidenceEntry) -> shared_workbench::EvidenceRecord {
    shared_workbench::EvidenceRecord {
        kind: shared_evidence_kind(entry.kind),
        status: shared_evidence_status(entry.status),
        summary: entry.summary,
        timestamp: entry.timestamp,
        goal_id: entry.goal_id,
        subject: None,
        severity: None,
        source: entry.source.map(shared_evidence_source),
        action_id: entry.action_id.and_then(shared_action_id),
        command: None,
        attachments: entry
            .attachments
            .into_iter()
            .map(shared_evidence_attachment)
            .collect(),
        related_spec_id: None,
    }
}

pub(super) fn shared_evidence_source(source: EvidenceSource) -> shared_workbench::EvidenceSource {
    match source {
        EvidenceSource::Action {
            action_id,
            action_label,
        } => shared_workbench::EvidenceSource::Action {
            action_id: action_id.and_then(shared_action_id),
            action_label,
        },
        EvidenceSource::Command { command } => {
            shared_workbench::EvidenceSource::Command { command }
        }
        EvidenceSource::System { component } => {
            shared_workbench::EvidenceSource::System { component }
        }
    }
}

pub(super) fn shared_evidence_attachment(
    attachment: EvidenceAttachment,
) -> shared_workbench::EvidenceAttachment {
    shared_workbench::EvidenceAttachment {
        label: attachment.label,
        mime_type: attachment.mime_type,
        summary: attachment.summary,
        content: attachment.content,
        truncated: attachment.truncated,
    }
}

pub(super) fn shared_assignment_state(
    assignment: AssignmentState,
) -> shared_workbench::AssignmentState {
    let include = assignment
        .scope
        .as_ref()
        .map(|scope| scope.allowed_ids.clone())
        .unwrap_or_default();
    let required_tests = assignment
        .scope
        .as_ref()
        .and_then(|scope| scope.range.clone())
        .into_iter()
        .collect();
    shared_workbench::Assignment {
        id: assignment
            .goal_id
            .as_ref()
            .map(|goal_id| format!("assignment-{}", goal_id.to_lowercase()))
            .unwrap_or_else(|| "assignment-1".to_string()),
        goal_id: assignment.goal_id,
        assignee: assignment.assignee.map(shared_assignee),
        scope: shared_workbench::AssignmentScope {
            include,
            required_tests,
            ..shared_workbench::AssignmentScope::default()
        },
        evidence_requirements: assignment
            .expected_evidence
            .iter()
            .map(|kind| shared_workbench::AssignmentEvidenceRequirement {
                id: shared_evidence_kind(kind.clone()).label().to_string(),
                description: shared_evidence_kind(kind.clone()).label().replace('_', " "),
                kind: shared_evidence_kind(kind.clone()),
                required: true,
            })
            .collect(),
        expected_evidence: assignment
            .expected_evidence
            .into_iter()
            .map(shared_evidence_kind)
            .collect(),
        ..shared_workbench::Assignment::default()
    }
}

pub(super) fn shared_assignee(assignee: AssignmentAssignee) -> shared_workbench::Assignee {
    match assignee {
        AssignmentAssignee::Human { name } => shared_workbench::Assignee::human(name),
        AssignmentAssignee::Ai { model } => {
            shared_workbench::Assignee::local_command(model.clone(), model)
        }
    }
}

pub(super) fn shared_job_state(job: JobState) -> shared_workbench::JobState {
    shared_workbench::JobState {
        status: match job.status {
            JobStatus::Idle => shared_workbench::JobStatus::Idle,
            JobStatus::Queued => shared_workbench::JobStatus::Queued,
            JobStatus::Running => shared_workbench::JobStatus::Running,
            JobStatus::Completed => shared_workbench::JobStatus::Completed,
            JobStatus::Failed | JobStatus::Cancelled => shared_workbench::JobStatus::Failed,
        },
        action_id: job.action_id.and_then(shared_action_id),
        message: job.message,
    }
}

pub(super) fn shared_command_palette_state(
    palette: CommandPaletteState,
) -> shared_workbench::CommandPaletteState {
    shared_workbench::CommandPaletteState {
        query: palette.query,
        selected_action_id: palette.selected_action_id.and_then(shared_action_id),
        visible_actions: palette
            .visible_actions
            .into_iter()
            .filter_map(shared_action_id)
            .collect(),
    }
}

pub(super) fn shared_confirmation_metadata(
    confirmation: WorkbenchConfirmationMetadata,
) -> shared_workbench::WorkbenchConfirmationMetadata {
    shared_workbench::WorkbenchConfirmationMetadata {
        confirmed_by: confirmation.confirmed_by,
        rationale: confirmation.rationale,
        scope_token: confirmation.scope_token,
    }
}

pub(super) fn shared_action_id(action_id: String) -> Option<shared_workbench::WorkbenchActionId> {
    match action_id.as_str() {
        "request.new" => Some(shared_workbench::WorkbenchActionId::RequestNew),
        "request.classify" => Some(shared_workbench::WorkbenchActionId::RequestClassify),
        "request.scope" => Some(shared_workbench::WorkbenchActionId::RequestScope),
        "request.scaffold" => Some(shared_workbench::WorkbenchActionId::RequestScaffold),
        "request.plan" => Some(shared_workbench::WorkbenchActionId::RequestPlan),
        "goal.test_select" => Some(shared_workbench::WorkbenchActionId::GoalTestSelect),
        "goal.check" => Some(shared_workbench::WorkbenchActionId::GoalCheck),
        "branch.scope" => Some(shared_workbench::WorkbenchActionId::BranchScope),
        "branch.infer_goal" => Some(shared_workbench::WorkbenchActionId::BranchInferGoal),
        "spec.impact" => Some(shared_workbench::WorkbenchActionId::SpecImpact),
        "trace.range" => Some(shared_workbench::WorkbenchActionId::TraceRange),
        "relate.range" => Some(shared_workbench::WorkbenchActionId::RelateRange),
        "validation.run" => Some(shared_workbench::WorkbenchActionId::ValidationRun),
        "history.show" => Some(shared_workbench::WorkbenchActionId::HistoryShow),
        "assignment.create" => Some(shared_workbench::WorkbenchActionId::AssignmentCreate),
        "assignment.preview" => Some(shared_workbench::WorkbenchActionId::AssignmentPreview),
        "assignment.run_dry" => Some(shared_workbench::WorkbenchActionId::AssignmentRunDry),
        "assignment.run" => Some(shared_workbench::WorkbenchActionId::AssignmentRun),
        "assignment.cancel" => Some(shared_workbench::WorkbenchActionId::AssignmentCancel),
        "assignment.record_manual" => {
            Some(shared_workbench::WorkbenchActionId::AssignmentRecordManual)
        }
        "assignment.collect_evidence" => {
            Some(shared_workbench::WorkbenchActionId::AssignmentCollectEvidence)
        }
        "agent.run" => Some(shared_workbench::WorkbenchActionId::AgentRun),
        _ => None,
    }
}

pub(super) fn shared_evidence_kind(
    kind: WorkbenchEvidenceKind,
) -> shared_workbench::WorkbenchEvidenceKind {
    match kind {
        WorkbenchEvidenceKind::RequestArtifact => {
            shared_workbench::WorkbenchEvidenceKind::RequestArtifact
        }
        WorkbenchEvidenceKind::ClassificationOutcome => {
            shared_workbench::WorkbenchEvidenceKind::ClassificationOutcome
        }
        WorkbenchEvidenceKind::ScopeOutcome => {
            shared_workbench::WorkbenchEvidenceKind::ScopeOutcome
        }
        WorkbenchEvidenceKind::ScaffoldPlan => {
            shared_workbench::WorkbenchEvidenceKind::ScaffoldPlan
        }
        WorkbenchEvidenceKind::GoalPlanArtifact => {
            shared_workbench::WorkbenchEvidenceKind::GoalPlanArtifact
        }
        WorkbenchEvidenceKind::TaskTestSelectionPlan => {
            shared_workbench::WorkbenchEvidenceKind::TaskTestSelectionPlan
        }
        WorkbenchEvidenceKind::GoalPlanCheckReport => {
            shared_workbench::WorkbenchEvidenceKind::GoalPlanCheckReport
        }
        WorkbenchEvidenceKind::BranchScopeReport => {
            shared_workbench::WorkbenchEvidenceKind::BranchScopeReport
        }
        WorkbenchEvidenceKind::ValidationReport => {
            shared_workbench::WorkbenchEvidenceKind::ValidationReport
        }
        WorkbenchEvidenceKind::HistoryResponse => {
            shared_workbench::WorkbenchEvidenceKind::HistoryResponse
        }
        WorkbenchEvidenceKind::AssignmentState => {
            shared_workbench::WorkbenchEvidenceKind::AssignmentState
        }
        WorkbenchEvidenceKind::JobState => shared_workbench::WorkbenchEvidenceKind::JobState,
    }
}

pub(super) fn shared_evidence_status(status: EvidenceStatus) -> shared_workbench::EvidenceStatus {
    match status {
        EvidenceStatus::Pending => shared_workbench::EvidenceStatus::Pending,
        EvidenceStatus::Pass => shared_workbench::EvidenceStatus::Pass,
        EvidenceStatus::Warn => shared_workbench::EvidenceStatus::Warn,
        EvidenceStatus::Fail => shared_workbench::EvidenceStatus::Fail,
        EvidenceStatus::Skipped => shared_workbench::EvidenceStatus::Skipped,
        EvidenceStatus::Unknown => shared_workbench::EvidenceStatus::Unknown,
    }
}
