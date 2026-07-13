#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
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
    pub settings: serde_yaml::Value,
    pub excludes: Vec<String>,
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
            collect(
                &context.workspace_root,
                root.as_path(),
                &context.excludes,
                &mut files,
            )?;
        }
        files.sort();
        files.dedup();
        let mut units = Vec::new();
        for path in files {
            units.push(unit(&context.workspace_root, &self.adapter, path.clone())?);
            if self.adapter == "markdown" {
                units.extend(markdown_headings(&context.workspace_root, path)?);
            }
        }
        Ok(InventoryFragment { units })
    }
}

fn openapi_operations(root: &Path, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let relative = path
        .strip_prefix(root)
        .context("OpenAPI path escaped workspace")?;
    let repo_path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
    let document: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
    let mut units = Vec::new();
    if let Some(paths) = document
        .get("paths")
        .and_then(serde_yaml::Value::as_mapping)
    {
        for (path_value, operations) in paths {
            let Some(path_value) = path_value.as_str() else {
                continue;
            };
            let Some(operations) = operations.as_mapping() else {
                continue;
            };
            for (method_value, _) in operations {
                let Some(method) = method_value.as_str() else {
                    continue;
                };
                if !matches!(
                    method.to_ascii_lowercase().as_str(),
                    "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace"
                ) {
                    continue;
                }
                units.push(ArtifactUnit {
                    adapter: "openapi".into(),
                    path: repo_path.clone(),
                    identity: format!(
                        "openapi:{}::{} {}",
                        repo_path.to_string_lossy(),
                        method.to_ascii_uppercase(),
                        path_value
                    ),
                    kind: ArtifactUnitKind::Operation,
                    exposure: ArtifactExposure::Public,
                    reachability: ArtifactReachability::Active,
                    span: SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line_start: 1,
                        line_end: 1,
                    },
                    digest: digest(format!("{} {}", method, path_value).as_bytes()),
                });
            }
        }
    }
    Ok(units)
}

