#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Meta, Token};
use syu_project_model::InventoryProfile;
use syu_spec_model::RepoPath;

#[derive(Debug, Clone)]
pub struct InventoryContext {
    pub workspace_root: PathBuf,
    pub profile: String,
    pub settings: serde_yaml::Value,
    pub excludes: Vec<String>,
    /// Candidate file contents keyed by absolute or repository-relative path.
    /// Providers must read through this map so an overlay cannot be mixed with
    /// stale bytes from disk.
    pub overlays: BTreeMap<PathBuf, Vec<u8>>,
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
    Marker,
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
            units.push(unit(context, &self.adapter, path.clone())?);
            if self.adapter == "markdown" {
                units.extend(markdown_headings(context, path)?);
            }
        }
        Ok(InventoryFragment { units })
    }
}

fn openapi_operations(context: &InventoryContext, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("OpenAPI path escaped workspace")?;
    let repo_path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("OpenAPI path {:?}: {error}", path))?;
    let document: serde_yaml::Value = serde_yaml::from_slice(&read_bytes(context, &path)?)?;
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

fn markdown_headings(context: &InventoryContext, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("markdown path escaped workspace")?;
    let repo_path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("Markdown path {:?}: {error}", path))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
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
fn unit(context: &InventoryContext, adapter: &str, path: PathBuf) -> Result<ArtifactUnit> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("inventory path escaped workspace")?;
    let path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("file inventory path {:?}: {error}", path))?;
    let bytes = read_bytes(context, &root.join(path.as_path()))?;
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

pub fn read_bytes(context: &InventoryContext, path: &Path) -> Result<Vec<u8>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let relative = canonical
        .strip_prefix(&context.workspace_root)
        .ok()
        .map(Path::to_path_buf);
    if let Some(bytes) = context.overlays.get(&canonical) {
        return Ok(bytes.clone());
    }
    if let Some(relative) = relative
        && let Some(bytes) = context.overlays.get(&relative)
    {
        return Ok(bytes.clone());
    }
    Ok(fs::read(path)?)
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
    // A provider's file unit is a container for its semantic units. Once a
    // language provider exposed symbols/operations from the same file, keep
    // that container as support metadata instead of allowing a broad file
    // ownership scope to mask a changed symbol. Files without semantic
    // children remain ordinary active file subjects (for example YAML and
    // CI metadata).
    let semantic_paths = units
        .iter()
        .filter(|unit| {
            !matches!(unit.kind, ArtifactUnitKind::File)
                && unit.exposure != ArtifactExposure::Support
        })
        .map(|unit| (unit.adapter.clone(), unit.path.clone()))
        .collect::<BTreeSet<_>>();
    for unit in &mut units {
        if matches!(unit.kind, ArtifactUnitKind::File)
            && semantic_paths.contains(&(unit.adapter.clone(), unit.path.clone()))
        {
            unit.exposure = ArtifactExposure::Support;
        }
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
                return Ok(Box::new(RustInventoryProvider { settings }));
            }
            return Ok(Box::new(ConfiguredRustInventoryProvider {
                roots,
                settings,
            }));
        }
        "javascript" => &["js", "jsx", "mjs", "cjs"],
        "typescript" => &["ts", "tsx", "mts", "cts"],
        "python" => &["py"],
        "go" => &["go"],
        "shell" => &["sh", "bash", "zsh"],
        "openapi" => &["yaml", "yml", "json"],
        "documentation" | "markdown" => &["md", "mdx"],
        "html" => &["html"],
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
            units.push(unit(context, &self.adapter, path.clone())?);
            if self.adapter == "markdown" {
                units.extend(markdown_headings(context, path)?);
            } else if self.adapter == "openapi" {
                units.extend(openapi_operations(context, path)?);
            } else if matches!(self.adapter.as_str(), "javascript" | "typescript") {
                units.extend(source_symbol_units(context, path, &self.adapter)?);
            } else if self.adapter == "html" {
                units.extend(html_marker_units(context, path)?);
            }
        }
        Ok(InventoryFragment { units })
    }
}

