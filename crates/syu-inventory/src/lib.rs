#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syu_project_model::InventoryProfile;
use syu_spec_model::RepoPath;

#[derive(Debug, Clone)]
pub struct InventoryContext {
    pub workspace_root: PathBuf,
    pub profile: String,
}

pub trait InventoryProvider: Send + Sync {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryFragment {
    pub units: Vec<ArtifactUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactUnit {
    pub adapter: String,
    pub path: RepoPath,
    pub identity: String,
    pub kind: ArtifactUnitKind,
    pub exposure: ArtifactExposure,
    pub reachability: ArtifactReachability,
    pub span: SourceSpan,
    pub digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactUnitKind {
    File,
    Symbol,
    Operation,
    Heading,
    Generated,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactExposure {
    Public,
    Workspace,
    Private,
    Test,
    Generated,
    Support,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ArtifactReachability {
    Active,
    Conditional { profile: String },
    Inactive,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

/// Deliberately boring provider used for declared/documentation assets. Language
/// providers can refine this fragment without changing the inventory contract.
pub struct FileInventoryProvider {
    pub adapter: String,
    pub roots: Vec<RepoPath>,
}
impl InventoryProvider for FileInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        let mut files = Vec::new();
        for root in &self.roots {
            collect(&context.workspace_root, root.as_path(), &mut files)?;
        }
        files.sort();
        files.dedup();
        let units = files
            .into_iter()
            .map(|path| unit(&context.workspace_root, &self.adapter, path))
            .collect::<Result<Vec<_>>>()?;
        Ok(InventoryFragment { units })
    }
}
fn collect(root: &Path, relative: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let path = root.join(relative);
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        out.push(path);
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect(
                root,
                path.strip_prefix(root).context("inventory path")?,
                out,
            )?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
fn unit(root: &Path, adapter: &str, path: PathBuf) -> Result<ArtifactUnit> {
    let relative = path
        .strip_prefix(root)
        .context("inventory path escaped workspace")?;
    let path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
    let bytes = fs::read(root.join(path.as_path()))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut hash = Sha256::new();
    hash.update(&bytes);
    Ok(ArtifactUnit {
        adapter: adapter.into(),
        identity: format!("{}:{}", adapter, path.to_string_lossy()),
        path,
        kind: ArtifactUnitKind::File,
        exposure: ArtifactExposure::Workspace,
        reachability: ArtifactReachability::Active,
        span: SourceSpan {
            byte_start: 0,
            byte_end: bytes.len(),
            line_start: 1,
            line_end: text.lines().count().max(1),
        },
        digest: format!("sha256:{:x}", hash.finalize()),
    })
}

pub fn union(
    context: &InventoryContext,
    providers: &[Box<dyn InventoryProvider>],
) -> Result<Vec<ArtifactUnit>> {
    let mut units = Vec::new();
    for provider in providers {
        units.extend(provider.discover(context)?.units);
    }
    units.sort_by(|a, b| a.identity.cmp(&b.identity));
    if units.is_empty() {
        bail!("active inventory is empty");
    }
    if units
        .windows(2)
        .any(|pair| pair[0].identity == pair[1].identity)
    {
        bail!("inventory contains duplicate identities");
    }
    Ok(units)
}

/// The canonical inventory entry point. Every enabled provider in the active
/// profile contributes a fragment; selecting Cargo must never silently turn
/// off non-Rust discovery.
pub struct InventoryRegistry;

impl InventoryRegistry {
    pub fn discover(
        context: &InventoryContext,
        profile: &InventoryProfile,
    ) -> Result<Vec<ArtifactUnit>> {
        let providers = profile
            .providers
            .keys()
            .filter_map(|adapter| provider_for(adapter))
            .collect::<Vec<_>>();
        if providers.is_empty() {
            bail!("active inventory profile has no supported providers");
        }
        union(context, &providers)
    }
}

fn provider_for(adapter: &str) -> Option<Box<dyn InventoryProvider>> {
    let extensions: &[&str] = match adapter {
        "rust" => return Some(Box::new(RustInventoryProvider)),
        "javascript" => &["js", "jsx", "mjs", "cjs"],
        "typescript" => &["ts", "tsx", "mts", "cts"],
        "python" => &["py"],
        "go" => &["go"],
        "shell" => &["sh", "bash", "zsh"],
        "openapi" => &["yaml", "yml", "json"],
        "documentation" | "markdown" => &["md", "mdx"],
        // Declared artifacts are explicitly configured in the profile. The
        // current v1 model intentionally does not treat a broad declaration
        // as a fallback inventory denominator.
        "declared" => return None,
        _ => return None,
    };
    Some(Box::new(ExtensionInventoryProvider {
        adapter: adapter.into(),
        extensions: extensions
            .iter()
            .map(|extension| (*extension).into())
            .collect(),
    }))
}

struct ExtensionInventoryProvider {
    adapter: String,
    extensions: Vec<String>,
}

impl InventoryProvider for ExtensionInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        let mut files = Vec::new();
        collect_matching(
            &context.workspace_root,
            &context.workspace_root,
            &self.extensions,
            &mut files,
        )?;
        files.sort();
        let units = files
            .into_iter()
            .map(|path| unit(&context.workspace_root, &self.adapter, path))
            .collect::<Result<Vec<_>>>()?;
        Ok(InventoryFragment { units })
    }
}

/// Rust inventory uses the syntax tree rather than line-oriented symbol
/// searches. Every declared item receives an exact identity and source span.
pub struct RustInventoryProvider;

impl InventoryProvider for RustInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        let mut files = Vec::new();
        collect_matching(
            &context.workspace_root,
            &context.workspace_root,
            &["rs".into()],
            &mut files,
        )?;
        let mut units = Vec::new();
        for path in files {
            let source = fs::read_to_string(&path)
                .with_context(|| format!("read Rust source {}", path.display()))?;
            let syntax = syn::parse_file(&source)
                .with_context(|| format!("parse Rust source {}", path.display()))?;
            let relative = path
                .strip_prefix(&context.workspace_root)
                .context("Rust inventory path escaped workspace")?;
            let repo_path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
            let mut visitor = RustVisitor {
                adapter: "rust".into(),
                path: repo_path,
                source: &source,
                offsets: line_offsets(&source),
                units: Vec::new(),
            };
            visitor.visit_file(&syntax);
            units.extend(visitor.units);
        }
        Ok(InventoryFragment { units })
    }
}

struct RustVisitor<'a> {
    adapter: String,
    path: RepoPath,
    source: &'a str,
    offsets: Vec<usize>,
    units: Vec<ArtifactUnit>,
}

