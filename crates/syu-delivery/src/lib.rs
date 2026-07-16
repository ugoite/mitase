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
        let post_workspace_fingerprint = match SpecWorkspace::load(&workspace.root)
            .and_then(|candidate| candidate.index().map(|_| candidate))
        {
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
            old.push((path.clone(), fs::read(&path)?));
            atomic_write(&path, &serde_yaml::to_string(&document)?)?;
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
    use std::process::Command;

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
        assert_eq!(
            syu_work_model::CompletionStatus::Blocked,
            syu_work_model::CompletionStatus::Blocked
        );
    }
}