fn html_marker_units(context: &InventoryContext, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("HTML path escaped workspace")?;
    let repo_path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("HTML marker path {:?}: {error}", path))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut units = Vec::new();

    // Marker selectors are semantic HTML attributes, not whole-file fallbacks.
    // Keep the exact attribute span so two targets in one document can still
    // own distinct artifacts and changed-marker planning remains precise.
    for (start, _) in source.match_indices("data-") {
        let name_end = source[start..]
            .find(|character: char| {
                character.is_ascii_whitespace() || matches!(character, '=' | '>' | '/')
            })
            .map(|offset| start + offset)
            .unwrap_or(source.len());
        let equals_start = name_end
            + source[name_end..]
                .find(|character: char| !character.is_ascii_whitespace())
                .unwrap_or(0);
        if source.as_bytes().get(equals_start) != Some(&b'=') {
            continue;
        }
        let value_start = equals_start + 1;
        let value_start = value_start
            + source[value_start..]
                .find(|character: char| !character.is_ascii_whitespace())
                .unwrap_or(0);
        let Some(quote) = source.as_bytes().get(value_start).copied() else {
            continue;
        };
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        let content_start = value_start + 1;
        let Some(content_end_offset) = source.as_bytes()[content_start..]
            .iter()
            .position(|byte| *byte == quote)
        else {
            continue;
        };
        let end = content_start + content_end_offset + 1;
        let marker = source[start..end].to_owned();
        let occurrence = occurrences.entry(marker.clone()).or_insert(0);
        let identity = format!(
            "html:{}::marker::{}@{}",
            repo_path.to_string_lossy(),
            marker,
            *occurrence
        );
        *occurrence += 1;
        let line_start = source[..start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let line_end = source[..end].bytes().filter(|byte| *byte == b'\n').count() + 1;
        units.push(ArtifactUnit {
            adapter: "html".into(),
            path: repo_path.clone(),
            identity,
            kind: ArtifactUnitKind::Marker,
            exposure: ArtifactExposure::Support,
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: start,
                byte_end: end,
                line_start,
                line_end,
            },
            digest: digest(marker.as_bytes()),
        });
    }
    Ok(units)
}

fn source_symbol_units(
    context: &InventoryContext,
    path: PathBuf,
    adapter: &str,
) -> Result<Vec<ArtifactUnit>> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("source path escaped workspace")?;
    let repo_path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("source symbol path {:?}: {error}", path))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let mut units = Vec::new();
    let mut identities = BTreeSet::new();
    let tokens = javascript_tokens(&source);
    let mut depth = 0usize;

    let mut add_symbol = |name: &str, exported: bool, start: usize, end: usize| {
        if name.is_empty() || name.starts_with('#') {
            return;
        }
        let identity = format!("{adapter}:{}::{name}", repo_path.to_string_lossy());
        if !identities.insert(identity.clone()) {
            return;
        }
        let line_start = source[..start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let line_end = source[..end].bytes().filter(|byte| *byte == b'\n').count() + 1;
        units.push(ArtifactUnit {
            adapter: adapter.into(),
            path: repo_path.clone(),
            identity,
            kind: ArtifactUnitKind::Symbol,
            exposure: if exported {
                ArtifactExposure::Public
            } else {
                ArtifactExposure::Workspace
            },
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: start,
                byte_end: end.max(start + 1),
                line_start,
                line_end,
            },
            digest: digest(&source.as_bytes()[start..end.max(start + 1)]),
        });
    };

    let mut index = 0usize;
    while index < tokens.len() {
        let before_depth = depth;
        if before_depth == 0 {
            let mut cursor = index;
            let mut exported = false;
            if tokens[cursor].text == "export" {
                exported = true;
                cursor += 1;
                if tokens
                    .get(cursor)
                    .is_some_and(|token| token.text == "default")
                {
                    cursor += 1;
                }
                if tokens.get(cursor).is_some_and(|token| token.text == "{") {
                    cursor += 1;
                    while let Some(token) = tokens.get(cursor) {
                        if token.text == "}" {
                            break;
                        }
                        if token.text != "," && token.text != "as" {
                            add_symbol(&token.text, true, token.start, token.end);
                        }
                        cursor += 1;
                    }
                    index = cursor;
                }
            }
            while tokens
                .get(cursor)
                .is_some_and(|token| matches!(token.text.as_str(), "async" | "declare"))
            {
                cursor += 1;
            }
            if let Some(keyword) = tokens.get(cursor).map(|token| token.text.as_str()) {
                if matches!(
                    keyword,
                    "function" | "class" | "interface" | "type" | "enum"
                ) {
                    let mut name_cursor = cursor + 1;
                    if tokens
                        .get(name_cursor)
                        .is_some_and(|token| token.text == "*")
                    {
                        name_cursor += 1;
                    }
                    if let Some(name) = tokens.get(name_cursor) {
                        add_symbol(
                            &name.text,
                            exported,
                            tokens[index].start,
                            javascript_declaration_end(&source, &tokens, index),
                        );
                    }
                } else if matches!(keyword, "const" | "let" | "var")
                    && let Some(name) = tokens.get(cursor + 1)
                {
                    add_symbol(
                        &name.text,
                        exported,
                        tokens[index].start,
                        javascript_declaration_end(&source, &tokens, index),
                    );
                }
            }
        }
        if tokens[index].text == "{" {
            depth += 1;
        } else if tokens[index].text == "}" {
            depth = depth.saturating_sub(1);
        }
        index += 1;
    }
    Ok(units)
}