fn markdown_headings(root: &Path, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let relative = path
        .strip_prefix(root)
        .context("markdown path escaped workspace")?;
    let repo_path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
    let source = fs::read_to_string(&path)?;
    let mut units = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(value) = trimmed
            .strip_prefix('#')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        units.push(ArtifactUnit {
            adapter: "markdown".into(),
            path: repo_path.clone(),
            identity: format!("markdown:{}::heading::{value}", repo_path.to_string_lossy()),
            kind: ArtifactUnitKind::Heading,
            exposure: ArtifactExposure::Workspace,
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: source
                    .lines()
                    .take(line_index)
                    .map(|line| line.len() + 1)
                    .sum(),
                byte_end: source
                    .lines()
                    .take(line_index + 1)
                    .map(|line| line.len() + 1)
                    .sum(),
                line_start: line_index + 1,
                line_end: line_index + 1,
            },
            digest: digest(line.as_bytes()),
        });
    }
    Ok(units)
}
fn collect(
    root: &Path,
    relative: &Path,
    excludes: &[String],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    let path = root.join(relative);
    if !path.exists() {
        return Ok(());
    }
    if excludes.iter().any(|pattern| glob_match(pattern, relative)) {
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
                excludes,
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
    units.sort_by(|a, b| {
        (a.path.to_string_lossy(), a.identity.as_str())
            .cmp(&(b.path.to_string_lossy(), b.identity.as_str()))
    });
    units.dedup_by(|a, b| {
        a.path == b.path && a.kind == b.kind && matches!(a.kind, ArtifactUnitKind::File)
    });
    if units.is_empty() {
        bail!("active inventory is empty");
    }
    if units
        .windows(2)
        .any(|pair| pair[0].identity == pair[1].identity)
    {
        let duplicates = units
            .windows(2)
            .filter(|pair| pair[0].identity == pair[1].identity)
            .map(|pair| pair[0].identity.clone())
            .collect::<Vec<_>>();
        bail!(
            "inventory contains duplicate identities: {}",
            duplicates.join(", ")
        );
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
        let mut excludes = context.excludes.clone();
        excludes.extend(
            profile
                .providers
                .values()
                .filter_map(|value| value.get("exclude"))
                .filter_map(|value| value.as_sequence())
                .flat_map(|values| values.iter().filter_map(|value| value.as_str()))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        );
        let providers = profile
            .providers
            .iter()
            .map(|(adapter, settings)| provider_for(adapter, settings.clone()))
            .collect::<Result<Vec<_>>>()?;
        let context = InventoryContext {
            settings: serde_yaml::Value::Null,
            excludes: excludes.clone(),
            ..context.clone()
        };
        if providers.is_empty() {
            bail!("active inventory profile has no supported providers");
        }
        union(&context, &providers)
    }
}

fn provider_for(adapter: &str, settings: serde_yaml::Value) -> Result<Box<dyn InventoryProvider>> {
    let roots = settings
        .get("roots")
        .or_else(|| settings.get("include"))
        .and_then(|value| value.as_sequence())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .filter_map(|value| RepoPath::new(value).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let extensions: &[&str] = match adapter {
        "rust" => {
            if roots.is_empty() {
                return Ok(Box::new(RustInventoryProvider));
            }
            return Ok(Box::new(ConfiguredRustInventoryProvider { roots }));
        }
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
        "declared" => {
            if roots.is_empty() {
                return Ok(Box::new(EmptyInventoryProvider));
            }
            return Ok(Box::new(FileInventoryProvider {
                adapter: adapter.into(),
                roots,
            }));
        }
        _ => bail!("inventory provider {adapter} is not implemented"),
    };
    Ok(Box::new(ExtensionInventoryProvider {
        adapter: adapter.into(),
        extensions: extensions
            .iter()
            .map(|extension| (*extension).into())
            .collect(),
        roots,
    }))
}

struct EmptyInventoryProvider;
impl InventoryProvider for EmptyInventoryProvider {
    fn discover(&self, _context: &InventoryContext) -> Result<InventoryFragment> {
        Ok(InventoryFragment::default())
    }
}

struct ExtensionInventoryProvider {
    adapter: String,
    extensions: Vec<String>,
    roots: Vec<RepoPath>,
}

impl InventoryProvider for ExtensionInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        let mut files = Vec::new();
        let roots = if self.roots.is_empty() {
            vec![PathBuf::new()]
        } else {
            self.roots
                .iter()
                .map(|root| root.as_path().to_path_buf())
                .collect()
        };
        for root in roots {
            collect_matching(
                &context.workspace_root,
                &context.workspace_root.join(root),
                &self.extensions,
                &context.excludes,
                &mut files,
            )?;
        }
        files.sort();
        let mut units = Vec::new();
        for path in files {
            units.push(unit(&context.workspace_root, &self.adapter, path.clone())?);
            if self.adapter == "markdown" {
                units.extend(markdown_headings(&context.workspace_root, path)?);
            } else if self.adapter == "openapi" {
                units.extend(openapi_operations(&context.workspace_root, path)?);
            } else if matches!(self.adapter.as_str(), "javascript" | "typescript") {
                units.extend(source_symbol_units(
                    &context.workspace_root,
                    path,
                    &self.adapter,
                )?);
            }
        }
        Ok(InventoryFragment { units })
    }
}

