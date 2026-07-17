#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use syu_delivery::DeliveryStore;
use syu_spec_model::BoundTargetRef;
use syu_validation::canonical_plan_for_execution;
use syu_work_model::{
    AGENT_EVENT_SCHEMA, AGENT_PATCH_SCHEMA, AGENT_RUN_SCHEMA, AgentBlocker, AgentContextPack,
    AgentEvent, AgentEventKind, AgentPatch, AgentPatchRecord, AgentPatchStatus, AgentRun,
    AgentRunStatus, AgentTargetChange, AgentTargetDigest, AgentTargetWrite, ContextPack,
    PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus, TargetAccessMode, TargetTransition,
};
use syu_workspace::SpecWorkspace;

pub const AGENT_CONTEXT_SCHEMA: &str = "syu/agent-context/v1";

pub fn start_run(
    workspace: &SpecWorkspace,
    approval: &PlanApproval,
    slice_id: &str,
) -> Result<AgentRun> {
    let index = workspace.index()?;
    let revision = repository_revision(&workspace.root)?;
    if approval.schema != PLAN_APPROVAL_SCHEMA {
        bail!("agent approval schema must be {PLAN_APPROVAL_SCHEMA}");
    }
    if approval.plan_digest != approval.plan.canonical_digest {
        bail!("agent approval does not contain its canonical plan");
    }
    if approval.revision != revision {
        bail!("agent approval revision is stale");
    }
    if approval.workspace_fingerprint != workspace.try_fingerprint()? {
        bail!("agent approval workspace basis is stale");
    }
    let plan = canonical_plan_for_execution(workspace, &index, &approval.plan, &revision)?;
    if !matches!(plan.status, PlanStatus::Ready) {
        bail!("only a ready plan can start an implementation agent");
    }
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    if slice.editable_targets.is_empty() {
        bail!("selected slice has no editable targets");
    }
    if slice.editable_targets.iter().any(|target| {
        target.access != TargetAccessMode::Editable || target.transition != TargetTransition::Modify
    }) {
        bail!("agent v1 only supports existing editable modify targets");
    }
    let context = syu_planner::export_context(&plan, slice_id, workspace, &index, &revision)?;
    let agent_context = agent_context(&plan.canonical_digest, slice_id, &context, slice)?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let run = AgentRun {
        schema: AGENT_RUN_SCHEMA.into(),
        run_id: store.new_id("agent-run"),
        approval_id: approval.approval_id.clone(),
        plan_digest: plan.canonical_digest,
        slice_id: slice_id.into(),
        status: AgentRunStatus::Active,
        context: agent_context,
        created_at: timestamp(),
    };
    let event = AgentEvent {
        schema: AGENT_EVENT_SCHEMA.into(),
        event_id: store.new_id("agent-event"),
        event_digest: String::new(),
        run_id: run.run_id.clone(),
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
        created_at: timestamp(),
        event: AgentEventKind::RunStarted {
            run: Box::new(run.clone()),
        },
    };
    store.append_agent_event(&event)?;
    Ok(run)
}

pub fn apply_scoped_patch(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    patch: &AgentPatch,
) -> Result<AgentPatchRecord> {
    let run = validate_run(workspace, run)?;
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let before_fingerprint = workspace.try_fingerprint()?;
    let now = timestamp();
    let patch_id = store.new_id("agent-patch");
    let result = apply_patch_inner(workspace, &run, patch);
    match result {
        Ok(applied) => {
            let record = AgentPatchRecord {
                schema: AGENT_PATCH_SCHEMA.into(),
                patch_id,
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                status: AgentPatchStatus::Accepted,
                writes: patch.writes.clone(),
                changes: applied.changes,
                before_workspace_fingerprint: before_fingerprint,
                after_workspace_fingerprint: applied.after_fingerprint,
                blockers: vec![],
                created_at: now,
            };
            match append_patch_event(&store, &run, record.clone()) {
                Ok(()) => Ok(record),
                Err(error) => {
                    restore_files(&applied.old_files)?;
                    Err(error)
                }
            }
        }
        Err(error) => {
            let blocker = AgentBlocker {
                code: "SYU-AGENT-PATCH-REJECTED".into(),
                message: error.to_string(),
                next_action: "Keep the approved slice unchanged, resolve the blocker, or request explicit scope expansion.".into(),
            };
            let record = AgentPatchRecord {
                schema: AGENT_PATCH_SCHEMA.into(),
                patch_id,
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                status: AgentPatchStatus::Rejected,
                writes: patch.writes.clone(),
                changes: vec![],
                before_workspace_fingerprint: before_fingerprint,
                after_workspace_fingerprint: String::new(),
                blockers: vec![blocker.clone()],
                created_at: now,
            };
            append_patch_event(&store, &run, record)?;
            Err(error)
        }
    }
}