#[derive(Debug, Clone)]
struct JavascriptToken {
    text: String,
    start: usize,
    end: usize,
}

fn javascript_declaration_end(
    source: &str,
    tokens: &[JavascriptToken],
    start_index: usize,
) -> usize {
    let start = tokens
        .get(start_index)
        .map(|token| token.start)
        .unwrap_or(source.len());
    let mut depth = 0usize;
    let mut saw_brace = false;
    let mut last_end = start;
    for token in tokens.iter().skip(start_index) {
        last_end = token.end;
        match token.text.as_str() {
            "{" => {
                depth += 1;
                saw_brace = true;
            }
            "}" if depth > 0 => {
                depth -= 1;
                if saw_brace && depth == 0 {
                    return token.end;
                }
            }
            ";" if depth == 0 => return token.end,
            _ => {}
        }
    }
    source[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(last_end.max(start + 1))
}

/// Tokenize JavaScript/TypeScript declarations without treating braces inside
/// strings, template literals, or comments as syntax. The old line scanner
/// misclassified later exports whenever an earlier function contained a
/// template expression or object literal.
fn javascript_tokens(source: &str) -> Vec<JavascriptToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'$') {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'$'))
            {
                index += 1;
            }
            tokens.push(JavascriptToken {
                text: source[start..index].into(),
                start,
                end: index,
            });
            continue;
        }
        tokens.push(JavascriptToken {
            text: source[index..index + 1].into(),
            start: index,
            end: index + 1,
        });
        index += 1;
    }
    tokens
}

/// Rust inventory uses the syntax tree rather than line-oriented symbol
/// searches. Every declared item receives an exact identity and source span.
pub struct RustInventoryProvider {
    settings: serde_yaml::Value,
}

struct ConfiguredRustInventoryProvider {
    roots: Vec<RepoPath>,
    settings: serde_yaml::Value,
}

impl InventoryProvider for RustInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        discover_rust(context, &[], &self.settings)
    }
}

impl InventoryProvider for ConfiguredRustInventoryProvider {
    fn discover(&self, context: &InventoryContext) -> Result<InventoryFragment> {
        discover_rust(context, &self.roots, &self.settings)
    }
}