fn source_symbol_units(root: &Path, path: PathBuf, adapter: &str) -> Result<Vec<ArtifactUnit>> {
    let relative = path
        .strip_prefix(root)
        .context("source path escaped workspace")?;
    let repo_path = RepoPath::new(relative).map_err(anyhow::Error::msg)?;
    let source = fs::read_to_string(&path)?;
    let mut units = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line
            .trim_start()
            .strip_prefix("export ")
            .unwrap_or(line.trim_start())
            .strip_prefix("default ")
            .unwrap_or(
                line.trim_start()
                    .strip_prefix("export ")
                    .unwrap_or(line.trim_start()),
            );
        let trimmed = trimmed.strip_prefix("async ").unwrap_or(trimmed);
        let name = ["function ", "class ", "const ", "let ", "var "]
            .iter()
            .find_map(|prefix| trimmed.strip_prefix(prefix))
            .and_then(|rest| {
                rest.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                    .next()
            })
            .filter(|name| !name.is_empty());
        let Some(name) = name else { continue };
        units.push(ArtifactUnit {
            adapter: adapter.into(),
            path: repo_path.clone(),
            identity: format!(
                "{adapter}:{}::{}@{}",
                repo_path.to_string_lossy(),
                name,
                line_index + 1
            ),
            kind: ArtifactUnitKind::Symbol,
            exposure: if line.trim_start().starts_with("export ") {
                ArtifactExposure::Public
            } else {
                ArtifactExposure::Workspace
            },
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: 0,
                byte_end: 0,
                line_start: line_index + 1,
                line_end: line_index + 1,
            },
            digest: digest(line.as_bytes()),
        });
    }
    Ok(units)
}

/// Rust inventory uses the syntax tree rather than line-oriented symbol
/// searches. Every declared item receives an exact identity and source span.
pub struct RustInventoryProvider;

struct ConfiguredRustInventoryProvider {
    roots: Vec<RepoPath>,
}

impl InventoryProvider for RustInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        discover_rust(context, &[])
    }
}

impl InventoryProvider for ConfiguredRustInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        discover_rust(context, &self.roots)
    }
}

fn discover_rust(
    context: &InventoryContext,
    configured_roots: &[RepoPath],
) -> Result<InventoryFragment> {
    let mut files = Vec::new();
    let roots = if configured_roots.is_empty() {
        cargo_roots(&context.workspace_root)?
    } else {
        configured_roots
            .iter()
            .map(|root| context.workspace_root.join(root.as_path()))
            .collect()
    };
    if roots.is_empty() {
        collect_matching(
            &context.workspace_root,
            &context.workspace_root,
            &["rs".into()],
            &context.excludes,
            &mut files,
        )?;
    } else {
        for root in roots {
            files.push(root.clone());
            collect_reachable_rust(
                &context.workspace_root,
                &root,
                &context.excludes,
                &mut files,
            )?;
        }
    }
    files.sort();
    files.dedup();
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
        let is_test_file = relative.components().any(|component| {
            component.as_os_str() == "tests" || component.as_os_str() == "benches"
        });
        let mut visitor = RustVisitor {
            adapter: "rust".into(),
            path: repo_path,
            source: &source,
            offsets: line_offsets(&source),
            units: Vec::new(),
            module_path: vec![module_name(&path)],
            impl_type: None,
            attributes: Vec::new(),
            test_file: is_test_file,
        };
        visitor.visit_file(&syntax);
        units.extend(visitor.units);
    }
    Ok(InventoryFragment { units })
}

struct RustVisitor<'a> {
    adapter: String,
    path: RepoPath,
    source: &'a str,
    offsets: Vec<usize>,
    units: Vec<ArtifactUnit>,
    module_path: Vec<String>,
    impl_type: Option<String>,
    attributes: Vec<String>,
    test_file: bool,
}

