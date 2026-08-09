#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use syu_delivery::DeliveryStore;
use syu_inventory::ArtifactUnit;
use syu_spec_model::{BoundTargetRef, format_sha256};
use syu_validation::canonical_plan_for_execution;
use syu_work_model::{
    AGENT_PATCH_SCHEMA, AGENT_RUN_SCHEMA, AgentBlocker, AgentContextPack, AgentEvent,
    AgentEventKind, AgentPatch, AgentPatchRecord, AgentRun, AgentRunStatus, AgentTargetChange,
    AgentTargetWrite, ExecutionIdentity, PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus,
    TargetAccessMode, TargetLifecycle, TargetTransition,
};
use syu_workspace::SpecWorkspace;

pub const AGENT_CONTEXT_SCHEMA: &str = syu_work_model::AGENT_CONTEXT_SCHEMA;
static PATCH_WORKSPACE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn start_run(
    workspace: &SpecWorkspace,
    approval: &PlanApproval,
    slice_id: &str,
    retry: bool,
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
        target.access != TargetAccessMode::Editable
            || !matches!(
                target.transition,
                TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove
            )
    }) {
        bail!("agent only supports editable Add, Modify, and Remove targets");
    }
    let context = syu_planner::export_context(&plan, slice_id, workspace, &index, &revision)?;
    let agent_context = AgentContextPack::from_slice(&plan.canonical_digest, context, slice);
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
    store.start_agent_run(&run, retry)?;
    Ok(run)
}