fn discover_rust(
    context: &InventoryContext,
    configured_roots: &[RepoPath],
    settings: &serde_yaml::Value,
) -> Result<InventoryFragment> {
    let mode = settings
        .get("mode")
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or("production");
    let test_mode = mode == "test";
    // Test and production inventories use separate cfg evaluation. Root
    // discovery still honors the independent `include_tests` setting below.
    let cfg = cfg_context(settings, test_mode);
    let mut files = Vec::new();
    let mut support_files = Vec::new();
    let mut file_visibility = BTreeMap::new();
    let mut roots = if configured_roots.is_empty() {
        cargo_roots(&context.workspace_root, settings)?
    } else {
        configured_roots
            .iter()
            .map(|root| context.workspace_root.join(root.as_path()))
            .collect()
    };
    if let Some(additional) = settings
        .get("additional_roots")
        .and_then(serde_yaml::Value::as_sequence)
    {
        roots.extend(
            additional
                .iter()
                .filter_map(serde_yaml::Value::as_str)
                .filter_map(|path| RepoPath::new(path).ok())
                .map(|path| context.workspace_root.join(path.as_path())),
        );
    }
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        collect_matching(
            &context.workspace_root,
            &context.workspace_root,
            &["rs".into()],
            &context.excludes,
            &mut files,
        )?;
    } else {
        let mut reachability = RustReachability {
            context,
            excludes: &context.excludes,
            out: &mut files,
            support: &mut support_files,
            file_visibility: &mut file_visibility,
            cfg: &cfg,
        };
        for root in roots {
            collect_reachable_rust(&mut reachability, &root, true)?;
        }
    }
    files.sort();
    files.dedup();
    let mut units = Vec::new();
    for path in files {
        let source = String::from_utf8(read_bytes(context, &path)?)
            .with_context(|| format!("read Rust source {}", path.display()))?;
        let syntax = syn::parse_file(&source)
            .with_context(|| format!("parse Rust source {}", path.display()))?;
        let relative = path
            .strip_prefix(&context.workspace_root)
            .context("Rust inventory path escaped workspace")?;
        let repo_path = RepoPath::new(relative)
            .map_err(|error| anyhow::anyhow!("Rust path {:?}: {error}", path))?;
        units.push(unit(context, "rust", path.clone())?);
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
            cfg: &cfg,
            public_module: file_visibility.get(&path).copied().unwrap_or(true),
        };
        visitor.visit_file(&syntax);
        units.extend(visitor.units);
    }
    support_files.sort();
    support_files.dedup();
    for path in support_files {
        units.push(support_unit(context, path)?);
    }
    Ok(InventoryFragment { units })
}

fn support_unit(context: &InventoryContext, path: PathBuf) -> Result<ArtifactUnit> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("included support path escaped workspace")?;
    let repo_path = RepoPath::new(relative)
        .map_err(|error| anyhow::anyhow!("support path {:?}: {error}", path))?;
    let bytes = read_bytes(context, &path)?;
    Ok(ArtifactUnit {
        adapter: "rust".into(),
        path: repo_path.clone(),
        identity: format!("rust:{}::support", repo_path.to_string_lossy()),
        kind: ArtifactUnitKind::Generated,
        exposure: ArtifactExposure::Support,
        reachability: ArtifactReachability::Active,
        span: SourceSpan {
            byte_start: 0,
            byte_end: bytes.len(),
            line_start: 1,
            line_end: 1,
        },
        digest: digest(&bytes),
    })
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
    cfg: &'a CfgContext,
    public_module: bool,
}

