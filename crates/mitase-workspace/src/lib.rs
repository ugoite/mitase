#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use mitase_code_intel::resolve_symbol;
use mitase_inventory::{ArtifactUnit, ArtifactUnitKind, InventoryContext, InventoryRegistry};
use mitase_project_model::{CONFIG_SCHEMA, ProjectConfig};
use mitase_spec_model::format_sha256;
use mitase_spec_model::*;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

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
    matcher: WorkspaceMatcher,
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
    /// Reverse governance relations derived from authored Rule `governed_by`
    /// anchors. Source documents must not duplicate this relation.
    pub principles_to_rules: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub binding_to_contracts: BTreeMap<SpecAnchor, Vec<SpecAnchor>>,
    pub item_anchors: BTreeMap<SpecId, Vec<SpecAnchor>>,
    pub item_paths: BTreeMap<SpecId, PathBuf>,
    pub path_to_targets: BTreeMap<String, Vec<BoundTargetRef>>,
    pub criterion_status: BTreeMap<SpecAnchor, ItemStatus>,
    pub artifact_units: Vec<ArtifactUnit>,
    pub artifact_owners: BTreeMap<String, Vec<OwnershipRef>>,
    /// Historical exact target identities, including targets whose lifecycle
    /// is absent. This is evidence context, not current executable scope.
    pub all_target_to_artifact: BTreeMap<BoundTargetRef, String>,
    pub target_to_artifact: BTreeMap<BoundTargetRef, String>,
    /// Current implementation claims from non-planned specification items.
    pub criteria_to_implementation_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    /// Current verification claims from non-planned specification items.
    pub criteria_to_verification_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    /// Full implementation graph, including planned catalog entries.
    pub all_criteria_to_implementation_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    /// Full verification graph, including planned catalog entries.
    pub all_criteria_to_verification_targets: BTreeMap<SpecAnchor, Vec<BoundTargetRef>>,
    pub contracts_by_target: BTreeMap<BoundTargetRef, Vec<SpecAnchor>>,
    /// Current exact verification coverage from non-planned items.
    pub verification_by_target: BTreeMap<BoundTargetRef, Vec<BoundTargetRef>>,
    /// Full exact verification coverage, including planned catalog entries.
    pub all_verification_by_target: BTreeMap<BoundTargetRef, Vec<BoundTargetRef>>,
    /// Generated target -> exact source targets.
    pub generated_from: BTreeMap<BoundTargetRef, Vec<BoundTargetRef>>,
    /// Exact source target -> generated targets derived from it.
    pub generated_by_source: BTreeMap<BoundTargetRef, Vec<BoundTargetRef>>,
    /// Public governance targets mapped to the capability boundary they expose.
    pub exposes_by_target: BTreeMap<BoundTargetRef, BoundTargetRef>,
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
    pub fn load(start: impl AsRef<Path>) -> Result<Self> {
        let root = find_root(start.as_ref())?;
        let config_path = root.join("mitase.yaml");
        let config_source = fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config: ProjectConfig = match serde_yaml::from_str(&config_source) {
            Ok(config) => config,
            Err(_) if is_obsolete_pre_release_config(&config_source) => bail!(
                "The document uses an obsolete pre-release mitase/config/v1 shape.\nRewrite it using the current mitase/config/v1 model."
            ),
            Err(error) => return Err(error).context("parse mitase/config/v1"),
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
                    "The document uses an obsolete pre-release mitase/spec/v1 shape.\nRewrite it using the current mitase/spec/v1 model."
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
            matcher,
        })
    }
    pub fn index(&self) -> Result<SpecIndex> {
        SpecIndex::build(self)
    }

    pub fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
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
        for (rule, principles) in &out.rules_to_principles {
            for principle in principles {
                out.principles_to_rules
                    .entry(principle.clone())
                    .or_default()
                    .push(rule.clone());
            }
        }
        for values in out.principles_to_rules.values_mut() {
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
                overlays: BTreeMap::new(),
            },
            profile,
        ) {
            Ok(units) => out.artifact_units = units,
            Err(error) => out.inventory_error = Some(error.to_string()),
        }
        for (binding_anchor, binding) in &out.bindings {
            let active_binding = !matches!(
                out.item_status.get(&binding_anchor.item),
                Some(ItemStatus::Planned)
            );
            for target in &binding.targets {
                let current_target = target.lifecycle != ArtifactTargetLifecycle::Absent;
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
                    out.all_target_to_artifact
                        .insert(target_ref.clone(), identity.clone());
                    if current_target {
                        out.target_to_artifact
                            .insert(target_ref.clone(), identity.clone());
                    }
                }
                if let Some(identity) = identities.first()
                    && current_target
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
                        TargetClaim::Satisfies { criterion } => {
                            out.all_criteria_to_implementation_targets
                                .entry(criterion.clone())
                                .or_default()
                                .push(target_ref.clone());
                            if active_binding && current_target {
                                out.criteria_to_implementation_targets
                                    .entry(criterion.clone())
                                    .or_default()
                                    .push(target_ref.clone());
                            }
                        }
                        TargetClaim::Verifies {
                            criterion, covers, ..
                        } => {
                            out.all_criteria_to_verification_targets
                                .entry(criterion.clone())
                                .or_default()
                                .push(target_ref.clone());
                            for covered in covers {
                                out.all_verification_by_target
                                    .entry(covered.clone())
                                    .or_default()
                                    .push(target_ref.clone());
                            }
                            if active_binding && current_target {
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
                        }
                        TargetClaim::Exposes { target } if active_binding && current_target => {
                            out.exposes_by_target
                                .insert(target_ref.clone(), target.clone());
                        }
                        TargetClaim::GeneratedFrom { targets }
                            if active_binding && current_target =>
                        {
                            out.generated_from
                                .entry(target_ref.clone())
                                .or_default()
                                .extend(targets.iter().cloned());
                            for source in targets {
                                out.generated_by_source
                                    .entry(source.clone())
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
            out.contracts_by_target
                .entry(contract.source.clone())
                .or_default()
                .push(contract_anchor.clone());
            for participant in &contract.participants {
                out.contracts_by_target
                    .entry(participant.target.clone())
                    .or_default()
                    .push(contract_anchor.clone());
            }
        }
        let current_target_refs = out
            .bindings
            .iter()
            .flat_map(|(binding_anchor, binding)| {
                binding
                    .targets
                    .iter()
                    .filter(|target| target.lifecycle != ArtifactTargetLifecycle::Absent)
                    .map(|target| BoundTargetRef {
                        binding: binding_anchor.clone(),
                        target_id: target.id.clone(),
                    })
            })
            .collect::<BTreeSet<_>>();
        for values in out
            .criteria_to_implementation_targets
            .values_mut()
            .chain(out.criteria_to_verification_targets.values_mut())
            .chain(out.verification_by_target.values_mut())
        {
            values.retain(|reference| current_target_refs.contains(reference));
        }
        out.verification_by_target
            .retain(|reference, _| current_target_refs.contains(reference));
        out.exposes_by_target
            .retain(|reference, _| current_target_refs.contains(reference));
        out.generated_from
            .retain(|reference, _| current_target_refs.contains(reference));
        out.generated_by_source
            .values_mut()
            .for_each(|values| values.retain(|reference| current_target_refs.contains(reference)));
        for values in out.artifact_owners.values_mut() {
            values.sort();
            values.dedup();
        }
        for values in out
            .criteria_to_implementation_targets
            .values_mut()
            .chain(out.criteria_to_verification_targets.values_mut())
            .chain(out.verification_by_target.values_mut())
            .chain(out.all_criteria_to_implementation_targets.values_mut())
            .chain(out.all_criteria_to_verification_targets.values_mut())
            .chain(out.all_verification_by_target.values_mut())
            .chain(out.generated_from.values_mut())
            .chain(out.generated_by_source.values_mut())
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
                    mitase_inventory::ArtifactReachability::Active
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
                Selector::File => unit.kind == ArtifactUnitKind::File,
                Selector::JsonPointer { value } => {
                    unit.kind == ArtifactUnitKind::SchemaNode
                        && unit.identity.ends_with(&format!("::pointer::{value}"))
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
        || identity.contains(&format!("::{name}["))
        || identity.contains(&format!("::{name}@"))
        || identity.ends_with(&format!("::{name})"))
        || name.rsplit_once("::").is_some_and(|(container, leaf)| {
            identity.ends_with(&format!("::impl({container})::{leaf}"))
        })
}

fn scope_matches(scope: &OwnershipScope, unit: &ArtifactUnit) -> bool {
    if scope.adapter != unit.adapter
        || !matches!(
            unit.reachability,
            mitase_inventory::ArtifactReachability::Active
        )
    {
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
        if current.join("mitase.yaml").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("could not find mitase.yaml");
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

fn compile_excludes(patterns: &[mitase_project_model::RepoPathPattern]) -> Result<Option<GlobSet>> {
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
        "json-schema",
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

/// Resolve an active semantic inventory unit to its exact source range. The
/// inventory may keep coarse spans (notably OpenAPI operations and JSON
/// pointer nodes), so callers receive the exact source range rather than a
/// coarse inventory span.
pub fn resolve_artifact_unit(
    workspace: &SpecWorkspace,
    unit: &ArtifactUnit,
) -> Result<ResolvedTarget> {
    if !matches!(
        unit.reachability,
        mitase_inventory::ArtifactReachability::Active
    ) {
        bail!("semantic artifact {} is not active", unit.identity);
    }
    let selector = match unit.kind {
        ArtifactUnitKind::Operation => {
            let operation = unit
                .identity
                .rsplit_once("::")
                .map(|(_, operation)| operation)
                .ok_or_else(|| {
                    anyhow::anyhow!("operation identity is malformed: {}", unit.identity)
                })?;
            let (method, path) = operation.split_once(' ').ok_or_else(|| {
                anyhow::anyhow!("operation identity is malformed: {}", unit.identity)
            })?;
            Selector::Operation {
                method: method.into(),
                path: path.into(),
            }
        }
        ArtifactUnitKind::SchemaNode => {
            let pointer = unit
                .identity
                .rsplit_once("::pointer::")
                .map(|(_, pointer)| pointer)
                .ok_or_else(|| {
                    anyhow::anyhow!("schema identity is malformed: {}", unit.identity)
                })?;
            Selector::JsonPointer {
                value: pointer.into(),
            }
        }
        _ => {
            // Symbol, heading and marker inventory spans are source-derived
            // already.  Preserve those exact spans instead of attempting to
            // recover a declared target from a potentially broader owner.
            let bytes = workspace.read_bytes(&workspace.root.join(unit.path.as_path()))?;
            let text = std::str::from_utf8(&bytes).map_err(|error| {
                anyhow::anyhow!("inventory target source is not UTF-8: {error}")
            })?;
            let start = unit.span.byte_start.min(bytes.len());
            let end = unit.span.byte_end.min(bytes.len()).max(start);
            let excerpt = text.get(start..end).ok_or_else(|| {
                anyhow::anyhow!(
                    "inventory span is not on UTF-8 boundaries: {}",
                    unit.identity
                )
            })?;
            return Ok(ResolvedTarget {
                path: unit.path.as_path().to_path_buf(),
                description: format!("semantic artifact {}", unit.identity),
                symbols: if matches!(unit.kind, ArtifactUnitKind::File) {
                    vec![]
                } else {
                    vec![
                        unit.identity
                            .rsplit("::")
                            .next()
                            .unwrap_or(&unit.identity)
                            .into(),
                    ]
                },
                content_hash: hash_bytes(&bytes),
                bytes: bytes.len(),
                byte_start: start,
                byte_end: end,
                line_start: unit.span.line_start,
                line_end: unit.span.line_end,
                excerpt: excerpt.into(),
                excerpt_hash: hash_bytes(excerpt.as_bytes()),
            });
        }
    };
    let target = ArtifactTarget {
        id: LocalId::from("semantic-unit"),
        adapter: unit.adapter.clone(),
        path: unit.path.clone(),
        selector,
        lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
        claims: vec![],
    };
    resolve_target_in_workspace(workspace, &target)
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
        "json-schema",
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
                let (byte_start, byte_end, line_start, line_end, excerpt) =
                    if is_json_document(&text) {
                        let json: serde_json::Value = serde_json::from_slice(&content)?;
                        let method = method.to_ascii_lowercase();
                        let exists = json
                            .get("paths")
                            .and_then(|value| value.get(path))
                            .and_then(|value| value.get(&method))
                            .is_some();
                        if !exists {
                            bail!("operation {method} {path} not found");
                        }
                        let pointer = format!(
                            "/paths/{}/{}",
                            path.replace('~', "~0").replace('/', "~1"),
                            method
                        );
                        json_pointer_span(&text, &pointer)?
                    } else {
                        let yaml: serde_yaml::Value = serde_yaml::from_slice(&content)?;
                        let exists = yaml
                            .get("paths")
                            .and_then(|value| value.get(path))
                            .and_then(|value| value.get(method.to_ascii_lowercase()))
                            .is_some();
                        if !exists {
                            bail!("operation {method} {path} not found");
                        }
                        openapi_operation_span(&text, method, path).ok_or_else(|| {
                            anyhow::anyhow!(
                                "operation {method} {path} has no unambiguous source span"
                            )
                        })?
                    };
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
                if target.adapter != "json" && target.adapter != "json-schema" {
                    bail!("pointer {value} requires a source-location-aware JSON document");
                }
                let (byte_start, byte_end, line_start, line_end, excerpt) =
                    json_pointer_span(&text, value)?;
                (
                    format!("json pointer {value}"),
                    vec![],
                    byte_start,
                    byte_end,
                    line_start,
                    line_end,
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
        content_hash: format_sha256(hash.finalize()),
        bytes: content.len(),
        byte_start,
        byte_end,
        line_start,
        line_end,
        excerpt,
        excerpt_hash,
    })
}

fn is_json_document(text: &str) -> bool {
    matches!(text.trim_start().as_bytes().first(), Some(b'{' | b'['))
}

pub fn selector_supports_editable(selector: &Selector) -> bool {
    matches!(
        selector,
        Selector::File
            | Selector::Symbol { .. }
            | Selector::Operation { .. }
            | Selector::Heading { .. }
            | Selector::JsonPointer { .. }
            | Selector::Marker { .. }
    )
}

/// Return an exact source span for a JSON Pointer without serializing the
/// document again. `serde_json::Value` deliberately does not retain locations,
/// so this small CST walk keeps the original byte ranges while using
/// `serde_json` to validate string decoding.
fn json_pointer_span(text: &str, pointer: &str) -> Result<(usize, usize, usize, usize, String)> {
    if !pointer.starts_with('/') {
        bail!("json pointer must start with '/'");
    }
    serde_json::from_str::<serde_json::Value>(text).context("parse JSON document")?;
    let mut parser = JsonSpanParser {
        source: text,
        offset: 0,
        spans: BTreeMap::new(),
    };
    parser.parse_value("")?;
    let Some((start, end)) = parser.spans.get(pointer).copied() else {
        bail!("pointer {pointer} not found");
    };
    exact_span(text, start, end)
}

struct JsonSpanParser<'a> {
    source: &'a str,
    offset: usize,
    spans: BTreeMap<String, (usize, usize)>,
}

impl JsonSpanParser<'_> {
    fn skip_whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }

    fn parse_value(&mut self, pointer: &str) -> Result<()> {
        self.skip_whitespace();
        let start = self.offset;
        match self.source.as_bytes().get(self.offset) {
            Some(b'{') => {
                self.offset += 1;
                self.skip_whitespace();
                while self.source.as_bytes().get(self.offset) != Some(&b'}') {
                    let (key_start, key_end) = self.parse_string()?;
                    let key: String = serde_json::from_str(&self.source[key_start..key_end])?;
                    self.skip_whitespace();
                    if self.source.as_bytes().get(self.offset) != Some(&b':') {
                        bail!("invalid JSON object separator");
                    }
                    self.offset += 1;
                    let child = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                    self.parse_value(&child)?;
                    self.skip_whitespace();
                    match self.source.as_bytes().get(self.offset) {
                        Some(b',') => {
                            self.offset += 1;
                            self.skip_whitespace();
                        }
                        Some(b'}') => {}
                        _ => bail!("invalid JSON object"),
                    }
                }
                self.offset += 1;
            }
            Some(b'[') => {
                self.offset += 1;
                self.skip_whitespace();
                let mut index = 0usize;
                while self.source.as_bytes().get(self.offset) != Some(&b']') {
                    self.parse_value(&format!("{pointer}/{index}"))?;
                    index += 1;
                    self.skip_whitespace();
                    match self.source.as_bytes().get(self.offset) {
                        Some(b',') => {
                            self.offset += 1;
                            self.skip_whitespace();
                        }
                        Some(b']') => {}
                        _ => bail!("invalid JSON array"),
                    }
                }
                self.offset += 1;
            }
            Some(b'"') => {
                self.parse_string()?;
            }
            Some(_) => {
                while self.source.as_bytes().get(self.offset).is_some_and(|byte| {
                    !byte.is_ascii_whitespace() && !matches!(byte, b',' | b']' | b'}')
                }) {
                    self.offset += 1;
                }
            }
            None => bail!("unexpected end of JSON"),
        }
        self.spans.insert(pointer.into(), (start, self.offset));
        Ok(())
    }

    fn parse_string(&mut self) -> Result<(usize, usize)> {
        let start = self.offset;
        if self.source.as_bytes().get(self.offset) != Some(&b'"') {
            bail!("expected JSON string");
        }
        self.offset += 1;
        while let Some(byte) = self.source.as_bytes().get(self.offset) {
            match byte {
                b'\\' => self.offset += 2,
                b'"' => {
                    self.offset += 1;
                    return Ok((start, self.offset));
                }
                _ => self.offset += 1,
            }
        }
        bail!("unterminated JSON string")
    }
}

fn openapi_operation_span(
    text: &str,
    method: &str,
    path: &str,
) -> Option<(usize, usize, usize, usize, String)> {
    let lines = text
        .split_inclusive('\n')
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len();
            // Git checkouts on Windows may present this YAML with CRLF line
            // endings. Keep the key parser independent of the line ending so
            // the same OpenAPI selector resolves on every runner.
            Some((start, line.trim_end_matches(['\r', '\n'])))
        })
        .collect::<Vec<_>>();
    let paths_index = lines
        .iter()
        .position(|(_, line)| yaml_key(line) == Some("paths"))?;
    let paths_indent = indentation(lines[paths_index].1)?;
    let path_index = lines
        .iter()
        .enumerate()
        .skip(paths_index + 1)
        .take_while(|(_, (_, line))| {
            line.trim().is_empty() || indentation(line).is_some_and(|indent| indent > paths_indent)
        })
        .find_map(|(index, (_, line))| {
            (indentation(line).is_some_and(|indent| indent > paths_indent)
                && yaml_key(line) == Some(path))
            .then_some(index)
        })?;
    let path_indent = indentation(lines[path_index].1)?;
    let method = method.to_ascii_lowercase();
    let method_index = lines
        .iter()
        .enumerate()
        .skip(path_index + 1)
        .take_while(|(_, (_, line))| {
            line.trim().is_empty() || indentation(line).is_some_and(|indent| indent > path_indent)
        })
        .find_map(|(index, (_, line))| {
            (indentation(line).is_some_and(|indent| indent > path_indent)
                && yaml_key(line) == Some(method.as_str()))
            .then_some(index)
        })?;
    let method_indent = indentation(lines[method_index].1)?;
    let end_index = lines
        .iter()
        .enumerate()
        .skip(method_index + 1)
        .find_map(|(index, (_, line))| {
            (!line.trim().is_empty()
                && indentation(line).is_some_and(|indent| indent <= method_indent))
            .then_some(index)
        })
        .unwrap_or(lines.len());
    exact_span(
        text,
        lines[method_index].0,
        lines.get(end_index).map_or(text.len(), |line| line.0),
    )
    .ok()
}