pub fn record_blocker(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    blocker: AgentBlocker,
) -> Result<AgentEvent> {
    let run = validate_run(workspace, run)?;
    if !matches!(run.status, AgentRunStatus::Active) {
        bail!("agent run is not active; resolve its blocker or start a new run");
    }
    if blocker.code.trim().is_empty()
        || blocker.message.trim().is_empty()
        || blocker.next_action.trim().is_empty()
    {
        bail!("agent blockers require a code, message, and next action");
    }
    append_event(workspace, &run, AgentEventKind::BlockerRecorded { blocker })
}

pub fn request_scope_expansion(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    reason: String,
    requested_targets: Vec<BoundTargetRef>,
) -> Result<AgentEvent> {
    let run = validate_run(workspace, run)?;
    if !matches!(run.status, AgentRunStatus::Active | AgentRunStatus::Blocked) {
        bail!("agent run is completed; start a new run to request scope expansion");
    }
    if reason.trim().is_empty() || requested_targets.is_empty() {
        bail!("scope expansion requires a reason and at least one target");
    }
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let request = syu_work_model::ScopeExpansionRequest {
        request_id: store.new_id("scope-expansion"),
        run_id: run.run_id.clone(),
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
        reason,
        requested_targets,
        created_at: timestamp(),
    };
    append_event(
        workspace,
        &run,
        AgentEventKind::ScopeExpansionRequested { request },
    )
}

pub fn record_verification(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    attempt_id: &str,
) -> Result<AgentEvent> {
    let run = validate_run(workspace, run)?;
    if !matches!(run.status, AgentRunStatus::Active) {
        bail!("agent run is not active; resolve its blocker or start a new run");
    }
    if attempt_id.trim().is_empty() {
        bail!("verification evidence requires an attempt id");
    }
    append_event(
        workspace,
        &run,
        AgentEventKind::VerificationRecorded {
            attempt_id: attempt_id.into(),
        },
    )
}

pub fn events(workspace: &SpecWorkspace, run_id: &str) -> Result<Vec<AgentEvent>> {
    DeliveryStore::for_workspace(&workspace.root)?.agent_events(run_id)
}

pub fn current_run(workspace: &SpecWorkspace, run: &AgentRun) -> Result<AgentRun> {
    validate_run(workspace, run)
}