impl<'ast> syn::visit::Visit<'ast> for RustVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.add(&item.sig.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.add(&item.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.add(&item.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.add(&item.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_trait(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.add(&item.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_mod(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.add(&item.sig.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.add(
            &item.sig.ident.to_string(),
            &syn::Visibility::Inherited,
            item.span(),
        );
        syn::visit::visit_trait_item_fn(self, item);
    }
}

impl RustVisitor<'_> {
    fn add(&mut self, name: &str, visibility: &syn::Visibility, span: proc_macro2::Span) {
        let start = span.start();
        let end = span.end();
        let line_start = start.line.max(1);
        let line_end = end.line.max(line_start);
        let byte_start = self.offsets.get(line_start - 1).copied().unwrap_or(0) + start.column;
        let byte_end = (self
            .offsets
            .get(line_end - 1)
            .copied()
            .unwrap_or(self.source.len())
            + end.column)
            .min(self.source.len());
        let excerpt = self.source.get(byte_start..byte_end).unwrap_or_default();
        self.units.push(ArtifactUnit {
            adapter: self.adapter.clone(),
            path: self.path.clone(),
            identity: format!(
                "rust:{}:{}@{}",
                self.path.to_string_lossy(),
                name,
                line_start
            ),
            kind: ArtifactUnitKind::Symbol,
            exposure: if matches!(visibility, syn::Visibility::Public(_)) {
                ArtifactExposure::Public
            } else {
                ArtifactExposure::Private
            },
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start,
                byte_end,
                line_start,
                line_end,
            },
            digest: digest(excerpt.as_bytes()),
        });
    }
}

fn line_offsets(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
        .collect()
}

fn digest(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn collect_matching(
    root: &Path,
    directory: &Path,
    extensions: &[String],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|name| name == ".git" || name == "target" || name == "node_modules")
        {
            continue;
        }
        if path.is_dir() {
            collect_matching(root, &path, extensions, out)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.iter().any(|candidate| candidate == extension))
            && path.starts_with(root)
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn active_profile_unions_rust_and_javascript_providers() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::create_dir_all(temp.path().join("web")).unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn api() {}\n").unwrap();
        fs::write(temp.path().join("web/app.js"), "export const app = 1;\n").unwrap();
        let profile = InventoryProfile {
            id: "default".into(),
            providers: BTreeMap::from([
                ("rust".into(), serde_yaml::Value::Null),
                ("javascript".into(), serde_yaml::Value::Null),
            ]),
        };
        let units = InventoryRegistry::discover(
            &InventoryContext {
                workspace_root: temp.path().into(),
                profile: "default".into(),
            },
            &profile,
        )
        .unwrap();
        assert!(units.iter().any(|unit| unit.adapter == "rust"));
        assert!(units.iter().any(|unit| unit.adapter == "javascript"));
        assert!(units.iter().any(|unit| {
            unit.identity == "rust:src/lib.rs:api@1"
                && unit.kind == ArtifactUnitKind::Symbol
                && unit.exposure == ArtifactExposure::Public
        }));
    }
}