pub fn apply_scoped_patch(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    patch: &AgentPatch,
) -> Result<AgentPatchRecord> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let _workspace_lock = store.lock_workspace()?;
    let _workspace_guard = PATCH_WORKSPACE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow::anyhow!("agent workspace mutation lock"))?;
    let fresh_workspace = SpecWorkspace::load(&workspace.root)?;
    let workspace = &fresh_workspace;
    let run = validate_run(workspace, run)?;
    let before_fingerprint = workspace.try_fingerprint()?;
    let patch_id = store.new_id("agent-patch");
    let result = apply_patch_inner(workspace, &run, patch, &store, &patch_id);
    match result {
        Ok(applied) => {
            let applied_workspace = SpecWorkspace::load(&workspace.root)?;
            let record = match store.record_agent_patch_after_apply_while_locked(
                &applied_workspace,
                &run,
                patch,
                &patch_id,
                &before_fingerprint,
            ) {
                Ok(record) => record,
                Err(error) => {
                    let restored = restore_rollback(&applied.rollback);
                    if let Err(restore) = restored {
                        return Err(anyhow::anyhow!(
                            "patch event append failed: {error}; rollback failed: {restore}"
                        ));
                    }
                    store.clear_mutation_journal()?;
                    return Err(error);
                }
            };
            {
                store.clear_mutation_journal()?;
                Ok(record)
            }
        }
        Err(error) => {
            let blocker = AgentBlocker {
                code: "SYU-AGENT-PATCH-REJECTED".into(),
                message: error.to_string(),
                next_action: "Keep the approved slice unchanged, resolve the blocker, or request explicit scope expansion.".into(),
            };
            store.record_rejected_agent_patch_while_locked(
                workspace,
                &run,
                patch,
                &patch_id,
                &before_fingerprint,
                blocker,
            )?;
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
    record_verification_inner(workspace, run, attempt_id, false)
}

pub fn record_verification_while_locked(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    attempt_id: &str,
) -> Result<AgentEvent> {
    record_verification_inner(workspace, run, attempt_id, true)
}

fn record_verification_inner(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    attempt_id: &str,
    workspace_locked: bool,
) -> Result<AgentEvent> {
    let run = validate_run(workspace, run)?;
    if !matches!(run.status, AgentRunStatus::Active) {
        bail!("agent run is not active; resolve its blocker or start a new run");
    }
    if attempt_id.trim().is_empty() {
        bail!("verification evidence requires an attempt id");
    }
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    if workspace_locked {
        store.record_agent_verification_while_locked(&run, attempt_id)
    } else {
        let _workspace_lock = store.lock_workspace()?;
        store.record_agent_verification_while_locked(&run, attempt_id)
    }
}

pub fn events(workspace: &SpecWorkspace, run: &AgentRun) -> Result<Vec<AgentEvent>> {
    let identity = ExecutionIdentity {
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
    };
    DeliveryStore::for_workspace(&workspace.root)?.agent_events(&identity, &run.run_id)
}

pub fn current_run(workspace: &SpecWorkspace, run: &AgentRun) -> Result<AgentRun> {
    validate_run(workspace, run)
}

fn apply_patch_inner(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    patch: &AgentPatch,
    store: &DeliveryStore,
    patch_id: &str,
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
    let mut appends: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    let mut approved_containers: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    let mut created_files: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    let mut removed_files: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    let mut seen_targets = std::collections::BTreeSet::new();
    for write in &patch.writes {
        let target = write_target(write);
        if !seen_targets.insert(target.clone()) {
            bail!("patch contains more than one write for target {target}");
        }
        let planned = slice
            .editable_targets
            .iter()
            .find(|candidate| candidate.reference == *target)
            .ok_or_else(|| {
                anyhow::anyhow!("target {target} is outside the selected editable slice")
            })?;
        if planned.access != TargetAccessMode::Editable {
            bail!("target {target} is not editable");
        }
        match write {
            AgentTargetWrite::Replace {
                expected_excerpt_hash,
                content,
                ..
            } => {
                ensure_transition(target, planned, TargetTransition::Modify, false)?;
                let resolved = current_target(workspace, &index, target)?;
                ensure_current_snapshot(target, planned, &resolved)?;
                if resolved.excerpt_hash != *expected_excerpt_hash {
                    bail!("target {target} excerpt digest does not match the current workspace");
                }
                ensure_replacement_budget(target, planned, &resolved.excerpt, content)?;
                let path = checked_target_path(workspace, planned)?;
                replacements.entry(path).or_default().push(Replacement {
                    start: resolved.byte_start,
                    end: resolved.byte_end,
                    old: resolved.excerpt,
                    new: content.clone(),
                    target: target.clone(),
                });
            }
            AgentTargetWrite::AddToFile {
                expected_path_hash,
                content,
                ..
            } => {
                ensure_transition(target, planned, TargetTransition::Add, false)?;
                ensure_target_absent(workspace, &index, target)?;
                let approved = planned.container_content_hash.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "target {target} has no approved existing-file digest; create it as a new file instead"
                    )
                })?;
                if approved != expected_path_hash {
                    bail!("target {target} insertion digest does not match the approved plan");
                }
                let path = checked_target_path(workspace, planned)?;
                let current = required_file_bytes(&path)?;
                if hash_bytes(&current) != *approved {
                    bail!(
                        "target {target} containing file is stale; refresh the plan before writing"
                    );
                }
                if let Some(previous) = approved_containers.get(&path)
                    && previous != &current
                {
                    bail!(
                        "target {target} containing file was read more than once with different bytes"
                    );
                }
                approved_containers.entry(path.clone()).or_insert(current);
                ensure_added_budget(target, planned, content)?;
                appends.entry(path).or_default().push(content.clone());
            }
            AgentTargetWrite::CreateFile { content, .. } => {
                ensure_transition(target, planned, TargetTransition::Add, true)?;
                ensure_target_absent(workspace, &index, target)?;
                if planned.container_content_hash.is_some() {
                    bail!("target {target} is approved for insertion into an existing file");
                }
                let path = checked_target_path(workspace, planned)?;
                if file_state(&path)?.is_some() {
                    bail!("target {target} now exists; refresh the plan before creating it");
                }
                ensure_added_budget(target, planned, content)?;
                if created_files
                    .insert(path, content.as_bytes().to_vec())
                    .is_some()
                {
                    bail!("patch creates the same file more than once");
                }
            }
            AgentTargetWrite::Remove {
                expected_excerpt_hash,
                ..
            } => {
                ensure_transition(target, planned, TargetTransition::Remove, false)?;
                let resolved = current_target(workspace, &index, target)?;
                ensure_current_snapshot(target, planned, &resolved)?;
                if resolved.excerpt_hash != *expected_excerpt_hash {
                    bail!("target {target} excerpt digest does not match the current workspace");
                }
                let path = checked_target_path(workspace, planned)?;
                replacements.entry(path).or_default().push(Replacement {
                    start: resolved.byte_start,
                    end: resolved.byte_end,
                    old: resolved.excerpt,
                    new: String::new(),
                    target: target.clone(),
                });
            }
            AgentTargetWrite::RemoveFile {
                expected_content_hash,
                ..
            } => {
                ensure_transition(target, planned, TargetTransition::Remove, true)?;
                let resolved = current_target(workspace, &index, target)?;
                ensure_current_snapshot(target, planned, &resolved)?;
                if resolved.content_hash != *expected_content_hash {
                    bail!("target {target} content digest does not match the current workspace");
                }
                let path = checked_target_path(workspace, planned)?;
                let bytes = required_file_bytes(&path)?;
                removed_files.insert(path, bytes);
            }
        }
    }
    for path in created_files.keys().chain(removed_files.keys()) {
        if replacements.contains_key(path) || appends.contains_key(path) {
            bail!(
                "patch combines a file lifecycle operation with another write in {}",
                path.display()
            );
        }
    }
    let mut files = BTreeMap::new();
    for (path, mut changes) in replacements {
        changes.sort_by_key(|change| std::cmp::Reverse(change.start));
        for pair in changes.windows(2) {
            if pair[0].start < pair[1].end {
                bail!("patch contains overlapping writes in {}", path.display());
            }
        }
        let original_bytes = approved_containers
            .remove(&path)
            .unwrap_or(fs::read(&path).with_context(|| format!("read {}", path.display()))?);
        let original = String::from_utf8(original_bytes.clone())
            .with_context(|| format!("decode {}", path.display()))?;
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
        for addition in appends.remove(&path).unwrap_or_default() {
            updated.push_str(&addition);
        }
        files.insert(
            path,
            FileMutation {
                original: Some(original_bytes),
                updated: Some(updated.into_bytes()),
            },
        );
    }
    for (path, additions) in appends {
        let original = approved_containers.remove(&path).ok_or_else(|| {
            anyhow::anyhow!(
                "missing the exact approved container bytes for {}",
                path.display()
            )
        })?;
        let mut updated = String::from_utf8(original.clone())?;
        for addition in additions {
            updated.push_str(&addition);
        }
        files.insert(
            path,
            FileMutation {
                original: Some(original),
                updated: Some(updated.into_bytes()),
            },
        );
    }
    for (path, content) in created_files {
        files.insert(
            path,
            FileMutation {
                original: None,
                updated: Some(content),
            },
        );
    }
    for (path, original) in removed_files {
        files.insert(
            path,
            FileMutation {
                original: Some(original),
                updated: None,
            },
        );
    }
    let journal_files = files
        .iter()
        .map(|(path, mutation)| syu_delivery::MutationJournalFile {
            path: path.to_string_lossy().into_owned(),
            original: mutation.original.clone(),
        })
        .collect::<Vec<_>>();
    let created_dirs = files
        .iter()
        .filter_map(|(path, mutation)| mutation.updated.as_ref().map(|_| path.clone()))
        .flat_map(|path| missing_parent_dirs(&path))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    store.write_mutation_journal("agent-patch", patch_id, journal_files, created_dirs)?;
    let rollback = apply_file_mutations(&files)?;
    let post_write = (|| {
        let candidate = SpecWorkspace::load(&workspace.root)?;
        let candidate_index = candidate.index()?;
        if let Some(error) = &candidate_index.inventory_error {
            bail!("patch produced an invalid inventory: {error}");
        }
        validate_post_patch(&index, &candidate, &candidate_index, slice, patch)?;
        Ok(())
    })();
    match post_write {
        Ok(applied) => applied,
        Err(error) => {
            let restored = restore_rollback(&rollback);
            if let Err(restore) = restored {
                return Err(anyhow::anyhow!(
                    "patch post-write validation failed: {error}; rollback failed: {restore}"
                ));
            }
            store.clear_mutation_journal()?;
            return Err(error);
        }
    };
    Ok(PatchApplied { rollback })
}