fn apply_patch_inner(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    patch: &AgentPatch,
) -> Result<PatchApplied> {
    if patch.schema != AGENT_PATCH_SCHEMA {
        bail!("patch schema must be {AGENT_PATCH_SCHEMA}");
    }
    if patch.run_id != run.run_id {
        bail!("patch run id does not match the active agent run");
    }
    if !matches!(run.status, AgentRunStatus::Active) {
        bail!("agent run is not active; resolve its blocker or start a new run");
    }
    if patch.writes.is_empty() {
        bail!("patch must contain at least one target write");
    }
    if run.context.context.basis.workspace_fingerprint != patch.expected_workspace_fingerprint {
        bail!("patch workspace basis is stale");
    }
    let plan = approved_plan(workspace, run)?;
    let index = workspace.index()?;
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == run.slice_id)
        .ok_or_else(|| anyhow::anyhow!("agent slice is absent from its approved plan"))?;
    let mut replacements: BTreeMap<PathBuf, Vec<Replacement>> = BTreeMap::new();
    for write in &patch.writes {
        let AgentTargetWrite::Replace {
            target,
            expected_excerpt_hash,
            content,
        } = write;
        let planned = slice
            .editable_targets
            .iter()
            .find(|candidate| candidate.reference == *target)
            .ok_or_else(|| {
                anyhow::anyhow!("target {target} is outside the selected editable slice")
            })?;
        if planned.access != TargetAccessMode::Editable
            || planned.transition != TargetTransition::Modify
        {
            bail!("target {target} is not an editable modify target");
        }
        let declared = index.target(target).ok_or_else(|| {
            anyhow::anyhow!("target {target} is not present in the current inventory")
        })?;
        let resolved = syu_workspace::resolve_target_in_workspace(workspace, declared)?;
        if resolved.content_hash != planned.content_hash
            || resolved.excerpt_hash != planned.excerpt_hash
        {
            bail!("target {target} is stale; refresh the plan before writing");
        }
        if resolved.excerpt_hash != *expected_excerpt_hash {
            bail!("target {target} excerpt digest does not match the current workspace");
        }
        let added_bytes = content.len().saturating_sub(resolved.excerpt.len());
        if planned.budget_bytes > 0 && added_bytes > planned.budget_bytes {
            bail!("target {target} exceeds its added-byte budget");
        }
        if let Some(limit) = planned.budget_lines {
            let old_lines = resolved.excerpt.lines().count();
            let new_lines = content.lines().count();
            if new_lines.saturating_sub(old_lines) > limit {
                bail!("target {target} exceeds its added-line budget");
            }
        }
        let path = workspace.root.join(&resolved.path);
        replacements.entry(path).or_default().push(Replacement {
            start: resolved.byte_start,
            end: resolved.byte_end,
            old: resolved.excerpt,
            new: content.clone(),
            target: target.clone(),
        });
    }
    let mut files = Vec::new();
    for (path, mut changes) in replacements {
        changes.sort_by_key(|change| std::cmp::Reverse(change.start));
        for pair in changes.windows(2) {
            if pair[0].start < pair[1].end {
                bail!("patch contains overlapping writes in {}", path.display());
            }
        }
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let mut updated = original.clone();
        for change in &changes {
            let current = updated
                .get(change.start..change.end)
                .ok_or_else(|| anyhow::anyhow!("target {} range is stale", change.target))?;
            if current != change.old {
                bail!(
                    "target {} changed while the patch was being prepared",
                    change.target
                );
            }
            updated.replace_range(change.start..change.end, &change.new);
        }
        files.push((path, original.into_bytes(), updated.into_bytes()));
    }
    let old_files = write_files_atomically(&files)?;
    let post_write = (|| {
        let candidate = SpecWorkspace::load(&workspace.root)?;
        let candidate_index = candidate.index()?;
        validate_post_patch(&index, &candidate, &candidate_index, slice, patch)?;
        candidate.try_fingerprint()
    })();
    let after_fingerprint = match post_write {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            restore_files(&old_files)?;
            return Err(error);
        }
    };
    let changes = patch
        .writes
        .iter()
        .map(|write| match write {
            AgentTargetWrite::Replace {
                target,
                expected_excerpt_hash,
                content,
            } => AgentTargetChange {
                reference: target.clone(),
                before_excerpt_hash: expected_excerpt_hash.clone(),
                after_excerpt_hash: hash_bytes(content.as_bytes()),
            },
        })
        .collect();
    Ok(PatchApplied {
        after_fingerprint,
        changes,
        old_files,
    })
}