impl<'ast> syn::visit::Visit<'ast> for RustVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.sig.ident.to_string(), &item.vis, item.span(), true);
        syn::visit::visit_item_fn(self, item);
        self.attributes = previous;
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        self.add(&item.ident.to_string(), &item.vis, item.span(), false);
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        self.add(&item.ident.to_string(), &item.vis, item.span(), false);
        syn::visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        let previous_attributes = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.ident.to_string(), &item.vis, item.span(), false);
        let previous = self.impl_type.replace(format!("trait({})", item.ident));
        syn::visit::visit_item_trait(self, item);
        self.impl_type = previous;
        self.attributes = previous_attributes;
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        let type_name = item.self_ty.to_token_stream().to_string().replace(' ', "");
        let type_name = type_name.split('<').next().unwrap_or(&type_name).to_owned();
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
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        self.add(&item.ident.to_string(), &item.vis, item.span(), false);
        let previous_len = self.module_path.len();
        let previous_public = self.public_module;
        self.module_path.push(item.ident.to_string());
        self.public_module = self.public_module && matches!(item.vis, syn::Visibility::Public(_));
        syn::visit::visit_item_mod(self, item);
        self.module_path.truncate(previous_len);
        self.public_module = previous_public;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(&item.sig.ident.to_string(), &item.vis, item.span(), true);
        syn::visit::visit_impl_item_fn(self, item);
        self.attributes = previous;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !cfg_active(&item.attrs, self.cfg) {
            return;
        }
        let previous = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        self.add(
            &item.sig.ident.to_string(),
            &syn::Visibility::Inherited,
            item.span(),
            false,
        );
        syn::visit::visit_trait_item_fn(self, item);
        self.attributes = previous;
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !self.public_module
            || !matches!(item.vis, syn::Visibility::Public(_))
            || !cfg_active(&item.attrs, self.cfg)
        {
            return;
        }
        let mut names = Vec::new();
        collect_use_names(&item.tree, &mut names);
        for name in names {
            self.add(&name, &item.vis, item.span(), true);
        }
    }
}

fn collect_use_names(tree: &syn::UseTree, names: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => collect_use_names(&path.tree, names),
        syn::UseTree::Name(name) => names.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => names.push(rename.rename.to_string()),
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_names(item, names);
            }
        }
    }
}

impl RustVisitor<'_> {
    fn add(
        &mut self,
        name: &str,
        visibility: &syn::Visibility,
        span: proc_macro2::Span,
        public_entrypoint: bool,
    ) {
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
            } else if public_entrypoint
                && self.public_module
                && matches!(visibility, syn::Visibility::Public(_))
            {
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

fn cfg_active(attributes: &[syn::Attribute], cfg: &CfgContext) -> bool {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .all(|attribute| match &attribute.meta {
            Meta::List(list) => {
                let nested = Punctuated::<Meta, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())
                    .unwrap_or_default();
                cfg_eval_list(
                    list.path.to_token_stream().to_string().as_str(),
                    &nested,
                    cfg,
                )
            }
            Meta::Path(path) => cfg.values.contains(&path.to_token_stream().to_string()),
            Meta::NameValue(_) => true,
        })
}

#[derive(Debug, Clone)]
struct CfgContext {
    values: BTreeSet<String>,
}

fn cfg_context(settings: &serde_yaml::Value, test_mode: bool) -> CfgContext {
    let mut values = BTreeSet::new();
    if test_mode {
        values.insert("test".into());
    } else {
        values.insert("not(test)".into());
    }
    if cfg!(debug_assertions) {
        values.insert("debug_assertions".into());
    }
    let target = settings
        .get("target")
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_owned)
        .or_else(|| std::env::var("TARGET").ok())
        .unwrap_or_else(|| std::env::consts::ARCH.into())
        .to_ascii_lowercase();
    let arch = target.split('-').next().unwrap_or(std::env::consts::ARCH);
    values.insert(format!("target_arch={arch}"));
    let os = if target.contains("windows") {
        "windows"
    } else if target.contains("darwin") || target.contains("apple") {
        "macos"
    } else if target.contains("linux") {
        "linux"
    } else if target.contains("freebsd") {
        "freebsd"
    } else if target.contains("openbsd") {
        "openbsd"
    } else if target.contains("netbsd") {
        "netbsd"
    } else if target.contains("wasm") {
        "unknown"
    } else {
        std::env::consts::OS
    };
    values.insert(format!("target_os={os}"));
    if os == "windows" {
        values.insert("windows".into());
        values.insert("target_family=windows".into());
    } else {
        values.insert("unix".into());
        values.insert("target_family=unix".into());
    }
    for feature in settings
        .get("features")
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(serde_yaml::Value::as_str)
    {
        values.insert(format!("feature={feature}"));
    }
    CfgContext { values }
}

