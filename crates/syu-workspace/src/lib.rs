#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syu_code_intel::resolve_symbol;
use syu_project_model::{CONFIG_SCHEMA, ProjectConfig};
use syu_spec_model::*;

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub document: SpecDocument,
}
#[derive(Debug, Clone)]
pub struct SpecWorkspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub documents: Vec<LoadedDocument>,
}

#[derive(Debug, Clone, Default)]
pub struct SpecIndex {
    pub anchors: BTreeMap<SpecAnchor, AnchorValue>,
    pub bindings: BTreeMap<SpecAnchor, ArtifactBinding>,
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
        let config_path = root.join("syu.yaml");
        let config: ProjectConfig = serde_yaml::from_str(
            &fs::read_to_string(&config_path)
                .with_context(|| format!("read {}", config_path.display()))?,
        )
        .context("parse syu/config/v1")?;
        if config.schema != CONFIG_SCHEMA {
            bail!("config schema must be {CONFIG_SCHEMA}");
        }
        let mut paths = Vec::new();
        for spec_root in &config.workspace.spec_roots {
            collect_yaml(&root.join(spec_root), &mut paths)?;
        }
        paths.sort();
        let mut documents = Vec::new();
        for path in paths {
            let document: SpecDocument = serde_yaml::from_str(&fs::read_to_string(&path)?)
                .with_context(|| format!("strict parse {}", path.display()))?;
            if document.schema() != SPEC_SCHEMA {
                bail!("{}: schema must be {SPEC_SCHEMA}", path.display());
            }
            documents.push(LoadedDocument { path, document });
        }
        Ok(Self {
            root,
            config,
            documents,
        })
    }
    pub fn index(&self) -> Result<SpecIndex> {
        SpecIndex::build(self)
    }
    pub fn fingerprint(&self) -> String {
        let mut hash = Sha256::new();
        if let Ok(config) = serde_yaml::to_string(&self.config) {
            hash.update(config.as_bytes());
        }
        for doc in &self.documents {
            hash.update(doc.path.to_string_lossy().as_bytes());
            hash.update(fs::read(&doc.path).unwrap_or_default());
        }
        format!("sha256:{:x}", hash.finalize())
    }
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
                                    .entry(p.binding.clone())
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
                let rendered = target.path.to_string_lossy();
                if !workspace.config.workspace.artifact_roots.is_empty()
                    && !path_is_within_roots(
                        rendered.as_ref(),
                        &workspace.config.workspace.artifact_roots,
                    )
                {
                    bail!("target path {rendered} is outside workspace.artifact_roots");
                }
                if path_is_excluded(rendered.as_ref(), &workspace.config.workspace.excludes) {
                    bail!("target path {rendered} is excluded by workspace.excludes");
                }
                out.path_to_targets
                    .entry(target.path.to_string_lossy().into_owned())
                    .or_default()
                    .push(BoundTargetRef {
                        binding: anchor.clone(),
                        target_id: target.id.clone(),
                    });
            }
            for criterion in &binding.satisfies {
                out.criteria_to_implementations
                    .entry(criterion.clone())
                    .or_default()
                    .push(anchor.clone());
            }
            for criterion in &binding.verifies {
                out.criteria_to_verifications
                    .entry(criterion.clone())
                    .or_default()
                    .push(anchor.clone());
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

fn unique_item(ids: &mut BTreeSet<SpecId>, id: &SpecId) -> Result<()> {
    if !ids.insert(id.clone()) {
        bail!("duplicate item id {id}");
    }
    Ok(())
}

fn path_is_within_roots(path: &str, roots: &[String]) -> bool {
    roots
        .iter()
        .any(|root| path == root || path.starts_with(&format!("{root}/")))
}

fn path_is_excluded(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        pattern
            .strip_suffix("/**")
            .map_or(path == pattern, |prefix| {
                path == prefix || path.starts_with(&format!("{prefix}/"))
            })
    })
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
fn collect_yaml(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !path.exists() {
        bail!("spec root does not exist: {}", path.display());
    }
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml(&path, out)?;
        } else if matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("yaml" | "yml")
        ) {
            out.push(path);
        }
    }
    Ok(())
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
pub fn resolve_target_with_adapters(
    root: &Path,
    target: &ArtifactTarget,
    enabled: &[String],
) -> Result<ResolvedTarget> {
    const KNOWN: &[&str] = &[
        "rust",
        "typescript",
        "shell",
        "python",
        "go",
        "markdown",
        "openapi",
        "yaml",
        "json",
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
    let text = String::from_utf8_lossy(&content);
    let (description, symbols, byte_start, byte_end, line_start, line_end, excerpt, excerpt_hash) =
        match &target.selector {
            Selector::File => (
                "file".into(),
                vec![],
                0,
                content.len(),
                1,
                text.lines().count(),
                text.to_string(),
                hash_bytes(&content),
            ),
            Selector::Symbol { names } => {
                let resolved = names
                    .iter()
                    .map(|name| resolve_symbol(&target.adapter, &text, name))
                    .collect::<Result<Vec<_>>>()?;
                let start = resolved.iter().map(|r| r.byte_start).min().unwrap_or(0);
                let end = resolved.iter().map(|r| r.byte_end).max().unwrap_or(0);
                let excerpt = text[start..end].to_string();
                (
                    format!("symbols {}", names.join(", ")),
                    names.clone(),
                    start,
                    end,
                    resolved.iter().map(|r| r.line_start).min().unwrap_or(1),
                    resolved.iter().map(|r| r.line_end).max().unwrap_or(1),
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Operation { method, path } => {
                let yaml: serde_yaml::Value = serde_yaml::from_slice(&content)?;
                let exists = yaml
                    .get("paths")
                    .and_then(|v| v.get(path))
                    .and_then(|v| v.get(method.to_ascii_lowercase()))
                    .is_some();
                if !exists {
                    bail!("operation {method} {path} not found");
                }
                let excerpt = text.to_string();
                (
                    format!("operation {} {path}", method.to_ascii_uppercase()),
                    vec![],
                    0,
                    content.len(),
                    1,
                    text.lines().count(),
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::Heading { value } => {
                if !text
                    .lines()
                    .any(|l| l.trim_start_matches('#').trim() == value)
                {
                    bail!("heading {value} not found");
                }
                let excerpt = text.to_string();
                (
                    format!("heading {value}"),
                    vec![],
                    0,
                    content.len(),
                    1,
                    text.lines().count(),
                    excerpt.clone(),
                    hash_bytes(excerpt.as_bytes()),
                )
            }
            Selector::JsonPointer { value } => {
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
                if !text.contains(value) {
                    bail!("marker {value} not found");
                }
                let excerpt = text.to_string();
                (
                    format!("marker {value}"),
                    vec![],
                    0,
                    content.len(),
                    1,
                    text.lines().count(),
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
        content_hash: format!("sha256:{:x}", hash.finalize()),
        bytes: content.len(),
        byte_start,
        byte_end,
        line_start,
        line_end,
        excerpt,
        excerpt_hash,
    })
}
fn hash_bytes(value: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(value);
    format!("sha256:{:x}", hash.finalize())
}