fn approved_plan(workspace: &SpecWorkspace, run: &AgentRun) -> Result<syu_work_model::WorkPlan> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let approval = store.approval(&run.plan_digest)?;
    if approval.schema != PLAN_APPROVAL_SCHEMA {
        bail!("stored agent approval schema must be {PLAN_APPROVAL_SCHEMA}");
    }
    if approval.approval_id != run.approval_id || approval.plan_digest != run.plan_digest {
        bail!("agent run is not tied to its approved plan");
    }
    let index = workspace.index()?;
    let revision = repository_revision(&workspace.root)?;
    let canonical = canonical_plan_for_execution(workspace, &index, &approval.plan, &revision)?;
    if canonical != approval.plan {
        bail!("approved agent plan is stale; review a new plan before writing");
    }
    Ok(canonical)
}

fn validate_run(workspace: &SpecWorkspace, run: &AgentRun) -> Result<AgentRun> {
    if run.schema != AGENT_RUN_SCHEMA {
        bail!("agent run schema must be {AGENT_RUN_SCHEMA}");
    }
    if run.context.schema != AGENT_CONTEXT_SCHEMA
        || run.context.plan_digest != run.plan_digest
        || run.context.slice_id != run.slice_id
    {
        bail!("agent run context does not match its plan and slice");
    }
    let stored = DeliveryStore::for_workspace(&workspace.root)?.agent_run(&run.run_id)?;
    let mut expected = run.clone();
    expected.status = stored.status.clone();
    if stored != expected {
        bail!("agent run does not match its persisted event history");
    }
    Ok(stored)
}

fn validate_post_patch(
    before: &syu_workspace::SpecIndex,
    candidate: &SpecWorkspace,
    after: &syu_workspace::SpecIndex,
    slice: &syu_work_model::ExecutionSlice,
    patch: &AgentPatch,
) -> Result<()> {
    let mut allowed_identities = std::collections::BTreeSet::new();
    for write in &patch.writes {
        let AgentTargetWrite::Replace { target, .. } = write;
        let planned = slice
            .editable_targets
            .iter()
            .find(|planned| planned.reference == *target)
            .ok_or_else(|| {
                anyhow::anyhow!("target {target} is outside the selected editable slice")
            })?;
        let before_target = before
            .target(target)
            .ok_or_else(|| anyhow::anyhow!("target {target} disappeared from the inventory"))?;
        let after_target = after
            .target(target)
            .ok_or_else(|| anyhow::anyhow!("target {target} is absent after applying the patch"))?;
        let before_resolved = syu_workspace::resolve_target_in_workspace(candidate, after_target)?;
        if before_resolved.content_hash == planned.content_hash {
            bail!("target {target} was not modified by the patch");
        }
        let before_identity = before.target_to_artifact.get(target).ok_or_else(|| {
            anyhow::anyhow!("target {target} has no inventory identity before the patch")
        })?;
        let after_identity = after.target_to_artifact.get(target).ok_or_else(|| {
            anyhow::anyhow!("target {target} no longer resolves to an inventory identity")
        })?;
        if before_identity != after_identity {
            bail!("target {target} no longer resolves to the same inventory identity");
        }
        allowed_identities.insert(before_identity.clone());
        for unit in &before.artifact_units {
            if unit.path == before_target.path
                && matches!(unit.kind, syu_inventory::ArtifactUnitKind::File)
            {
                allowed_identities.insert(unit.identity.clone());
            }
        }
        for unit in &after.artifact_units {
            if unit.path == after_target.path
                && matches!(unit.kind, syu_inventory::ArtifactUnitKind::File)
            {
                allowed_identities.insert(unit.identity.clone());
            }
        }
    }
    let before_units = before
        .artifact_units
        .iter()
        .map(|unit| (unit.identity.clone(), unit))
        .collect::<BTreeMap<_, _>>();
    let after_units = after
        .artifact_units
        .iter()
        .map(|unit| (unit.identity.clone(), unit))
        .collect::<BTreeMap<_, _>>();
    for identity in before_units.keys().chain(after_units.keys()) {
        if allowed_identities.contains(identity) {
            continue;
        }
        if before_units.get(identity) != after_units.get(identity) {
            bail!("patch added, removed, or changed unapproved inventory unit {identity}");
        }
    }
    Ok(())
}