fn cfg_eval_list<'a>(
    name: &str,
    nested: impl IntoIterator<Item = &'a Meta>,
    cfg: &CfgContext,
) -> bool {
    let nested = nested.into_iter().collect::<Vec<_>>();
    match name {
        "all" => nested.iter().all(|meta| cfg_eval(meta, cfg)),
        "any" => nested.iter().any(|meta| cfg_eval(meta, cfg)),
        "not" => nested.first().is_some_and(|meta| !cfg_eval(meta, cfg)),
        _ => nested.iter().all(|meta| cfg_eval(meta, cfg)),
    }
}

fn cfg_eval(meta: &Meta, cfg: &CfgContext) -> bool {
    match meta {
        Meta::Path(path) => cfg.values.contains(&path.to_token_stream().to_string()),
        Meta::NameValue(value) => {
            let key = value.path.to_token_stream().to_string();
            let value = match &value.value {
                syn::Expr::Lit(expression) => expression.lit.to_token_stream().to_string(),
                expression => expression.to_token_stream().to_string(),
            };
            cfg.values
                .contains(&format!("{key}={}", value.trim_matches('"')))
        }
        Meta::List(list) => {
            let nested = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .unwrap_or_default();
            cfg_eval_list(
                list.path.to_token_stream().to_string().as_str(),
                &nested,
                cfg,
            )
        }
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
    if directory.is_file() {
        let relative = directory.strip_prefix(root).unwrap_or(directory);
        if !excludes.iter().any(|pattern| glob_match(pattern, relative))
            && directory
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extensions.iter().any(|candidate| candidate == extension))
            && directory.starts_with(root)
        {
            out.push(directory.to_path_buf());
        }
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

fn cargo_roots(root: &Path, settings: &serde_yaml::Value) -> Result<Vec<PathBuf>> {
    let include_tests = settings
        .get("include_tests")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or_else(|| {
            settings.get("mode").and_then(serde_yaml::Value::as_str) == Some("test")
        });
    if root.join("Cargo.toml").is_file()
        && let Ok(output) = cargo_metadata(root, settings)
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
                if !target.kind.is_empty()
                    && (include_tests
                        || !target
                            .kind
                            .iter()
                            .any(|kind| matches!(kind.as_str(), "test" | "bench")))
                {
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
        let mut directories = vec![dir.join("src/bin"), dir.join("examples")];
        if include_tests {
            directories.push(dir.join("tests"));
        }
        for directory in directories {
            if directory.is_dir() {
                collect_rust_files(&directory, &mut roots)?;
            }
        }
    }
    Ok(roots.into_iter().collect())
}

fn cargo_metadata(
    root: &Path,
    settings: &serde_yaml::Value,
) -> std::io::Result<std::process::Output> {
    let mut command = std::process::Command::new("cargo");
    command
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"));
    if settings
        .get("all_features")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false)
    {
        command.arg("--all-features");
    } else if let Some(features) = settings
        .get("features")
        .and_then(serde_yaml::Value::as_sequence)
    {
        let features = features
            .iter()
            .filter_map(serde_yaml::Value::as_str)
            .collect::<Vec<_>>();
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
    }
    if settings
        .get("no_default_features")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false)
    {
        command.arg("--no-default-features");
    }
    if let Some(target) = settings.get("target").and_then(serde_yaml::Value::as_str) {
        command.args(["--filter-platform", target]);
    }
    command.current_dir(root).output()
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

struct RustReachability<'a> {
    context: &'a InventoryContext,
    excludes: &'a [String],
    out: &'a mut Vec<PathBuf>,
    support: &'a mut Vec<PathBuf>,
    file_visibility: &'a mut BTreeMap<PathBuf, bool>,
    cfg: &'a CfgContext,
}