fn missing_parent_dirs(path: &Path) -> Vec<PathBuf> {
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let mut missing = Vec::new();
    let mut cursor = parent.to_path_buf();
    while !cursor.exists() {
        missing.push(cursor.clone());
        if !cursor.pop() {
            break;
        }
    }
    missing.reverse();
    missing
}

fn approved_plan(workspace: &SpecWorkspace, run: &AgentRun) -> Result<syu_work_model::WorkPlan> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    let identity = ExecutionIdentity {
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
    };
    let approval = store.approval(&identity)?;
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
    let identity = ExecutionIdentity {
        plan_digest: run.plan_digest.clone(),
        slice_id: run.slice_id.clone(),
    };
    let stored =
        DeliveryStore::for_workspace(&workspace.root)?.agent_run(&identity, &run.run_id)?;
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
) -> Result<Vec<AgentTargetChange>> {
    let mut allowed_identities = std::collections::BTreeSet::new();
    let mut proofs = Vec::new();
    for write in &patch.writes {
        let target = write_target(write);
        let planned = slice
            .editable_targets
            .iter()
            .find(|planned| planned.reference == *target)
            .ok_or_else(|| {
                anyhow::anyhow!("target {target} is outside the selected editable slice")
            })?;
        let before_identity = before
            .target_to_artifact
            .get(target)
            .or_else(|| before.all_target_to_artifact.get(target));
        let after_identity = after.target_to_artifact.get(target);
        let after_resolved = after.target(target).and_then(|declared| {
            syu_workspace::resolve_target_in_workspace(candidate, declared).ok()
        });
        match planned.transition {
            TargetTransition::Modify => {
                if let (Some(before_identity), Some(after_identity)) =
                    (before_identity, after_identity)
                    && before_identity != after_identity
                {
                    bail!("target {target} no longer resolves to the same inventory identity");
                }
                let resolved = after_resolved.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("target {target} no longer resolves after applying the patch")
                })?;
                if resolved.content_hash == planned.content_hash {
                    bail!("target {target} was not modified by the patch");
                }
            }
            TargetTransition::Add => {
                if before_identity.is_some() {
                    bail!("target {target} already existed before the patch");
                }
                let resolved = after_resolved.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("target {target} was not created by the patch")
                })?;
                if let Some(content) = patch.writes.iter().find_map(|write| match write {
                    AgentTargetWrite::AddToFile {
                        target: candidate,
                        content,
                        ..
                    } if candidate == target => Some(content),
                    _ => None,
                }) && content.trim() != resolved.excerpt.trim()
                {
                    bail!(
                        "target {target} insertion content does not exactly match the approved target"
                    );
                }
            }
            TargetTransition::Remove => {
                if after_resolved.is_some() {
                    bail!("target {target} remains after the patch");
                }
            }
            TargetTransition::RunOnly | TargetTransition::Readonly => {
                bail!("target {target} is not an editable lifecycle transition");
            }
        }
        if is_file_target(planned) {
            allow_path_identities(&mut allowed_identities, before, &planned.resolved_path);
            allow_path_identities(&mut allowed_identities, after, &planned.resolved_path);
        } else {
            if let Some(identity) = before_identity {
                allowed_identities.insert(identity.clone());
            }
            if let Some(identity) = after_identity {
                allowed_identities.insert(identity.clone());
            }
            allow_file_identity(&mut allowed_identities, before, &planned.resolved_path);
            allow_file_identity(&mut allowed_identities, after, &planned.resolved_path);
        }
        proofs.push(lifecycle_proof(candidate, after, planned)?);
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
        if !same_artifact_unit_semantics(before_units.get(identity), after_units.get(identity)) {
            bail!("patch added, removed, or changed unapproved inventory unit {identity}");
        }
    }
    Ok(proofs)
}