fn agent_context(
    plan_digest: &str,
    slice_id: &str,
    context: &ContextPack,
    slice: &syu_work_model::ExecutionSlice,
) -> Result<AgentContextPack> {
    let targets = |targets: &[syu_work_model::PlannedTarget]| {
        targets
            .iter()
            .map(|target| AgentTargetDigest {
                reference: target.reference.clone(),
                path: target.resolved_path.clone(),
                access: target.access,
                transition: target.transition,
                content_hash: target.content_hash.clone(),
                excerpt_hash: target.excerpt_hash.clone(),
                line_start: target.line_start,
                line_end: target.line_end,
                budget_bytes: target.budget_bytes,
                budget_lines: target.budget_lines,
            })
            .collect()
    };
    Ok(AgentContextPack {
        schema: AGENT_CONTEXT_SCHEMA.into(),
        plan_digest: plan_digest.into(),
        slice_id: slice_id.into(),
        context: context.clone(),
        budget: slice.budget.clone(),
        editable_targets: targets(&slice.editable_targets),
        verification_targets: targets(&slice.verification_targets),
        readonly_targets: targets(&slice.readonly_context),
    })
}

#[derive(Debug)]
struct Replacement {
    start: usize,
    end: usize,
    old: String,
    new: String,
    target: BoundTargetRef,
}

struct PatchApplied {
    after_fingerprint: String,
    changes: Vec<AgentTargetChange>,
    old_files: Vec<(PathBuf, Vec<u8>)>,
}

fn write_files_atomically(
    files: &[(PathBuf, Vec<u8>, Vec<u8>)],
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut temporary = Vec::new();
    for (path, original, content) in files {
        let parent = path.parent().context("target path has no parent")?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        std::io::Write::write_all(&mut file, content)?;
        file.as_file().sync_all()?;
        temporary.push((path.clone(), original.clone(), file));
    }
    let mut applied = Vec::new();
    for (path, expected, file) in temporary {
        let old = fs::read(&path)?;
        if old != expected {
            let temp_path = file.into_temp_path();
            let _ = temp_path.close();
            restore_files(&applied)?;
            bail!(
                "target {} changed while the patch was being applied",
                path.display()
            );
        }
        let temp_path = file.into_temp_path();
        if let Err(error) = fs::rename(&temp_path, &path) {
            for (old_path, old) in applied {
                let _ = fs::write(old_path, old);
            }
            let _ = temp_path.close();
            return Err(error.into());
        }
        applied.push((path, old));
    }
    Ok(applied)
}

fn restore_files(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    for (path, content) in files {
        fs::write(path, content)?;
    }
    Ok(())
}

fn append_patch_event(
    store: &DeliveryStore,
    run: &AgentRun,
    patch: AgentPatchRecord,
) -> Result<()> {
    let event = AgentEvent {
        schema: AGENT_EVENT_SCHEMA.into(),
        event_id: store.new_id("agent-event"),
        event_digest: String::new(),
        run_id: run.run_id.clone(),
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
        created_at: timestamp(),
        event: AgentEventKind::PatchRecorded { patch },
    };
    store.append_agent_event(&event).map(|_| ())
}

fn append_event(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    event: AgentEventKind,
) -> Result<AgentEvent> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let value = AgentEvent {
        schema: AGENT_EVENT_SCHEMA.into(),
        event_id: store.new_id("agent-event"),
        event_digest: String::new(),
        run_id: run.run_id.clone(),
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
        created_at: timestamp(),
        event,
    };
    store.append_agent_event(&value)
}

fn repository_revision(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        bail!("resolve workspace revision");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn hash_bytes(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(value);
    format!("sha256:{:x}", hash.finalize())
}