fn collect_reachable_rust(
    reachability: &mut RustReachability<'_>,
    file: &Path,
    public_module: bool,
) -> Result<()> {
    let root = &reachability.context.workspace_root;
    if !file.is_file() {
        return Ok(());
    }
    let relative = file
        .strip_prefix(root)
        .context("Rust inventory path escaped workspace")?;
    if reachability
        .excludes
        .iter()
        .any(|pattern| glob_match(pattern, relative))
    {
        return Ok(());
    }
    if !reachability.cfg.values.contains("test")
        && relative
            .components()
            .any(|component| matches!(component.as_os_str().to_str(), Some("tests" | "benches")))
    {
        return Ok(());
    }
    let was_public = reachability
        .file_visibility
        .get(file)
        .copied()
        .unwrap_or(false);
    let previous_visibility = reachability
        .file_visibility
        .insert(file.to_path_buf(), was_public || public_module);
    if reachability.out.iter().any(|existing| existing == file)
        && previous_visibility.is_some_and(|was_public| was_public || !public_module)
    {
        return Ok(());
    }
    if !reachability.out.iter().any(|existing| existing == file) {
        reachability.out.push(file.to_path_buf());
    }
    let source = String::from_utf8(read_bytes(reachability.context, file)?)?;
    let syntax = syn::parse_file(&source)
        .with_context(|| format!("parse Rust source {}", file.display()))?;
    for item in syntax.items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        if !cfg_active(&module.attrs, reachability.cfg) {
            continue;
        }
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
            collect_reachable_rust(
                reachability,
                &candidate,
                public_module && matches!(module.vis, syn::Visibility::Public(_)),
            )?;
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
                .join(&after_quote[..quote_end])
                .canonicalize()
                .unwrap_or_else(|_| {
                    file.parent()
                        .unwrap_or(root)
                        .join(&after_quote[..quote_end])
                });
            if candidate.is_file() {
                if macro_name == "include!" {
                    collect_reachable_rust(reachability, &candidate, public_module)?;
                } else if !reachability.excludes.iter().any(|pattern| {
                    glob_match(pattern, candidate.strip_prefix(root).unwrap_or(&candidate))
                }) {
                    reachability.support.push(candidate);
                }
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
                overlays: BTreeMap::new(),
            },
            &profile,
        )
        .unwrap();
        assert!(units.iter().any(|unit| unit.adapter == "rust"));
        assert!(units.iter().any(|unit| unit.adapter == "javascript"));
        assert!(units.iter().any(|unit| {
            unit.identity == "rust:src/lib.rs"
                && unit.kind == ArtifactUnitKind::File
                && unit.exposure == ArtifactExposure::Support
        }));
        assert!(units.iter().any(|unit| {
            unit.identity == "rust:src/lib.rs::lib::api"
                && unit.kind == ArtifactUnitKind::Symbol
                && unit.exposure == ArtifactExposure::Public
        }));
    }

    #[test]
    fn javascript_symbol_span_covers_the_changed_declaration_body() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("model.js");
        fs::write(
            &path,
            "function helper() {\n  return true;\n}\n\nconst value = helper();\n",
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let units = source_symbol_units(&context, path, "javascript").unwrap();
        let helper = units
            .iter()
            .find(|unit| unit.identity.ends_with("::helper"))
            .unwrap();
        assert!(helper.span.line_start == 1);
        assert!(helper.span.line_end >= 3);
    }

    #[test]
    fn html_marker_inventory_keeps_attributes_as_distinct_units() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("workbench.html");
        fs::write(
            &path,
            r#"<aside data-page="work" data-i18n-aria="a11y.main_pages"></aside>"#,
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let units = html_marker_units(&context, path).unwrap();
        assert_eq!(units.len(), 2);
        assert!(units.iter().all(|unit| {
            unit.kind == ArtifactUnitKind::Marker
                && unit.exposure == ArtifactExposure::Support
                && unit.identity.contains("::marker::")
        }));
        assert_ne!(units[0].identity, units[1].identity);
        assert!(
            units
                .iter()
                .all(|unit| unit.span.byte_end > unit.span.byte_start)
        );
    }
}