fn same_artifact_unit_semantics(
    before: Option<&&ArtifactUnit>,
    after: Option<&&ArtifactUnit>,
) -> bool {
    before.map(|unit| {
        (
            &unit.adapter,
            &unit.path,
            &unit.identity,
            &unit.kind,
            &unit.exposure,
            &unit.reachability,
            &unit.digest,
            &unit.structural_digest,
        )
    }) == after.map(|unit| {
        (
            &unit.adapter,
            &unit.path,
            &unit.identity,
            &unit.kind,
            &unit.exposure,
            &unit.reachability,
            &unit.digest,
            &unit.structural_digest,
        )
    })
}

fn write_target(write: &AgentTargetWrite) -> &BoundTargetRef {
    match write {
        AgentTargetWrite::Replace { target, .. }
        | AgentTargetWrite::AddToFile { target, .. }
        | AgentTargetWrite::CreateFile { target, .. }
        | AgentTargetWrite::Remove { target, .. }
        | AgentTargetWrite::RemoveFile { target, .. } => target,
    }
}

fn ensure_transition(
    target: &BoundTargetRef,
    planned: &syu_work_model::PlannedTarget,
    transition: TargetTransition,
    requires_file: bool,
) -> Result<()> {
    if planned.transition != transition
        || (transition == TargetTransition::Add
            && planned.lifecycle != TargetLifecycle::EnsurePresent)
        || (transition == TargetTransition::Remove
            && planned.lifecycle != TargetLifecycle::EnsureAbsent)
        || (transition == TargetTransition::Modify && planned.lifecycle != TargetLifecycle::Stable)
    {
        bail!("target {target} does not permit this lifecycle write");
    }
    if is_file_target(planned) != requires_file {
        bail!("target {target} requires a different scoped file operation");
    }
    Ok(())
}

