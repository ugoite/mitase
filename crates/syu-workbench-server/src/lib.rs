#![forbid(unsafe_code)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use syu_diagnostics::ValidationResult;
use syu_planner::plan;
use syu_project_model::ProjectConfig;
use syu_spec_model::LocalAnchorKind;
use syu_work_model::{WorkPlan, WorkRequest};
use syu_workspace::SpecWorkspace;

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjection {
    pub workspace: WorkspaceSummary,
    pub config: ProjectConfig,
    pub items: Vec<ItemSummary>,
    pub plan: Option<WorkPlan>,
    pub validation: ValidationRunView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRunState {
    NotRun,
    Running,
    Passed,
    Issues,
    Failed,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRunView {
    pub state: ValidationRunState,
    pub context: String,
    pub basis: Option<String>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub duration_ms: Option<u64>,
    pub evaluated_rule_count: usize,
    pub applicable_phase_count: usize,
    pub skipped_phase_count: usize,
    pub phases: Vec<ValidationPhaseView>,
    pub diagnostics: Vec<ValidationDiagnosticView>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationPhaseView {
    pub id: String,
    pub state: ValidationRunState,
    pub issue_count: usize,
    pub not_applicable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDiagnosticView {
    pub phase: String,
    #[serde(flatten)]
    pub diagnostic: syu_diagnostics::Diagnostic,
}

impl ValidationRunView {
    pub fn not_run() -> Self {
        Self {
            state: ValidationRunState::NotRun,
            context: "workspace".into(),
            basis: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            evaluated_rule_count: 0,
            applicable_phase_count: 0,
            skipped_phase_count: 5,
            phases: phase_views(&[], false, false),
            diagnostics: vec![],
            reason: None,
        }
    }

    pub fn completed(
        context: impl Into<String>,
        basis: Option<String>,
        result: ValidationResult,
        has_changes: bool,
        has_plan: bool,
        started_at: SystemTime,
    ) -> Self {
        let context = context.into();
        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|diagnostic| ValidationDiagnosticView {
                phase: diagnostic_phase(&diagnostic.rule_id).into(),
                diagnostic,
            })
            .collect::<Vec<_>>();
        let phases = phase_views(&diagnostics, has_changes, has_plan);
        let applicable_phase_count = phases
            .iter()
            .filter(|p| !matches!(p.state, ValidationRunState::NotApplicable))
            .count();
        let completed_at = SystemTime::now();
        Self {
            state: if diagnostics.is_empty() {
                ValidationRunState::Passed
            } else {
                ValidationRunState::Issues
            },
            context,
            basis,
            started_at: epoch_ms(started_at),
            completed_at: epoch_ms(completed_at),
            duration_ms: completed_at
                .duration_since(started_at)
                .ok()
                .map(|d| d.as_millis() as u64),
            evaluated_rule_count: applicable_phase_count,
            applicable_phase_count,
            skipped_phase_count: phases.len() - applicable_phase_count,
            phases,
            diagnostics,
            reason: None,
        }
    }
}

fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn phase_views(
    diagnostics: &[ValidationDiagnosticView],
    has_changes: bool,
    has_plan: bool,
) -> Vec<ValidationPhaseView> {
    [
        ("config", true, None),
        ("graph", true, None),
        ("targets", true, None),
        (
            "scope",
            has_changes,
            Some("No changed-file range was selected"),
        ),
        ("plan", has_plan, Some("No work plan or slice is selected")),
    ]
    .into_iter()
    .map(|(id, applicable, reason)| {
        let issue_count = diagnostics.iter().filter(|d| d.phase == id).count();
        ValidationPhaseView {
            id: id.into(),
            state: if !applicable {
                ValidationRunState::NotApplicable
            } else if issue_count == 0 {
                ValidationRunState::Passed
            } else {
                ValidationRunState::Issues
            },
            issue_count,
            not_applicable_reason: (!applicable).then(|| reason.unwrap().into()),
        }
    })
    .collect()
}

fn diagnostic_phase(rule: &str) -> &'static str {
    let upper = rule.to_ascii_uppercase();
    if upper.contains("WORK") || upper.contains("PLAN") || upper.contains("SLICE") {
        "plan"
    } else if upper.contains("TARGET") || upper.contains("BIND") || upper.contains("CONTRACT") {
        "targets"
    } else if upper.contains("CHANGE") || upper.contains("RANGE") || upper.contains("OWN") {
        "scope"
    } else if upper.contains("SPEC") || upper.contains("ANCHOR") || upper.contains("GRAPH") {
        "graph"
    } else {
        "config"
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub root: String,
    pub revision: String,
    pub fingerprint: String,
    pub config_schema: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemSummary {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub principles: usize,
    pub rules: usize,
    pub criteria: usize,
    pub bindings: usize,
    pub contracts: usize,
}
pub fn project(
    workspace: &SpecWorkspace,
    request: Option<&WorkRequest>,
    revision: &str,
) -> Result<WorkspaceProjection> {
    let index = workspace.index()?;
    let items = index
        .item_paths
        .iter()
        .map(|(id, path)| {
            let anchors = index.item_anchors.get(id).cloned().unwrap_or_default();
            let count = |kind| anchors.iter().filter(|anchor| anchor.kind == kind).count();
            ItemSummary {
                id: id.to_string(),
                kind: if count(LocalAnchorKind::Principle) > 0 {
                    "philosophy"
                } else if count(LocalAnchorKind::Rule) > 0 {
                    "policy"
                } else if count(LocalAnchorKind::Criterion) > 0 {
                    "requirement"
                } else {
                    "feature"
                }
                .to_string(),
                path: relative_display(&workspace.root, path),
                principles: count(LocalAnchorKind::Principle),
                rules: count(LocalAnchorKind::Rule),
                criteria: count(LocalAnchorKind::Criterion),
                bindings: count(LocalAnchorKind::Binding),
                contracts: count(LocalAnchorKind::Contract),
            }
        })
        .collect();
    let plan = request
        .map(|r| plan(r, workspace, &index, revision))
        .transpose()?;
    let validation = ValidationRunView::not_run();
    Ok(WorkspaceProjection {
        workspace: WorkspaceSummary {
            root: workspace.root.display().to_string(),
            revision: revision.to_string(),
            fingerprint: workspace.fingerprint(),
            config_schema: workspace.config.schema.clone(),
        },
        config: workspace.config.clone(),
        items,
        plan,
        validation,
    })
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .or_else(|_| {
            let root = root.canonicalize()?;
            let path = path.canonicalize()?;
            path.strip_prefix(root)
                .map(Path::to_path_buf)
                .map_err(std::io::Error::other)
        })
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}
