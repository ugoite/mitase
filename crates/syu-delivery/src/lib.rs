#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use syu_spec_model::{ItemStatus, SpecDocument, SpecItemRef};
use syu_work_model::{
    CompletionAttempt, CompletionBlocker, CompletionStatus, FINALIZATION_RECEIPT_SCHEMA,
    FinalizationPreview, FinalizationReceipt, PlanApproval,
};
use syu_workspace::SpecWorkspace;
use uuid::Uuid;

pub const STORE_SCHEMA: &str = "syu/completion/v1";

#[derive(Debug, Clone)]
pub struct DeliveryStore {
    root: PathBuf,
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
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> Result<()> {
        for path in [
            self.approvals_dir(),
            self.attempts_dir(),
            self.finalizations_dir(),
        ] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    pub fn approve(&self, approval: &PlanApproval) -> Result<PlanApproval> {
        self.ensure()?;
        if approval.plan_digest != approval.plan.canonical_digest {
            bail!("approval plan digest does not match its canonical plan");
        }
        let path = self.approval_path(&approval.plan_digest);
        if path.exists() {
            let existing: PlanApproval = read_json(&path)?;
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

    pub fn approval(&self, plan_digest: &str) -> Result<PlanApproval> {
        let approval: PlanApproval = read_json(&self.approval_path(plan_digest))?;
        if approval.plan_digest != plan_digest
            || approval.plan_digest != approval.plan.canonical_digest
        {
            bail!("stored approval for {plan_digest} is invalid");
        }
        Ok(approval)
    }

    pub fn append_attempt(&self, attempt: &CompletionAttempt) -> Result<CompletionAttempt> {
        self.ensure()?;
        validate_attempt_digest(attempt)?;
        let path = self.attempt_path(attempt);
        write_immutable_json(&path, attempt)
    }

    pub fn attempt(&self, attempt_id: &str) -> Result<CompletionAttempt> {
        for path in json_files(&self.attempts_dir())? {
            let value: CompletionAttempt = read_json(&path)?;
            validate_attempt_digest(&value)?;
            if value.attempt_id == attempt_id {
                return Ok(value);
            }
        }
        bail!("completion attempt {attempt_id} not found")
    }

    pub fn attempts(&self) -> Result<Vec<CompletionAttempt>> {
        let mut values = Vec::new();
        for path in json_files(&self.attempts_dir())? {
            let value = read_json::<CompletionAttempt>(&path)?;
            validate_attempt_digest(&value)?;
            values.push(value);
        }
        values.sort_by(|a, b| {
            b.completed_at
                .cmp(&a.completed_at)
                .then_with(|| b.attempt_id.cmp(&a.attempt_id))
        });
        Ok(values)
    }

    pub fn append_finalization(
        &self,
        receipt: &FinalizationReceipt,
    ) -> Result<FinalizationReceipt> {
        self.ensure()?;
        let path = self.finalization_path(&receipt.attempt_id);
        if path.exists() {
            return read_json(&path);
        }
        write_immutable_json(&path, receipt)
    }

    pub fn finalization(&self, attempt_id: &str) -> Result<Option<FinalizationReceipt>> {
        let path = self.finalization_path(attempt_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }

    pub fn finalization_preview(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
    ) -> Result<FinalizationPreview> {
        let approval = self.approval(&attempt.plan_digest)?;
        if approval.plan != attempt_plan(&approval, attempt)? {
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
        preview.preview_token = Self::digest(&preview_without_token(&preview))?;
        Ok(preview)
    }

    pub fn apply_finalization(
        &self,
        workspace: &SpecWorkspace,
        attempt: &CompletionAttempt,
        preview: &FinalizationPreview,
        token: &str,
    ) -> Result<FinalizationReceipt> {
        if preview.preview_token != token || preview.status != CompletionStatus::Complete {
            bail!("finalization preview token is stale or blocked");
        }
        let current = self.finalization_preview(workspace, attempt)?;
        if current.preview_token != token
            || current.pre_workspace_fingerprint != preview.pre_workspace_fingerprint
        {
            bail!("workspace changed after finalization preview; preview again");
        }
        if let Some(existing) = self.finalization(&attempt.attempt_id)? {
            return Ok(existing);
        }
        let old = apply_status_overlay(workspace, &preview.promoted_items)?;
        let post_workspace_fingerprint = match validate_finalized_workspace(&workspace.root) {
            Ok(candidate) => candidate.try_fingerprint()?,
            Err(error) => {
                restore_files(&old)?;
                return Err(error);
            }
        };
        let receipt = FinalizationReceipt {
            schema: FINALIZATION_RECEIPT_SCHEMA.into(),
            finalization_id: self.new_id("finalization"),
            attempt_id: attempt.attempt_id.clone(),
            attempt_digest: attempt.attempt_digest.clone(),
            plan_digest: attempt.plan_digest.clone(),
            slice_id: attempt.slice_id.clone(),
            pre_workspace_fingerprint: preview.pre_workspace_fingerprint.clone(),
            post_workspace_fingerprint,
            promoted_items: preview.promoted_items.clone(),
            changed_files: preview.changed_files.clone(),
            completed_at: now_nanos().to_string(),
        };
        match self.append_finalization(&receipt) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                restore_files(&old)?;
                Err(error)
            }
        }
    }

    pub fn new_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}-{}", now_nanos(), Uuid::new_v4())
    }

    pub fn digest<T: Serialize>(value: &T) -> Result<String> {
        let bytes = serde_json::to_vec(value)?;
        let mut hash = Sha256::new();
        hash.update(bytes);
        Ok(format!("sha256:{:x}", hash.finalize()))
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
    fn approval_path(&self, digest: &str) -> PathBuf {
        self.approvals_dir()
            .join(component(digest))
            .with_extension("json")
    }
    fn attempt_path(&self, attempt: &CompletionAttempt) -> PathBuf {
        self.attempts_dir()
            .join(component(&attempt.plan_digest))
            .join(component(&attempt.slice_id))
            .join(format!("{}.json", component(&attempt.attempt_id)))
    }
    fn finalization_path(&self, attempt_id: &str) -> PathBuf {
        self.finalizations_dir()
            .join(format!("{}.json", component(attempt_id)))
    }
}

fn component(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
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

fn validate_attempt_digest(attempt: &CompletionAttempt) -> Result<()> {
    let mut copy = attempt.clone();
    let expected = copy.attempt_digest.clone();
    copy.attempt_digest.clear();
    if expected != DeliveryStore::digest(&copy)? {
        bail!(
            "completion attempt {} has an invalid digest",
            attempt.attempt_id
        );
    }
    Ok(())
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

fn changed_document_paths(workspace: &SpecWorkspace, items: &[SpecItemRef]) -> Result<Vec<String>> {
    let ids = items
        .iter()
        .map(|item| item.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
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
                .strip_prefix(&workspace.root)
                .unwrap_or(&document.path)
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
                    let _ = restore_files(&old);
                    return Err(error.into());
                }
            };
            let serialized = match serde_yaml::to_string(&document) {
                Ok(value) => value,
                Err(error) => {
                    let _ = restore_files(&old);
                    return Err(error.into());
                }
            };
            old.push((path.clone(), original));
            if let Err(error) = atomic_write(&path, serialized) {
                // A previous document may already have been promoted. Restore
                // it before exposing the error so finalization is all-or-none.
                let _ = restore_files(&old);
                return Err(error);
            }
        }
    }
    Ok(old)
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
        COMPLETION_ATTEMPT_SCHEMA, COMPLETION_REPORT_SCHEMA, CompletionReport,
        PLAN_APPROVAL_SCHEMA, VERIFICATION_RECEIPT_SCHEMA, VerificationAttemptResult,
        VerificationAttemptStatus, VerificationReceipt, WORK_REQUEST_SCHEMA, WorkOperation,
        WorkRequest, WorkSeed,
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
                summary: "modify fixture behavior".into(),
                operation: WorkOperation::Modify,
                seeds: vec![WorkSeed::Anchor(
                    "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
                )],
                constraints: Default::default(),
                requested_targets: vec![],
            },
            &workspace,
            &index,
            revision,
        )
        .unwrap()
    }

    #[test]
    fn store_is_outside_worktree() {
        let root = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(root.path())
            .status()
            .unwrap();
        let store = DeliveryStore::for_workspace(root.path()).unwrap();
        assert!(!store.root().starts_with(root.path().join("docs")));
        assert!(store.root().display().to_string().contains("syu"));
    }

    #[test]
    fn finalization_preview_requires_complete_attempt() {
        let temp = tempfile::tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), temp.path());
        let revision = init_git_repo(temp.path());
        let workspace = SpecWorkspace::load(temp.path()).unwrap();
        let plan = fixture_plan(temp.path(), &revision);
        let slice_id = plan.slices[0].id.clone();
        let store = DeliveryStore::for_workspace(temp.path()).unwrap();
        let approval = store
            .approve(&PlanApproval {
                schema: PLAN_APPROVAL_SCHEMA.into(),
                approval_id: "approval-test".into(),
                plan_digest: plan.canonical_digest.clone(),
                workspace_fingerprint: workspace.try_fingerprint().unwrap(),
                revision: revision.clone(),
                reviewed_at: "0".into(),
                plan: plan.clone(),
            })
            .unwrap();
        let receipt = VerificationReceipt {
            schema: VERIFICATION_RECEIPT_SCHEMA.into(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id: slice_id.clone(),
            revision,
            workspace_fingerprint: "sha256:stale".into(),
            started_at: "0".into(),
            completed_at: "1".into(),
            executions: vec![],
        };
        let mut attempt = CompletionAttempt {
            schema: COMPLETION_ATTEMPT_SCHEMA.into(),
            attempt_id: "attempt-test".into(),
            attempt_digest: String::new(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id,
            approved_plan_digest: approval.plan_digest,
            started_at: "0".into(),
            completed_at: "1".into(),
            verification: VerificationAttemptResult {
                status: VerificationAttemptStatus::Complete,
                executions: vec![],
                failure: None,
            },
            receipt: Some(receipt),
            report: CompletionReport {
                schema: COMPLETION_REPORT_SCHEMA.into(),
                attempt_id: "attempt-test".into(),
                plan_digest: plan.canonical_digest.clone(),
                slice_id: plan.slices[0].id.clone(),
                receipt_digest: None,
                status: CompletionStatus::Complete,
                demonstrated: vec![],
                checks: vec![],
                blockers: vec![],
            },
        };
        attempt.attempt_digest =
            DeliveryStore::digest(&attempt_with_empty_digest(&attempt)).unwrap();
        let attempt = store.append_attempt(&attempt).unwrap();

        let preview = store.finalization_preview(&workspace, &attempt).unwrap();
        assert_eq!(preview.status, CompletionStatus::Blocked);
        assert!(
            preview
                .blockers
                .iter()
                .any(|blocker| blocker.code == "SYU-FINALIZE-STALE-EVIDENCE")
        );

        let mut incomplete = attempt.clone();
        incomplete.attempt_id = "attempt-incomplete".into();
        incomplete.attempt_digest.clear();
        incomplete.verification.status = syu_work_model::VerificationAttemptStatus::Failed;
        incomplete.receipt = None;
        incomplete.report.attempt_id = incomplete.attempt_id.clone();
        incomplete.report.status = CompletionStatus::Blocked;
        incomplete.attempt_digest =
            DeliveryStore::digest(&attempt_with_empty_digest(&incomplete)).unwrap();
        let incomplete = store.append_attempt(&incomplete).unwrap();
        let preview = store.finalization_preview(&workspace, &incomplete).unwrap();
        assert_eq!(preview.status, CompletionStatus::Blocked);
        assert!(
            preview
                .blockers
                .iter()
                .any(|blocker| blocker.code == "SYU-FINALIZE-INCOMPLETE")
        );
    }

    fn attempt_with_empty_digest(attempt: &CompletionAttempt) -> CompletionAttempt {
        let mut copy = attempt.clone();
        copy.attempt_digest.clear();
        copy
    }
}