fn is_file_target(target: &syu_work_model::PlannedTarget) -> bool {
    target.resolved_selector.description == "file" && target.resolved_selector.symbols.is_empty()
}

fn current_target(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    target: &BoundTargetRef,
) -> Result<syu_workspace::ResolvedTarget> {
    let declared = index.target(target).ok_or_else(|| {
        anyhow::anyhow!("target {target} is not present in the current inventory")
    })?;
    syu_workspace::resolve_target_in_workspace(workspace, declared)
        .with_context(|| format!("target {target} is absent or cannot be resolved"))
}

fn ensure_target_absent(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    target: &BoundTargetRef,
) -> Result<()> {
    if index.target_to_artifact.contains_key(target) {
        bail!("target {target} now exists; refresh the plan before writing");
    }
    let declared = index.target(target).ok_or_else(|| {
        anyhow::anyhow!("target {target} is not present in the approved inventory")
    })?;
    if syu_workspace::resolve_target_in_workspace(workspace, declared).is_ok() {
        bail!("target {target} now exists; refresh the plan before writing");
    }
    Ok(())
}

fn ensure_current_snapshot(
    target: &BoundTargetRef,
    planned: &syu_work_model::PlannedTarget,
    resolved: &syu_workspace::ResolvedTarget,
) -> Result<()> {
    if resolved.content_hash != planned.content_hash
        || resolved.excerpt_hash != planned.excerpt_hash
    {
        bail!("target {target} is stale; refresh the plan before writing");
    }
    Ok(())
}

fn ensure_replacement_budget(
    target: &BoundTargetRef,
    planned: &syu_work_model::PlannedTarget,
    old: &str,
    content: &str,
) -> Result<()> {
    let added_bytes = content.len().saturating_sub(old.len());
    if planned.budget_bytes > 0 && added_bytes > planned.budget_bytes {
        bail!("target {target} exceeds its added-byte budget");
    }
    if let Some(limit) = planned.budget_lines
        && content.lines().count().saturating_sub(old.lines().count()) > limit
    {
        bail!("target {target} exceeds its added-line budget");
    }
    Ok(())
}

fn ensure_added_budget(
    target: &BoundTargetRef,
    planned: &syu_work_model::PlannedTarget,
    content: &str,
) -> Result<()> {
    if content.len() > planned.budget_bytes {
        bail!("target {target} exceeds its added-byte budget");
    }
    if let Some(limit) = planned.budget_lines
        && content.lines().count() > limit
    {
        bail!("target {target} exceeds its added-line budget");
    }
    Ok(())
}

fn checked_target_path(
    workspace: &SpecWorkspace,
    planned: &syu_work_model::PlannedTarget,
) -> Result<PathBuf> {
    let root = workspace.root.canonicalize()?;
    let path = workspace.root.join(&planned.resolved_path);
    let ancestor = path
        .ancestors()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow::anyhow!("target path has no existing workspace ancestor"))?;
    if !ancestor.canonicalize()?.starts_with(&root) {
        bail!("target path escapes the workspace through a symlink");
    }
    Ok(path)
}