impl<'ast> syn::visit::Visit<'ast> for RustVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.sig.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_item_fn(self, item);
        self.attributes = previous;
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
        let previous_attributes = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.ident.to_string(), &item.vis, item.span());
        let previous = self.impl_type.replace(format!("trait({})", item.ident));
        syn::visit::visit_item_trait(self, item);
        self.impl_type = previous;
        self.attributes = previous_attributes;
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let type_name = item.self_ty.to_token_stream().to_string().replace(' ', "");
        let trait_name = item
            .trait_
            .as_ref()
            .map(|(_, path, _)| path.to_token_stream().to_string().replace(' ', ""));
        let type_name = trait_name
            .map(|name| format!("{name}for{type_name}"))
            .unwrap_or(type_name);
        let previous = self.impl_type.replace(type_name);
        let previous_attributes = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        syn::visit::visit_item_impl(self, item);
        self.impl_type = previous;
        self.attributes = previous_attributes;
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.add(&item.ident.to_string(), &item.vis, item.span());
        let previous_len = self.module_path.len();
        self.module_path.push(item.ident.to_string());
        syn::visit::visit_item_mod(self, item);
        self.module_path.truncate(previous_len);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.sig.ident.to_string(), &item.vis, item.span());
        syn::visit::visit_impl_item_fn(self, item);
        self.attributes = previous;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(
            &item.sig.ident.to_string(),
            &syn::Visibility::Inherited,
            item.span(),
        );
        syn::visit::visit_trait_item_fn(self, item);
        self.attributes = previous;
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
                "rust:{}::{}",
                self.path.to_string_lossy(),
                self.semantic_name(name)
            ),
            kind: ArtifactUnitKind::Symbol,
            exposure: if self.test_file
                || self
                    .attributes
                    .iter()
                    .any(|attribute| attribute.contains("cfg(test)"))
            {
                ArtifactExposure::Test
            } else if matches!(visibility, syn::Visibility::Public(_)) {
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

    fn semantic_name(&self, name: &str) -> String {
        let mut path = self.module_path.join("::");
        if let Some(impl_type) = &self.impl_type {
            path.push_str("::impl(");
            path.push_str(impl_type);
            path.push(')');
        }
        path.push_str("::");
        path.push_str(name);
        if !self.attributes.is_empty() {
            path.push('[');
            path.push_str(&self.attributes.join(","));
            path.push(']');
        }
        path
    }
}

fn attribute_keys(attributes: &[syn::Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("path"))
        .map(|attribute| {
            attribute
                .meta
                .to_token_stream()
                .to_string()
                .replace(' ', "")
        })
        .collect()
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
    excludes: &[String],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if path
            .file_name()
            .is_some_and(|name| name == ".git" || name == "target" || name == "node_modules")
            || excludes.iter().any(|pattern| glob_match(pattern, relative))
        {
            continue;
        }
        if path.is_dir() {
            collect_matching(root, &path, extensions, excludes, out)?;
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

fn glob_match(pattern: &str, path: &Path) -> bool {
    let pattern = pattern.trim_end_matches('/');
    let value = path.to_string_lossy();
    if let Some(prefix) = pattern.strip_suffix("/**") {
        value == prefix || value.starts_with(&format!("{prefix}/"))
    } else if let Some(extension) = pattern.strip_prefix("**/*.") {
        value.ends_with(&format!(".{extension}"))
    } else {
        value == pattern
    }
}

fn cargo_roots(root: &Path) -> Result<Vec<PathBuf>> {
    if root.join("Cargo.toml").is_file()
        && let Ok(output) = std::process::Command::new("cargo")
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(root.join("Cargo.toml"))
            .current_dir(root)
            .output()
        && output.status.success()
    {
        #[derive(Deserialize)]
        struct Metadata {
            packages: Vec<Package>,
        }
        #[derive(Deserialize)]
        struct Package {
            manifest_path: PathBuf,
            targets: Vec<PackageTarget>,
        }
        #[derive(Deserialize)]
        struct PackageTarget {
            src_path: PathBuf,
            kind: Vec<String>,
        }
        let metadata: Metadata =
            serde_json::from_slice(&output.stdout).context("parse cargo metadata output")?;
        let mut roots = BTreeSet::new();
        for package in metadata.packages {
            for target in package.targets {
                if target.kind.iter().any(|kind| {
                    matches!(kind.as_str(), "lib" | "bin" | "example" | "test" | "bench")
                }) {
                    roots.insert(target.src_path);
                }
            }
            let _ = package.manifest_path;
        }
        let roots = roots.into_iter().collect::<Vec<_>>();
        return Ok(roots);
    }
    let mut manifests = Vec::new();
    collect_manifests(root, root, &mut manifests)?;
    let mut roots = BTreeSet::new();
    for manifest in manifests {
        let dir = manifest.parent().context("Cargo manifest has no parent")?;
        for candidate in [
            dir.join("src/lib.rs"),
            dir.join("src/main.rs"),
            dir.join("build.rs"),
        ] {
            if candidate.is_file() {
                roots.insert(candidate);
            }
        }
        for directory in [dir.join("src/bin"), dir.join("tests"), dir.join("examples")] {
            if directory.is_dir() {
                collect_rust_files(&directory, &mut roots)?;
            }
        }
    }
    Ok(roots.into_iter().collect())
}

fn collect_manifests(root: &Path, directory: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|name| name == ".git" || name == "target" || name == "node_modules")
        {
            continue;
        }
        if path.is_dir() {
            collect_manifests(root, &path, out)?;
        } else if path.file_name().is_some_and(|name| name == "Cargo.toml")
            && path.starts_with(root)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_rust_files(directory: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.insert(path);
        }
    }
    Ok(())
}

