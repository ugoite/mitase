#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use syu_code_intel::resolve_symbol;
use syu_inventory::{ArtifactUnit, ArtifactUnitKind, InventoryContext, InventoryRegistry};
use syu_project_model::{CONFIG_SCHEMA, ProjectConfig};
use syu_spec_model::*;

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub document: SpecDocument,
}
#[derive(Clone)]
pub struct SpecWorkspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub documents: Vec<LoadedDocument>,
    /// Candidate bytes used by every overlay-aware reader.
    pub overlays: BTreeMap<PathBuf, Vec<u8>>,
    matcher: WorkspaceMatcher,
    fingerprint_cache: Arc<OnceLock<Result<String, String>>>,
}
#[derive(Debug, Clone)]
struct WorkspaceMatcher {
    spec_roots: Vec<RepoPath>,
    excludes: Option<GlobSet>,
}

#[derive(Debug, Clone, Default)]
pub struct SpecIndex {
    pub anchors: BTreeMap<SpecAnchor, AnchorValue>,
    pub bindings: BTreeMap<SpecAnchor, ArtifactBinding>,
    /// Status for status-bearing specification items. Planned items are kept
    /// in the graph for discovery, but their ownership must not govern the
    /// current workspace.
    pub item_status: BTreeMap<SpecId, ItemStatus>,
    pub contracts: BTreeMap<SpecAnchor, Contract>,
    pub criteria_to_implementations: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub criteria_to_verifications: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub criteria_to_rules: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub rules_to_principles: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub binding_to_contracts: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub item_anchors: BTreeMap<SpecId, Vec<SpecAnchor>>,
    pub item_paths: BTreeMap<SpecId, PathBuf>,
    pub path_to_targets: BTreeMap<String, Vec<BoundTargetRef>>,
    pub criterion_status: BTreeMap<SpecAnchor, ItemStatus>,
    pub artifact_units: Vec<ArtifactUnit>,
    pub artifact_owners: BTreeMap<String, Vec<OwnershipRef>>,
    pub target_to_artifact: BTreeMap<BoundTargetRef, String>,
    pub criteria_to_implementation_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    pub criteria_to_verification_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    pub contracts_by_target: BTreeMap<BoundTargetRef, Vec<SpecAnchor>>,
    pub verification_by_target: BTreeMap<BoundTargetRef, Vec<BoundTargetRef>>,
    pub inventory_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnershipRef {
    pub binding: SpecAnchor,
    pub scope_id: LocalId,
    pub target_id: Option<LocalId>,
}
#[derive(Debug, Clone)]
pub enum AnchorValue {
    Principle(Principle),
    Rule(Rule),
    Criterion(Criterion),
    Binding(ArtifactBinding),
    Contract(Contract),
}

impl SpecWorkspace {
    pub fn overlay_document(&self, path: &Path, document: SpecDocument) -> Result<Self> {
        let mut overlay = self.clone();
        overlay.fingerprint_cache = Arc::new(OnceLock::new());
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let loaded = overlay
            .documents
            .iter_mut()
            .find(|loaded| loaded.path == canonical)
            .ok_or_else(|| anyhow::anyhow!("overlay path is not a loaded specification"))?;
        loaded.document = document;
        let bytes = serde_yaml::to_string(&loaded.document)?.into_bytes();
        overlay.overlays.insert(canonical.clone(), bytes.clone());
        if let Ok(relative) = canonical.strip_prefix(&overlay.root) {
            overlay.overlays.insert(relative.to_path_buf(), bytes);
        }
        Ok(overlay)
    }

    pub fn overlay_config(&self, config: ProjectConfig) -> Result<Self> {
        let mut overlay = self.clone();
        overlay.fingerprint_cache = Arc::new(OnceLock::new());
        overlay.matcher = WorkspaceMatcher::build(&config)?;
        overlay.config = config;
        let bytes = serde_yaml::to_string(&overlay.config)?.into_bytes();
        let path = overlay.root.join("syu.yaml");
        let canonical = path.canonicalize().unwrap_or(path);
        overlay.overlays.insert(canonical, bytes.clone());
        overlay.overlays.insert(PathBuf::from("syu.yaml"), bytes);
        Ok(overlay)
    }