fn file_state(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                bail!("target path is not a regular file: {}", path.display());
            }
            Ok(Some(fs::read(path)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn required_file_bytes(path: &Path) -> Result<Vec<u8>> {
    file_state(path)?.ok_or_else(|| anyhow::anyhow!("target file is missing: {}", path.display()))
}

fn allow_file_identity(
    allowed: &mut std::collections::BTreeSet<String>,
    index: &syu_workspace::SpecIndex,
    path: &str,
) {
    for unit in &index.artifact_units {
        if unit.path.to_string_lossy() == path
            && matches!(unit.kind, syu_inventory::ArtifactUnitKind::File)
        {
            allowed.insert(unit.identity.clone());
        }
    }
}

fn allow_path_identities(
    allowed: &mut std::collections::BTreeSet<String>,
    index: &syu_workspace::SpecIndex,
    path: &str,
) {
    allowed.extend(
        index
            .artifact_units
            .iter()
            .filter(|unit| unit.path.to_string_lossy() == path)
            .map(|unit| unit.identity.clone()),
    );
}

fn lifecycle_proof(
    workspace: &SpecWorkspace,
    index: &syu_workspace::SpecIndex,
    planned: &syu_work_model::PlannedTarget,
) -> Result<AgentTargetChange> {
    let current = index
        .target(&planned.reference)
        .and_then(|declared| syu_workspace::resolve_target_in_workspace(workspace, declared).ok());
    match (planned.lifecycle, current) {
        (TargetLifecycle::EnsureAbsent, None) => Ok(AgentTargetChange {
            reference: planned.reference.clone(),
            transition: planned.transition,
            lifecycle: planned.lifecycle,
            before_content_hash: planned.content_hash.clone(),
            after_content_hash: String::new(),
            before_excerpt_hash: planned.excerpt_hash.clone(),
            after_excerpt_hash: String::new(),
        }),
        (TargetLifecycle::EnsureAbsent, Some(_)) => {
            bail!("target {} remains after the patch", planned.reference)
        }
        (_, Some(resolved)) => Ok(AgentTargetChange {
            reference: planned.reference.clone(),
            transition: planned.transition,
            lifecycle: planned.lifecycle,
            before_content_hash: planned.content_hash.clone(),
            after_content_hash: resolved.content_hash,
            before_excerpt_hash: planned.excerpt_hash.clone(),
            after_excerpt_hash: resolved.excerpt_hash,
        }),
        (_, None) => bail!("target {} is absent after the patch", planned.reference),
    }
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
    rollback: PatchRollback,
}

struct FileMutation {
    original: Option<Vec<u8>>,
    updated: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct FileRollback {
    path: PathBuf,
    original: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
}

struct PatchRollback {
    files: Vec<FileRollback>,
    created_dirs: Vec<PathBuf>,
}

fn apply_file_mutations(files: &BTreeMap<PathBuf, FileMutation>) -> Result<PatchRollback> {
    let mut original_permissions = BTreeMap::new();
    for (path, mutation) in files {
        if file_state(path)? != mutation.original {
            bail!(
                "target {} changed while the patch was being prepared",
                path.display()
            );
        }
        original_permissions.insert(
            path.clone(),
            mutation
                .original
                .as_ref()
                .map(|_| fs::metadata(path).map(|metadata| metadata.permissions()))
                .transpose()?,
        );
    }
    let mut temporary = Vec::new();
    let mut created_dirs = Vec::new();
    for (path, mutation) in files {
        let Some(content) = &mutation.updated else {
            continue;
        };
        let parent = match path.parent().context("target path has no parent") {
            Ok(parent) => parent,
            Err(error) => return Err(cleanup_preparation_error(error, &created_dirs)),
        };
        let mut missing = Vec::new();
        let mut cursor = parent.to_path_buf();
        while !cursor.exists() {
            missing.push(cursor.clone());
            if !cursor.pop() {
                break;
            }
        }
        missing.reverse();
        created_dirs.extend(missing);
        if let Err(error) = fs::create_dir_all(parent) {
            return Err(cleanup_preparation_error(error.into(), &created_dirs));
        }
        let mut file = match tempfile::NamedTempFile::new_in(parent) {
            Ok(file) => file,
            Err(error) => return Err(cleanup_preparation_error(error.into(), &created_dirs)),
        };
        if let Err(error) = file.write_all(content) {
            return Err(cleanup_preparation_error(error.into(), &created_dirs));
        }
        if let Err(error) = file.as_file().sync_all() {
            return Err(cleanup_preparation_error(error.into(), &created_dirs));
        }
        temporary.push((path.clone(), file));
    }
    let mut temporary = temporary.into_iter().collect::<BTreeMap<_, _>>();
    let mut applied = Vec::new();
    for (path, mutation) in files {
        let current = match file_state(path) {
            Ok(current) => current,
            Err(error) => {
                return Err(rollback_error(
                    error,
                    PatchRollback {
                        files: applied,
                        created_dirs: created_dirs.clone(),
                    },
                ));
            }
        };
        if current != mutation.original {
            return Err(rollback_error(
                anyhow::anyhow!(
                    "target {} changed while the patch was being applied",
                    path.display()
                ),
                PatchRollback {
                    files: applied,
                    created_dirs: created_dirs.clone(),
                },
            ));
        }
        let result = match (&mutation.original, &mutation.updated) {
            (Some(_), Some(_)) => {
                let file = temporary
                    .remove(path)
                    .expect("existing writes have a prepared temporary file");
                fs::rename(file.into_temp_path(), path)
                    .with_context(|| format!("replace {}", path.display()))
            }
            (None, Some(_)) => temporary
                .remove(path)
                .expect("new files have a prepared temporary file")
                .persist_noclobber(path)
                .map(|_| ())
                .map_err(|error| anyhow::anyhow!(error.error))
                .with_context(|| format!("create {}", path.display())),
            (Some(_), None) => {
                fs::remove_file(path).with_context(|| format!("remove {}", path.display()))
            }
            (None, None) => unreachable!("every patch mutation changes a file"),
        };
        if let Err(error) = result {
            return Err(rollback_error(
                error,
                PatchRollback {
                    files: applied,
                    created_dirs: created_dirs.clone(),
                },
            ));
        }
        let permissions = original_permissions.get(path).cloned().flatten();
        let file_rollback = FileRollback {
            path: path.clone(),
            original: mutation.original.clone(),
            permissions: permissions.clone(),
        };
        if mutation.updated.is_some()
            && let Some(permissions) = &permissions
            && let Err(error) = fs::set_permissions(path, permissions.clone())
        {
            let mut files_to_restore = applied;
            files_to_restore.push(file_rollback);
            return Err(rollback_error(
                error.into(),
                PatchRollback {
                    files: files_to_restore,
                    created_dirs: created_dirs.clone(),
                },
            ));
        }
        applied.push(file_rollback);
    }
    let rollback = PatchRollback {
        files: applied,
        created_dirs,
    };
    if let Err(error) = sync_mutation_directories(&rollback) {
        return Err(rollback_error(error, rollback));
    }
    Ok(rollback)
}

fn sync_mutation_directories(rollback: &PatchRollback) -> Result<()> {
    #[cfg(unix)]
    {
        let mut directories = std::collections::BTreeSet::new();
        directories.extend(
            rollback
                .files
                .iter()
                .filter_map(|file| file.path.parent().map(Path::to_path_buf)),
        );
        directories.extend(rollback.created_dirs.iter().cloned());
        for directory in directories {
            fs::File::open(&directory)
                .with_context(|| format!("open mutation directory {}", directory.display()))?
                .sync_all()
                .with_context(|| format!("sync mutation directory {}", directory.display()))?;
        }
    }
    Ok(())
}

fn cleanup_preparation_error(error: anyhow::Error, created_dirs: &[PathBuf]) -> anyhow::Error {
    match remove_created_dirs(created_dirs) {
        Ok(()) => error,
        Err(cleanup) => anyhow::anyhow!("{error}; preparation cleanup failed: {cleanup}"),
    }
}

fn rollback_error(error: anyhow::Error, rollback: PatchRollback) -> anyhow::Error {
    match restore_rollback(&rollback) {
        Ok(()) => error,
        Err(restore) => anyhow::anyhow!("{error}; rollback failed: {restore}"),
    }
}

fn remove_created_dirs(created_dirs: &[PathBuf]) -> Result<()> {
    let mut errors = Vec::new();
    for directory in created_dirs.iter().rev() {
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => errors.push(format!("{}: {error}", directory.display())),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "failed to remove created directories: {}",
            errors.join("; ")
        ))
    }
}

fn restore_rollback(rollback: &PatchRollback) -> Result<()> {
    let mut errors = Vec::new();
    for file in rollback.files.iter().rev() {
        let result = match &file.original {
            Some(content) => (|| -> Result<()> {
                let parent = file.path.parent().context("target path has no parent")?;
                fs::create_dir_all(parent)?;
                if file.path.exists() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = fs::metadata(&file.path)?.permissions().mode() | 0o200;
                        fs::set_permissions(&file.path, fs::Permissions::from_mode(mode))?;
                    }
                    #[cfg(not(unix))]
                    {
                        let mut writable = fs::metadata(&file.path)?.permissions();
                        writable.set_readonly(false);
                        fs::set_permissions(&file.path, writable)?;
                    }
                }
                fs::write(&file.path, content)?;
                if let Some(permissions) = &file.permissions {
                    fs::set_permissions(&file.path, permissions.clone())?;
                }
                Ok(())
            })(),
            None => match fs::remove_file(&file.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            },
        };
        if let Err(error) = result {
            errors.push(format!("{}: {error}", file.path.display()));
        }
    }
    if let Err(error) = remove_created_dirs(&rollback.created_dirs) {
        errors.push(error.to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("rollback errors: {}", errors.join("; ")))
    }
}