fn indentation(line: &str) -> Option<usize> {
    (!line.starts_with('\t')).then_some(line.len() - line.trim_start_matches(' ').len())
}

fn yaml_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let key = trimmed
        .strip_suffix(':')
        .or_else(|| trimmed.split_once(": #").map(|(key, _)| key))?;
    Some(key.trim_matches('"').trim_matches('\''))
}

fn exact_span(
    text: &str,
    start: usize,
    end: usize,
) -> Result<(usize, usize, usize, usize, String)> {
    let excerpt = text
        .get(start..end)
        .context("source span is not on UTF-8 boundaries")?
        .to_owned();
    let line_start = text[..start].bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_end = line_start + excerpt.bytes().filter(|byte| *byte == b'\n').count();
    Ok((start, end, line_start, line_end.max(line_start), excerpt))
}

fn hash_bytes(value: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(value);
    format_sha256(hash.finalize())
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

    fn target(selector: Selector) -> mitase_spec_model::ArtifactTarget {
        mitase_spec_model::ArtifactTarget {
            id: "doc".into(),
            adapter: "rust".into(),
            path: RepoPath::new("src/doc.md").expect("path"),
            selector,
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
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
    fn json_pointer_and_openapi_operation_spans_are_exact() {
        let json =
            "{\n  \"editable\": { \"name\": \"one\" },\n  \"readonly\": { \"name\": \"two\" }\n}\n";
        let (start, end, _, _, excerpt) = json_pointer_span(json, "/editable").unwrap();
        assert_eq!(excerpt, "{ \"name\": \"one\" }");
        assert!(json[start..end].contains("one"));
        assert!(!json[start..end].contains("two"));

        let openapi = concat!(
            "paths:\n",
            "  /items:\n",
            "    get:\n",
            "      responses: {}\n",
            "    post:\n",
            "      responses: {}\n",
        );
        let (_, _, _, _, excerpt) = openapi_operation_span(openapi, "get", "/items").unwrap();
        assert!(excerpt.contains("get:"));
        assert!(!excerpt.contains("post:"));
    }

    #[test]
    fn openapi_operation_span_accepts_crlf_line_endings() {
        let openapi = "paths:\r\n  /sessions:\r\n    post:\r\n      responses: {}\r\n";
        let (_, _, _, _, excerpt) = openapi_operation_span(openapi, "post", "/sessions").unwrap();
        assert!(excerpt.contains("post:"));
    }

    #[test]
    fn json_openapi_operations_resolve_to_their_exact_escaped_pointer_span() {
        let tempdir = tempdir().expect("tempdir");
        let source = concat!(
            "{\n",
            "  \"paths\": {\n",
            "    \"/users/~current\": {\n",
            "      \"get\": { \"summary\": \"current\" },\n",
            "      \"post\": { \"summary\": \"other method\" }\n",
            "    },\n",
            "    \"/users\": {\n",
            "      \"get\": { \"summary\": \"other path\" }\n",
            "    }\n",
            "  }\n",
            "}\n",
        );
        fs::write(tempdir.path().join("openapi.json"), source).expect("openapi");
        let target = ArtifactTarget {
            id: "operation".into(),
            adapter: "openapi".into(),
            path: RepoPath::new("openapi.json").expect("repo path"),
            selector: Selector::Operation {
                method: "GET".into(),
                path: "/users/~current".into(),
            },
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let resolved = resolve_target(tempdir.path(), &target).expect("exact JSON operation");
        assert!(resolved.excerpt.contains("current"));
        assert!(!resolved.excerpt.contains("other method"));
        assert!(!resolved.excerpt.contains("other path"));
        assert_eq!(resolved.line_start, 4);
        assert_eq!(resolved.line_end, 4);
    }

    #[test]
    fn nested_spec_directories_load_from_noncanonical_workspace_roots() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("spec/requirements")).expect("nested spec dir");
        fs::write(
            tempdir.path().join("mitase.yaml"),
            concat!(
                "schema: mitase/config/v1\n",
                "workspace:\n",
                "  spec_roots: [spec]\n",
                "  excludes: []\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles:\n",
                "    - id: default\n",
                "      providers: { rust: {} }\n",
                "validation:\n",
                "  preset: strict\n",
                "  readiness:\n",
                "    target: verifiable\n",
                "    limits: { max_ownership_scope_units: 64 }\n",
                "  changed:\n",
                "    baseline:\n",
                "      strategy: merge-base\n",
                "      against: origin/main\n",
                "    require_owned_changes: true\n",
                "verification: { runners: {} }\n",
            ),
        )
        .expect("config");
        fs::write(
            tempdir.path().join("spec/requirements/req-new.yaml"),
            concat!(
                "schema: mitase/spec/v1\n",
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

    #[test]
    fn planned_features_do_not_own_active_artifacts() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("docs/mitase")).expect("spec dir");
        fs::create_dir_all(tempdir.path().join("src")).expect("source dir");
        fs::write(tempdir.path().join("src/lib.rs"), "pub fn api() {}\n").expect("source");
        fs::write(
            tempdir.path().join("mitase.yaml"),
            concat!(
                "schema: mitase/config/v1\n",
                "workspace: { spec_roots: [docs/mitase], excludes: [] }\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles: [{ id: default, providers: { rust: {} } }]\n",
                "validation:\n",
                "  preset: strict\n",
                "  readiness:\n",
                "    target: 'off'\n",
                "    limits: { max_ownership_scope_units: 64 }\n",
                "  changed: { require_owned_changes: false }\n",
                "verification: { runners: {} }\n",
            ),
        )
        .expect("config");
        fs::write(
            tempdir.path().join("docs/mitase/planned.yaml"),
            concat!(
                "schema: mitase/spec/v1\n",
                "kind: features\n",
                "namespace: test\n",
                "category: Test\n",
                "features:\n",
                "  - id: FEAT-TEST-001\n",
                "    title: Absent target\n",
                "    summary: Preserve an explicit removed target.\n",
                "    status: implemented\n",
                "    bindings:\n",
                "      - id: implementation\n",
                "        role: implementation\n",
                "        facet: test\n",
                "        responsibility: Describe future work.\n",
                "        owns:\n",
                "          - { id: module, adapter: rust, path: src/lib.rs, selector: { kind: module, name: lib } }\n",
                "        targets:\n",
                "          - { id: api, adapter: rust, path: src/lib.rs, selector: { kind: symbol, name: api }, lifecycle: absent, claims: [{ kind: satisfies, criterion: REQ-TEST-001#criterion.api }] }\n",
                "      - id: verification\n",
                "        role: verification\n",
                "        facet: test\n",
                "        responsibility: Describe future verification.\n",
                "        targets:\n",
                "          - id: api-test\n",
                "            adapter: rust\n",
                "            path: src/lib.rs\n",
                "            selector: { kind: symbol, name: api }\n",
                "            claims:\n",
                "              - kind: verifies\n",
                "                criterion: REQ-TEST-001#criterion.api\n",
                "                covers: [FEAT-TEST-001#binding.implementation/target.api]\n",
                "                runner: { runner: cargo-test, arguments: { package: test, test: api } }\n",
            ),
        )
        .expect("feature");

        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let identity = "rust:src/lib.rs::lib::api";
        assert!(
            index
                .artifact_units
                .iter()
                .any(|unit| unit.identity == identity)
        );
        assert!(index.artifact_owners.get(identity).is_none_or(|owners| {
            owners.iter().all(|owner| {
                owner
                    .target_id
                    .as_ref()
                    .is_none_or(|target_id| target_id.0 != "api")
            })
        }));
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.api".parse().expect("criterion");
        let implementation: BoundTargetRef = "FEAT-TEST-001#binding.implementation/target.api"
            .parse()
            .expect("implementation target");
        let verification: BoundTargetRef = "FEAT-TEST-001#binding.verification/target.api-test"
            .parse()
            .expect("verification target");
        assert!(
            index
                .all_criteria_to_implementation_targets
                .get(&criterion)
                .is_some_and(|targets| targets.contains(&implementation))
        );
        assert!(
            index
                .all_criteria_to_verification_targets
                .get(&criterion)
                .is_some_and(|targets| targets.contains(&verification))
        );
        assert!(
            index
                .all_verification_by_target
                .get(&implementation)
                .is_some_and(|targets| targets.contains(&verification))
        );
        assert!(
            index
                .criteria_to_implementation_targets
                .get(&criterion)
                .is_none_or(Vec::is_empty)
        );
        assert!(
            index
                .criteria_to_verification_targets
                .get(&criterion)
                .is_some_and(|targets| targets.contains(&verification))
        );
        assert!(
            index
                .verification_by_target
                .get(&implementation)
                .is_none_or(Vec::is_empty)
        );
    }
}