    pub fn load(start: impl AsRef<Path>) -> Result<Self> {
        let root = find_root(start.as_ref())?;
        let config_path = root.join("syu.yaml");
        let config_source = fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config: ProjectConfig = match serde_yaml::from_str(&config_source) {
            Ok(config) => config,
            Err(_) if is_obsolete_pre_release_config(&config_source) => bail!(
                "The document uses an obsolete pre-release syu/config/v1 shape.\nRewrite it using the current syu/config/v1 model."
            ),
            Err(error) => return Err(error).context("parse syu/config/v1"),
        };
        if config.schema != CONFIG_SCHEMA {
            bail!("config schema must be {CONFIG_SCHEMA}");
        }
        let matcher = WorkspaceMatcher::build(&config)?;
        let mut paths = Vec::new();
        for spec_root in &config.workspace.spec_roots {
            collect_yaml(&root, spec_root.as_path(), &mut paths)?;
        }
        paths.sort();
        let mut documents = Vec::new();
        for path in paths {
            let source = fs::read_to_string(&path)?;
            let document: SpecDocument = match serde_yaml::from_str(&source) {
                Ok(document) => document,
                Err(_error) if is_obsolete_pre_release_spec(&source) => bail!(
                    "The document uses an obsolete pre-release syu/spec/v1 shape.\nRewrite it using the current syu/spec/v1 model."
                ),
                Err(error) => {
                    return Err(error).with_context(|| format!("strict parse {}", path.display()));
                }
            };
            if document.schema() != SPEC_SCHEMA {
                bail!("{}: schema must be {SPEC_SCHEMA}", path.display());
            }
            documents.push(LoadedDocument { path, document });
        }
        Ok(Self {
            root,
            config,
            documents,
            overlays: BTreeMap::new(),
            matcher,
            fingerprint_cache: Arc::new(OnceLock::new()),
        })
    }
    pub fn index(&self) -> Result<SpecIndex> {
        SpecIndex::build(self)
    }
    pub fn try_fingerprint(&self) -> Result<String> {
        self.fingerprint_cache
            .get_or_init(|| {
                self.compute_fingerprint()
                    .map_err(|error| error.to_string())
            })
            .clone()
            .map_err(anyhow::Error::msg)
    }

