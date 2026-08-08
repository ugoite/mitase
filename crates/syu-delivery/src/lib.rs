#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use syu_spec_model::{ItemStatus, SpecDocument, SpecItemRef, format_sha256, lowercase_hex};
use syu_work_model::{
    AGENT_EVENT_SCHEMA, AGENT_PATCH_SCHEMA, AGENT_RUN_SCHEMA, AgentBlocker, AgentContextPack,
    AgentEvent, AgentEventKind, AgentPatchRecord, AgentRun, AgentRunStatus,
    COMPLETION_ATTEMPT_SCHEMA, COMPLETION_REPORT_SCHEMA, CONTEXT_PACK_SCHEMA, CompletionAttempt,
    CompletionBlocker, CompletionStatus, ExecutionIdentity, FINALIZATION_RECEIPT_SCHEMA,
    FinalizationPreview, FinalizationReceipt, PLAN_APPROVAL_SCHEMA, PlanApproval,
    ScopeExpansionRequest, VERIFICATION_RECEIPT_SCHEMA, WORK_PLAN_SCHEMA, work_plan_digest,
};
use syu_workspace::SpecWorkspace;
use uuid::Uuid;

pub const STORE_SCHEMA: &str = "syu/completion/v1";
static AGENT_EVENT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct DeliveryStore {
    root: PathBuf,
    workspace_root: PathBuf,
}

/// A repository-local exclusive lock shared by every process that mutates
/// Workbench evidence or the governed workspace.  The lock is deliberately a
/// file lock rather than a process-global mutex: Workbench and the CLI may be
/// active in different processes against the same pre-v1 workspace.
pub struct WorkspaceLock {
    file: File,
}

/// A small durable write-ahead record for the two multi-file mutations in the
/// delivery boundary. It is intentionally pre-v1 and workspace-local: the
/// next process either observes the committed evidence or restores the exact
/// bytes captured before the mutation began.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationJournal {
    pub schema: String,
    pub operation: String,
    pub operation_id: String,
    pub files: Vec<MutationJournalFile>,
    pub created_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationJournalFile {
    pub path: String,
    pub original: Option<Vec<u8>>,
}

pub const MUTATION_JOURNAL_SCHEMA: &str = "syu/workspace-mutation-journal/v1";

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