fn collect_reachable_rust(
    root: &Path,
    file: &Path,
    excludes: &[String],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if !file.is_file() {
        return Ok(());
    }
    let relative = file
        .strip_prefix(root)
        .context("Rust inventory path escaped workspace")?;
    if excludes.iter().any(|pattern| glob_match(pattern, relative)) {
        return Ok(());
    }
    if out.iter().any(|existing| existing == file) {
        return Ok(());
    }
    out.push(file.to_path_buf());
    let source = fs::read_to_string(file)?;
    let syntax = syn::parse_file(&source)
        .with_context(|| format!("parse Rust source {}", file.display()))?;
    for item in syntax.items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        if module.content.is_some() {
            continue;
        }
        let mut candidate = module.attrs.iter().find_map(|attr| {
            if !attr.path().is_ident("path") {
                return None;
            }
            match &attr.meta {
                syn::Meta::NameValue(value) => {
                    if let syn::Expr::Lit(expr) = &value.value
                        && let syn::Lit::Str(lit) = &expr.lit
                    {
                        return Some(file.parent()?.join(lit.value()));
                    }
                    None
                }
                _ => None,
            }
        });
        if candidate.is_none() {
            let parent = file.parent().unwrap_or(root);
            let sibling = parent.join(format!("{}.rs", module.ident));
            let nested = parent.join(module.ident.to_string()).join("mod.rs");
            candidate = sibling
                .is_file()
                .then_some(sibling)
                .or_else(|| nested.is_file().then_some(nested));
        }
        if let Some(candidate) = candidate {
            collect_reachable_rust(root, &candidate, excludes, out)?;
        }
    }
    for macro_name in ["include!", "include_str!", "include_bytes!"] {
        let mut remainder = source.as_str();
        while let Some(start) = remainder.find(macro_name) {
            remainder = &remainder[start + macro_name.len()..];
            let Some(quote_start) = remainder.find('"') else {
                break;
            };
            let after_quote = &remainder[quote_start + 1..];
            let Some(quote_end) = after_quote.find('"') else {
                break;
            };
            let candidate = file
                .parent()
                .unwrap_or(root)
                .join(&after_quote[..quote_end]);
            if candidate.is_file() {
                collect_reachable_rust(root, &candidate, excludes, out)?;
            }
            remainder = &after_quote[quote_end + 1..];
        }
    }
    Ok(())
}

fn module_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("crate")
        .into()
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
                settings: serde_yaml::Value::Null,
                excludes: vec![],
            },
            &profile,
        )
        .unwrap();
        assert!(units.iter().any(|unit| unit.adapter == "rust"));
        assert!(units.iter().any(|unit| unit.adapter == "javascript"));
        assert!(units.iter().any(|unit| {
            unit.identity == "rust:src/lib.rs::lib::api"
                && unit.kind == ArtifactUnitKind::Symbol
                && unit.exposure == ArtifactExposure::Public
        }));
    }
}