    fn compute_fingerprint(&self) -> Result<String> {
        let mut hash = Sha256::new();
        hash.update(serde_yaml::to_string(&self.config)?.as_bytes());
        for doc in &self.documents {
            if let Ok(relative) = doc.path.strip_prefix(&self.root) {
                hash.update(relative.to_string_lossy().as_bytes());
            }
            hash.update(self.read_bytes(&doc.path)?);
        }
        if let Some(profile) = self
            .config
            .inventory
            .profiles
            .iter()
            .find(|profile| profile.id == self.config.inventory.active_profile)
        {
            hash.update(b"inventory-profile:v1");
            hash.update(profile.id.as_bytes());
            if let Ok(value) = serde_yaml::to_string(profile) {
                hash.update(value.as_bytes());
            }
            let units = InventoryRegistry::discover(
                &InventoryContext {
                    workspace_root: self.root.clone(),
                    profile: profile.id.clone(),
                    settings: serde_yaml::Value::Null,
                    excludes: self
                        .config
                        .workspace
                        .excludes
                        .iter()
                        .map(|p| p.0.clone())
                        .collect(),
                    overlays: self.overlays.clone(),
                },
                profile,
            )?;
            for unit in units {
                hash.update(unit.identity.as_bytes());
                hash.update(unit.digest.as_bytes());
            }
        }
        for (runner, definition) in &self.config.verification.runners {
            hash.update(runner.as_bytes());
            hash.update(definition.executable.as_bytes());
            for argument in &definition.arguments {
                hash.update(argument.as_bytes());
            }
        }
        Ok(format!(
            "sha256:{}",
            hash.finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    /// Snapshots may still be rendered for an invalid workspace. Execution
    /// bases use `try_fingerprint` and therefore reject inventory failures.
    /// Fingerprints are unavailable when any enabled inventory provider
    /// fails. Callers must propagate this error instead of manufacturing an
    /// invalid-but-plausible execution basis.
    pub fn fingerprint(&self) -> Result<String> {
        self.try_fingerprint()
    }

    /// Fingerprint only the specification/configuration inputs. Inventory
    /// artifact bytes are intentionally excluded so an editable source change
    /// does not make an otherwise valid work plan stale.
    pub fn spec_fingerprint(&self) -> Result<String> {
        let mut hash = Sha256::new();
        hash.update(serde_yaml::to_string(&self.config)?.as_bytes());
        for document in &self.documents {
            if let Ok(relative) = document.path.strip_prefix(&self.root) {
                hash.update(relative.to_string_lossy().as_bytes());
            }
            hash.update(self.read_bytes(&document.path)?);
        }
        Ok(format!(
            "sha256:{}",
            hash.finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    pub fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(bytes) = self.overlays.get(&canonical) {
            return Ok(bytes.clone());
        }
        if let Ok(relative) = canonical.strip_prefix(&self.root)
            && let Some(bytes) = self.overlays.get(relative)
        {
            return Ok(bytes.clone());
        }
        Ok(fs::read(path)?)
    }

    pub fn read_to_string(&self, path: &Path) -> Result<String> {
        Ok(String::from_utf8(self.read_bytes(path)?)?)
    }
    pub fn path_is_spec(&self, path: &Path) -> bool {
        self.matcher.contains(&self.matcher.spec_roots, path)
    }
    pub fn path_is_artifact(&self, path: &Path) -> bool {
        !path.is_absolute() && !self.path_is_excluded(path)
    }
    pub fn path_is_excluded(&self, path: &Path) -> bool {
        self.matcher.is_excluded(path)
    }
}

fn is_obsolete_pre_release_spec(source: &str) -> bool {
    [
        "satisfies:",
        "verifies:",
        "documents:",
        "enforces:",
        "generated_from:",
        "evidences:",
        "names:",
        "binding:",
    ]
    .iter()
    .any(|field| {
        source
            .lines()
            .any(|line| line.trim_start().starts_with(field))
    })
}

fn is_obsolete_pre_release_config(source: &str) -> bool {
    ["version:", "spec:", "validate:"]
        .iter()
        .any(|field| source.lines().any(|line| line.starts_with(field)))
}

impl SpecIndex {
    pub fn build(workspace: &SpecWorkspace) -> Result<Self> {
        let mut out = Self::default();
        let mut ids = BTreeSet::new();
        for loaded in &workspace.documents {
            match &loaded.document {
                SpecDocument::Philosophies { philosophies, .. } => {
                    for item in philosophies {
                        unique_item(&mut ids, &item.id)?;
                        out.item_paths.insert(item.id.clone(), loaded.path.clone());
                        for p in &item.principles {
                            out.insert(
                                item.id.clone(),
                                LocalAnchorKind::Principle,
                                p.id.clone(),
                                AnchorValue::Principle(p.clone()),
                            )?;
                        }
                        for b in &item.bindings {
                            out.insert_binding(&item.id, b)?;
                        }
                    }
                }
                SpecDocument::Policies { policies, .. } => {
                    for item in policies {
                        unique_item(&mut ids, &item.id)?;
                        out.item_paths.insert(item.id.clone(), loaded.path.clone());
                        for rule in &item.rules {
                            let anchor = out.insert(
                                item.id.clone(),
                                LocalAnchorKind::Rule,
                                rule.id.clone(),
                                AnchorValue::Rule(rule.clone()),
                            )?;
                            out.rules_to_principles
                                .insert(anchor, rule.governed_by.clone());
                        }
                        for b in &item.bindings {
                            out.insert_binding(&item.id, b)?;
                        }
                    }
                }
                SpecDocument::Requirements { requirements, .. } => {
                    for item in requirements {
                        unique_item(&mut ids, &item.id)?;
                        out.item_status.insert(item.id.clone(), item.status);
                        out.item_paths.insert(item.id.clone(), loaded.path.clone());
                        for criterion in &item.criteria {
                            let anchor = out.insert(
                                item.id.clone(),
                                LocalAnchorKind::Criterion,
                                criterion.id.clone(),
                                AnchorValue::Criterion(criterion.clone()),
                            )?;
                            out.criterion_status.insert(anchor.clone(), item.status);
                            out.criteria_to_rules
                                .insert(anchor, criterion.governed_by.clone());
                        }
                        for b in &item.bindings {
                            out.insert_binding(&item.id, b)?;
                        }
                    }
                }
                SpecDocument::Features { features, .. } => {
                    for item in features {
                        unique_item(&mut ids, &item.id)?;
                        out.item_status.insert(item.id.clone(), item.status);
                        out.item_paths.insert(item.id.clone(), loaded.path.clone());
                        for b in &item.bindings {
                            out.insert_binding(&item.id, b)?;
                        }
                        for contract in &item.contracts {
                            let anchor = out.insert(
                                item.id.clone(),
                                LocalAnchorKind::Contract,
                                contract.id.clone(),
                                AnchorValue::Contract(contract.clone()),
                            )?;
                            out.contracts.insert(anchor.clone(), contract.clone());
                            for p in &contract.participants {
                                out.binding_to_contracts
                                    .entry(p.target.binding.clone())
                                    .or_default()
                                    .push(anchor.clone());
                            }
                        }
                    }
                }
            }
        }
        for (anchor, binding) in &out.bindings {
            for target in &binding.targets {
                if !workspace.path_is_artifact(target.path.as_path()) {
                    bail!(
                        "target path {} is excluded from inventory",
                        target.path.display()
                    );
                }
                if workspace.path_is_excluded(target.path.as_path()) {
                    bail!(
                        "target path {} is excluded by workspace.excludes",
                        target.path.display()
                    );
                }
                out.path_to_targets
                    .entry(target.path.to_string_lossy().into_owned())
                    .or_default()
                    .push(BoundTargetRef {
                        binding: anchor.clone(),
                        target_id: target.id.clone(),
                    });
            }
            for target in &binding.targets {
                for claim in &target.claims {
                    match claim {
                        TargetClaim::Satisfies { criterion } => out
                            .criteria_to_implementations
                            .entry(criterion.clone())
                            .or_default()
                            .push(anchor.clone()),
                        TargetClaim::Verifies { criterion, .. } => out
                            .criteria_to_verifications
                            .entry(criterion.clone())
                            .or_default()
                            .push(anchor.clone()),
                        _ => {}
                    }
                }
            }
        }
        for values in out
            .criteria_to_implementations
            .values_mut()
            .chain(out.criteria_to_verifications.values_mut())
            .chain(out.binding_to_contracts.values_mut())
        {
            values.sort();
            values.dedup();
        }
        let profile = workspace
            .config
            .inventory
            .profiles
            .iter()
            .find(|profile| profile.id == workspace.config.inventory.active_profile)
            .ok_or_else(|| anyhow::anyhow!("active inventory profile is not defined"))?;
        match InventoryRegistry::discover(
            &InventoryContext {
                workspace_root: workspace.root.clone(),
                profile: profile.id.clone(),
                settings: serde_yaml::Value::Null,
                excludes: workspace
                    .config
                    .workspace
                    .excludes
                    .iter()
                    .map(|p| p.0.clone())
                    .collect(),
                overlays: workspace.overlays.clone(),
            },
            profile,
        ) {
            Ok(units) => out.artifact_units = units,
            Err(error) => out.inventory_error = Some(error.to_string()),
        }
        for (binding_anchor, binding) in &out.bindings {
            for target in &binding.targets {
                let target_ref = BoundTargetRef {
                    binding: binding_anchor.clone(),
                    target_id: target.id.clone(),
                };
                let identities = artifact_identities_for_target(&out.artifact_units, target);
                if identities.len() > 1 {
                    bail!(
                        "target {target_ref} resolves to {} active artifact identities; exact selectors must resolve exactly one",
                        identities.len()
                    );
                }
                if let Some(identity) = identities.first() {
                    out.target_to_artifact
                        .insert(target_ref.clone(), identity.clone());
                }
                if let Some(identity) = identities.first()
                    && matches!(
                        binding.role,
                        BindingRole::Implementation | BindingRole::Verification
                    )
                    && !matches!(
                        out.item_status.get(&binding_anchor.item),
                        Some(ItemStatus::Planned)
                    )
                {
                    out.artifact_owners
                        .entry(identity.clone())
                        .or_default()
                        .push(OwnershipRef {
                            binding: binding_anchor.clone(),
                            scope_id: target.id.clone(),
                            target_id: Some(target.id.clone()),
                        });
                }
                for claim in &target.claims {
                    match claim {
                        TargetClaim::Satisfies { criterion } => out
                            .criteria_to_implementation_targets
                            .entry(criterion.clone())
                            .or_default()
                            .push(target_ref.clone()),
                        TargetClaim::Verifies {
                            criterion, covers, ..
                        } => {
                            out.criteria_to_verification_targets
                                .entry(criterion.clone())
                                .or_default()
                                .push(target_ref.clone());
                            for covered in covers {
                                out.verification_by_target
                                    .entry(covered.clone())
                                    .or_default()
                                    .push(target_ref.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for (binding_anchor, binding) in &out.bindings {
            if matches!(
                out.item_status.get(&binding_anchor.item),
                Some(ItemStatus::Planned)
            ) {
                continue;
            }
            for scope in &binding.owns {
                for unit in &out.artifact_units {
                    if scope_matches(scope, unit) {
                        let exact_owner_exists = out
                            .artifact_owners
                            .get(&unit.identity)
                            .is_some_and(|owners| {
                                owners.iter().any(|owner| owner.target_id.is_some())
                            });
                        // Exact target ownership has an explicit precedence rule.
                        // Overlapping scopes belonging to different bindings remain
                        // visible as multiple OwnershipRefs and are ambiguous.
                        if exact_owner_exists {
                            continue;
                        }
                        out.artifact_owners
                            .entry(unit.identity.clone())
                            .or_default()
                            .push(OwnershipRef {
                                binding: binding_anchor.clone(),
                                scope_id: scope.id.clone(),
                                target_id: None,
                            });
                    }
                }
            }
        }
        for (contract_anchor, contract) in &out.contracts {
            for participant in &contract.participants {
                out.contracts_by_target
                    .entry(participant.target.clone())
                    .or_default()
                    .push(contract_anchor.clone());
            }
        }
        for values in out.artifact_owners.values_mut() {
            values.sort();
            values.dedup();
        }
        for values in out
            .criteria_to_implementation_targets
            .values_mut()
            .chain(out.criteria_to_verification_targets.values_mut())
            .chain(out.verification_by_target.values_mut())
        {
            values.sort();
            values.dedup();
        }
        for values in out.contracts_by_target.values_mut() {
            values.sort();
            values.dedup();
        }
        for values in out.path_to_targets.values_mut() {
            values.sort();
            values.dedup();
        }
        Ok(out)
    }

    /// Fingerprint graph ownership without including mutable artifact bytes.
    /// This detects changes to bindings, exact targets, and reverse ownership
    /// relationships while allowing an editable target's implementation body
    /// to change after planning.
    pub fn ownership_fingerprint(&self) -> String {
        let mut hash = Sha256::new();
        for (anchor, binding) in &self.bindings {
            hash.update(anchor.to_string().as_bytes());
            hash.update(
                serde_json::to_vec(binding).expect("artifact binding serializes for fingerprint"),
            );
        }
        for (target, identity) in &self.target_to_artifact {
            hash.update(target.to_string().as_bytes());
            hash.update(identity.as_bytes());
        }
        for (identity, owners) in &self.artifact_owners {
            hash.update(identity.as_bytes());
            for owner in owners {
                hash.update(owner.binding.to_string().as_bytes());
                hash.update(owner.scope_id.to_string().as_bytes());
                if let Some(target_id) = &owner.target_id {
                    hash.update(target_id.to_string().as_bytes());
                }
            }
        }
        format!(
            "sha256:{}",
            hash.finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }
    fn insert(
        &mut self,
        item: SpecId,
        kind: LocalAnchorKind,
        local_id: LocalId,
        value: AnchorValue,
    ) -> Result<SpecAnchor> {
        let anchor = SpecAnchor {
            item: item.clone(),
            kind,
            local_id,
        };
        if self.anchors.insert(anchor.clone(), value).is_some() {
            bail!("duplicate local anchor {anchor}");
        }
        self.item_anchors
            .entry(item)
            .or_default()
            .push(anchor.clone());
        Ok(anchor)
    }
    fn insert_binding(&mut self, item: &SpecId, binding: &ArtifactBinding) -> Result<()> {
        let anchor = self.insert(
            item.clone(),
            LocalAnchorKind::Binding,
            binding.id.clone(),
            AnchorValue::Binding(binding.clone()),
        )?;
        self.bindings.insert(anchor, binding.clone());
        Ok(())
    }
    pub fn target(&self, reference: &BoundTargetRef) -> Option<&ArtifactTarget> {
        self.bindings
            .get(&reference.binding)?
            .targets
            .iter()
            .find(|t| t.id == reference.target_id)
    }
    pub fn anchor(&self, anchor: &SpecAnchor) -> Option<&AnchorValue> {
        self.anchors.get(anchor)
    }
}

fn artifact_identities_for_target(units: &[ArtifactUnit], target: &ArtifactTarget) -> Vec<String> {
    units
        .iter()
        .filter(|unit| {
            if unit.adapter != target.adapter
                || unit.path != target.path
                || !matches!(
                    unit.reachability,
                    syu_inventory::ArtifactReachability::Active
                )
            {
                return false;
            }
            match &target.selector {
                Selector::Symbol { name } => {
                    unit.kind == ArtifactUnitKind::Symbol
                        && symbol_identity_matches(&unit.identity, name)
                }
                Selector::Operation { method, path } => {
                    unit.kind == ArtifactUnitKind::Operation
                        && unit.identity.ends_with(&format!(
                            "::{} {}",
                            method.to_ascii_uppercase(),
                            path
                        ))
                }
                Selector::Heading { value } => {
                    unit.kind == ArtifactUnitKind::Heading
                        && unit.identity.ends_with(&format!("::heading::{value}"))
                }
                Selector::File | Selector::JsonPointer { .. } => {
                    unit.kind == ArtifactUnitKind::File
                }
                Selector::Marker { value } => {
                    unit.kind == ArtifactUnitKind::Marker
                        && unit.identity.starts_with(&format!(
                            "{}:{}::marker::{}@",
                            target.adapter,
                            target.path.to_string_lossy(),
                            value
                        ))
                }
            }
        })
        .map(|unit| unit.identity.clone())
        .collect()
}

fn symbol_identity_matches(identity: &str, name: &str) -> bool {
    identity.ends_with(&format!("::{name}"))
        || identity.contains(&format!("::{name}@"))
        || identity.ends_with(&format!("::{name})"))
        || name.rsplit_once("::").is_some_and(|(container, leaf)| {
            identity.ends_with(&format!("::impl({container})::{leaf}"))
        })
}

fn scope_matches(scope: &OwnershipScope, unit: &ArtifactUnit) -> bool {
    if scope.adapter != unit.adapter {
        return false;
    }
    match &scope.selector {
        OwnershipSelector::File => scope.path == unit.path && unit.kind == ArtifactUnitKind::File,
        OwnershipSelector::Module { name } => {
            scope.path == unit.path
                && (name == "*"
                    || unit.identity.contains(&format!("::{name}::"))
                    || unit.identity.ends_with(&format!("::{name}")))
        }
        OwnershipSelector::PathPrefix { value } => unit.path.as_path().starts_with(value.as_path()),
    }
}

fn unique_item(ids: &mut BTreeSet<SpecId>, id: &SpecId) -> Result<()> {
    if !ids.insert(id.clone()) {
        bail!("duplicate item id {id}");
    }
    Ok(())
}

fn find_root(start: &Path) -> Result<PathBuf> {
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    if current.is_relative() {
        current = std::env::current_dir()?.join(current);
    }
    loop {
        if current.join("syu.yaml").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("could not find syu.yaml");
        }
    }
}
fn collect_yaml(root: &Path, relative: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let candidate = root.join(relative);
    let canonical_root = root.canonicalize()?;
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("spec root does not exist: {}", relative.display()))?;
    if !canonical.starts_with(&canonical_root) {
        bail!("spec root escapes workspace: {}", relative.display());
    }
    if canonical.is_file() {
        if matches!(
            canonical.extension().and_then(|v| v.to_str()),
            Some("yaml" | "yml")
        ) {
            out.push(canonical);
            return Ok(());
        }
        bail!("spec root is not a yaml file: {}", relative.display());
    }
    for entry in fs::read_dir(canonical)? {
        let path = entry?.path();
        if path.is_dir() {
            let relative = path
                .strip_prefix(&canonical_root)
                .context("spec path must stay relative to the workspace")?;
            collect_yaml(root, relative, out)?;
        } else if matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("yaml" | "yml")
        ) {
            out.push(path);
        }
    }
    Ok(())
}

impl WorkspaceMatcher {
    fn build(config: &ProjectConfig) -> Result<Self> {
        let excludes = compile_excludes(&config.workspace.excludes)?;
        Ok(Self {
            spec_roots: config.workspace.spec_roots.clone(),
            excludes,
        })
    }
    fn contains(&self, roots: &[RepoPath], path: &Path) -> bool {
        roots
            .iter()
            .any(|root| path == root.as_path() || path.starts_with(root.as_path()))
    }
    fn is_excluded(&self, path: &Path) -> bool {
        self.excludes.as_ref().is_some_and(|set| set.is_match(path))
    }
}

fn compile_excludes(patterns: &[syu_project_model::RepoPathPattern]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(
            Glob::new(&pattern.0)
                .with_context(|| format!("invalid workspace exclude pattern {}", pattern.0))?,
        );
    }
    Ok(Some(builder.build()?))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub path: PathBuf,
    pub description: String,
    pub symbols: Vec<String>,
    pub content_hash: String,
    pub bytes: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub excerpt: String,
    pub excerpt_hash: String,
}
pub fn resolve_target(root: &Path, target: &ArtifactTarget) -> Result<ResolvedTarget> {
    resolve_target_with_adapters(root, target, std::slice::from_ref(&target.adapter))
}

pub fn resolve_target_in_workspace(
    workspace: &SpecWorkspace,
    target: &ArtifactTarget,
) -> Result<ResolvedTarget> {
    const KNOWN: &[&str] = &[
        "rust",
        "typescript",
        "javascript",
        "shell",
        "python",
        "go",
        "java",
        "ruby",
        "csharp",
        "markdown",
        "openapi",
        "yaml",
        "json",
        "html",
        "declared",
    ];
    if !KNOWN.contains(&target.adapter.as_str()) {
        bail!("unknown adapter {}", target.adapter);
    }
    if !workspace
        .config
        .inventory
        .profiles
        .iter()
        .find(|profile| profile.id == workspace.config.inventory.active_profile)
        .is_some_and(|profile| profile.providers.contains_key(&target.adapter))
    {
        bail!("adapter {} is disabled", target.adapter);
    }
    let canonical_root = workspace.root.canonicalize()?;
    let path = workspace.root.join(target.path.as_path());
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("target path does not exist: {}", target.path.display()))?;
    if !canonical_path.starts_with(&canonical_root) {
        bail!("target path escapes workspace through a symlink");
    }
    let content = workspace.read_bytes(&canonical_path)?;
    resolve_target_from_content(&workspace.root, target, content)
}

/// Resolve a target from the exact semantic span already produced by the
/// active inventory. This is used by planning and post-state validation so a
/// canonical plan and its later validation share the same candidate-overlay
/// source of truth without reparsing every source file.
pub fn resolve_indexed_target(
    workspace: &SpecWorkspace,
    target: &ArtifactTarget,
    unit: &ArtifactUnit,
) -> Result<Option<ResolvedTarget>> {
    if unit.adapter != target.adapter
        || unit.path != target.path
        || !matches!(
            unit.reachability,
            syu_inventory::ArtifactReachability::Active
        )
        || unit.span.byte_end <= unit.span.byte_start
    {
        return Ok(None);
    }
    match (&target.selector, &unit.kind) {
        (ExactSelector::Symbol { .. }, ArtifactUnitKind::Symbol)
        | (ExactSelector::Heading { .. }, ArtifactUnitKind::Heading)
        | (ExactSelector::Marker { .. }, ArtifactUnitKind::Marker)
        | (ExactSelector::File, ArtifactUnitKind::File) => {}
        _ => return Ok(None),
    }
    let content = workspace.read_bytes(&workspace.root.join(unit.path.as_path()))?;
    let text = std::str::from_utf8(&content)
        .map_err(|error| anyhow::anyhow!("inventory target source is not UTF-8: {error}"))?;
    let Some(excerpt) = text.get(unit.span.byte_start..unit.span.byte_end) else {
        return Ok(None);
    };
    let (description, symbols) = match &target.selector {
        ExactSelector::File => ("file".into(), vec![]),
        ExactSelector::Symbol { name } => (format!("symbol {name}"), vec![name.clone()]),
        ExactSelector::Heading { value } => (format!("heading {value}"), vec![]),
        ExactSelector::Marker { value } => (format!("marker {value}"), vec![]),
        _ => return Ok(None),
    };
    Ok(Some(ResolvedTarget {
        path: unit.path.as_path().to_path_buf(),
        description,
        symbols,
        content_hash: hash_bytes(&content),
        bytes: content.len(),
        byte_start: unit.span.byte_start,
        byte_end: unit.span.byte_end,
        line_start: unit.span.line_start,
        line_end: unit.span.line_end,
        excerpt: excerpt.to_owned(),
        excerpt_hash: hash_bytes(excerpt.as_bytes()),
    }))
}

pub fn resolve_target_with_adapters(
    root: &Path,
    target: &ArtifactTarget,
    enabled: &[String],
) -> Result<ResolvedTarget> {
    const KNOWN: &[&str] = &[
        "rust",
        "typescript",
        "javascript",
        "shell",
        "python",
        "go",
        "java",
        "ruby",
        "csharp",
        "markdown",
        "openapi",
        "yaml",
        "json",
        "html",
        "declared",
    ];
    if !KNOWN.contains(&target.adapter.as_str()) {
        bail!("unknown adapter {}", target.adapter);
    }
    if !enabled.contains(&target.adapter) {
        bail!("adapter {} is disabled", target.adapter);
    }
    let canonical_root = root.canonicalize()?;
    let path = root.join(target.path.as_path());
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("target path does not exist: {}", target.path.display()))?;
    if !canonical_path.starts_with(&canonical_root) {
        bail!("target path escapes workspace through a symlink");
    }
    let content = fs::read(&canonical_path)?;
    resolve_target_from_content(root, target, content)
}

fn resolve_target_from_content(
    _root: &Path,
    target: &ArtifactTarget,
    content: Vec<u8>,
) -> Result<ResolvedTarget> {
    let text = String::from_utf8_lossy(&content);
    let (description, symbols, byte_start, byte_end, line_start, line_end, excerpt, excerpt_hash) =
        match &target.selector {
            Selector::File => {
                let excerpt = text.to_string();
                (
                    "file".into(),
                    vec![],
                    0,
                    content.len(),
                    1,
                    text.lines().count().max(1),
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Symbol { name } => {
                if name.trim().is_empty() {
                    bail!("symbol selector must not be empty");
                }
                let resolved = resolve_symbol(&target.adapter, &text, name)?;
                let start = resolved.byte_start;
                let end = resolved.byte_end;
                let excerpt = text[start..end].to_string();
                (
                    format!("symbol {name}"),
                    vec![name.clone()],
                    start,
                    end,
                    resolved.line_start,
                    resolved.line_end,
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Operation { method, path } => {
                if method.trim().is_empty() || path.trim().is_empty() {
                    bail!("operation selector must not be empty");
                }
                let yaml: serde_yaml::Value = serde_yaml::from_slice(&content)?;
                let exists = yaml
                    .get("paths")
                    .and_then(|v| v.get(path))
                    .and_then(|v| v.get(method.to_ascii_lowercase()))
                    .is_some();
                if !exists {
                    bail!("operation {method} {path} not found");
                }
                let (byte_start, byte_end, line_start, line_end, excerpt) =
                    extract_yaml_block(&text, |line| {
                        line.trim_start().starts_with(path.as_str())
                            || line
                                .trim_start()
                                .starts_with(&format!("{}:", method.to_ascii_lowercase()))
                    })
                    .unwrap_or_else(|| {
                        let excerpt = text.to_string();
                        (0, content.len(), 1, text.lines().count(), excerpt)
                    });
                (
                    format!("operation {} {path}", method.to_ascii_uppercase()),
                    vec![],
                    byte_start,
                    byte_end,
                    line_start,
                    line_end,
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Heading { value } => {
                if value.trim().is_empty() {
                    bail!("heading selector must not be empty");
                }
                let heading_matches = text
                    .lines()
                    .filter(|line| {
                        let trimmed = line.trim_start();
                        trimmed.starts_with('#') && trimmed.trim_start_matches('#').trim() == value
                    })
                    .count();
                if heading_matches == 0 {
                    bail!("heading {value} not found");
                }
                if heading_matches > 1 {
                    bail!("heading {value} is ambiguous");
                }
                let (byte_start, byte_end, line_start, line_end, excerpt) =
                    extract_heading_block(&text, value).unwrap_or_else(|| {
                        let excerpt = text.to_string();
                        (0, content.len(), 1, text.lines().count(), excerpt)
                    });
                (
                    format!("heading {value}"),
                    vec![],
                    byte_start,
                    byte_end,
                    line_start,
                    line_end,
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::JsonPointer { value } => {
                if value.trim().is_empty() {
                    bail!("json pointer selector must not be empty");
                }
                let json: serde_json::Value = if target.adapter == "json" {
                    serde_json::from_slice(&content)?
                } else {
                    serde_json::to_value(serde_yaml::from_slice::<serde_yaml::Value>(&content)?)?
                };
                if json.pointer(value).is_none() {
                    bail!("pointer {value} not found");
                }
                let excerpt = text.to_string();
                (
                    format!("json pointer {value}"),
                    vec![],
                    0,
                    content.len(),
                    1,
                    text.lines().count(),
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Marker { value } => {
                if value.trim().is_empty() {
                    bail!("marker selector must not be empty");
                }
                let marker_matches = text.match_indices(value).count();
                if marker_matches == 0 {
                    bail!("marker {value} not found");
                }
                if marker_matches > 1 {
                    bail!("marker {value} is ambiguous");
                }
                let (byte_start, byte_end, line_start, line_end, excerpt) =
                    extract_marker_block(&text, value).unwrap_or_else(|| {
                        let excerpt = text.to_string();
                        (0, content.len(), 1, text.lines().count(), excerpt)
                    });
                (
                    format!("marker {value}"),
                    vec![],
                    byte_start,
                    byte_end,
                    line_start,
                    line_end,
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
        };
    let mut hash = Sha256::new();
    hash.update(&content);
    Ok(ResolvedTarget {
        path: target.path.as_path().to_path_buf(),
        description,
        symbols,
        content_hash: format!(
            "sha256:{}",
            hash.finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        bytes: content.len(),
        byte_start,
        byte_end,
        line_start,
        line_end,
        excerpt,
        excerpt_hash,
    })
}

pub fn selector_supports_editable(selector: &Selector) -> bool {
    matches!(
        selector,
        Selector::File
            | Selector::Symbol { .. }
            | Selector::Heading { .. }
            | Selector::Marker { .. }
    )
}
fn hash_bytes(value: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(value);
    format!(
        "sha256:{}",
        hash.finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn extract_yaml_block(
    text: &str,
    predicate: impl Fn(&str) -> bool,
) -> Option<(usize, usize, usize, usize, String)> {
    let mut start = None;
    let mut end = None;
    let mut byte = 0usize;
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        if start.is_none() && predicate(line) {
            start = Some((byte, line_no));
        } else if start.is_some() {
            let trimmed = line.trim_start();
            if !trimmed.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                end = Some((byte, line_no.saturating_sub(1)));
                break;
            }
        }
        byte += line.len() + 1;
    }
    let (start_byte, start_line) = start?;
    let (end_byte, end_line) = end.unwrap_or((text.len(), text.lines().count()));
    Some((
        start_byte,
        end_byte.max(start_byte),
        start_line,
        end_line.max(start_line),
        text[start_byte..end_byte.max(start_byte)].to_string(),
    ))
}

fn extract_heading_block(
    text: &str,
    heading: &str,
) -> Option<(usize, usize, usize, usize, String)> {
    let mut start = None;
    let mut end = None;
    let mut byte = 0usize;
    let mut level = 0usize;
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = line.trim_start();
        if start.is_none()
            && trimmed.starts_with('#')
            && trimmed.trim_start_matches('#').trim() == heading
        {
            level = trimmed.chars().take_while(|c| *c == '#').count();
            start = Some((byte, line_no));
        } else if start.is_some() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let next_level = trimmed.chars().take_while(|c| *c == '#').count();
                if next_level <= level {
                    end = Some((byte, line_no.saturating_sub(1)));
                    break;
                }
            }
        }
        byte += line.len() + 1;
    }
    let (start_byte, start_line) = start?;
    let (end_byte, end_line) = end.unwrap_or((text.len(), text.lines().count()));
    Some((
        start_byte,
        end_byte.max(start_byte),
        start_line,
        end_line.max(start_line),
        text[start_byte..end_byte.max(start_byte)].to_string(),
    ))
}

fn extract_marker_block(text: &str, marker: &str) -> Option<(usize, usize, usize, usize, String)> {
    let start_byte = text.find(marker)?;
    let mut byte = 0usize;
    for (index, line) in text.lines().enumerate() {
        let line_start = index + 1;
        let next = byte + line.len() + 1;
        if next > start_byte {
            let line_end = byte + line.len();
            return Some((
                byte,
                line_end,
                line_start,
                line_start,
                text[byte..line_end].to_string(),
            ));
        }
        byte = next;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_workspace(root: &Path) {
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::write(
            root.join("src/doc.md"),
            concat!(
                "# Shared\n",
                "marker: ::dup::\n",
                "## Shared\n",
                "marker: ::dup::\n",
            ),
        )
        .expect("doc");
    }

    fn target(selector: Selector) -> syu_spec_model::ArtifactTarget {
        syu_spec_model::ArtifactTarget {
            id: "doc".into(),
            adapter: "rust".into(),
            path: RepoPath::new("src/doc.md").expect("path"),
            selector,
            claims: vec![],
        }
    }

    #[test]
    fn heading_selectors_reject_ambiguity() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let result = resolve_target_with_adapters(
            tempdir.path(),
            &target(Selector::Heading {
                value: "Shared".into(),
            }),
            &["rust".into()],
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("heading Shared is ambiguous")
        );
    }

    #[test]
    fn marker_selectors_reject_ambiguity() {
        let tempdir = tempdir().expect("tempdir");
        write_workspace(tempdir.path());
        let result = resolve_target_with_adapters(
            tempdir.path(),
            &target(Selector::Marker {
                value: "::dup::".into(),
            }),
            &["rust".into()],
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("marker ::dup:: is ambiguous")
        );
    }

    #[test]
    fn nested_spec_directories_load_from_noncanonical_workspace_roots() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("spec/requirements")).expect("nested spec dir");
        fs::write(
            tempdir.path().join("syu.yaml"),
            concat!(
                "schema: syu/config/v1\n",
                "workspace:\n",
                "  spec_roots: [spec]\n",
                "  excludes: []\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles:\n",
                "    - id: default\n",
                "      providers: { rust: {} }\n",
                "validation:\n",
                "  preset: agent-ready\n",
                "  readiness:\n",
                "    target: closed-loop\n",
                "    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_seed: 4 }\n",
                "  changed:\n",
                "    baseline:\n",
                "      strategy: merge-base\n",
                "      against: origin/main\n",
                "    require_owned_changes: true\n",
                "    require_plan: true\n",
                "verification: { runners: {} }\n",
                "work:\n",
                "  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 6, max_readonly_targets: 12, max_total_bytes: 120000 }\n",
            ),
        )
        .expect("config");
        fs::write(
            tempdir.path().join("spec/requirements/req-new.yaml"),
            concat!(
                "schema: syu/spec/v1\n",
                "kind: requirements\n",
                "namespace: test\n",
                "category: Test\n",
                "requirements:\n",
                "  - id: REQ-NEW-001\n",
                "    title: New requirement\n",
                "    description: Nested spec file\n",
                "    priority: medium\n",
                "    status: planned\n",
                "    criteria:\n",
                "      - id: acceptance\n",
                "        kind: behavior\n",
                "        statement: Loads from nested directory\n",
                "        governed_by: []\n",
                "    bindings: []\n",
            ),
        )
        .expect("spec");

        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        assert_eq!(workspace.documents.len(), 1);
    }
}