fn append_event(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    event: AgentEventKind,
) -> Result<AgentEvent> {
    append_event_with_lock_mode(workspace, run, event, false)
}

fn append_event_with_lock_mode(
    workspace: &SpecWorkspace,
    run: &AgentRun,
    event: AgentEventKind,
    workspace_locked: bool,
) -> Result<AgentEvent> {
    let store = DeliveryStore::for_workspace(&workspace.root)?;
    match event {
        AgentEventKind::BlockerRecorded { blocker } => {
            if workspace_locked {
                store.record_agent_blocker_while_locked(run, blocker)
            } else {
                store.record_agent_blocker(run, blocker)
            }
        }
        AgentEventKind::ScopeExpansionRequested { request } => {
            if !workspace_locked {
                let _workspace_lock = store.lock_workspace()?;
                store.request_agent_scope_expansion_while_locked(run, request)
            } else {
                store.request_agent_scope_expansion_while_locked(run, request)
            }
        }
        AgentEventKind::RunAbandoned { reason } => {
            if workspace_locked {
                store.abandon_agent_run_while_locked(run, reason)
            } else {
                store.abandon_agent_run(run, reason)
            }
        }
        AgentEventKind::RunStarted { .. } | AgentEventKind::PatchRecorded { .. } => {
            bail!("agent event kind is not appendable through the generic agent API")
        }
        AgentEventKind::VerificationRecorded { attempt_id } => {
            if workspace_locked {
                store.record_agent_verification_while_locked(run, &attempt_id)
            } else {
                let _workspace_lock = store.lock_workspace()?;
                store.record_agent_verification_while_locked(run, &attempt_id)
            }
        }
    }
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
    format_sha256(hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_file_permissions_and_created_directories() {
        let temp = tempfile::tempdir().expect("rollback tempdir");
        let existing = temp.path().join("existing.txt");
        fs::write(&existing, "before\n").expect("existing file");
        let mut readonly = fs::metadata(&existing).unwrap().permissions();
        readonly.set_readonly(true);
        fs::set_permissions(&existing, readonly).expect("readonly file");

        let mut update = BTreeMap::new();
        update.insert(
            existing.clone(),
            FileMutation {
                original: Some(b"before\n".to_vec()),
                updated: Some(b"after\n".to_vec()),
            },
        );
        let rollback = apply_file_mutations(&update).expect("apply update");
        assert_eq!(fs::read_to_string(&existing).unwrap(), "after\n");
        assert!(fs::metadata(&existing).unwrap().permissions().readonly());
        restore_rollback(&rollback).expect("restore update");
        assert_eq!(fs::read_to_string(&existing).unwrap(), "before\n");
        assert!(fs::metadata(&existing).unwrap().permissions().readonly());

        let created = temp.path().join("new/nested/file.txt");
        let mut create = BTreeMap::new();
        create.insert(
            created.clone(),
            FileMutation {
                original: None,
                updated: Some(b"created\n".to_vec()),
            },
        );
        let rollback = apply_file_mutations(&create).expect("apply create");
        assert!(created.is_file());
        restore_rollback(&rollback).expect("restore create");
        assert!(!created.exists());
        assert!(!temp.path().join("new").exists());
    }

    #[test]
    fn preparation_failure_removes_new_directories() {
        let temp = tempfile::tempdir().expect("preparation tempdir");
        let blocking_parent = temp.path().join("blocking");
        fs::write(&blocking_parent, "not a directory").expect("blocking file");
        let target = blocking_parent.join("nested/file.txt");
        let mut create = BTreeMap::new();
        create.insert(
            target.clone(),
            FileMutation {
                original: None,
                updated: Some(b"created\n".to_vec()),
            },
        );

        assert!(apply_file_mutations(&create).is_err());
        assert!(!blocking_parent.join("nested").exists());
        assert!(!target.exists());
    }

    #[test]
    fn rollback_attempts_later_files_after_an_earlier_restore_error() {
        let temp = tempfile::tempdir().expect("rollback tempdir");
        let blocking_parent = temp.path().join("blocking");
        fs::write(&blocking_parent, "not a directory").expect("blocking file");
        let failed = blocking_parent.join("nested/file.txt");
        let later = temp.path().join("later.txt");
        fs::write(&later, "after\n").expect("later file");
        let rollback = PatchRollback {
            files: vec![
                FileRollback {
                    path: later.clone(),
                    original: Some(b"before\n".to_vec()),
                    permissions: None,
                },
                FileRollback {
                    path: failed,
                    original: Some(b"before\n".to_vec()),
                    permissions: None,
                },
            ],
            created_dirs: vec![],
        };

        assert!(restore_rollback(&rollback).is_err());
        assert_eq!(fs::read_to_string(later).unwrap(), "before\n");
    }
}