impl DeliveryStore {
    pub fn for_workspace(workspace_root: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--git-path", "syu"])
            .current_dir(workspace_root)
            .output()
            .context("resolve git syu path")?;
        if !output.status.success() {
            bail!("git rev-parse --git-path syu failed");
        }
        let raw = String::from_utf8(output.stdout)?.trim().to_owned();
        let path = PathBuf::from(raw);
        let root = if path.is_absolute() {
            path
        } else {
            workspace_root.join(path)
        };
        Ok(Self {
            root,
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> Result<()> {
        for path in [
            self.approvals_dir(),
            self.attempts_dir(),
            self.finalizations_dir(),
            self.agent_events_dir(),
        ] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    pub fn lock_workspace(&self) -> Result<WorkspaceLock> {
        self.ensure()?;
        let path = self.root.join("workspace.lock");
        let file = File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        file.lock_exclusive()?;
        let lock = WorkspaceLock { file };
        if let Err(error) = self.recover_mutation_journal() {
            drop(lock);
            return Err(error);
        }
        Ok(lock)
    }

    /// Persist the bytes needed to recover a mutation. Callers must already
    /// hold `lock_workspace`; this method deliberately does not acquire the
    /// file lock again.
    pub fn write_mutation_journal(
        &self,
        operation: &str,
        operation_id: &str,
        files: Vec<MutationJournalFile>,
        created_dirs: Vec<PathBuf>,
    ) -> Result<()> {
        if operation.trim().is_empty() || operation_id.trim().is_empty() {
            bail!("mutation journal requires an operation and operation id");
        }
        let files = files
            .into_iter()
            .map(|mut file| {
                file.path = relative_workspace_path(&self.workspace_root, Path::new(&file.path))?;
                Ok(file)
            })
            .collect::<Result<Vec<_>>>()?;
        let created_dirs = created_dirs
            .into_iter()
            .map(|path| relative_workspace_path(&self.workspace_root, &path))
            .collect::<Result<Vec<_>>>()?;
        let journal = MutationJournal {
            schema: MUTATION_JOURNAL_SCHEMA.into(),
            operation: operation.into(),
            operation_id: operation_id.into(),
            files,
            created_dirs,
        };
        let path = self.mutation_journal_path();
        if path.exists() {
            bail!(
                "workspace mutation journal already exists at {}",
                path.display()
            );
        }
        write_atomic_json(&path, &journal)
    }

    pub fn clear_mutation_journal(&self) -> Result<()> {
        let path = self.mutation_journal_path();
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn recover_mutation_journal(&self) -> Result<()> {
        let path = self.mutation_journal_path();
        if !path.exists() {
            return Ok(());
        }
        let journal: MutationJournal = read_json(&path)?;
        if journal.schema != MUTATION_JOURNAL_SCHEMA
            || journal.operation.trim().is_empty()
            || journal.operation_id.trim().is_empty()
        {
            bail!("workspace mutation journal is invalid");
        }
        for relative in journal
            .files
            .iter()
            .map(|file| file.path.as_str())
            .chain(journal.created_dirs.iter().map(String::as_str))
        {
            let path = Path::new(relative);
            if path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                bail!("workspace mutation journal contains a path outside the workspace");
            }
        }
        let committed = match journal.operation.as_str() {
            "agent-patch" => {
                let mut committed = false;
                for path in json_files(&self.agent_events_dir())? {
                    let event: AgentEvent = read_json(&path)?;
                    validate_agent_event_schema(&event)?;
                    validate_agent_event_digest(&path, &event)?;
                    if let AgentEventKind::PatchRecorded { patch } = event.event
                        && patch.patch_id == journal.operation_id
                    {
                        committed = true;
                    }
                }
                committed
            }
            "finalization" => {
                let mut committed = false;
                for path in json_files(&self.finalizations_dir())? {
                    let receipt: FinalizationReceipt = read_json(&path)?;
                    validate_finalization_schema(&receipt)?;
                    let mut without_digest = receipt.clone();
                    let supplied = without_digest.finalization_digest.clone();
                    without_digest.finalization_digest.clear();
                    if supplied != Self::finalization_digest(&without_digest)? {
                        bail!("stored finalization has an invalid digest");
                    }
                    if receipt.attempt_id == journal.operation_id {
                        committed = true;
                    }
                }
                committed
            }
            _ => bail!("unknown workspace mutation journal operation"),
        };
        if !committed {
            let mut errors = Vec::new();
            for file in &journal.files {
                let path = self.workspace_root.join(&file.path);
                let result = match &file.original {
                    Some(bytes) => atomic_write(&path, bytes),
                    None => match fs::remove_file(&path) {
                        Ok(()) => Ok(()),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(error.into()),
                    },
                };
                if let Err(error) = result {
                    errors.push(format!("{}: {error}", path.display()));
                }
            }
            for directory in journal.created_dirs.iter().rev() {
                let path = self.workspace_root.join(directory);
                if let Err(error) = fs::remove_dir(&path)
                    && error.kind() != std::io::ErrorKind::NotFound
                    && error.kind() != std::io::ErrorKind::DirectoryNotEmpty
                {
                    errors.push(format!("{}: {error}", path.display()));
                }
            }
            if !errors.is_empty() {
                bail!("workspace mutation recovery failed: {}", errors.join("; "));
            }
        }
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn approve(&self, approval: &PlanApproval) -> Result<PlanApproval> {
        let _workspace_lock = self.lock_workspace()?;
        self.approve_while_locked(approval)
    }

    pub fn approve_while_locked(&self, approval: &PlanApproval) -> Result<PlanApproval> {
        self.ensure()?;
        validate_plan_approval_schema(approval)?;
        if approval.plan_digest != approval.plan.canonical_digest {
            bail!("approval plan digest does not match its canonical plan");
        }
        if approval.plan.slices.len() != 1 || approval.slice_id != approval.plan.slices[0].id {
            bail!("approval must bind exactly one canonical execution slice");
        }
        let workspace = SpecWorkspace::load(&self.workspace_root)?;
        if approval.workspace_fingerprint != workspace.try_fingerprint()?
            || approval.revision != repository_revision(&workspace.root)?
        {
            bail!("approval basis is stale; review the current workspace again");
        }
        let identity = ExecutionIdentity {
            plan_digest: approval.plan_digest.clone(),
            slice_id: approval.slice_id.clone(),
        };
        let path = self.approval_path(&identity);
        if path.exists() {
            let existing = self.approval(&identity)?;
            if existing.plan_digest != approval.plan_digest || existing.plan != approval.plan {
                bail!(
                    "a different approval already exists for plan {}",
                    approval.plan_digest
                );
            }
            return Ok(existing);
        }
        write_immutable_json(&path, approval)
    }

    pub fn approval(&self, identity: &ExecutionIdentity) -> Result<PlanApproval> {
        let approval: PlanApproval = read_json(&self.approval_path(identity))?;
        validate_plan_approval_schema(&approval)?;
        if approval.plan_digest != identity.plan_digest
            || approval.slice_id != identity.slice_id
            || approval.plan_digest != approval.plan.canonical_digest
            || approval.plan.slices.len() != 1
            || approval.plan.slices[0].id != identity.slice_id
        {
            bail!("stored approval for execution identity is invalid");
        }
        Ok(approval)
    }

    pub fn has_approval_for_plan(&self, plan_digest: &str) -> Result<bool> {
        for path in json_files(&self.approvals_dir())? {
            let approval: PlanApproval = read_json(&path)?;
            validate_plan_approval_schema(&approval)?;
            if approval.plan_digest == plan_digest {
                if approval.plan_digest != approval.plan.canonical_digest
                    || approval.plan.slices.len() != 1
                    || approval.slice_id != approval.plan.slices[0].id
                {
                    bail!("stored approval for {plan_digest} is invalid");
                }
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn has_current_approval_for_plan(
        &self,
        workspace: &SpecWorkspace,
        plan_digest: &str,
    ) -> Result<bool> {
        self.require_workspace(workspace)?;
        for path in json_files(&self.approvals_dir())? {
            let approval: PlanApproval = read_json(&path)?;
            validate_plan_approval_schema(&approval)?;
            if approval.plan_digest != plan_digest {
                continue;
            }
            return Ok(
                approval.workspace_fingerprint == workspace.try_fingerprint()?
                    && approval.revision == repository_revision(&workspace.root)?,
            );
        }
        Ok(false)
    }

    pub fn execute_and_append_attempt(
        &self,
        workspace: &SpecWorkspace,
        plan: &syu_work_model::WorkPlan,
        slice_id: &str,
    ) -> Result<CompletionAttempt> {
        let _workspace_lock = self.lock_workspace()?;
        self.execute_and_append_attempt_while_locked(workspace, plan, slice_id)
    }

    pub fn execute_and_append_attempt_while_locked(
        &self,
        workspace: &SpecWorkspace,
        plan: &syu_work_model::WorkPlan,
        slice_id: &str,
    ) -> Result<CompletionAttempt> {
        self.execute_and_append_attempt_for_agent_while_locked(workspace, plan, slice_id, None)
    }

    pub fn execute_and_append_attempt_for_agent_while_locked(
        &self,
        workspace: &SpecWorkspace,
        plan: &syu_work_model::WorkPlan,
        slice_id: &str,
        agent_run_id: Option<&str>,
    ) -> Result<CompletionAttempt> {
        self.require_workspace(workspace)?;
        let fresh_workspace = SpecWorkspace::load(&self.workspace_root)?;
        let workspace = &fresh_workspace;
        let identity = ExecutionIdentity {
            plan_digest: plan.canonical_digest.clone(),
            slice_id: slice_id.into(),
        };
        let approval = self.approval(&identity)?;
        if approval.plan != *plan {
            bail!("verification requires the exact approved plan");
        }
        if plan.basis.spec_fingerprint != workspace.spec_fingerprint()? {
            bail!("verification plan is stale against the current specification and config");
        }
        let slice = plan
            .slices
            .iter()
            .find(|slice| slice.id == slice_id)
            .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
        if slice.verification_targets.is_empty() {
            bail!("selected slice has no verification targets");
        }
        let index = workspace.index()?;
        let attempt_id = self.new_id("attempt");
        let started_at = now_nanos().to_string();
        let (verification, receipt, mut report) = syu_validation::execute_verification_attempt(
            workspace,
            &index,
            plan,
            slice_id,
            &repository_revision(&workspace.root)?,
            &attempt_id,
        )?;
        report.attempt_id = attempt_id.clone();
        let mut attempt = CompletionAttempt {
            schema: COMPLETION_ATTEMPT_SCHEMA.into(),
            attempt_id,
            attempt_digest: String::new(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id: slice_id.into(),
            agent_run_id: agent_run_id.map(str::to_owned),
            approved_plan_digest: approval.plan_digest,
            started_at,
            completed_at: now_nanos().to_string(),
            verification,
            receipt,
            report,
        };
        let mut without_digest = attempt.clone();
        without_digest.attempt_digest.clear();
        attempt.attempt_digest = Self::verification_digest(&without_digest)?;
        self.append_attempt(workspace, &attempt)
    }

    pub fn remove_unfinalized_attempt_while_locked(
        &self,
        attempt: &CompletionAttempt,
    ) -> Result<()> {
        let identity = ExecutionIdentity {
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
        };
        if self
            .finalization_path(&identity, &attempt.attempt_id)
            .exists()
        {
            bail!("cannot remove an attempt that already has a finalization");
        }
        let path = self.attempt_path(attempt);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn append_attempt(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
    ) -> Result<CompletionAttempt> {
        self.ensure()?;
        self.require_workspace(workspace)?;
        validate_completion_attempt_schema(attempt)?;
        validate_attempt_digest(attempt)?;
        let identity = ExecutionIdentity {
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
        };
        let approval = self.approval(&identity).with_context(|| {
            format!(
                "completion attempt {} requires an approval for its plan and slice",
                attempt.attempt_id
            )
        })?;
        validate_completion_attempt_against_plan(attempt, &approval.plan)?;
        if approval.plan_digest != attempt.approved_plan_digest
            || approval.plan_digest != attempt.plan_digest
        {
            bail!("completion attempt is not tied to its approved plan");
        }
        if let Some(receipt) = &attempt.receipt {
            let index = workspace.index()?;
            let revision = repository_revision(&workspace.root)?;
            syu_validation::validate_verification_receipt(
                workspace,
                &index,
                &approval.plan,
                &attempt.slice_id,
                receipt,
                &revision,
            )?;
        }
        let path = self.attempt_path(attempt);
        write_immutable_json(&path, attempt)
    }

    pub fn attempt(
        &self,
        identity: &ExecutionIdentity,
        attempt_id: &str,
    ) -> Result<CompletionAttempt> {
        for path in json_files(&self.attempts_dir())? {
            let value: CompletionAttempt = read_json(&path)?;
            validate_completion_attempt_schema(&value)?;
            validate_attempt_digest(&value)?;
            if value.attempt_id == attempt_id
                && value.plan_digest == identity.plan_digest
                && value.slice_id == identity.slice_id
            {
                let approval = self.approval(identity)?;
                validate_completion_attempt_against_plan(&value, &approval.plan)?;
                return Ok(value);
            }
        }
        bail!("completion attempt {attempt_id} not found for execution identity")
    }

    pub fn attempts(&self) -> Result<Vec<CompletionAttempt>> {
        let mut values = Vec::new();
        for path in json_files(&self.attempts_dir())? {
            let value = read_json::<CompletionAttempt>(&path)?;
            validate_completion_attempt_schema(&value)?;
            validate_attempt_digest(&value)?;
            let identity = ExecutionIdentity {
                plan_digest: value.plan_digest.clone(),
                slice_id: value.slice_id.clone(),
            };
            let approval = self.approval(&identity)?;
            validate_completion_attempt_against_plan(&value, &approval.plan)?;
            values.push(value);
        }
        values.sort_by(|a, b| {
            b.completed_at
                .cmp(&a.completed_at)
                .then_with(|| b.attempt_id.cmp(&a.attempt_id))
        });
        Ok(values)
    }

    fn append_finalization(&self, receipt: &FinalizationReceipt) -> Result<FinalizationReceipt> {
        self.ensure()?;
        validate_finalization_schema(receipt)?;
        let mut without_digest = receipt.clone();
        let supplied = without_digest.finalization_digest.clone();
        without_digest.finalization_digest.clear();
        if supplied != Self::finalization_digest(&without_digest)? {
            bail!("finalization receipt has an invalid digest");
        }
        let identity = ExecutionIdentity {
            plan_digest: receipt.plan_digest.clone(),
            slice_id: receipt.slice_id.clone(),
        };
        let attempt = self.attempt(&identity, &receipt.attempt_id)?;
        if attempt.attempt_digest != receipt.attempt_digest
            || attempt.approved_plan_digest != receipt.plan_digest
            || attempt.report.status != CompletionStatus::Complete
            || attempt.receipt.is_none()
            || attempt
                .receipt
                .as_ref()
                .map(|value| &value.lifecycle_proofs)
                != Some(&receipt.lifecycle_proofs)
        {
            bail!("finalization is not tied to one complete attempt for its execution identity");
        }
        let path = self.finalization_path(&identity, &receipt.attempt_id);
        if path.exists() {
            return self
                .finalization(&identity, &receipt.attempt_id)?
                .context("stored finalization disappeared while reading immutable evidence");
        }
        write_immutable_json(&path, receipt)
    }

    pub fn finalization(
        &self,
        identity: &ExecutionIdentity,
        attempt_id: &str,
    ) -> Result<Option<FinalizationReceipt>> {
        let path = self.finalization_path(identity, attempt_id);
        if !path.exists() {
            return Ok(None);
        }
        let receipt: FinalizationReceipt = read_json(&path)?;
        validate_finalization_schema(&receipt)?;
        if receipt.plan_digest != identity.plan_digest
            || receipt.slice_id != identity.slice_id
            || receipt.attempt_id != attempt_id
        {
            bail!("stored finalization does not match execution identity");
        }
        let mut without_digest = receipt.clone();
        let supplied = without_digest.finalization_digest.clone();
        without_digest.finalization_digest.clear();
        let expected = Self::finalization_digest(&without_digest)?;
        if supplied != expected {
            bail!("stored finalization has an invalid digest: {supplied} != {expected}");
        }
        let attempt = self.attempt(identity, attempt_id)?;
        if attempt.attempt_digest != receipt.attempt_digest
            || attempt.approved_plan_digest != receipt.plan_digest
            || attempt.report.status != CompletionStatus::Complete
            || attempt.receipt.is_none()
            || attempt
                .receipt
                .as_ref()
                .map(|value| &value.lifecycle_proofs)
                != Some(&receipt.lifecycle_proofs)
        {
            bail!("stored finalization is not tied to one complete attempt");
        }
        Ok(Some(receipt))
    }

    pub fn finalization_preview(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
    ) -> Result<FinalizationPreview> {
        let _workspace_lock = self.lock_workspace()?;
        self.finalization_preview_while_locked(workspace, attempt)
    }

    pub fn finalization_preview_while_locked(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
    ) -> Result<FinalizationPreview> {
        self.require_workspace(workspace)?;
        let identity = ExecutionIdentity {
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
        };
        let attempt = self.attempt(&identity, &attempt.attempt_id)?;
        let approval = self.approval(&identity)?;
        if approval.plan != attempt_plan(&approval, &attempt)? {
            bail!("attempt is not tied to its approved plan");
        }
        let pre_workspace_fingerprint = workspace.try_fingerprint()?;
        let mut blockers = attempt.report.blockers.clone();
        if attempt.report.status != CompletionStatus::Complete || attempt.receipt.is_none() {
            blockers.push(CompletionBlocker {
                code: "SYU-FINALIZE-INCOMPLETE".into(),
                message: "only a complete completion attempt can be finalized".into(),
                next_action: "Resolve blockers and create a new complete attempt.".into(),
            });
        }
        if let Some(receipt) = &attempt.receipt {
            let index = workspace.index()?;
            if receipt.workspace_fingerprint != pre_workspace_fingerprint
                || syu_validation::validate_verification_receipt(
                    workspace,
                    &index,
                    &approval.plan,
                    &attempt.slice_id,
                    receipt,
                    &receipt.revision,
                )
                .is_err()
            {
                blockers.push(CompletionBlocker {
                    code: "SYU-FINALIZE-STALE-EVIDENCE".into(),
                    message: "the completion attempt no longer matches the current workspace"
                        .into(),
                    next_action:
                        "Re-run verification for the approved plan and slice before finalizing."
                            .into(),
                });
            }
        }
        let promoted_items = if blockers.is_empty() {
            let index = workspace.index()?;
            let slice = approval
                .plan
                .slices
                .iter()
                .find(|slice| slice.id == attempt.slice_id)
                .ok_or_else(|| anyhow::anyhow!("attempt slice is absent from approved plan"))?;
            let mut items = std::collections::BTreeSet::new();
            for evidence in &attempt.report.demonstrated {
                if index.item_status.get(&evidence.anchor.item) == Some(&ItemStatus::Planned) {
                    items.insert(evidence.anchor.item.clone());
                }
            }
            for target in &slice.editable_targets {
                if index.item_status.get(&target.reference.binding.item)
                    == Some(&ItemStatus::Planned)
                {
                    items.insert(target.reference.binding.item.clone());
                }
            }
            items.into_iter().map(SpecItemRef).collect()
        } else {
            vec![]
        };
        let changed_files = changed_document_paths(workspace, &promoted_items)?;
        let mut preview = FinalizationPreview {
            schema: "syu/finalization-preview/v1".into(),
            attempt_id: attempt.attempt_id.clone(),
            attempt_digest: attempt.attempt_digest.clone(),
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
            preview_token: String::new(),
            status: if blockers.is_empty() {
                CompletionStatus::Complete
            } else {
                CompletionStatus::Blocked
            },
            pre_workspace_fingerprint,
            promoted_items,
            changed_files,
            blockers,
        };
        preview.preview_token = Self::finalization_digest(&preview_without_token(&preview))?;
        Ok(preview)
    }

    pub fn apply_finalization(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
        preview: &FinalizationPreview,
        token: &str,
    ) -> Result<FinalizationReceipt> {
        let _workspace_lock = self.lock_workspace()?;
        self.apply_finalization_while_locked(workspace, attempt, preview, token)
    }

    pub fn apply_finalization_while_locked(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
        preview: &FinalizationPreview,
        token: &str,
    ) -> Result<FinalizationReceipt> {
        self.require_workspace(workspace)?;
        let identity = ExecutionIdentity {
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
        };
        let attempt = self.attempt(&identity, &attempt.attempt_id)?;
        if preview.preview_token != token || preview.status != CompletionStatus::Complete {
            bail!("finalization preview token is stale or blocked");
        }
        if let Some(existing) = self.finalization(&identity, &attempt.attempt_id)? {
            return Ok(existing);
        }
        let current = self.finalization_preview_while_locked(workspace, &attempt)?;
        if current != preview.clone()
            || current.preview_token != token
            || current.pre_workspace_fingerprint != preview.pre_workspace_fingerprint
        {
            bail!("workspace changed after finalization preview; preview again");
        }
        let journal_files = preview
            .changed_files
            .iter()
            .map(|relative| {
                let path = workspace.root.join(relative);
                Ok(MutationJournalFile {
                    path: path.to_string_lossy().into_owned(),
                    original: Some(
                        fs::read(&path).with_context(|| format!("read {}", path.display()))?,
                    ),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.write_mutation_journal(
            "finalization",
            &attempt.attempt_id,
            journal_files,
            Vec::new(),
        )?;
        let old = match apply_status_overlay(workspace, &preview.promoted_items) {
            Ok(old) => old,
            Err(error) => return Err(error),
        };
        let post_workspace_fingerprint = match (|| -> Result<String> {
            let candidate = validate_finalized_workspace(&workspace.root)?;
            Ok(candidate.try_fingerprint()?)
        })() {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                let restored = restore_files(&old);
                let cleared = self.clear_mutation_journal();
                if let Err(restore) = restored {
                    return Err(anyhow::anyhow!(
                        "finalization validation failed: {error}; rollback failed: {restore}"
                    ));
                }
                cleared?;
                return Err(error);
            }
        };
        let receipt = FinalizationReceipt {
            schema: FINALIZATION_RECEIPT_SCHEMA.into(),
            finalization_id: self.new_id("finalization"),
            finalization_digest: String::new(),
            attempt_id: attempt.attempt_id.clone(),
            attempt_digest: attempt.attempt_digest.clone(),
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
            pre_workspace_fingerprint: preview.pre_workspace_fingerprint.clone(),
            post_workspace_fingerprint,
            promoted_items: preview.promoted_items.clone(),
            changed_files: preview.changed_files.clone(),
            lifecycle_proofs: attempt
                .receipt
                .as_ref()
                .map(|receipt| receipt.lifecycle_proofs.clone())
                .unwrap_or_default(),
            completed_at: now_nanos().to_string(),
        };
        let mut receipt = receipt;
        receipt.finalization_digest =
            Self::finalization_digest(&finalization_without_digest(&receipt))?;
        match self.append_finalization(&receipt) {
            Ok(receipt) => {
                self.clear_mutation_journal()?;
                Ok(receipt)
            }
            Err(error) => {
                let restored = restore_files(&old);
                let cleared = self.clear_mutation_journal();
                if let Err(restore) = restored {
                    Err(anyhow::anyhow!(
                        "finalization evidence append failed: {error}; rollback failed: {restore}"
                    ))
                } else {
                    cleared?;
                    Err(error)
                }
            }
        }
    }

    pub fn new_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}-{}", now_nanos(), Uuid::new_v4())
    }

    pub fn start_agent_run(&self, run: &AgentRun, retry: bool) -> Result<AgentEvent> {
        let _workspace_lock = self.lock_workspace()?;
        self.start_agent_run_while_locked(run, retry)
    }

    pub fn start_agent_run_while_locked(&self, run: &AgentRun, retry: bool) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::RunStarted {
                    run: Box::new(run.clone()),
                },
            },
            retry,
        )
    }

    pub fn record_agent_patch_while_locked(
        &self,
        run: &AgentRun,
        patch: AgentPatchRecord,
    ) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::PatchRecorded { patch },
            },
            false,
        )
    }

    pub fn record_agent_blocker(
        &self,
        run: &AgentRun,
        blocker: AgentBlocker,
    ) -> Result<AgentEvent> {
        let _workspace_lock = self.lock_workspace()?;
        self.record_agent_blocker_while_locked(run, blocker)
    }

    pub fn record_agent_blocker_while_locked(
        &self,
        run: &AgentRun,
        blocker: AgentBlocker,
    ) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::BlockerRecorded { blocker },
            },
            false,
        )
    }

    pub fn request_agent_scope_expansion_while_locked(
        &self,
        run: &AgentRun,
        request: ScopeExpansionRequest,
    ) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::ScopeExpansionRequested { request },
            },
            false,
        )
    }

    pub fn record_agent_verification_while_locked(
        &self,
        run: &AgentRun,
        attempt_id: &str,
    ) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::VerificationRecorded {
                    attempt_id: attempt_id.into(),
                },
            },
            false,
        )
    }

    pub fn abandon_agent_run(&self, run: &AgentRun, reason: String) -> Result<AgentEvent> {
        let _workspace_lock = self.lock_workspace()?;
        self.abandon_agent_run_while_locked(run, reason)
    }

    pub fn abandon_agent_run_while_locked(
        &self,
        run: &AgentRun,
        reason: String,
    ) -> Result<AgentEvent> {
        self.append_agent_event_while_locked(
            &AgentEvent {
                schema: AGENT_EVENT_SCHEMA.into(),
                event_id: self.new_id("agent-event"),
                event_digest: String::new(),
                run_id: run.run_id.clone(),
                plan_digest: run.plan_digest.clone(),
                slice_id: run.slice_id.clone(),
                created_at: now_nanos().to_string(),
                event: AgentEventKind::RunAbandoned { reason },
            },
            false,
        )
    }

    #[cfg(test)]
    fn append_agent_event(&self, event: &AgentEvent) -> Result<AgentEvent> {
        let _workspace_lock = self.lock_workspace()?;
        self.append_agent_event_with_retry(event, false)
    }

    #[cfg(test)]
    fn append_agent_event_for_retry(&self, event: &AgentEvent, retry: bool) -> Result<AgentEvent> {
        let _workspace_lock = self.lock_workspace()?;
        self.append_agent_event_with_retry(event, retry)
    }

    /// Append the event while the caller already owns `lock_workspace`.
    /// Keeping this explicit prevents verification and its terminal event from
    /// being interleaved by another Workbench or CLI process.
    fn append_agent_event_while_locked(
        &self,
        event: &AgentEvent,
        retry: bool,
    ) -> Result<AgentEvent> {
        self.append_agent_event_with_retry(event, retry)
    }

    fn append_agent_event_with_retry(&self, event: &AgentEvent, retry: bool) -> Result<AgentEvent> {
        self.ensure()?;
        let _event_guard = AGENT_EVENT_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| anyhow::anyhow!("agent event lifecycle lock"))?;
        validate_agent_event_schema(event)?;
        validate_agent_event_references(self, event, true)?;
        let identity = ExecutionIdentity {
            plan_digest: event.plan_digest.clone(),
            slice_id: event.slice_id.clone(),
        };
        if let AgentEventKind::RunStarted { .. } = &event.event {
            if let Some(current) = self.latest_agent_run_for_identity(&identity)? {
                match current.status {
                    AgentRunStatus::Active => {
                        bail!(
                            "execution {} already has an active agent run {}",
                            identity.slice_id,
                            current.run_id
                        )
                    }
                    AgentRunStatus::Blocked if !retry => {
                        bail!(
                            "execution {} is blocked; retry the existing agent run",
                            identity.slice_id
                        )
                    }
                    AgentRunStatus::Blocked => {}
                    AgentRunStatus::Completed => {
                        bail!(
                            "execution {} already has a completed agent run",
                            identity.slice_id
                        )
                    }
                    AgentRunStatus::Abandoned => {}
                }
            } else if retry {
                bail!(
                    "retry requires an existing blocked agent run for execution {}",
                    identity.slice_id
                );
            }
        } else {
            let current = self.agent_run(&identity, &event.run_id)?;
            let allowed = match &event.event {
                AgentEventKind::PatchRecorded { .. }
                | AgentEventKind::BlockerRecorded { .. }
                | AgentEventKind::VerificationRecorded { .. } => {
                    matches!(current.status, AgentRunStatus::Active)
                }
                AgentEventKind::ScopeExpansionRequested { .. } => matches!(
                    current.status,
                    AgentRunStatus::Active | AgentRunStatus::Blocked
                ),
                AgentEventKind::RunAbandoned { .. } => matches!(
                    current.status,
                    AgentRunStatus::Active | AgentRunStatus::Blocked
                ),
                AgentEventKind::RunStarted { .. } => true,
            };
            if !allowed {
                bail!(
                    "agent event cannot be appended after run {} reaches {:?}",
                    event.run_id,
                    current.status
                );
            }
        }
        let mut canonical = event.clone();
        let supplied = canonical.event_digest.clone();
        canonical.event_digest.clear();
        let digest = Self::digest(&canonical)?;
        if !supplied.is_empty() && supplied != digest {
            bail!("agent event {} has an invalid digest", event.event_id);
        }
        canonical.event_digest = digest;
        let path = self.agent_event_path(&canonical);
        write_immutable_json(&path, &canonical)
    }

    pub fn agent_events(
        &self,
        identity: &ExecutionIdentity,
        run_id: &str,
    ) -> Result<Vec<AgentEvent>> {
        let mut events = Vec::new();
        for path in json_files(&self.agent_events_dir())? {
            let event: AgentEvent = read_json(&path)?;
            validate_agent_event_schema(&event)?;
            validate_agent_event_references(self, &event, false)?;
            validate_agent_event_digest(&path, &event)?;
            if event.run_id == run_id
                && event.plan_digest == identity.plan_digest
                && event.slice_id == identity.slice_id
            {
                events.push(event);
            }
        }
        events.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.event_id.cmp(&b.event_id))
        });
        Ok(events)
    }

    /// Reconstruct the authoritative state of an agent run from its immutable
    /// event stream. Callers must not trust a process-local `AgentRun` after a
    /// terminal event has been recorded.
    pub fn agent_run(&self, identity: &ExecutionIdentity, run_id: &str) -> Result<AgentRun> {
        let events = self.agent_events(identity, run_id)?;
        let started = events.iter().find_map(|event| match &event.event {
            AgentEventKind::RunStarted { run } => Some((**run).clone()),
            _ => None,
        });
        let mut run =
            started.ok_or_else(|| anyhow::anyhow!("agent run {run_id} was not started"))?;
        for event in events {
            match event.event {
                AgentEventKind::RunStarted { .. } => {}
                AgentEventKind::PatchRecorded { .. } => {
                    if !matches!(run.status, AgentRunStatus::Active) {
                        bail!("agent patch follows a non-active run {}", run.run_id);
                    }
                }
                AgentEventKind::ScopeExpansionRequested { .. } => {
                    if !matches!(run.status, AgentRunStatus::Active | AgentRunStatus::Blocked) {
                        bail!("scope expansion follows a completed run {}", run.run_id);
                    }
                }
                AgentEventKind::BlockerRecorded { .. } => {
                    if !matches!(run.status, AgentRunStatus::Active) {
                        bail!("blocker follows a non-active run {}", run.run_id);
                    }
                    run.status = AgentRunStatus::Blocked;
                }
                AgentEventKind::VerificationRecorded { attempt_id } => {
                    if !matches!(run.status, AgentRunStatus::Active) {
                        bail!("verification follows a non-active run {}", run.run_id);
                    }
                    run.status = match self.attempt(identity, &attempt_id)?.report.status {
                        CompletionStatus::Complete => AgentRunStatus::Completed,
                        CompletionStatus::Blocked => AgentRunStatus::Blocked,
                    };
                }
                AgentEventKind::RunAbandoned { reason } => {
                    if reason.trim().is_empty() {
                        bail!("agent abandonment requires a reason");
                    }
                    if !matches!(run.status, AgentRunStatus::Active | AgentRunStatus::Blocked) {
                        bail!("agent abandonment follows a terminal run {}", run.run_id);
                    }
                    run.status = AgentRunStatus::Abandoned;
                }
            }
        }
        Ok(run)
    }

    fn latest_agent_run_for_identity(
        &self,
        identity: &ExecutionIdentity,
    ) -> Result<Option<AgentRun>> {
        let mut started = self
            .agent_events_all()?
            .into_iter()
            .filter_map(|event| match event.event {
                AgentEventKind::RunStarted { run }
                    if run.plan_digest == identity.plan_digest
                        && run.slice_id == identity.slice_id =>
                {
                    Some((event.created_at, event.event_id, run.run_id.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        started.sort();
        started
            .last()
            .map(|(_, _, run_id)| self.agent_run(identity, run_id))
            .transpose()
    }

    pub fn latest_agent_run(&self) -> Result<Option<AgentRun>> {
        let mut started = self
            .agent_events_all()?
            .into_iter()
            .filter_map(|event| match event.event {
                AgentEventKind::RunStarted { run } => Some((
                    event.created_at,
                    event.event_id,
                    ExecutionIdentity {
                        plan_digest: run.plan_digest.clone(),
                        slice_id: run.slice_id.clone(),
                    },
                    run.run_id.clone(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        started.sort();
        started
            .last()
            .map(|(_, _, identity, run_id)| self.agent_run(identity, run_id))
            .transpose()
    }

    pub fn active_agent_runs(&self) -> Result<Vec<AgentRun>> {
        self.agent_runs_with_status(|status| matches!(status, AgentRunStatus::Active))
    }

    pub fn unresolved_agent_runs(&self) -> Result<Vec<AgentRun>> {
        self.agent_runs_with_status(|status| {
            matches!(status, AgentRunStatus::Active | AgentRunStatus::Blocked)
        })
    }

    fn agent_runs_with_status(
        &self,
        include: impl Fn(&AgentRunStatus) -> bool,
    ) -> Result<Vec<AgentRun>> {
        let mut started = self
            .agent_events_all()?
            .into_iter()
            .filter_map(|event| match event.event {
                AgentEventKind::RunStarted { run } => Some((
                    run.plan_digest.clone(),
                    run.slice_id.clone(),
                    run.run_id.clone(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        started.sort();
        started.dedup();
        let mut active = Vec::new();
        for (plan_digest, slice_id, run_id) in started {
            let identity = ExecutionIdentity {
                plan_digest,
                slice_id,
            };
            let run = self.agent_run(&identity, &run_id)?;
            if include(&run.status) {
                active.push(run);
            }
        }
        Ok(active)
    }

    fn agent_events_all(&self) -> Result<Vec<AgentEvent>> {
        let mut events = Vec::new();
        for path in json_files(&self.agent_events_dir())? {
            let event: AgentEvent = read_json(&path)?;
            validate_agent_event_schema(&event)?;
            validate_agent_event_references(self, &event, false)?;
            validate_agent_event_digest(&path, &event)?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn digest<T: Serialize>(value: &T) -> Result<String> {
        Self::digest_with_domain("syu/agent-event-digest/v1\0", value)
    }

    pub fn verification_digest<T: Serialize>(value: &T) -> Result<String> {
        Self::digest_with_domain(syu_work_model::VERIFICATION_RECEIPT_DIGEST_DOMAIN, value)
    }

    pub fn finalization_digest<T: Serialize>(value: &T) -> Result<String> {
        Self::digest_with_domain(syu_work_model::FINALIZATION_RECEIPT_DIGEST_DOMAIN, value)
    }

    fn digest_with_domain<T: Serialize>(domain: &str, value: &T) -> Result<String> {
        let bytes = syu_work_model::canonical_json_bytes(serde_json::to_value(value)?);
        let mut hash = Sha256::new();
        hash.update(domain.as_bytes());
        hash.update(bytes);
        Ok(format_sha256(hash.finalize()))
    }

    fn approvals_dir(&self) -> PathBuf {
        self.root.join("completion/v1/approvals")
    }
    fn attempts_dir(&self) -> PathBuf {
        self.root.join("completion/v1/attempts")
    }
    fn finalizations_dir(&self) -> PathBuf {
        self.root.join("completion/v1/finalizations")
    }
    fn agent_events_dir(&self) -> PathBuf {
        self.root.join("agent/v1/events")
    }
    fn approval_path(&self, identity: &ExecutionIdentity) -> PathBuf {
        self.approvals_dir()
            .join(component(&identity.plan_digest))
            .join(component(&identity.slice_id))
            .with_extension("json")
    }
    fn attempt_path(&self, attempt: &CompletionAttempt) -> PathBuf {
        self.attempts_dir()
            .join(component(&attempt.plan_digest))
            .join(component(&attempt.slice_id))
            .join(format!("{}.json", component(&attempt.attempt_id)))
    }
    fn finalization_path(&self, identity: &ExecutionIdentity, attempt_id: &str) -> PathBuf {
        self.finalizations_dir()
            .join(component(&identity.plan_digest))
            .join(component(&identity.slice_id))
            .join(format!("{}.json", component(attempt_id)))
    }
    fn agent_event_path(&self, event: &AgentEvent) -> PathBuf {
        self.agent_events_dir()
            .join(component(&event.plan_digest))
            .join(component(&event.slice_id))
            .join(format!("{}.json", component(&event.event_id)))
    }

    fn mutation_journal_path(&self) -> PathBuf {
        self.root.join("mutation-journal.json")
    }

    fn require_workspace(&self, workspace: &SpecWorkspace) -> Result<()> {
        if workspace.root.canonicalize()? != self.workspace_root.canonicalize()? {
            bail!("delivery evidence must use the store's workspace");
        }
        Ok(())
    }
}

fn component(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    lowercase_hex(hash.finalize())
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

fn write_atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    atomic_write(path, serde_json::to_vec_pretty(value)?)
}

fn relative_workspace_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("mutation path {} is outside the workspace", path.display()))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!(
            "mutation path {} is not a workspace-relative file",
            path.display()
        );
    }
    Ok(relative.to_string_lossy().into_owned())
}

fn require_schema(actual: &str, expected: &str, artifact: &str) -> Result<()> {
    if actual != expected {
        bail!("{artifact} schema must be {expected}");
    }
    Ok(())
}

fn validate_plan_approval_schema(approval: &PlanApproval) -> Result<()> {
    require_schema(
        approval.schema.as_str(),
        PLAN_APPROVAL_SCHEMA,
        "plan approval",
    )?;
    require_schema(
        approval.plan.schema.as_str(),
        WORK_PLAN_SCHEMA,
        "approved work plan",
    )?;
    if approval.plan_digest != approval.plan.canonical_digest
        || approval.plan_digest != work_plan_digest(&approval.plan)
        || approval.revision != approval.plan.basis.revision
        || approval.workspace_fingerprint != approval.plan.basis.workspace_fingerprint
        || approval.slice_id
            != approval
                .plan
                .slices
                .first()
                .map(|slice| slice.id.clone())
                .unwrap_or_default()
        || approval.plan.slices.len() != 1
    {
        bail!("plan approval nested work plan does not match its execution identity");
    }
    Ok(())
}

fn validate_completion_attempt_schema(attempt: &CompletionAttempt) -> Result<()> {
    require_schema(
        attempt.schema.as_str(),
        COMPLETION_ATTEMPT_SCHEMA,
        "completion attempt",
    )?;
    require_schema(
        attempt.report.schema.as_str(),
        COMPLETION_REPORT_SCHEMA,
        "completion report",
    )?;
    if attempt.approved_plan_digest != attempt.plan_digest
        || attempt.report.attempt_id != attempt.attempt_id
        || attempt.report.plan_digest != attempt.plan_digest
        || attempt.report.slice_id != attempt.slice_id
    {
        bail!("completion report does not match its attempt execution identity");
    }
    if let Some(receipt) = &attempt.receipt {
        require_schema(
            receipt.schema.as_str(),
            VERIFICATION_RECEIPT_SCHEMA,
            "verification receipt",
        )?;
        if receipt.plan_digest != attempt.plan_digest || receipt.slice_id != attempt.slice_id {
            bail!("verification receipt does not match its attempt execution identity");
        }
        if attempt.verification.status != syu_work_model::VerificationAttemptStatus::Complete
            || attempt.report.receipt_digest.as_deref()
                != Some(DeliveryStore::verification_digest(receipt)?.as_str())
        {
            bail!("completion attempt receipt is not fully bound to its report");
        }
    } else if attempt.verification.status != syu_work_model::VerificationAttemptStatus::Failed
        || attempt.report.receipt_digest.is_some()
    {
        bail!("failed completion attempt must not carry a verification receipt");
    }
    Ok(())
}

fn validate_completion_attempt_against_plan(
    attempt: &CompletionAttempt,
    plan: &syu_work_model::WorkPlan,
) -> Result<()> {
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == attempt.slice_id)
        .ok_or_else(|| {
            anyhow::anyhow!("completion attempt slice is absent from its approved plan")
        })?;

    let expected_executions = slice
        .verification_targets
        .iter()
        .map(|target| (target.reference.clone(), target.verification_claim.clone()))
        .collect::<BTreeSet<_>>();
    let actual_executions = attempt
        .receipt
        .as_ref()
        .map(|receipt| {
            receipt
                .executions
                .iter()
                .map(|execution| (execution.target.clone(), execution.claim.clone()))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    match (&attempt.verification.status, &attempt.receipt) {
        (syu_work_model::VerificationAttemptStatus::Complete, Some(receipt)) => {
            if receipt.executions.len() != expected_executions.len()
                || actual_executions != expected_executions
                || attempt.verification.executions.len() != receipt.executions.len()
                || attempt.verification.failure.is_some()
            {
                bail!("completion attempt verification execution set is not exact");
            }
            for execution in &receipt.executions {
                if execution.exit_code != 0
                    || execution.command.is_empty()
                    || execution.proof.matched_count == 0
                    || execution
                        .claim
                        .as_ref()
                        .is_some_and(|claim| claim.target != execution.target)
                {
                    bail!("completion attempt contains an invalid verification execution");
                }
                let Some(attempt_execution) =
                    attempt.verification.executions.iter().find(|candidate| {
                        candidate.target.as_ref() == Some(&execution.target)
                            && candidate.claim == execution.claim
                    })
                else {
                    bail!("completion attempt execution is not mirrored by its receipt");
                };
                if attempt_execution.runner != execution.runner
                    || attempt_execution.command != execution.command
                    || attempt_execution.exit_code != Some(execution.exit_code)
                    || attempt_execution.stdout_digest.as_deref()
                        != Some(execution.stdout_digest.as_str())
                    || attempt_execution.stderr_digest.as_deref()
                        != Some(execution.stderr_digest.as_str())
                    || attempt_execution.proof.as_ref() != Some(&execution.proof)
                    || attempt_execution.error.is_some()
                {
                    bail!("completion attempt execution evidence is inconsistent");
                }
            }
            let expected_lifecycle_refs = slice
                .editable_targets
                .iter()
                .map(|target| target.reference.clone())
                .collect::<BTreeSet<_>>();
            let actual_lifecycle_refs = receipt
                .lifecycle_proofs
                .iter()
                .map(|proof| proof.reference.clone())
                .collect::<BTreeSet<_>>();
            if receipt.lifecycle_proofs.len() != slice.editable_targets.len()
                || actual_lifecycle_refs != expected_lifecycle_refs
                || receipt.lifecycle_proofs.iter().any(|proof| {
                    slice
                        .editable_targets
                        .iter()
                        .find(|target| target.reference == proof.reference)
                        .is_none_or(|target| {
                            target.transition != proof.transition
                                || target.lifecycle != proof.lifecycle
                        })
                })
            {
                bail!("completion attempt lifecycle proof set is not exact");
            }
            if attempt.report.receipt_digest.as_deref()
                != Some(DeliveryStore::verification_digest(receipt)?.as_str())
            {
                bail!("completion report does not bind its verification receipt");
            }
        }
        (syu_work_model::VerificationAttemptStatus::Failed, None) => {
            if !attempt.verification.executions.is_empty() || attempt.verification.failure.is_none()
            {
                bail!("failed completion attempt must contain only its structured failure");
            }
            if attempt.report.status != CompletionStatus::Blocked
                || attempt.report.blockers.is_empty()
                || attempt.report.receipt_digest.is_some()
            {
                bail!("failed completion attempt must have a blocked report");
            }
        }
        _ => bail!("completion attempt verification state and receipt are inconsistent"),
    }

    match attempt.report.status {
        CompletionStatus::Complete => {
            if !attempt.report.blockers.is_empty()
                || attempt.report.checks.iter().any(|check| !check.passed)
                || !matches!(
                    attempt.verification.status,
                    syu_work_model::VerificationAttemptStatus::Complete
                )
            {
                bail!("complete completion report still contains blockers or failed checks");
            }
            validate_completion_report_against_slice(attempt, slice)?;
        }
        CompletionStatus::Blocked if attempt.report.blockers.is_empty() => {
            bail!("blocked completion report must explain its blocker");
        }
        CompletionStatus::Blocked => {}
    }
    Ok(())
}

fn validate_completion_report_against_slice(
    attempt: &CompletionAttempt,
    slice: &syu_work_model::ExecutionSlice,
) -> Result<()> {
    let expected_checks = slice
        .completion
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual_checks = attempt
        .report
        .checks
        .iter()
        .map(|check| serde_json::to_string(&check.check))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if attempt.report.checks.len() != expected_checks.len()
        || actual_checks != expected_checks
        || attempt
            .report
            .checks
            .iter()
            .any(|check| !check.passed || check.evidence.is_empty())
    {
        bail!("complete completion report checks do not exactly cover the selected slice");
    }

    let receipt = attempt
        .receipt
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("complete completion report has no receipt"))?;
    let executed_claims = receipt
        .executions
        .iter()
        .filter_map(|execution| execution.claim.clone())
        .collect::<BTreeSet<_>>();
    let mut expected_demonstrated = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    for acceptance in &slice.acceptance {
        let verification_targets = slice
            .verification_targets
            .iter()
            .filter(|target| {
                target.verification_claim.as_ref().is_some_and(|claim| {
                    claim.criterion == acceptance.anchor && executed_claims.contains(claim)
                })
            })
            .map(|target| target.reference.to_string())
            .collect::<BTreeSet<_>>();
        if verification_targets.is_empty()
            || expected_demonstrated
                .insert(
                    acceptance.anchor.to_string(),
                    (acceptance.statement.clone(), verification_targets),
                )
                .is_some()
        {
            bail!(
                "complete completion report cannot demonstrate every acceptance criterion exactly"
            );
        }
    }
    let mut actual_demonstrated = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    for evidence in &attempt.report.demonstrated {
        if actual_demonstrated
            .insert(
                evidence.anchor.to_string(),
                (
                    evidence.statement.clone(),
                    evidence
                        .verification_targets
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                ),
            )
            .is_some()
        {
            bail!("complete completion report demonstrates one criterion more than once");
        }
    }
    if actual_demonstrated != expected_demonstrated {
        bail!(
            "complete completion report demonstrated criteria do not exactly match the selected slice"
        );
    }
    Ok(())
}

fn validate_finalization_schema(receipt: &FinalizationReceipt) -> Result<()> {
    require_schema(
        receipt.schema.as_str(),
        FINALIZATION_RECEIPT_SCHEMA,
        "finalization receipt",
    )
}

fn validate_agent_event_schema(event: &AgentEvent) -> Result<()> {
    require_schema(event.schema.as_str(), AGENT_EVENT_SCHEMA, "agent event")?;
    match &event.event {
        AgentEventKind::RunStarted { run } => {
            require_schema(run.schema.as_str(), AGENT_RUN_SCHEMA, "agent run")?;
            if event.run_id != run.run_id
                || event.plan_digest != run.plan_digest
                || event.slice_id != run.slice_id
                || run.context.plan_digest != run.plan_digest
                || run.context.slice_id != run.slice_id
            {
                bail!("agent run does not match its enclosing event identity");
            }
            Ok(())
        }
        AgentEventKind::PatchRecorded { patch } => {
            require_schema(patch.schema.as_str(), AGENT_PATCH_SCHEMA, "agent patch")?;
            if event.run_id != patch.run_id
                || event.plan_digest != patch.plan_digest
                || event.slice_id != patch.slice_id
            {
                bail!("agent patch does not match its enclosing event identity");
            }
            Ok(())
        }
        AgentEventKind::BlockerRecorded { blocker } => {
            if blocker.code.trim().is_empty()
                || blocker.message.trim().is_empty()
                || blocker.next_action.trim().is_empty()
            {
                bail!("agent blocker event requires a complete blocker");
            }
            Ok(())
        }
        AgentEventKind::ScopeExpansionRequested { request } => {
            if event.run_id != request.run_id
                || event.plan_digest != request.plan_digest
                || event.slice_id != request.slice_id
                || request.requested_targets.is_empty()
            {
                bail!("scope expansion does not match its enclosing event identity");
            }
            Ok(())
        }
        AgentEventKind::VerificationRecorded { attempt_id } => {
            if attempt_id.trim().is_empty() {
                bail!("verification event requires an attempt id");
            }
            Ok(())
        }
        AgentEventKind::RunAbandoned { reason } => {
            if reason.trim().is_empty() {
                bail!("agent abandonment requires a reason");
            }
            Ok(())
        }
    }
}

fn validate_agent_event_references(
    store: &DeliveryStore,
    event: &AgentEvent,
    validate_current_context: bool,
) -> Result<()> {
    match &event.event {
        AgentEventKind::RunStarted { run } => {
            validate_run_started_reference(store, event, run, validate_current_context)?;
        }
        AgentEventKind::PatchRecorded { patch } => {
            if patch.status == syu_work_model::AgentPatchStatus::Accepted {
                if patch.changes.is_empty()
                    || patch.before_workspace_fingerprint.trim().is_empty()
                    || patch.after_workspace_fingerprint.trim().is_empty()
                    || patch.before_workspace_fingerprint == patch.after_workspace_fingerprint
                {
                    bail!("accepted agent patch must carry a non-empty workspace transition");
                }
            } else if !patch.changes.is_empty() || !patch.after_workspace_fingerprint.is_empty() {
                bail!("rejected agent patch must not claim workspace changes");
            }
            if validate_current_context {
                let workspace = SpecWorkspace::load(&store.workspace_root)?;
                let current = workspace.try_fingerprint()?;
                let expected = if patch.status == syu_work_model::AgentPatchStatus::Accepted {
                    &patch.after_workspace_fingerprint
                } else {
                    &patch.before_workspace_fingerprint
                };
                if expected != &current {
                    bail!("agent patch record is not bound to the current workspace fingerprint");
                }
            }
        }
        AgentEventKind::VerificationRecorded { attempt_id } => {
            let identity = ExecutionIdentity {
                plan_digest: event.plan_digest.clone(),
                slice_id: event.slice_id.clone(),
            };
            let attempt = store.attempt(&identity, attempt_id).with_context(|| {
                format!(
                    "verification event references attempt {attempt_id} outside its execution identity"
                )
            })?;
            if attempt.plan_digest != event.plan_digest || attempt.slice_id != event.slice_id {
                bail!("verification event attempt does not match its execution identity");
            }
            if attempt.agent_run_id.as_deref() != Some(event.run_id.as_str()) {
                bail!("verification event attempt is not bound to its originating agent run");
            }
            let run = store.persisted_run_for_event(event)?;
            if run.run_id != event.run_id
                || run.plan_digest != event.plan_digest
                || run.slice_id != event.slice_id
            {
                bail!("verification event run does not match its execution identity");
            }
        }
        AgentEventKind::RunAbandoned { reason } => {
            if reason.trim().is_empty() {
                bail!("agent abandonment requires a reason");
            }
            let run = store.persisted_run_for_event(event)?;
            if run.run_id != event.run_id
                || run.plan_digest != event.plan_digest
                || run.slice_id != event.slice_id
            {
                bail!("agent abandonment run does not match its execution identity");
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_run_started_reference(
    store: &DeliveryStore,
    event: &AgentEvent,
    run: &AgentRun,
    validate_current_context: bool,
) -> Result<()> {
    let identity = ExecutionIdentity {
        plan_digest: event.plan_digest.clone(),
        slice_id: event.slice_id.clone(),
    };
    let approval = store.approval(&identity).with_context(|| {
        format!(
            "agent run {} requires an approval for its plan and slice",
            run.run_id
        )
    })?;
    if run.approval_id != approval.approval_id
        || run.plan_digest != identity.plan_digest
        || run.slice_id != identity.slice_id
    {
        bail!("agent run does not reference its exact approval identity");
    }
    let expected = canonical_agent_context(store, &approval, run, validate_current_context)?;
    if run.context != expected {
        bail!("agent run context is not the canonical context for its approval");
    }
    for path in json_files(&store.agent_events_dir())? {
        let candidate: AgentEvent = read_json(&path)?;
        validate_agent_event_schema(&candidate)?;
        validate_agent_event_digest(&path, &candidate)?;
        if candidate.event_id != event.event_id
            && candidate.run_id == event.run_id
            && matches!(&candidate.event, AgentEventKind::RunStarted { .. })
        {
            bail!("agent run {} has duplicate RunStarted events", run.run_id);
        }
    }
    Ok(())
}

fn canonical_agent_context(
    store: &DeliveryStore,
    approval: &PlanApproval,
    run: &AgentRun,
    validate_current_context: bool,
) -> Result<AgentContextPack> {
    let slice = approval
        .plan
        .slices
        .iter()
        .find(|slice| slice.id == run.slice_id)
        .ok_or_else(|| anyhow::anyhow!("agent run slice is absent from its approved plan"))?;
    if !validate_current_context {
        if run.context.context.schema != CONTEXT_PACK_SCHEMA
            || run.context.context.plan_digest != run.plan_digest
            || run.context.context.slice_id != run.slice_id
            || run.context.context.basis != approval.plan.basis
        {
            bail!("persisted agent run context is not tied to its approved plan");
        }
        return Ok(AgentContextPack::from_slice(
            &run.plan_digest,
            run.context.context.clone(),
            slice,
        ));
    }
    let workspace = SpecWorkspace::load(&store.workspace_root)?;
    let index = workspace.index()?;
    let revision = repository_revision(&workspace.root)?;
    if approval.revision != revision
        || approval.workspace_fingerprint != workspace.try_fingerprint()?
    {
        bail!("agent run approval is stale for the current workspace");
    }
    let context =
        syu_planner::export_context(&approval.plan, &run.slice_id, &workspace, &index, &revision)?;
    Ok(AgentContextPack::from_slice(
        &run.plan_digest,
        context,
        slice,
    ))
}

impl DeliveryStore {
    fn persisted_run_for_event(&self, event: &AgentEvent) -> Result<AgentRun> {
        for path in json_files(&self.agent_events_dir())? {
            let candidate: AgentEvent = read_json(&path)?;
            validate_agent_event_schema(&candidate)?;
            validate_agent_event_digest(&path, &candidate)?;
            let AgentEventKind::RunStarted { ref run } = candidate.event else {
                continue;
            };
            if candidate.run_id != event.run_id
                || candidate.plan_digest != event.plan_digest
                || candidate.slice_id != event.slice_id
                || run.run_id != event.run_id
            {
                continue;
            }
            validate_run_started_reference(self, &candidate, run, false)?;
            return Ok((**run).clone());
        }
        bail!(
            "verification event references run {} without a persisted RunStarted event",
            event.run_id
        )
    }
}

fn validate_agent_event_digest(path: &Path, event: &AgentEvent) -> Result<()> {
    fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let mut digest = event.clone();
    let expected = digest.event_digest.clone();
    digest.event_digest.clear();
    let actual = DeliveryStore::digest(&digest)?;
    if expected != actual {
        bail!("agent event {} has an invalid digest", event.event_id);
    }
    Ok(())
}

fn validate_attempt_digest(attempt: &CompletionAttempt) -> Result<()> {
    let mut copy = attempt.clone();
    let expected = copy.attempt_digest.clone();
    copy.attempt_digest.clear();
    let actual = DeliveryStore::verification_digest(&copy)?;
    if expected != actual {
        bail!(
            "completion attempt {} has an invalid digest",
            attempt.attempt_id
        );
    }
    Ok(())
}

fn repository_revision(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        bail!("resolve workspace revision");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn write_immutable_json<T: Serialize + DeserializeOwned>(path: &Path, value: &T) -> Result<T> {
    if path.exists() {
        bail!("immutable evidence already exists at {}", path.display());
    }
    let parent = path.parent().context("evidence path has no parent")?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let bytes = serde_json::to_vec_pretty(value)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    if path.exists() {
        bail!("immutable evidence already exists at {}", path.display());
    }
    temporary
        .persist_noclobber(path)
        .map_err(|error| anyhow::anyhow!(error))?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn json_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(json_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    Ok(files)
}

fn attempt_plan(
    approval: &PlanApproval,
    attempt: &CompletionAttempt,
) -> Result<syu_work_model::WorkPlan> {
    if approval.plan_digest != attempt.approved_plan_digest
        || approval.plan_digest != attempt.plan_digest
    {
        bail!("attempt approval digest does not match");
    }
    Ok(approval.plan.clone())
}

fn preview_without_token(preview: &FinalizationPreview) -> FinalizationPreview {
    let mut copy = preview.clone();
    copy.preview_token.clear();
    copy
}

fn finalization_without_digest(receipt: &FinalizationReceipt) -> FinalizationReceipt {
    let mut copy = receipt.clone();
    copy.finalization_digest.clear();
    copy
}

fn changed_document_paths(workspace: &SpecWorkspace, items: &[SpecItemRef]) -> Result<Vec<String>> {
    let ids = items
        .iter()
        .map(|item| item.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let canonical_root = workspace.root.canonicalize()?;
    Ok(workspace
        .documents
        .iter()
        .filter(|document| match &document.document {
            SpecDocument::Requirements { requirements, .. } => {
                requirements.iter().any(|item| ids.contains(&item.id))
            }
            SpecDocument::Features { features, .. } => {
                features.iter().any(|item| ids.contains(&item.id))
            }
            _ => false,
        })
        .map(|document| {
            document
                .path
                .canonicalize()
                .ok()
                .and_then(|path| path.strip_prefix(&canonical_root).ok().map(PathBuf::from))
                .unwrap_or_else(|| document.path.clone())
                .to_string_lossy()
                .into_owned()
        })
        .collect())
}

fn validate_finalized_workspace(root: &Path) -> Result<SpecWorkspace> {
    let workspace = SpecWorkspace::load(root)?;
    let index = workspace.index()?;
    let result = syu_validation::validate(&syu_validation::ValidationContext {
        config: &workspace.config,
        workspace: &workspace,
        index: &index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: None,
        selected_slice: None,
        plan_mode: syu_validation::PlanValidationMode::PostState,
        preset: workspace.config.validation.preset,
        revision: None,
        change_base_revision: None,
    });
    if !result.is_valid() {
        bail!(
            "finalization overlay validation failed: {}",
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    Ok(workspace)
}

fn apply_status_overlay(
    workspace: &SpecWorkspace,
    items: &[SpecItemRef],
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let ids = items
        .iter()
        .map(|item| item.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut old = Vec::new();
    for loaded in &workspace.documents {
        let mut document = loaded.document.clone();
        let mut changed = false;
        match &mut document {
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    if ids.contains(&item.id) && item.status == ItemStatus::Planned {
                        item.status = ItemStatus::Implemented;
                        changed = true;
                    }
                }
            }
            SpecDocument::Features { features, .. } => {
                for item in features {
                    if ids.contains(&item.id) && item.status == ItemStatus::Planned {
                        item.status = ItemStatus::Implemented;
                        changed = true;
                    }
                }
            }
            _ => {}
        }
        if changed {
            let path = loaded.path.clone();
            let original = match fs::read(&path) {
                Ok(value) => value,
                Err(error) => {
                    return Err(rollback_or_error(error.into(), &old));
                }
            };
            let serialized = match serde_yaml::to_string(&document) {
                Ok(value) => value,
                Err(error) => {
                    return Err(rollback_or_error(error.into(), &old));
                }
            };
            old.push((path.clone(), original));
            if let Err(error) = atomic_write(&path, serialized) {
                // A previous document may already have been promoted. Restore
                // it before exposing the error so finalization is all-or-none.
                return Err(rollback_or_error(error, &old));
            }
        }
    }
    Ok(old)
}

fn rollback_or_error(error: anyhow::Error, old: &[(PathBuf, Vec<u8>)]) -> anyhow::Error {
    match restore_files(old) {
        Ok(()) => error,
        Err(restore) => anyhow::anyhow!("{error}; rollback failed: {restore}"),
    }
}

fn restore_files(old: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    for (path, bytes) in old {
        atomic_write(path, bytes)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    let parent = path.parent().context("file has no parent")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_ref())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| anyhow::anyhow!(error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Command};
    use syu_work_model::{
        AgentEvent, AgentEventKind, AgentPatchRecord, AgentPatchStatus, AgentRun,
        COMPLETION_ATTEMPT_SCHEMA, CONTEXT_PACK_SCHEMA, FinalizationReceipt, PLAN_APPROVAL_SCHEMA,
        WORK_REQUEST_SCHEMA, WorkOperation, WorkOrigin, WorkRequest,
    };

    fn workbench_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v1/valid-workbench-flow")
            .canonicalize()
            .expect("Workbench fixture root")
    }

    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("create dir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            let destination = to.join(entry.file_name());
            if entry.file_type().expect("file type").is_dir() {
                copy_dir(&path, &destination);
            } else {
                fs::copy(&path, &destination).expect("copy file");
            }
        }
    }

    fn init_git_repo(root: &Path) -> String {
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.name", "Codex"],
            vec!["config", "user.email", "codex@example.com"],
            vec!["add", "."],
            vec!["commit", "-qm", "baseline"],
        ] {
            let status = Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap();
            assert!(status.success(), "git command failed");
        }
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn fixture_plan(root: &Path, revision: &str) -> syu_work_model::WorkPlan {
        let workspace = SpecWorkspace::load(root).unwrap();
        let index = workspace.index().unwrap();
        syu_planner::plan(
            &WorkRequest {
                schema: WORK_REQUEST_SCHEMA.into(),
                id: "WORK-DELIVERY-TEST".into(),
                title: "modify fixture behavior".into(),
                operation: WorkOperation::Modify,
                origin: WorkOrigin::RequirementCriterion {
                    criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                },
                constraints: Default::default(),
                requested_targets: vec![],
            },
            &workspace,
            &index,
            revision,
        )
        .unwrap()
    }

    fn fixture_approval(
        workspace: &SpecWorkspace,
        plan: &syu_work_model::WorkPlan,
        revision: &str,
    ) -> PlanApproval {
        PlanApproval {
            schema: PLAN_APPROVAL_SCHEMA.into(),
            approval_id: "approval-test".into(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id: plan.slices[0].id.clone(),
            workspace_fingerprint: workspace.try_fingerprint().unwrap(),
            revision: revision.into(),
            reviewed_at: "0".into(),
            plan: plan.clone(),
        }
    }

    fn fixture_attempt(
        workspace: &SpecWorkspace,
        plan: &syu_work_model::WorkPlan,
        _approval: &PlanApproval,
        revision: &str,
    ) -> CompletionAttempt {
        let attempt_id = "attempt-test";
        let index = workspace.index().unwrap();
        let (verification, receipt, mut report) = syu_validation::execute_verification_attempt(
            workspace,
            &index,
            plan,
            &plan.slices[0].id,
            revision,
            attempt_id,
        )
        .unwrap();
        report.attempt_id = attempt_id.into();
        let mut attempt = CompletionAttempt {
            schema: COMPLETION_ATTEMPT_SCHEMA.into(),
            attempt_id: attempt_id.into(),
            attempt_digest: String::new(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id: plan.slices[0].id.clone(),
            agent_run_id: None,
            approved_plan_digest: plan.canonical_digest.clone(),
            started_at: "0".into(),
            completed_at: "1".into(),
            verification,
            receipt,
            report,
        };
        attempt.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&attempt)).unwrap();
        attempt
    }

    #[test]
    fn store_boundary_is_repository_local_and_explicit() {
        let root = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(root.path())
            .status()
            .unwrap();
        let store = DeliveryStore::for_workspace(root.path()).unwrap();
        assert!(!store.root().starts_with(root.path().join("docs")));
        assert!(store.root().display().to_string().contains("syu"));
        store.ensure().unwrap();
        assert!(store.approvals_dir().is_dir());
        assert!(store.attempts_dir().is_dir());
        assert!(store.finalizations_dir().is_dir());
        assert!(store.agent_events_dir().is_dir());
    }

    #[test]
    fn uncommitted_mutation_journal_restores_workspace_on_next_lock() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        init_git_repo(temp.path());
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let path = temp.path().join("src/lib.rs");
        let original = fs::read(&path).unwrap();
        let lock = store.lock_workspace().unwrap();
        fs::write(&path, b"partially-written").unwrap();
        store
            .write_mutation_journal(
                "agent-patch",
                "patch-crashed",
                vec![MutationJournalFile {
                    path: path.to_string_lossy().into_owned(),
                    original: Some(original.clone()),
                }],
                Vec::new(),
            )
            .unwrap();
        drop(lock);

        let _recovery_lock = store.lock_workspace().unwrap();
        assert_eq!(fs::read(path).unwrap(), original);
        assert!(!store.mutation_journal_path().exists());
    }

    #[test]
    fn receipt_digest_domains_have_literal_vectors() {
        let value = serde_json::json!({
            "attempt_id": "attempt-1",
            "status": "complete"
        });
        assert_eq!(
            DeliveryStore::verification_digest(&value).unwrap(),
            "sha256:780571812383f90ca481a3c73f061a3d14fe4072e5f7041dbfe0afbd0396045f"
        );
        assert_eq!(
            DeliveryStore::finalization_digest(&value).unwrap(),
            "sha256:0ceffac2ca2472e2edc56ad1fc83efbc300de44820369343a88e974605872ace"
        );
    }

    #[test]
    fn approvals_require_canonical_scope_and_are_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = fixture_approval(&workspace, &plan, &revision);

        let mut invalid = approval.clone();
        invalid.plan_digest = "sha256:not-the-canonical-plan".into();
        assert!(store.approve(&invalid).is_err());

        assert_eq!(store.approve(&approval).unwrap(), approval);
        assert_eq!(store.approve(&approval).unwrap(), approval);
        assert_eq!(
            store
                .approval(&ExecutionIdentity {
                    plan_digest: approval.plan_digest.clone(),
                    slice_id: approval.slice_id.clone(),
                })
                .unwrap(),
            approval
        );
    }

    #[test]
    fn durable_artifacts_require_their_declared_v1_schemas() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = fixture_approval(&workspace, &plan, &revision);
        store.approve(&approval).unwrap();

        let mut invalid_approval = approval.clone();
        invalid_approval.schema = "syu/legacy-approval/v0".into();
        assert!(store.approve(&invalid_approval).is_err());
        let mut tampered_approval = approval.clone();
        tampered_approval.plan.request.constraints.max_slices = Some(2);
        assert!(store.approve(&tampered_approval).is_err());

        let mut attempt = fixture_attempt(&workspace, &plan, &approval, &revision);
        attempt.agent_run_id = Some("run-start-test".into());
        attempt.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&attempt)).unwrap();
        let mut invalid_attempt = attempt.clone();
        invalid_attempt.schema = "syu/legacy-attempt/v0".into();
        assert!(store.append_attempt(&workspace, &invalid_attempt).is_err());
        let mut invalid_report_identity = attempt.clone();
        invalid_report_identity.report.plan_digest = "sha256:other-plan".into();
        invalid_report_identity.attempt_digest = String::new();
        invalid_report_identity.attempt_digest =
            DeliveryStore::verification_digest(&invalid_report_identity).unwrap();
        assert!(
            store
                .append_attempt(&workspace, &invalid_report_identity)
                .is_err()
        );
        let mut invalid_attempt_identity = attempt.clone();
        invalid_attempt_identity.approved_plan_digest = "sha256:other-plan".into();
        invalid_attempt_identity.attempt_digest = DeliveryStore::verification_digest(
            &attempt_with_empty_digest(&invalid_attempt_identity),
        )
        .unwrap();
        assert!(
            store
                .append_attempt(&workspace, &invalid_attempt_identity)
                .is_err()
        );
        let mut invalid_report_attempt = attempt.clone();
        invalid_report_attempt.report.attempt_id = "attempt-other".into();
        invalid_report_attempt.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&invalid_report_attempt))
                .unwrap();
        assert!(
            store
                .append_attempt(&workspace, &invalid_report_attempt)
                .is_err()
        );

        let mut invalid_execution_set = attempt.clone();
        invalid_execution_set
            .receipt
            .as_mut()
            .unwrap()
            .executions
            .clear();
        invalid_execution_set.verification.executions.clear();
        invalid_execution_set.report.receipt_digest = Some(
            DeliveryStore::verification_digest(invalid_execution_set.receipt.as_ref().unwrap())
                .unwrap(),
        );
        invalid_execution_set.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&invalid_execution_set))
                .unwrap();
        assert!(
            store
                .append_attempt(&workspace, &invalid_execution_set)
                .is_err()
        );

        let identity = ExecutionIdentity {
            plan_digest: plan.canonical_digest.clone(),
            slice_id: plan.slices[0].id.clone(),
        };
        let invalid_finalization = FinalizationReceipt {
            schema: "syu/legacy-finalization/v0".into(),
            finalization_id: "finalization-test".into(),
            finalization_digest: String::new(),
            attempt_id: attempt.attempt_id.clone(),
            attempt_digest: attempt.attempt_digest.clone(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            pre_workspace_fingerprint: String::new(),
            post_workspace_fingerprint: String::new(),
            promoted_items: vec![],
            changed_files: vec![],
            lifecycle_proofs: vec![],
            completed_at: String::new(),
        };
        assert!(store.append_finalization(&invalid_finalization).is_err());

        let invalid_event = AgentEvent {
            schema: "syu/legacy-agent-event/v0".into(),
            event_id: "event-test".into(),
            event_digest: String::new(),
            run_id: "run-test".into(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: String::new(),
            event: AgentEventKind::VerificationRecorded {
                attempt_id: attempt.attempt_id.clone(),
            },
        };
        assert!(store.append_agent_event(&invalid_event).is_err());
        let missing_attempt_event = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-missing-attempt".into(),
            event_digest: String::new(),
            run_id: "run-test".into(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: String::new(),
            event: AgentEventKind::VerificationRecorded {
                attempt_id: "attempt-missing".into(),
            },
        };
        assert!(store.append_agent_event(&missing_attempt_event).is_err());
        let invalid_nested_patch = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-patch-test".into(),
            event_digest: String::new(),
            run_id: "run-test".into(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: String::new(),
            event: AgentEventKind::PatchRecorded {
                patch: AgentPatchRecord {
                    schema: syu_work_model::AGENT_PATCH_SCHEMA.into(),
                    patch_id: "patch-test".into(),
                    run_id: "different-run".into(),
                    plan_digest: "sha256:different-plan".into(),
                    slice_id: "different-slice".into(),
                    status: AgentPatchStatus::Rejected,
                    writes: vec![],
                    changes: vec![],
                    before_workspace_fingerprint: String::new(),
                    after_workspace_fingerprint: String::new(),
                    blockers: vec![],
                    created_at: String::new(),
                },
            },
        };
        assert!(store.append_agent_event(&invalid_nested_patch).is_err());

        let run_json = |approval_id: &str| {
            serde_json::json!({
                "schema": syu_work_model::AGENT_RUN_SCHEMA,
                "run_id": "run-start-test",
                "approval_id": approval_id,
                "plan_digest": identity.plan_digest.clone(),
                "slice_id": identity.slice_id.clone(),
                "status": "active",
                "created_at": "0",
                "context": {
                    "schema": "syu/agent-context/v1",
                    "plan_digest": identity.plan_digest.clone(),
                    "slice_id": identity.slice_id.clone(),
                    "context": {
                        "schema": CONTEXT_PACK_SCHEMA,
                        "plan_digest": identity.plan_digest.clone(),
                        "slice_id": identity.slice_id.clone(),
                        "basis": {
                            "revision": revision.clone(),
                            "workspace_fingerprint": approval.workspace_fingerprint.clone(),
                            "spec_fingerprint": "",
                            "ownership_fingerprint": "",
                            "readonly_fingerprint": ""
                        },
                        "instructions": { "goal": "", "non_goals": [] },
                        "spec_context": [],
                        "artifact_context": [],
                        "completion": []
                    },
                    "budget": {
                        "editable_files": 0,
                        "editable_symbols": 0,
                        "verification_targets": 0,
                        "readonly_targets": 0,
                        "total_bytes": 0
                    },
                    "editable_targets": [],
                    "verification_targets": [],
                    "readonly_targets": []
                }
            })
        };
        let invalid_run: AgentRun = serde_json::from_value(run_json("approval-other")).unwrap();
        let invalid_start = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-invalid-run-start".into(),
            event_digest: String::new(),
            run_id: invalid_run.run_id.clone(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: "0".into(),
            event: AgentEventKind::RunStarted {
                run: Box::new(invalid_run),
            },
        };
        assert!(store.append_agent_event(&invalid_start).is_err());

        let mut valid_run: AgentRun =
            serde_json::from_value(run_json(&approval.approval_id)).unwrap();
        valid_run.context = canonical_agent_context(&store, &approval, &valid_run, true).unwrap();
        let valid_start = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-valid-run-start".into(),
            event_digest: String::new(),
            run_id: valid_run.run_id.clone(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: "1".into(),
            event: AgentEventKind::RunStarted {
                run: Box::new(valid_run),
            },
        };
        store.append_agent_event(&valid_start).unwrap();
        let mut duplicate_start = valid_start.clone();
        duplicate_start.event_id = "event-duplicate-run-start".into();
        assert!(store.append_agent_event(&duplicate_start).is_err());
        let mut concurrent_start = valid_start.clone();
        concurrent_start.event_id = "event-concurrent-run-start".into();
        concurrent_start.run_id = "run-concurrent-start".into();
        if let AgentEventKind::RunStarted { run } = &mut concurrent_start.event {
            let mut replacement = (**run).clone();
            replacement.run_id = concurrent_start.run_id.clone();
            *run = Box::new(replacement);
        }
        assert!(store.append_agent_event(&concurrent_start).is_err());

        assert_eq!(attempt.report.status, CompletionStatus::Blocked);
        store.append_attempt(&workspace, &attempt).unwrap();
        store
            .append_agent_event(&AgentEvent {
                schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
                event_id: "event-verification-test".into(),
                event_digest: String::new(),
                run_id: "run-start-test".into(),
                plan_digest: identity.plan_digest.clone(),
                slice_id: identity.slice_id.clone(),
                created_at: "2".into(),
                event: AgentEventKind::VerificationRecorded {
                    attempt_id: attempt.attempt_id.clone(),
                },
            })
            .unwrap();
        assert_eq!(
            store.agent_run(&identity, "run-start-test").unwrap().status,
            AgentRunStatus::Blocked
        );
        let late_blocker = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-late-blocker".into(),
            event_digest: String::new(),
            run_id: "run-start-test".into(),
            plan_digest: identity.plan_digest.clone(),
            slice_id: identity.slice_id.clone(),
            created_at: "3".into(),
            event: AgentEventKind::BlockerRecorded {
                blocker: syu_work_model::AgentBlocker {
                    code: "late".into(),
                    message: "must be rejected".into(),
                    next_action: "start a new run".into(),
                },
            },
        };
        assert!(store.append_agent_event(&late_blocker).is_err());
        let mut retry_run = match &valid_start.event {
            AgentEventKind::RunStarted { run } => (**run).clone(),
            _ => unreachable!("valid_start is a RunStarted event"),
        };
        retry_run.run_id = "run-retry-test".into();
        retry_run.status = AgentRunStatus::Active;
        let retry_event = AgentEvent {
            schema: syu_work_model::AGENT_EVENT_SCHEMA.into(),
            event_id: "event-retry-run-start".into(),
            event_digest: String::new(),
            run_id: retry_run.run_id.clone(),
            plan_digest: identity.plan_digest,
            slice_id: identity.slice_id,
            created_at: "4".into(),
            event: AgentEventKind::RunStarted {
                run: Box::new(retry_run),
            },
        };
        store
            .append_agent_event_for_retry(&retry_event, true)
            .unwrap();
    }

    #[test]
    fn immutable_attempts_validate_digests_and_preserve_history() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = store
            .approve(&fixture_approval(&workspace, &plan, &revision))
            .unwrap();
        let attempt = fixture_attempt(&workspace, &plan, &approval, &revision);

        assert_eq!(store.append_attempt(&workspace, &attempt).unwrap(), attempt);
        assert_eq!(
            store
                .attempt(
                    &ExecutionIdentity {
                        plan_digest: attempt.plan_digest.clone(),
                        slice_id: attempt.slice_id.clone(),
                    },
                    &attempt.attempt_id,
                )
                .unwrap(),
            attempt
        );
        assert_eq!(store.attempts().unwrap(), vec![attempt.clone()]);
        assert!(store.append_attempt(&workspace, &attempt).is_err());

        let mut tampered = attempt;
        tampered.completed_at = "later".into();
        assert!(store.append_attempt(&workspace, &tampered).is_err());
    }

    #[test]
    fn finalization_preview_requires_complete_attempt() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = store
            .approve(&fixture_approval(&workspace, &plan, &revision))
            .unwrap();
        let attempt = fixture_attempt(&workspace, &plan, &approval, &revision);
        let attempt = store.append_attempt(&workspace, &attempt).unwrap();

        let preview = store.finalization_preview(&workspace, &attempt).unwrap();
        assert_eq!(preview.status, CompletionStatus::Blocked);
        assert!(
            preview
                .blockers
                .iter()
                .all(|blocker| blocker.code != "SYU-FINALIZE-STALE-EVIDENCE")
        );

        let mut incomplete = attempt.clone();
        incomplete.attempt_id = "attempt-incomplete".into();
        incomplete.attempt_digest.clear();
        incomplete.verification.status = syu_work_model::VerificationAttemptStatus::Failed;
        incomplete.verification.executions.clear();
        incomplete.verification.failure = Some(syu_work_model::VerificationAttemptFailure {
            code: "SYU-FIXTURE-INCOMPLETE".into(),
            message: "fixture incomplete attempt".into(),
            next_action: "rerun verification".into(),
        });
        incomplete.receipt = None;
        incomplete.report.attempt_id = incomplete.attempt_id.clone();
        incomplete.report.receipt_digest = None;
        incomplete.report.status = CompletionStatus::Blocked;
        incomplete.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&incomplete)).unwrap();
        let incomplete = store.append_attempt(&workspace, &incomplete).unwrap();
        let preview = store.finalization_preview(&workspace, &incomplete).unwrap();
        assert_eq!(preview.status, CompletionStatus::Blocked);
        assert!(
            preview
                .blockers
                .iter()
                .any(|blocker| blocker.code == "SYU-FINALIZE-INCOMPLETE")
        );
    }

    #[test]
    fn complete_attempt_requires_exact_completion_report_evidence() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = store
            .approve(&fixture_approval(&workspace, &plan, &revision))
            .unwrap();
        let mut complete = fixture_attempt(&workspace, &plan, &approval, &revision);
        let slice = &plan.slices[0];
        let executed_claims = complete
            .receipt
            .as_ref()
            .unwrap()
            .executions
            .iter()
            .filter_map(|execution| execution.claim.clone())
            .collect::<BTreeSet<_>>();
        complete.report.status = CompletionStatus::Complete;
        complete.report.blockers.clear();
        complete.report.checks = slice
            .completion
            .iter()
            .cloned()
            .map(|check| syu_work_model::CompletionCheckEvidence {
                check,
                passed: true,
                evidence: vec!["fixture evidence".into()],
            })
            .collect();
        complete.report.demonstrated = slice
            .acceptance
            .iter()
            .map(|acceptance| syu_work_model::CompletionCriterionEvidence {
                anchor: acceptance.anchor.clone(),
                statement: acceptance.statement.clone(),
                verification_targets: slice
                    .verification_targets
                    .iter()
                    .filter(|target| {
                        target.verification_claim.as_ref().is_some_and(|claim| {
                            claim.criterion == acceptance.anchor && executed_claims.contains(claim)
                        })
                    })
                    .map(|target| target.reference.clone())
                    .collect(),
            })
            .collect();
        complete.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&complete)).unwrap();
        assert!(store.append_attempt(&workspace, &complete).is_ok());

        let mut invalid = complete.clone();
        invalid.attempt_id = "attempt-invalid-report".into();
        invalid.report.attempt_id = invalid.attempt_id.clone();
        invalid.report.demonstrated.clear();
        invalid.attempt_digest =
            DeliveryStore::verification_digest(&attempt_with_empty_digest(&invalid)).unwrap();
        assert!(store.append_attempt(&workspace, &invalid).is_err());
    }

    fn attempt_with_empty_digest(attempt: &CompletionAttempt) -> CompletionAttempt {
        let mut copy = attempt.clone();
        copy.attempt_digest.clear();
        copy
    }
}
