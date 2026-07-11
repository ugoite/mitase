#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syu_code_intel::{InventorySymbol, inventory_symbols, resolve_symbol};
use syu_project_model::{CONFIG_SCHEMA, ProjectConfig};
use syu_spec_model::*;

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub document: SpecDocument,
}
pub struct SpecWorkspace {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub documents: Vec<LoadedDocument>,
    matcher: WorkspaceMatcher,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInventoryEntry {
    pub path: RepoPath,
    pub adapter: String,
    pub symbol: InventorySymbol,
}
#[derive(Debug)]
struct WorkspaceMatcher {
    spec_roots: Vec<RepoPath>,
    artifact_roots: Vec<RepoPath>,
    excludes: Option<GlobSet>,
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
        let matcher = WorkspaceMatcher::build(&config)?;
        let mut paths = Vec::new();
        for spec_root in &config.workspace.spec_roots {
            collect_yaml(&root, spec_root.as_path(), &mut paths)?;
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
            matcher,
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
            if let Ok(relative) = doc.path.strip_prefix(&self.root) {
                hash.update(relative.to_string_lossy().as_bytes());
            }
            hash.update(fs::read(&doc.path).unwrap_or_default());
        }
        format!("sha256:{:x}", hash.finalize())
    }
    pub fn path_is_spec(&self, path: &Path) -> bool {
        self.matcher.contains(&self.matcher.spec_roots, path)
    }
    pub fn path_is_artifact(&self, path: &Path) -> bool {
        self.matcher.contains(&self.matcher.artifact_roots, path)
    }
    pub fn path_is_excluded(&self, path: &Path) -> bool {
        self.matcher.is_excluded(path)
    }

    /// Discover every addressable implementation symbol and test case in the
    /// configured artifact scope.  Non-code adapters deliberately contribute no
    /// entries: coverage only has a symbol/test denominator where an adapter can
    /// resolve the same identity later for planning.
    pub fn artifact_inventory(&self) -> Result<Vec<ArtifactInventoryEntry>> {
        let mut files = BTreeSet::new();
        for root in &self.config.workspace.artifact_roots {
            collect_artifact_files(&self.root, root.as_path(), &mut files)?;
        }
        let mut entries = Vec::new();
        for path in files {
            let relative = path
                .strip_prefix(&self.root)
                .context("artifact path must stay within workspace")?;
            if self.path_is_excluded(relative) || self.path_is_spec(relative) {
                continue;
            }
            let Some(adapter) = adapter_for_path(relative) else {
                continue;
            };
            if !self
                .config
                .adapters
                .enabled
                .iter()
                .any(|enabled| enabled == adapter)
            {
                continue;
            }
            let source = fs::read_to_string(&path)
                .with_context(|| format!("read inventory source {}", path.display()))?;
            let repo_path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
            for symbol in inventory_symbols(adapter, &source)
                .with_context(|| format!("inventory {}", path.display()))?
            {
                entries.push(ArtifactInventoryEntry {
                    path: repo_path.clone(),
                    adapter: adapter.to_string(),
                    symbol,
                });
            }
        }
        entries.sort_by(|left, right| {
            (&left.path.to_string_lossy(), &left.symbol.identity)
                .cmp(&(&right.path.to_string_lossy(), &right.symbol.identity))
        });
        Ok(entries)
    }
}

fn collect_artifact_files(root: &Path, relative: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("artifact root does not exist: {}", relative.display()))?;
    if metadata.file_type().is_symlink() {
        bail!(
            "artifact root must not be a symlink: {}",
            relative.display()
        );
    }
    if metadata.is_file() {
        out.insert(path);
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            let relative = path
                .strip_prefix(root)
                .context("artifact path must stay within workspace")?;
            collect_artifact_files(root, relative, out)?;
        } else if metadata.is_file() {
            out.insert(path);
        }
    }
    Ok(())
}

fn adapter_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Some("rust"),
        Some("ts" | "tsx") => Some("typescript"),
        Some("js" | "mjs" | "cjs") => Some("javascript"),
        Some("py") => Some("python"),
        Some("go") => Some("go"),
        Some("sh") => Some("shell"),
        _ => None,
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
                if workspace.config.workspace.artifact_roots.is_empty() {
                    bail!("workspace.artifact_roots cannot be empty when bindings exist");
                }
                if !workspace.path_is_artifact(target.path.as_path()) {
                    bail!(
                        "target path {} is outside workspace.artifact_roots",
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
            artifact_roots: config.workspace.artifact_roots.clone(),
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
                if names.is_empty() {
                    bail!("symbol selector must contain at least one symbol");
                }
                let mut unique = names.clone();
                unique.sort();
                unique.dedup();
                if unique.len() != names.len() {
                    bail!("symbol selector must not contain duplicate symbol names");
                }
                // Full-repository ownership bindings group many exact
                // identities in one target.  Parse the file once for those
                // bindings instead of resolving every identity independently.
                let resolved = if names.len() > 1 {
                    match inventory_symbols(&target.adapter, &text) {
                        Ok(inventory) => {
                            for name in names {
                                if !inventory.iter().any(|symbol| symbol.identity == *name) {
                                    bail!("symbol {name} has no definition");
                                }
                            }
                            vec![syu_code_intel::SymbolResolution {
                                identity: names.join(", "),
                                kind: "inventory".to_string(),
                                byte_start: 0,
                                byte_end: content.len(),
                                line_start: 1,
                                line_end: text.lines().count().max(1),
                                excerpt: text.to_string(),
                                excerpt_hash: hash_bytes(&content),
                            }]
                        }
                        Err(_) => names
                            .iter()
                            .map(|name| resolve_symbol(&target.adapter, &text, name))
                            .collect::<Result<Vec<_>>>()?,
                    }
                } else {
                    names
                        .iter()
                        .map(|name| resolve_symbol(&target.adapter, &text, name))
                        .collect::<Result<Vec<_>>>()?
                };
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
    format!("sha256:{:x}", hash.finalize())
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
                "  artifact_roots: [src]\n",
                "  excludes: []\n",
                "profiles: { active: [], custom: {} }\n",
                "validation:\n",
                "  preset: agent-ready\n",
"  coverage: { artifact_ownership: off, spec_fulfillment: off }\n",
                "  deny_warnings: false\n",
                "  rules: {}\n",
                "  changed:\n",
                "    baseline:\n",
                "      strategy: merge-base\n",
                "      against: origin/main\n",
                "    require_owned_changes: true\n",
                "work:\n",
                "  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 6, max_readonly_targets: 12, max_total_bytes: 120000 }\n",
                "  context: { include_parent_principles: true, include_parent_rules: true }\n",
                "adapters: { enabled: [rust] }\n",
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
