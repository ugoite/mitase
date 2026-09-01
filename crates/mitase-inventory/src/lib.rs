#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use mitase_project_model::InventoryProfile;
use mitase_spec_model::{RepoPath, format_sha256};
use proc_macro2::LineColumn;
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
use tree_sitter::{Node, Parser as TsParser};

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
    /// Digest of the artifact shape with its declared name removed. Unlike
    /// `digest`, this stays stable across a pure rename and lets inventory
    /// comparison retain semantic identity without line-based heuristics.
    #[serde(default)]
    pub structural_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactUnitKind {
    File,
    Symbol,
    Marker,
    Operation,
    Heading,
    SchemaNode,
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

/// An execution-free identity and source range for one native test.
///
/// Inventory exposes these identities so a verification target can name an
/// existing test without requiring the repository to reshape its test code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResolution {
    pub identity: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

/// Discover native test identities for a supported source adapter.
pub fn discover_tests(adapter: &str, source: &str) -> Result<Vec<TestResolution>> {
    match adapter {
        "rust" => discover_rust_tests(source),
        "typescript" | "javascript" => discover_javascript_tests(adapter, source),
        _ => bail!("adapter {adapter} does not support test selectors"),
    }
}

fn discover_tests_for_path(
    adapter: &str,
    path: &RepoPath,
    source: &str,
) -> Result<Vec<TestResolution>> {
    match adapter {
        "rust" => discover_rust_tests(source),
        "typescript" | "javascript" => discover_javascript_tests_for_path(adapter, path, source),
        _ => bail!("adapter {adapter} does not support test selectors"),
    }
}

/// Resolve one unique native test identity without executing the test.
pub fn resolve_test(adapter: &str, source: &str, name: &str) -> Result<TestResolution> {
    resolve_test_from_tests(discover_tests(adapter, source)?, name)
}

/// Resolve one native test using the source path to select the language
/// grammar, including JSX/TSX files.
pub fn resolve_test_in_path(
    adapter: &str,
    path: &RepoPath,
    source: &str,
    name: &str,
) -> Result<TestResolution> {
    resolve_test_from_tests(discover_tests_for_path(adapter, path, source)?, name)
}

fn resolve_test_from_tests(tests: Vec<TestResolution>, name: &str) -> Result<TestResolution> {
    if name.trim().is_empty() {
        bail!("test selector must not be empty");
    }
    let matches = tests
        .into_iter()
        .filter(|test| test.identity == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => bail!("test {name} not found"),
        [test] => Ok(test.clone()),
        _ => bail!("test {name} is ambiguous"),
    }
}

fn has_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

fn test_reachability(active: bool, profile: &str, attributes: &[String]) -> ArtifactReachability {
    if active {
        ArtifactReachability::Active
    } else {
        ArtifactReachability::Conditional {
            profile: if attributes.is_empty() {
                profile.to_owned()
            } else {
                attributes.join(",")
            },
        }
    }
}

fn discover_rust_tests(source: &str) -> Result<Vec<TestResolution>> {
    let syntax = syn::parse_file(source)?;
    let offsets = line_offsets(source);
    struct Visitor<'a> {
        source: &'a str,
        offsets: Vec<usize>,
        modules: Vec<String>,
        tests: Vec<TestResolution>,
    }

    impl<'ast> Visit<'ast> for Visitor<'_> {
        fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
            self.modules.push(item.ident.to_string());
            syn::visit::visit_item_mod(self, item);
            self.modules.pop();
        }

        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            if !has_test_attribute(&item.attrs) {
                return;
            }
            let start = item.span().start();
            let end = item.span().end();
            let byte_start = line_column_to_byte(self.source, &self.offsets, start);
            let byte_end = line_column_to_byte(self.source, &self.offsets, end);
            let identity = self
                .modules
                .iter()
                .chain(std::iter::once(&item.sig.ident.to_string()))
                .cloned()
                .collect::<Vec<_>>()
                .join("::");
            self.tests.push(TestResolution {
                identity,
                byte_start,
                byte_end,
                line_start: start.line.max(1),
                line_end: end.line.max(start.line.max(1)),
            });
        }
    }

    let mut visitor = Visitor {
        source,
        offsets,
        modules: Vec::new(),
        tests: Vec::new(),
    };
    visitor.visit_file(&syntax);
    Ok(visitor.tests)
}

fn discover_javascript_tests(adapter: &str, source: &str) -> Result<Vec<TestResolution>> {
    discover_javascript_tests_with_language(adapter, source, false)
}

fn discover_javascript_tests_for_path(
    adapter: &str,
    path: &RepoPath,
    source: &str,
) -> Result<Vec<TestResolution>> {
    let path = path.to_string_lossy();
    let tsx = matches!(
        (
            adapter,
            path.rsplit_once('.').map(|(_, extension)| extension)
        ),
        ("javascript" | "typescript", Some("jsx" | "tsx"))
    );
    discover_javascript_tests_with_language(adapter, source, tsx)
}

fn discover_javascript_tests_with_language(
    adapter: &str,
    source: &str,
    tsx: bool,
) -> Result<Vec<TestResolution>> {
    let mut parser = TsParser::new();
    parser
        .set_language(&javascript_language(adapter, tsx))
        .map_err(|error| anyhow::anyhow!("load JavaScript/TypeScript grammar: {error}"))?;
    let tree = parser
        .parse(source, None)
        .context("parse JavaScript/TypeScript tests")?;
    if tree.root_node().has_error() {
        bail!(
            "JavaScript/TypeScript source has syntax errors; refusing approximate test inventory"
        );
    }

    fn visit(node: Node<'_>, source: &str, tests: &mut Vec<TestResolution>) -> Result<()> {
        if node.kind() == "call_expression"
            && let Some(function) = node.child_by_field_name("function")
            && function.kind() == "identifier"
            && matches!(&source[function.byte_range()], "it" | "test")
            && let Some(arguments) = node.child_by_field_name("arguments")
            && let Some(title) = arguments.named_child(0)
            && title.kind() == "string"
        {
            let name = javascript_string_value(&source[title.byte_range()])?;
            if name.trim().is_empty() {
                bail!("test title must not be empty");
            }
            tests.push(TestResolution {
                identity: name,
                byte_start: node.start_byte(),
                byte_end: node.end_byte(),
                line_start: node.start_position().row + 1,
                line_end: node.end_position().row + 1,
            });
        }
        for child in node.named_children(&mut node.walk()) {
            visit(child, source, tests)?;
        }
        Ok(())
    }

    let mut tests = Vec::new();
    visit(tree.root_node(), source, &mut tests)?;
    Ok(tests)
}

fn javascript_language(adapter: &str, tsx: bool) -> tree_sitter::Language {
    if adapter == "javascript" {
        if tsx {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_javascript::LANGUAGE.into()
        }
    } else if tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }
}

fn javascript_string_value(literal: &str) -> Result<String> {
    let Some(quote @ ('\'' | '"')) = literal.chars().next() else {
        bail!("test title must be a quoted string");
    };
    if !literal.ends_with(quote) {
        bail!("test title string is unterminated");
    }
    let characters = literal[quote.len_utf8()..literal.len() - quote.len_utf8()]
        .chars()
        .collect::<Vec<_>>();
    let mut value = String::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        index += 1;
        if character != '\\' {
            value.push(character);
            continue;
        }
        let escape = characters
            .get(index)
            .copied()
            .context("test title string has an incomplete escape")?;
        index += 1;
        match escape {
            'n' => value.push('\n'),
            'r' => value.push('\r'),
            't' => value.push('\t'),
            'b' => value.push('\u{0008}'),
            'f' => value.push('\u{000c}'),
            'v' => value.push('\u{000b}'),
            '0' if !characters
                .get(index)
                .is_some_and(|character| character.is_ascii_digit()) =>
            {
                value.push('\0')
            }
            '\n' => {}
            '\r' => {
                if characters.get(index) == Some(&'\n') {
                    index += 1;
                }
            }
            '\\' | '\'' | '"' => value.push(escape),
            'x' => value.push(
                char::from_u32(parse_javascript_hex(&characters, &mut index, 2)?)
                    .context("test title string has an invalid escape code point")?,
            ),
            'u' => {
                if characters.get(index) == Some(&'{') {
                    index += 1;
                    let start = index;
                    while characters
                        .get(index)
                        .is_some_and(|character| *character != '}')
                    {
                        index += 1;
                    }
                    if characters.get(index) != Some(&'}') || start == index || index - start > 6 {
                        bail!("test title string has an invalid Unicode code-point escape");
                    }
                    let code_point = characters[start..index]
                        .iter()
                        .try_fold(0u32, |value, character| {
                            character.to_digit(16).map(|digit| value * 16 + digit)
                        })
                        .context("test title string has an invalid Unicode code-point escape")?;
                    index += 1;
                    value.push(
                        char::from_u32(code_point)
                            .context("test title string has an invalid Unicode code point")?,
                    );
                } else {
                    let code_unit = parse_javascript_hex(&characters, &mut index, 4)?;
                    if (0xD800..=0xDBFF).contains(&code_unit) {
                        if characters.get(index..index + 2) != Some(&['\\', 'u']) {
                            bail!("test title string has an unpaired Unicode surrogate");
                        }
                        index += 2;
                        let low = parse_javascript_hex(&characters, &mut index, 4)?;
                        if !(0xDC00..=0xDFFF).contains(&low) {
                            bail!("test title string has an invalid Unicode surrogate pair");
                        }
                        let code_point = 0x1_0000 + ((code_unit - 0xD800) << 10) + (low - 0xDC00);
                        value.push(
                            char::from_u32(code_point)
                                .context("test title string has an invalid Unicode code point")?,
                        );
                    } else if (0xDC00..=0xDFFF).contains(&code_unit) {
                        bail!("test title string has an unpaired Unicode surrogate");
                    } else {
                        value.push(
                            char::from_u32(code_unit)
                                .context("test title string has an invalid escape code point")?,
                        );
                    }
                }
            }
            // ECMAScript's NonEscapeCharacter sequence evaluates to the
            // character itself (for example, `\a` evaluates to `a`). It is
            // still distinct from malformed hexadecimal/Unicode escapes,
            // which are rejected by the branches above.
            other => value.push(other),
        }
    }
    Ok(value)
}

fn parse_javascript_hex(characters: &[char], index: &mut usize, length: usize) -> Result<u32> {
    let end = index.saturating_add(length);
    let digits = characters
        .get(*index..end)
        .context("test title string has an incomplete hexadecimal escape")?;
    let value = digits
        .iter()
        .try_fold(0u32, |value, character| {
            character.to_digit(16).map(|digit| value * 16 + digit)
        })
        .context("test title string has an invalid hexadecimal escape")?;
    *index = end;
    Ok(value)
}

fn line_column_to_byte(source: &str, offsets: &[usize], location: LineColumn) -> usize {
    offsets
        .get(location.line.saturating_sub(1))
        .copied()
        .unwrap_or(source.len())
        .saturating_add(location.column)
        .min(source.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticChangeKind {
    PublicAddition,
    PublicRemoval,
    Addition,
    PrivateModification,
    Modification,
    ReachabilityChange,
    Rename,
    Deletion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticChange {
    pub kind: SemanticChangeKind,
    pub adapter: String,
    pub path: RepoPath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_identity: Option<String>,
    pub exposure: ArtifactExposure,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_reachability: Option<ArtifactReachability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_reachability: Option<ArtifactReachability>,
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
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("OpenAPI path {path:?}: {error}"))?;
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
            for (method_value, operation) in operations {
                let Some(method) = method_value.as_str() else {
                    continue;
                };
                if !matches!(
                    method.to_ascii_lowercase().as_str(),
                    "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace"
                ) {
                    continue;
                }
                let serialized = serde_json::to_vec(operation)?;
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
                    digest: digest(&serialized),
                    // A path move can preserve an operation's contract shape,
                    // but changing its HTTP method cannot. Keep the method in
                    // the rename corroborator while deliberately omitting the
                    // path so GET /old -> GET /new remains an endpoint rename.
                    structural_digest: openapi_structural_digest(method, &serialized),
                });
            }
        }
    }
    Ok(units)
}

fn schema_nodes(
    context: &InventoryContext,
    path: PathBuf,
    adapter: &str,
) -> Result<Vec<ArtifactUnit>> {
    let relative = path
        .strip_prefix(&context.workspace_root)
        .context("schema path escaped workspace")?;
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("schema path {path:?}: {error}"))?;
    let bytes = read_bytes(context, &path)?;
    let document: serde_yaml::Value = if adapter == "json" || adapter == "json-schema" {
        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse JSON schema {}", repo_path.display()))?;
        serde_yaml::to_value(json)?
    } else {
        serde_yaml::from_slice(&bytes)
            .with_context(|| format!("parse YAML schema {}", repo_path.display()))?
    };
    let line_end = String::from_utf8_lossy(&bytes).lines().count().max(1);
    let mut units = Vec::new();
    collect_schema_nodes(
        adapter,
        &repo_path,
        &document,
        String::new(),
        bytes.len(),
        line_end,
        &mut units,
    )?;
    Ok(units)
}

fn collect_schema_nodes(
    adapter: &str,
    path: &RepoPath,
    value: &serde_yaml::Value,
    pointer: String,
    byte_end: usize,
    line_end: usize,
    units: &mut Vec<ArtifactUnit>,
) -> Result<()> {
    if !pointer.is_empty() {
        let serialized = serde_json::to_vec(value)?;
        units.push(ArtifactUnit {
            adapter: adapter.into(),
            path: path.clone(),
            identity: format!("{adapter}:{}::pointer::{pointer}", path.to_string_lossy()),
            kind: ArtifactUnitKind::SchemaNode,
            exposure: if adapter == "json-schema" {
                ArtifactExposure::Public
            } else {
                ArtifactExposure::Workspace
            },
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: 0,
                byte_end,
                line_start: 1,
                line_end,
            },
            digest: digest(&serialized),
            structural_digest: digest(&serialized),
        });
    }
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, child) in mapping {
                let Some(key) = key.as_str() else {
                    continue;
                };
                let escaped = key.replace('~', "~0").replace('/', "~1");
                collect_schema_nodes(
                    adapter,
                    path,
                    child,
                    format!("{pointer}/{escaped}"),
                    byte_end,
                    line_end,
                    units,
                )?;
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for (index, child) in sequence.iter().enumerate() {
                collect_schema_nodes(
                    adapter,
                    path,
                    child,
                    format!("{pointer}/{index}"),
                    byte_end,
                    line_end,
                    units,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn markdown_headings(context: &InventoryContext, path: PathBuf) -> Result<Vec<ArtifactUnit>> {
    let root = &context.workspace_root;
    let relative = path
        .strip_prefix(root)
        .context("markdown path escaped workspace")?;
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("Markdown path {path:?}: {error}"))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut units = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(value) = trimmed
            .strip_prefix('#')
            .map(|_| trimmed.trim_start_matches('#').trim())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let occurrence = occurrences.entry(value.to_owned()).or_insert(0);
        let identity = if *occurrence == 0 {
            format!("markdown:{}::heading::{value}", repo_path.to_string_lossy())
        } else {
            format!(
                "markdown:{}::heading::{value}@{}",
                repo_path.to_string_lossy(),
                *occurrence
            )
        };
        *occurrence += 1;
        units.push(ArtifactUnit {
            adapter: "markdown".into(),
            path: repo_path.clone(),
            identity,
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
            structural_digest: digest(
                trimmed
                    .chars()
                    .take_while(|character| *character == '#')
                    .collect::<String>()
                    .as_bytes(),
            ),
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
    let path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("file inventory path {path:?}: {error}"))?;
    let bytes = read_bytes(context, &root.join(path.as_path()))?;
    let text = String::from_utf8_lossy(&bytes);
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
        digest: digest(&bytes),
        structural_digest: digest(&bytes),
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
        // Inventory providers resolve include! and include_str! paths through
        // the filesystem. Normalize the root as well so temporary workspaces
        // mounted through a symlink (notably revision baselines) do not make
        // an otherwise in-workspace support file look external.
        let workspace_root = context
            .workspace_root
            .canonicalize()
            .unwrap_or_else(|_| context.workspace_root.clone());
        let context = InventoryContext {
            workspace_root,
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
        "json" | "json-schema" => &["json"],
        "yaml" => &["yaml", "yml"],
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
            } else if matches!(self.adapter.as_str(), "json" | "json-schema" | "yaml") {
                units.extend(schema_nodes(context, path, &self.adapter)?);
            } else if matches!(self.adapter.as_str(), "javascript" | "typescript") {
                units.extend(source_symbol_units(context, path, &self.adapter)?);
            } else if matches!(self.adapter.as_str(), "python" | "go" | "shell") {
                units.extend(source_text_symbol_units(context, path, &self.adapter)?);
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
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("HTML marker path {path:?}: {error}"))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut units = Vec::new();

    // Marker selectors are semantic HTML attributes, not whole-file fallbacks.
    // Keep the exact attribute span so two targets in one document can still
    // own distinct artifacts and changed-marker planning remains precise.
    for (start, _) in source.match_indices("data-") {
        if !is_html_attribute_start(&source, start) {
            continue;
        }
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
            structural_digest: digest(marker.as_bytes()),
        });
    }
    Ok(units)
}

fn is_html_attribute_start(source: &str, start: usize) -> bool {
    let Some(tag_start) = source[..start].rfind('<') else {
        return false;
    };
    if matches!(
        source.as_bytes().get(tag_start + 1),
        Some(b'/' | b'!' | b'?')
    ) || source[tag_start..start].contains('>')
    {
        return false;
    }
    source[..start]
        .chars()
        .next_back()
        .is_some_and(char::is_whitespace)
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
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("source symbol path {path:?}: {error}"))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let mut units: Vec<ArtifactUnit> = Vec::new();
    let mut identities: BTreeMap<String, usize> = BTreeMap::new();
    let mut parser = TsParser::new();
    let extension = path.extension().and_then(|extension| extension.to_str());
    parser
        .set_language(&match extension {
            // Grammar selection follows the source document, not the
            // provider name: a JavaScript provider can legitimately own
            // both plain JavaScript and JSX files.
            Some("tsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Some("jsx") | Some("js") | Some("mjs") | Some("cjs") => {
                tree_sitter_javascript::LANGUAGE.into()
            }
            Some("ts") | Some("mts") | Some("cts") => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            _ if adapter == "javascript" => tree_sitter_javascript::LANGUAGE.into(),
            _ => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        })
        .map_err(|error| anyhow::anyhow!("load JavaScript/TypeScript grammar: {error}"))?;
    let tree = parser
        .parse(&source, None)
        .context("parse JavaScript/TypeScript source")?;
    if tree.root_node().has_error() {
        bail!("JavaScript/TypeScript source has syntax errors; refusing approximate inventory");
    }
    let mut star_reexports = Vec::new();
    {
        let mut add_symbol = |name: &str, exported: bool, node: Node<'_>| {
            if name.is_empty() || name.starts_with('#') {
                return;
            }
            let identity = format!("{adapter}:{}::{name}", repo_path.to_string_lossy());
            if let Some(index) = identities.get(&identity).copied() {
                if exported {
                    units[index].exposure = ArtifactExposure::Public;
                }
                return;
            }
            let start = node.start_byte();
            let end = node.end_byte();
            let line_start = source[..start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_end = source[..end].bytes().filter(|byte| *byte == b'\n').count() + 1;
            identities.insert(identity.clone(), units.len());
            units.push(ArtifactUnit {
                adapter: adapter.into(),
                path: repo_path.clone(),
                identity,
                kind: ArtifactUnitKind::Symbol,
                exposure: if exported {
                    ArtifactExposure::Public
                } else {
                    ArtifactExposure::Private
                },
                reachability: ArtifactReachability::Active,
                span: SourceSpan {
                    byte_start: start,
                    byte_end: end.max(start + 1),
                    line_start,
                    line_end,
                },
                digest: digest(&source.as_bytes()[start..end.max(start + 1)]),
                structural_digest: structural_digest(&source[start..end.max(start + 1)], name),
            });
        };
        // Discover declarations before export clauses. This makes `export { foo
        // }; const foo = ...` promote the declaration just as reliably as the
        // conventional declaration-before-export spelling.
        for child in tree
            .root_node()
            .named_children(&mut tree.root_node().walk())
        {
            if child.kind() == "export_statement" {
                let export_source = &source[child.byte_range()];
                if let Some(declaration) = child.child_by_field_name("declaration") {
                    javascript_declaration_names(
                        &source,
                        declaration,
                        !export_source.starts_with("export default"),
                        child,
                        &mut add_symbol,
                    );
                } else if let Some(value) = child.child_by_field_name("value") {
                    javascript_declaration_names(
                        &source,
                        value,
                        !export_source.starts_with("export default"),
                        child,
                        &mut add_symbol,
                    );
                }
            } else {
                javascript_declaration_names(&source, child, false, child, &mut add_symbol);
            }
        }
        for child in tree
            .root_node()
            .named_children(&mut tree.root_node().walk())
            .filter(|child| child.kind() == "export_statement")
        {
            let export_source = &source[child.byte_range()];
            if export_source.starts_with("export default") {
                add_symbol("default", true, child);
                continue;
            }
            if let Some(clause) = child
                .named_child(0)
                .filter(|node| node.kind() == "export_clause")
            {
                for specifier in clause.named_children(&mut clause.walk()) {
                    let exported = specifier
                        .child_by_field_name("alias")
                        .or_else(|| specifier.child_by_field_name("name"));
                    if let Some(exported) = exported {
                        // `export { foo }` promotes foo. `export { foo as bar }`
                        // exposes the API identity bar and leaves local foo
                        // private; the export itself remains the exact source
                        // span for a re-export with no local declaration.
                        add_symbol(&source[exported.byte_range()], true, child);
                    }
                }
            } else if export_source.starts_with("export *") {
                star_reexports.push(child);
            }
        }
    }
    let mut test_occurrences = BTreeMap::<String, usize>::new();
    for test in discover_javascript_tests_for_path(adapter, &repo_path, &source)? {
        let occurrence = test_occurrences.entry(test.identity.clone()).or_insert(0);
        let identity_occurrence = *occurrence;
        *occurrence += 1;
        units.push(test_unit(
            adapter,
            repo_path.clone(),
            &source,
            test,
            ArtifactReachability::Active,
            identity_occurrence,
        ));
    }
    for child in star_reexports {
        {
            // Star re-exports cannot be expanded without following another
            // module graph. Preserve that boundary as support context rather
            // than inventing public symbols that may not exist.
            let identity = format!("{adapter}:{}::re-export::*", repo_path.to_string_lossy());
            if !identities.contains_key(&identity) {
                let start = child.start_byte();
                let end = child.end_byte().max(start + 1);
                identities.insert(identity.clone(), units.len());
                units.push(ArtifactUnit {
                    adapter: adapter.into(),
                    path: repo_path.clone(),
                    identity,
                    kind: ArtifactUnitKind::Symbol,
                    exposure: ArtifactExposure::Support,
                    reachability: ArtifactReachability::Active,
                    span: SourceSpan {
                        byte_start: start,
                        byte_end: end,
                        line_start: source[..start]
                            .bytes()
                            .filter(|byte| *byte == b'\n')
                            .count()
                            + 1,
                        line_end: source[..end].bytes().filter(|byte| *byte == b'\n').count() + 1,
                    },
                    digest: digest(&source.as_bytes()[start..end]),
                    structural_digest: digest(&source.as_bytes()[start..end]),
                });
            }
        }
    }
    Ok(units)
}

/// Inventory symbols for the line-oriented languages whose exact identity
/// grammar is intentionally small and explicit. The workspace resolver still
/// owns source-range recovery; inventory owns only stable semantic names.
fn source_text_symbol_units(
    context: &InventoryContext,
    path: PathBuf,
    adapter: &str,
) -> Result<Vec<ArtifactUnit>> {
    let relative = path
        .strip_prefix(&context.workspace_root)
        .context("source path escaped workspace")?;
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("source path {path:?}: {error}"))?;
    let source = String::from_utf8(read_bytes(context, &path)?)?;
    let lines = source.lines().collect::<Vec<_>>();
    let test_file = adapter == "go"
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_test.go"));
    let mut declarations = Vec::<(String, usize, ArtifactExposure)>::new();
    let mut scopes = Vec::<(usize, String)>::new();
    for (line_index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let indent = line.len() - trimmed.len();
        while scopes
            .last()
            .is_some_and(|(scope_indent, _)| *scope_indent >= indent)
        {
            scopes.pop();
        }
        let (name, qualified, is_scope) = match adapter {
            "python" => {
                let (keyword, rest) = trimmed.split_once(' ').unwrap_or((trimmed, ""));
                if keyword != "def" && keyword != "class" {
                    continue;
                }
                let name = rest.split(['(', ':']).next().unwrap_or_default().trim();
                if name.is_empty() {
                    continue;
                }
                let qualified = scopes
                    .iter()
                    .map(|(_, scope)| scope.as_str())
                    .chain(std::iter::once(name))
                    .collect::<Vec<_>>()
                    .join("::");
                (name.to_owned(), qualified, keyword == "class")
            }
            "go" => {
                if let Some(rest) = trimmed.strip_prefix("type ") {
                    let name = rest.split_whitespace().next().unwrap_or_default();
                    if name.is_empty() {
                        continue;
                    }
                    (name.to_owned(), name.to_owned(), false)
                } else if let Some(rest) = trimmed.strip_prefix("func ") {
                    let (receiver, method) = if rest.starts_with('(') {
                        let Some(close) = rest.find(')') else {
                            continue;
                        };
                        let receiver = rest[1..close]
                            .split_whitespace()
                            .last()
                            .unwrap_or_default()
                            .trim_start_matches('*');
                        (Some(receiver), rest[close + 1..].trim_start())
                    } else {
                        (None, rest)
                    };
                    let name = method.split(['(', ' ']).next().unwrap_or_default();
                    if name.is_empty() {
                        continue;
                    }
                    let qualified = receiver
                        .filter(|receiver| !receiver.is_empty())
                        .map_or_else(|| name.to_owned(), |receiver| format!("{receiver}::{name}"));
                    (name.to_owned(), qualified, false)
                } else {
                    continue;
                }
            }
            "shell" => {
                let function_form = trimmed.strip_prefix("function ").map(str::trim_start);
                let candidate = function_form.unwrap_or(trimmed);
                let name = candidate.split(['(', ' ', '{']).next().unwrap_or_default();
                let suffix = &candidate[name.len()..];
                let is_function = suffix.trim_start().starts_with("()")
                    || function_form.is_some_and(|_| suffix.trim_start().starts_with('{'));
                if name.is_empty() || !is_function {
                    continue;
                }
                (name.to_owned(), name.to_owned(), false)
            }
            _ => continue,
        };
        declarations.push((
            qualified,
            line_index,
            if test_file || name.starts_with("test") {
                ArtifactExposure::Test
            } else {
                ArtifactExposure::Private
            },
        ));
        if is_scope {
            scopes.push((indent, name));
        }
    }
    let mut units = Vec::new();
    for (identity, line_index, exposure) in declarations {
        let start = line_start_offset(&source, line_index);
        let end = start + lines[line_index].len();
        let excerpt = &source[start..end];
        units.push(ArtifactUnit {
            adapter: adapter.into(),
            path: repo_path.clone(),
            identity: format!("{adapter}:{}::{identity}", repo_path.to_string_lossy()),
            kind: ArtifactUnitKind::Symbol,
            exposure,
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: start,
                byte_end: end,
                line_start: line_index + 1,
                line_end: line_index + 1,
            },
            digest: digest(excerpt.as_bytes()),
            structural_digest: structural_digest(
                excerpt,
                identity.rsplit("::").next().unwrap_or(&identity),
            ),
        });
    }
    Ok(units)
}

fn line_start_offset(source: &str, line_index: usize) -> usize {
    source
        .lines()
        .take(line_index)
        .map(|line| line.len() + 1)
        .sum()
}

fn test_unit(
    adapter: &str,
    path: RepoPath,
    source: &str,
    test: TestResolution,
    reachability: ArtifactReachability,
    occurrence: usize,
) -> ArtifactUnit {
    let excerpt = &source[test.byte_start..test.byte_end];
    let identity = format!(
        "{adapter}:{}::test::{}@{occurrence}",
        path.to_string_lossy(),
        test.identity,
    );
    ArtifactUnit {
        adapter: adapter.into(),
        path: path.clone(),
        identity,
        kind: ArtifactUnitKind::Symbol,
        exposure: ArtifactExposure::Test,
        reachability,
        span: SourceSpan {
            byte_start: test.byte_start,
            byte_end: test.byte_end,
            line_start: test.line_start,
            line_end: test.line_end,
        },
        digest: digest(excerpt.as_bytes()),
        structural_digest: structural_digest(excerpt, &test.identity),
    }
}

fn javascript_declaration_names(
    source: &str,
    declaration: Node<'_>,
    exported: bool,
    span: Node<'_>,
    add_symbol: &mut impl FnMut(&str, bool, Node<'_>),
) {
    match declaration.kind() {
        "function_declaration"
        | "class_declaration"
        | "interface_declaration"
        | "type_alias_declaration"
        | "enum_declaration"
        | "internal_module" => {
            if let Some(name) = declaration.child_by_field_name("name") {
                add_symbol(&source[name.byte_range()], exported, span);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            for declarator in declaration.named_children(&mut declaration.walk()) {
                if declarator.kind() != "variable_declarator" {
                    continue;
                }
                if let Some(name) = declarator.child_by_field_name("name") {
                    javascript_pattern_names(source, name, exported, span, add_symbol);
                }
            }
        }
        "ambient_declaration" => {
            for child in declaration.named_children(&mut declaration.walk()) {
                javascript_declaration_names(source, child, exported, span, add_symbol);
            }
        }
        _ => {}
    }
}

fn javascript_pattern_names(
    source: &str,
    pattern: Node<'_>,
    exported: bool,
    span: Node<'_>,
    add_symbol: &mut impl FnMut(&str, bool, Node<'_>),
) {
    if pattern.kind() == "identifier" || pattern.kind().ends_with("_identifier_pattern") {
        add_symbol(&source[pattern.byte_range()], exported, span);
        return;
    }
    for child in pattern.named_children(&mut pattern.walk()) {
        javascript_pattern_names(source, child, exported, span, add_symbol);
    }
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
    let cfg = cfg_context(settings, test_mode, &context.profile);
    let mut files = Vec::new();
    let mut support_files = Vec::new();
    let mut file_visibility = BTreeMap::new();
    let mut file_activity = BTreeMap::new();
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
            file_activity: &mut file_activity,
            cfg: &cfg,
        };
        for root in roots {
            collect_reachable_rust(&mut reachability, &root, true, true)?;
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
        let repo_path = RepoPath::from_path(relative)
            .map_err(|error| anyhow::anyhow!("Rust path {path:?}: {error}"))?;
        units.push(unit(context, "rust", path.clone())?);
        let is_test_file = relative.components().any(|component| {
            component.as_os_str() == "tests" || component.as_os_str() == "benches"
        });
        let mut visitor = RustVisitor {
            adapter: "rust".into(),
            path: repo_path.clone(),
            source: &source,
            offsets: line_offsets(&source),
            units: Vec::new(),
            module_path: vec![module_name(&path)],
            impl_type: None,
            attributes: Vec::new(),
            current_active: file_activity.get(&path).copied().unwrap_or(true),
            test_file: is_test_file,
            cfg: &cfg,
            profile: &context.profile,
            public_module: file_visibility.get(&path).copied().unwrap_or(true),
            trait_public: false,
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
    let repo_path = RepoPath::from_path(relative)
        .map_err(|error| anyhow::anyhow!("support path {path:?}: {error}"))?;
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
        structural_digest: digest(&bytes),
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
    current_active: bool,
    test_file: bool,
    cfg: &'a CfgContext,
    profile: &'a str,
    public_module: bool,
    trait_public: bool,
}

impl<'ast> syn::visit::Visit<'ast> for RustVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        let active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.current_active = active;
        self.add(&item.sig.ident.to_string(), &item.vis, item.span(), true);
        if has_test_attribute(&item.attrs) {
            let start = item.span().start();
            let end = item.span().end();
            let mut identity = self.module_path.iter().skip(1).cloned().collect::<Vec<_>>();
            identity.push(item.sig.ident.to_string());
            self.units.push(test_unit(
                "rust",
                self.path.clone(),
                self.source,
                TestResolution {
                    identity: identity.join("::"),
                    byte_start: line_column_to_byte(self.source, &self.offsets, start),
                    byte_end: line_column_to_byte(self.source, &self.offsets, end),
                    line_start: start.line.max(1),
                    line_end: end.line.max(start.line.max(1)),
                },
                test_reachability(active, self.profile, &self.attributes),
                0,
            ));
        }
        // Function bodies can contain local items (for example `const KNOWN`
        // helper tables). They are not module-level semantic targets and
        // would otherwise collide with an item in a different function.
        self.attributes = previous;
        self.current_active = previous_active;
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        self.current_active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.add(&item.ident.to_string(), &item.vis, item.span(), true);
        if self.current_active {
            syn::visit::visit_item_struct(self, item);
        }
        self.attributes = previous;
        self.current_active = previous_active;
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        self.current_active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.add(&item.ident.to_string(), &item.vis, item.span(), true);
        if self.current_active {
            syn::visit::visit_item_enum(self, item);
        }
        self.attributes = previous;
        self.current_active = previous_active;
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.visit_named_item(
            &item.attrs,
            &item.ident.to_string(),
            &item.vis,
            item.span(),
            |this| {
                syn::visit::visit_item_const(this, item);
            },
        );
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.visit_named_item(
            &item.attrs,
            &item.ident.to_string(),
            &item.vis,
            item.span(),
            |this| {
                syn::visit::visit_item_static(this, item);
            },
        );
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.visit_named_item(
            &item.attrs,
            &item.ident.to_string(),
            &item.vis,
            item.span(),
            |this| {
                syn::visit::visit_item_type(this, item);
            },
        );
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.visit_named_item(
            &item.attrs,
            &item.ident.to_string(),
            &item.vis,
            item.span(),
            |this| {
                syn::visit::visit_item_union(this, item);
            },
        );
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        let previous_attributes = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        let active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.current_active = active;
        self.add(&item.ident.to_string(), &item.vis, item.span(), true);
        let previous = self.impl_type.replace(format!("trait({})", item.ident));
        let previous_trait_public = self.trait_public;
        self.trait_public = self.public_module && matches!(item.vis, syn::Visibility::Public(_));
        syn::visit::visit_item_trait(self, item);
        self.impl_type = previous;
        self.trait_public = previous_trait_public;
        self.attributes = previous_attributes;
        self.current_active = previous_active;
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let previous_active = self.current_active;
        self.current_active = previous_active && cfg_active(&item.attrs, self.cfg);
        let type_name = item.self_ty.to_token_stream().to_string().replace(' ', "");
        let type_name = type_name.split('<').next().unwrap_or(&type_name).to_owned();
        let trait_name = item
            .trait_
            .as_ref()
            .map(|(path, _)| path.to_token_stream().to_string().replace(' ', ""));
        let type_name = trait_name
            .map(|name| format!("{name}for{type_name}"))
            .unwrap_or(type_name);
        let previous = self.impl_type.replace(type_name);
        let previous_attributes = self.attributes.clone();
        self.attributes = attribute_keys(&item.attrs);
        syn::visit::visit_item_impl(self, item);
        self.impl_type = previous;
        self.attributes = previous_attributes;
        self.current_active = previous_active;
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let previous_attributes = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        let active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.current_active = active;
        self.add(&item.ident.to_string(), &item.vis, item.span(), true);
        let previous_len = self.module_path.len();
        let previous_public = self.public_module;
        self.module_path.push(item.ident.to_string());
        self.public_module = self.public_module && matches!(item.vis, syn::Visibility::Public(_));
        syn::visit::visit_item_mod(self, item);
        self.module_path.truncate(previous_len);
        self.public_module = previous_public;
        self.attributes = previous_attributes;
        self.current_active = previous_active;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        let active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.current_active = active;
        self.add(&item.sig.ident.to_string(), &item.vis, item.span(), true);
        self.attributes = previous;
        self.current_active = previous_active;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        let active = previous_active && cfg_active(&item.attrs, self.cfg);
        self.current_active = active;
        self.add(
            &item.sig.ident.to_string(),
            &syn::Visibility::Inherited,
            item.span(),
            true,
        );
        if active {
            syn::visit::visit_trait_item_fn(self, item);
        }
        self.attributes = previous;
        self.current_active = previous_active;
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !self.public_module || !matches!(item.vis, syn::Visibility::Public(_)) {
            return;
        }
        let previous = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(&item.attrs);
        self.current_active = previous_active && cfg_active(&item.attrs, self.cfg);
        let mut names = Vec::new();
        collect_use_names(&item.tree, &mut names);
        for name in names {
            self.add(&name, &item.vis, item.span(), true);
        }
        self.attributes = previous;
        self.current_active = previous_active;
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
    fn visit_named_item(
        &mut self,
        attributes: &[syn::Attribute],
        name: &str,
        visibility: &syn::Visibility,
        span: proc_macro2::Span,
        visit: impl FnOnce(&mut Self),
    ) {
        let previous_attributes = self.attributes.clone();
        let previous_active = self.current_active;
        self.attributes = attribute_keys(attributes);
        self.current_active = previous_active && cfg_active(attributes, self.cfg);
        self.add(name, visibility, span, true);
        if self.current_active {
            visit(self);
        }
        self.attributes = previous_attributes;
        self.current_active = previous_active;
    }

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
                && (matches!(visibility, syn::Visibility::Public(_))
                    || (self.trait_public && matches!(visibility, syn::Visibility::Inherited)))
            {
                ArtifactExposure::Public
            } else {
                ArtifactExposure::Private
            },
            reachability: if self.current_active {
                ArtifactReachability::Active
            } else {
                ArtifactReachability::Conditional {
                    profile: if self.attributes.is_empty() {
                        self.profile.to_owned()
                    } else {
                        self.attributes.join(",")
                    },
                }
            },
            span: SourceSpan {
                byte_start,
                byte_end,
                line_start,
                line_end,
            },
            digest: digest(excerpt.as_bytes()),
            structural_digest: structural_digest(excerpt, name),
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

fn cfg_context(settings: &serde_yaml::Value, test_mode: bool, profile: &str) -> CfgContext {
    let mut values = BTreeSet::new();
    if test_mode {
        values.insert("test".into());
    } else {
        values.insert("not(test)".into());
    }
    // `debug_assertions` belongs to the inspected Cargo profile, never to the
    // Mitase binary that happens to be running this inventory.
    if !matches!(profile, "release" | "production") {
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
    format_sha256(hash.finalize())
}

fn structural_digest(source: &str, declared_name: &str) -> String {
    // This digest is used only to corroborate a rename. It must therefore be
    // strictly more conservative than source equality: preserve every byte
    // (including literals, comments, and formatting) and replace only lexical
    // identifiers. This covers a declaration plus its references without
    // confusing a same-spelled string value with a rename.
    let mut normalized = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if matches!(bytes[offset], b'\'' | b'"' | b'`') {
            let quote = bytes[offset];
            let start = offset;
            offset += 1;
            while offset < bytes.len() {
                if bytes[offset] == b'\\' {
                    offset = (offset + 2).min(bytes.len());
                } else if bytes[offset] == quote {
                    offset += 1;
                    break;
                } else {
                    offset += 1;
                }
            }
            normalized.push_str(&source[start..offset]);
            continue;
        }
        if bytes[offset] == b'/' && bytes.get(offset + 1) == Some(&b'/') {
            let start = offset;
            offset = source[offset..]
                .find('\n')
                .map(|index| offset + index)
                .unwrap_or(bytes.len());
            normalized.push_str(&source[start..offset]);
            continue;
        }
        if bytes[offset] == b'/' && bytes.get(offset + 1) == Some(&b'*') {
            let start = offset;
            offset = source[offset + 2..]
                .find("*/")
                .map(|index| offset + 2 + index + 2)
                .unwrap_or(bytes.len());
            normalized.push_str(&source[start..offset]);
            continue;
        }
        if bytes[offset] == b'/' {
            // Without language-specific lexer context, `/` could be division
            // or a regex literal. Treat the whole candidate as opaque: this
            // may miss a rename, but it cannot erase a regex value change.
            let start = offset;
            offset += 1;
            while offset < bytes.len() && bytes[offset] != b'\n' {
                if bytes[offset] == b'\\' {
                    offset = (offset + 2).min(bytes.len());
                } else if bytes[offset] == b'/' {
                    offset += 1;
                    break;
                } else {
                    offset += 1;
                }
            }
            normalized.push_str(&source[start..offset]);
            continue;
        }
        let character = source[offset..]
            .chars()
            .next()
            .expect("offset is in source");
        if character == '_' || character == '$' || character.is_alphabetic() {
            let start = offset;
            offset += character.len_utf8();
            while let Some(character) = source[offset..].chars().next() {
                if character == '_' || character == '$' || character.is_alphanumeric() {
                    offset += character.len_utf8();
                } else {
                    break;
                }
            }
            if &source[start..offset] == declared_name {
                normalized.push_str("<identity>");
            } else {
                normalized.push_str(&source[start..offset]);
            }
        } else {
            normalized.push(character);
            offset += character.len_utf8();
        }
    }
    digest(normalized.as_bytes())
}

fn openapi_structural_digest(method: &str, serialized_operation: &[u8]) -> String {
    let mut bytes = method.to_ascii_lowercase().into_bytes();
    bytes.push(b'\n');
    bytes.extend_from_slice(serialized_operation);
    digest(&bytes)
}

/// Compare two inventory snapshots by semantic identity. Exact identities are
/// independent of source spans, while a structural digest connects a pure
/// rename without guessing from line movement.
pub fn semantic_diff(before: &[ArtifactUnit], after: &[ArtifactUnit]) -> Vec<SemanticChange> {
    let semantic_unit = |unit: &&ArtifactUnit| {
        !(unit.kind == ArtifactUnitKind::File && unit.exposure == ArtifactExposure::Support)
    };
    let before_by_identity = before
        .iter()
        .filter(semantic_unit)
        .map(|unit| (unit.identity.as_str(), unit))
        .collect::<BTreeMap<_, _>>();
    let after_by_identity = after
        .iter()
        .filter(semantic_unit)
        .map(|unit| (unit.identity.as_str(), unit))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    let mut removed = before
        .iter()
        .filter(semantic_unit)
        .filter(|unit| !after_by_identity.contains_key(unit.identity.as_str()))
        .collect::<Vec<_>>();
    let mut added = after
        .iter()
        .filter(semantic_unit)
        .filter(|unit| !before_by_identity.contains_key(unit.identity.as_str()))
        .collect::<Vec<_>>();

    for unit in before.iter().filter(semantic_unit) {
        let Some(current) = after_by_identity.get(unit.identity.as_str()) else {
            continue;
        };
        if unit.digest == current.digest
            && unit.exposure == current.exposure
            && unit.reachability == current.reachability
        {
            continue;
        }
        changes.push(SemanticChange {
            kind: if unit.reachability != current.reachability {
                SemanticChangeKind::ReachabilityChange
            } else if current.exposure == ArtifactExposure::Public
                && unit.exposure != ArtifactExposure::Public
            {
                SemanticChangeKind::PublicAddition
            } else if unit.exposure == ArtifactExposure::Public
                && current.exposure != ArtifactExposure::Public
            {
                SemanticChangeKind::PublicRemoval
            } else if matches!(
                current.exposure,
                ArtifactExposure::Private | ArtifactExposure::Workspace
            ) {
                SemanticChangeKind::PrivateModification
            } else {
                SemanticChangeKind::Modification
            },
            adapter: current.adapter.clone(),
            path: current.path.clone(),
            before_identity: Some(unit.identity.clone()),
            after_identity: Some(current.identity.clone()),
            exposure: current.exposure.clone(),
            before_reachability: Some(unit.reachability.clone()),
            after_reachability: Some(current.reachability.clone()),
        });
    }

    let mut removed_used = BTreeSet::new();
    let mut added_used = BTreeSet::new();
    for (before_index, previous) in removed.iter().enumerate() {
        let structural = if previous.structural_digest.is_empty() {
            previous.digest.as_str()
        } else {
            previous.structural_digest.as_str()
        };
        let candidates = added
            .iter()
            .enumerate()
            .filter(|(after_index, current)| {
                !added_used.contains(after_index)
                    && previous.adapter == current.adapter
                    && previous.kind == current.kind
                    && previous.exposure == current.exposure
                    && previous.path == current.path
                    && (previous.kind != ArtifactUnitKind::Operation
                        || openapi_operation_method(&previous.identity)
                            == openapi_operation_method(&current.identity))
                    && structural
                        == if current.structural_digest.is_empty() {
                            current.digest.as_str()
                        } else {
                            current.structural_digest.as_str()
                        }
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            continue;
        }
        let (after_index, current) = candidates[0];
        removed_used.insert(before_index);
        added_used.insert(after_index);
        changes.push(SemanticChange {
            kind: SemanticChangeKind::Rename,
            adapter: current.adapter.clone(),
            path: current.path.clone(),
            before_identity: Some(previous.identity.clone()),
            after_identity: Some(current.identity.clone()),
            exposure: current.exposure.clone(),
            before_reachability: Some(previous.reachability.clone()),
            after_reachability: Some(current.reachability.clone()),
        });
    }

    for (index, unit) in removed.drain(..).enumerate() {
        if removed_used.contains(&index) {
            continue;
        }
        changes.push(SemanticChange {
            kind: SemanticChangeKind::Deletion,
            adapter: unit.adapter.clone(),
            path: unit.path.clone(),
            before_identity: Some(unit.identity.clone()),
            after_identity: None,
            exposure: unit.exposure.clone(),
            before_reachability: Some(unit.reachability.clone()),
            after_reachability: None,
        });
    }
    for (index, unit) in added.drain(..).enumerate() {
        if added_used.contains(&index) {
            continue;
        }
        changes.push(SemanticChange {
            kind: if unit.exposure == ArtifactExposure::Public {
                SemanticChangeKind::PublicAddition
            } else {
                SemanticChangeKind::Addition
            },
            adapter: unit.adapter.clone(),
            path: unit.path.clone(),
            before_identity: None,
            after_identity: Some(unit.identity.clone()),
            exposure: unit.exposure.clone(),
            before_reachability: None,
            after_reachability: Some(unit.reachability.clone()),
        });
    }
    changes.sort_by(|left, right| {
        (
            left.path.to_string_lossy(),
            left.before_identity.as_deref(),
            left.after_identity.as_deref(),
        )
            .cmp(&(
                right.path.to_string_lossy(),
                right.before_identity.as_deref(),
                right.after_identity.as_deref(),
            ))
    });
    changes
}

fn openapi_operation_method(identity: &str) -> Option<&str> {
    identity
        .rsplit_once("::")
        .and_then(|(_, operation)| operation.split_whitespace().next())
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
            #[serde(default)]
            features: BTreeMap<String, Vec<String>>,
        }
        #[derive(Deserialize)]
        struct PackageTarget {
            src_path: PathBuf,
            kind: Vec<String>,
            #[serde(default, rename = "required-features")]
            required_features: Vec<String>,
        }
        let metadata: Metadata =
            serde_json::from_slice(&output.stdout).context("parse cargo metadata output")?;
        let mut roots = BTreeSet::new();
        for package in metadata.packages {
            let enabled_features = cargo_enabled_features(&package.features, settings);
            for target in package.targets {
                if !target.kind.is_empty()
                    && (include_tests
                        || !target
                            .kind
                            .iter()
                            .any(|kind| matches!(kind.as_str(), "test" | "bench")))
                    && target
                        .required_features
                        .iter()
                        .all(|feature| enabled_features.contains(feature))
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

fn cargo_enabled_features(
    declared: &BTreeMap<String, Vec<String>>,
    settings: &serde_yaml::Value,
) -> BTreeSet<String> {
    let mut requested = settings
        .get("features")
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(serde_yaml::Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if settings
        .get("all_features")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false)
    {
        requested.extend(declared.keys().cloned());
    } else if !settings
        .get("no_default_features")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false)
    {
        requested.push("default".into());
    }
    let mut enabled = BTreeSet::new();
    while let Some(feature) = requested.pop() {
        if !enabled.insert(feature.clone()) {
            continue;
        }
        for nested in declared.get(&feature).into_iter().flatten() {
            // Cargo feature definitions may also activate optional deps. Those
            // are not feature names and cannot satisfy required-features.
            if !nested.starts_with("dep:") && !nested.contains('/') {
                requested.push(nested.strip_prefix("?").unwrap_or(nested).into());
            }
        }
    }
    enabled
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
    file_activity: &'a mut BTreeMap<PathBuf, bool>,
    cfg: &'a CfgContext,
}

fn collect_reachable_rust(
    reachability: &mut RustReachability<'_>,
    file: &Path,
    public_module: bool,
    active: bool,
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
    let was_active = reachability
        .file_activity
        .get(file)
        .copied()
        .unwrap_or(false);
    reachability
        .file_activity
        .insert(file.to_path_buf(), was_active || active);
    if reachability.out.iter().any(|existing| existing == file)
        && previous_visibility.is_some_and(|was_public| was_public || !public_module)
        && (was_active || !active)
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
                active && cfg_active(&module.attrs, reachability.cfg),
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
                    collect_reachable_rust(reachability, &candidate, public_module, active)?;
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
    fn javascript_inventory_uses_export_semantics_and_unicode_safe_ast_spans() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("model.ts");
        fs::write(
            &path,
            concat!(
                "const pattern = /{/;\n",
                "const local = 1;\n",
                "export { local as public_name };\n",
                "export const { value } = source;\n",
                "export const 日本語 = `value: ${value}`;\n",
                "export default function () {}\n",
                "export function api() { return pattern; }\n",
            ),
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let units = source_symbol_units(&context, path, "typescript").unwrap();
        let public = |name: &str| {
            units.iter().any(|unit| {
                unit.identity.ends_with(&format!("::{name}"))
                    && unit.exposure == ArtifactExposure::Public
            })
        };
        assert!(public("public_name"));
        assert!(public("value"));
        assert!(public("日本語"));
        assert!(public("api"));
        assert!(!public("local"));
    }

    #[test]
    fn html_marker_inventory_keeps_attributes_as_distinct_units() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("workbench.html");
        fs::write(
            &path,
            r#"<aside data-page="work" data-i18n-aria="a11y.main_pages">data-fake="no"</aside>"#,
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
        assert!(!units.iter().any(|unit| unit.identity.contains("data-fake")));
    }

    #[test]
    fn semantic_diff_distinguishes_public_private_rename_and_deletion() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("model.ts");
        fs::write(
            &path,
            "export function stable() {}\nfunction helper() { return 1; }\nfunction oldName(value) { return value ? oldName(false) : 2; }\nfunction removed() { return 3; }\n",
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let before = source_symbol_units(&context, path.clone(), "typescript").unwrap();
        fs::write(
            &path,
            "export function stable() {}\nexport function added() {}\nfunction helper() { return 10; }\nfunction newName(value) { return value ? newName(false) : 2; }\n",
        )
        .unwrap();
        let after = source_symbol_units(&context, path, "typescript").unwrap();
        let changes = semantic_diff(&before, &after);

        assert!(changes.iter().any(|change| {
            change.kind == SemanticChangeKind::PublicAddition
                && change
                    .after_identity
                    .as_deref()
                    .is_some_and(|id| id.ends_with("::added"))
        }));
        assert!(changes.iter().any(|change| {
            change.kind == SemanticChangeKind::PrivateModification
                && change
                    .after_identity
                    .as_deref()
                    .is_some_and(|id| id.ends_with("::helper"))
        }));
        assert!(changes.iter().any(|change| {
            change.kind == SemanticChangeKind::Rename
                && change
                    .before_identity
                    .as_deref()
                    .is_some_and(|id| id.ends_with("::oldName"))
                && change
                    .after_identity
                    .as_deref()
                    .is_some_and(|id| id.ends_with("::newName"))
        }));
        assert!(changes.iter().any(|change| {
            change.kind == SemanticChangeKind::Deletion
                && change
                    .before_identity
                    .as_deref()
                    .is_some_and(|id| id.ends_with("::removed"))
        }));
    }

    #[test]
    fn semantic_identity_survives_line_movement_but_reports_content_changes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("model.js");
        fs::write(&path, "export function api(){return true;}\n").unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let before = source_symbol_units(&context, path.clone(), "javascript").unwrap();
        fs::write(&path, "\n\nexport function api() {\n  return true;\n}\n").unwrap();
        let after = source_symbol_units(&context, path, "javascript").unwrap();
        assert_eq!(before[0].identity, after[0].identity);
        assert_ne!(before[0].span.line_start, after[0].span.line_start);
        assert!(
            semantic_diff(&before, &after)
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Modification)
        );
    }

    #[test]
    fn javascript_exports_promote_local_symbols_and_model_export_forms_explicitly() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("module.ts");
        fs::write(
            &path,
            concat!(
                "const foo = 1;\n",
                "export { foo };\n",
                "const aliased = 2;\n",
                "export { aliased as publicAlias };\n",
                "export { declaredLater };\n",
                "const declaredLater = 3;\n",
                "export { remote } from \"./remote.js\";\n",
                "export default function internalDefault() {}\n",
                "type Shape = { id: string };\n",
                "export type { Shape };\n",
                "const helper = () => {};\n",
                "export * from \"./star.js\";\n",
            ),
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let units = source_symbol_units(&context, path, "typescript").unwrap();
        let exposure = |name: &str| {
            units
                .iter()
                .find(|unit| unit.identity.ends_with(&format!("::{name}")))
                .map(|unit| unit.exposure.clone())
        };
        assert_eq!(exposure("foo"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("publicAlias"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("aliased"), Some(ArtifactExposure::Private));
        assert_eq!(exposure("declaredLater"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("remote"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("default"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("internalDefault"), Some(ArtifactExposure::Private));
        assert_eq!(exposure("Shape"), Some(ArtifactExposure::Public));
        assert_eq!(exposure("helper"), Some(ArtifactExposure::Private));
        assert!(units.iter().any(|unit| {
            unit.identity.ends_with("::re-export::*") && unit.exposure == ArtifactExposure::Support
        }));
    }

    #[test]
    fn javascript_and_typescript_extensions_select_their_native_grammars() {
        let temp = tempfile::tempdir().unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        for (name, source, adapter) in [
            (
                "plain.js",
                "export const value = { get ready() { return true; } };",
                "javascript",
            ),
            (
                "view.jsx",
                "export const View = () => <main>ok</main>;",
                "javascript",
            ),
            (
                "types.ts",
                "export interface Shape { id: string }",
                "typescript",
            ),
            (
                "view.tsx",
                "export const View = () => <main>ok</main>;",
                "typescript",
            ),
        ] {
            let path = temp.path().join(name);
            fs::write(&path, source).unwrap();
            assert!(
                source_symbol_units(&context, path, adapter).is_ok(),
                "{name}"
            );
        }
    }

    #[test]
    fn semantic_diff_names_visibility_transitions_without_downgrading_public_removal() {
        let base = |exposure, reachability| ArtifactUnit {
            adapter: "typescript".into(),
            path: RepoPath::new("src/api.ts").unwrap(),
            identity: "typescript:src/api.ts::api".into(),
            kind: ArtifactUnitKind::Symbol,
            exposure,
            reachability,
            span: SourceSpan {
                byte_start: 0,
                byte_end: 1,
                line_start: 1,
                line_end: 1,
            },
            digest: "same".into(),
            structural_digest: "same".into(),
        };
        let classify = |before, after| semantic_diff(&[before], &[after])[0].kind;
        assert_eq!(
            classify(
                base(ArtifactExposure::Private, ArtifactReachability::Active),
                base(ArtifactExposure::Public, ArtifactReachability::Active),
            ),
            SemanticChangeKind::PublicAddition
        );
        assert_eq!(
            classify(
                base(ArtifactExposure::Workspace, ArtifactReachability::Active),
                base(ArtifactExposure::Public, ArtifactReachability::Active),
            ),
            SemanticChangeKind::PublicAddition
        );
        assert_eq!(
            classify(
                base(ArtifactExposure::Public, ArtifactReachability::Active),
                base(ArtifactExposure::Private, ArtifactReachability::Active),
            ),
            SemanticChangeKind::PublicRemoval
        );
        assert_eq!(
            classify(
                base(ArtifactExposure::Public, ArtifactReachability::Active),
                base(
                    ArtifactExposure::Public,
                    ArtifactReachability::Conditional {
                        profile: "enterprise".into(),
                    },
                ),
            ),
            SemanticChangeKind::ReachabilityChange
        );
        assert_eq!(
            classify(
                base(
                    ArtifactExposure::Public,
                    ArtifactReachability::Conditional {
                        profile: "enterprise".into(),
                    },
                ),
                base(ArtifactExposure::Public, ArtifactReachability::Active),
            ),
            SemanticChangeKind::ReachabilityChange
        );
    }

    #[test]
    fn semantic_diff_preserves_literal_meaning_and_rejects_false_renames() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("model.ts");
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        fs::write(&path, "function api() { return \"a b\"; }\n").unwrap();
        let before = source_symbol_units(&context, path.clone(), "typescript").unwrap();
        fs::write(&path, "function api() { return \"ab\"; }\n").unwrap();
        let after = source_symbol_units(&context, path.clone(), "typescript").unwrap();
        assert!(
            semantic_diff(&before, &after)
                .iter()
                .any(|change| change.kind == SemanticChangeKind::PrivateModification)
        );

        fs::write(&path, "function old() { return \"old\"; }\n").unwrap();
        let before = source_symbol_units(&context, path.clone(), "typescript").unwrap();
        fs::write(&path, "function new() { return \"new\"; }\n").unwrap();
        let after = source_symbol_units(&context, path.clone(), "typescript").unwrap();
        let changes = semantic_diff(&before, &after);
        assert!(
            !changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Rename)
        );
        assert!(
            changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Deletion)
        );
        assert!(
            changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Addition)
        );
    }

    #[test]
    fn semantic_diff_does_not_infer_cross_file_renames() {
        let path_a = RepoPath::new("src/a.ts").unwrap();
        let path_b = RepoPath::new("src/b.ts").unwrap();
        let before = vec![ArtifactUnit {
            adapter: "typescript".into(),
            path: path_a,
            identity: "typescript:src/a.ts::old".into(),
            kind: ArtifactUnitKind::Symbol,
            exposure: ArtifactExposure::Private,
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: 0,
                byte_end: 1,
                line_start: 1,
                line_end: 1,
            },
            digest: "before".into(),
            structural_digest: "shape".into(),
        }];
        let after = vec![ArtifactUnit {
            adapter: "typescript".into(),
            path: path_b,
            identity: "typescript:src/b.ts::new".into(),
            kind: ArtifactUnitKind::Symbol,
            exposure: ArtifactExposure::Private,
            reachability: ArtifactReachability::Active,
            span: SourceSpan {
                byte_start: 0,
                byte_end: 1,
                line_start: 1,
                line_end: 1,
            },
            digest: "after".into(),
            structural_digest: "shape".into(),
        }];
        let changes = semantic_diff(&before, &after);
        assert_eq!(changes.len(), 2);
        assert!(
            changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Deletion)
        );
        assert!(
            changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Addition)
        );
    }

    #[test]
    fn inactive_rust_cfg_units_are_inventory_only() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("src/lib.rs"),
            concat!(
                "#[cfg(feature = \"enterprise\")]\n",
                "pub fn enterprise() {}\n",
                "#[cfg(feature = \"enterprise\")]\n",
                "pub mod external;\n",
                "#[cfg(feature = \"enterprise\")]\n",
                "pub mod inline { pub fn nested() {} }\n",
                "#[cfg(feature = \"enterprise\")]\n",
                "#[test]\n",
                "fn enterprise_test() {}\n",
                "pub fn active() {}\n",
            ),
        )
        .unwrap();
        fs::write(
            temp.path().join("src/external.rs"),
            "pub fn nested_external() {}\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        let profile = InventoryProfile {
            id: "default".into(),
            providers: BTreeMap::from([("rust".into(), serde_yaml::Value::Null)]),
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
        let enterprise = units
            .iter()
            .find(|unit| unit.identity.contains("enterprise"))
            .unwrap();
        assert!(matches!(
            enterprise.reachability,
            ArtifactReachability::Conditional { .. }
        ));
        for name in ["nested", "nested_external"] {
            assert!(units.iter().any(|unit| {
                unit.identity.contains(name)
                    && matches!(unit.reachability, ArtifactReachability::Conditional { .. })
            }));
        }
        assert!(units.iter().any(|unit| {
            unit.identity.ends_with("::active") && unit.reachability == ArtifactReachability::Active
        }));
        assert!(units.iter().any(|unit| {
            unit.identity.ends_with("::test::enterprise_test@0")
                && matches!(unit.reachability, ArtifactReachability::Conditional { .. })
        }));
    }

    #[test]
    fn rust_inventory_models_effective_public_items_and_private_modules() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("src/lib.rs"),
            concat!(
                "pub struct Request;\n",
                "pub enum Error { Failed }\n",
                "pub trait Service { fn call(&self); }\n",
                "pub const LIMIT: usize = 1;\n",
                "pub type Alias = Request;\n",
                "mod private { pub fn helper() {} }\n",
                "pub use Request as ExportedRequest;\n",
            ),
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        let units = discover_rust(
            &context,
            &[RepoPath::new("src/lib.rs").unwrap()],
            &serde_yaml::Value::Null,
        )
        .unwrap()
        .units;
        let exposure = |suffix: &str| {
            units
                .iter()
                .find(|unit| unit.identity.ends_with(suffix))
                .map(|unit| unit.exposure.clone())
        };
        for name in [
            "::Request",
            "::Error",
            "::Service",
            "::LIMIT",
            "::Alias",
            "::ExportedRequest",
        ] {
            assert_eq!(exposure(name), Some(ArtifactExposure::Public), "{name}");
        }
        assert_eq!(
            exposure("::impl(trait(Service))::call"),
            Some(ArtifactExposure::Public)
        );
        assert_eq!(
            exposure("::private::helper"),
            Some(ArtifactExposure::Private)
        );
    }

    #[test]
    fn cargo_roots_exclude_targets_with_disabled_required_features() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            concat!(
                "[package]\nname = \"feature-roots\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
                "[features]\ndefault = [\"default_api\"]\ndefault_api = []\nadmin = []\n",
                "[[bin]]\nname = \"admin\"\npath = \"src/admin.rs\"\nrequired-features = [\"admin\"]\n",
            ),
        )
        .unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn api() {}\n").unwrap();
        fs::write(temp.path().join("src/admin.rs"), "fn main() {}\n").unwrap();
        let default_roots = cargo_roots(temp.path(), &serde_yaml::Value::Null).unwrap();
        assert!(
            !default_roots
                .iter()
                .any(|path| path.ends_with("src/admin.rs"))
        );
        let enabled = serde_yaml::from_str("features: [admin]").unwrap();
        let enabled_roots = cargo_roots(temp.path(), &enabled).unwrap();
        assert!(
            enabled_roots
                .iter()
                .any(|path| path.ends_with("src/admin.rs"))
        );
        let no_default = serde_yaml::from_str("no_default_features: true").unwrap();
        assert!(
            !cargo_enabled_features(
                &BTreeMap::from([("default".into(), vec!["default_api".into()])]),
                &no_default,
            )
            .contains("default_api")
        );
    }

    #[test]
    fn language_aware_profile_discovers_each_supported_semantic_boundary() {
        let temp = tempfile::tempdir().unwrap();
        for directory in ["src", "web", "api", "docs", "schema"] {
            fs::create_dir_all(temp.path().join(directory)).unwrap();
        }
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn rust_api() {}\n").unwrap();
        fs::write(
            temp.path().join("web/app.js"),
            "export function javascriptApi() {}\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("web/app.ts"),
            "export interface TypeScriptApi { value: string }\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("api/openapi.yaml"),
            "openapi: 3.1.0\npaths:\n  /users:\n    get:\n      responses: {}\n",
        )
        .unwrap();
        fs::write(temp.path().join("docs/guide.md"), "# Semantic guide\n").unwrap();
        fs::write(
            temp.path().join("schema/data.json"),
            r#"{"user":{"name":"Ada"}}"#,
        )
        .unwrap();
        fs::write(temp.path().join("schema/data.yaml"), "user:\n  name: Ada\n").unwrap();
        fs::write(
            temp.path().join("schema/model.schema.json"),
            r#"{"properties":{"user":{"type":"string"}}}"#,
        )
        .unwrap();
        let roots = |value: &str| {
            serde_yaml::from_str::<serde_yaml::Value>(&format!("{{ roots: [{value}] }}")).unwrap()
        };
        let profile = InventoryProfile {
            id: "default".into(),
            providers: BTreeMap::from([
                ("rust".into(), serde_yaml::Value::Null),
                ("javascript".into(), roots("web/app.js")),
                ("typescript".into(), roots("web/app.ts")),
                ("openapi".into(), roots("api/openapi.yaml")),
                ("markdown".into(), roots("docs/guide.md")),
                ("json".into(), roots("schema/data.json")),
                ("yaml".into(), roots("schema/data.yaml")),
                ("json-schema".into(), roots("schema/model.schema.json")),
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
        for expected in [
            "rust:src/lib.rs::lib::rust_api",
            "javascript:web/app.js::javascriptApi",
            "typescript:web/app.ts::TypeScriptApi",
            "openapi:api/openapi.yaml::GET /users",
            "markdown:docs/guide.md::heading::Semantic guide",
            "json:schema/data.json::pointer::/user/name",
            "yaml:schema/data.yaml::pointer::/user/name",
            "json-schema:schema/model.schema.json::pointer::/properties/user/type",
        ] {
            assert!(
                units.iter().any(|unit| unit.identity == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn text_language_inventory_exposes_qualified_symbols_and_tests() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::create_dir_all(temp.path().join("tests")).unwrap();
        fs::write(
            temp.path().join("src/service.py"),
            "class Service:\n    def submit(self):\n        return True\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("src/service.go"),
            "type Service struct{}\nfunc (s *Service) Submit() {}\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("tests/service_test.go"),
            "func TestService(t *testing.T) {}\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("src/service.sh"),
            "function run_service {\n  echo ok\n}\n",
        )
        .unwrap();
        let profile = InventoryProfile {
            id: "default".into(),
            providers: BTreeMap::from([
                ("python".into(), serde_yaml::Value::Null),
                ("go".into(), serde_yaml::Value::Null),
                ("shell".into(), serde_yaml::Value::Null),
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
        for expected in [
            "python:src/service.py::Service",
            "python:src/service.py::Service::submit",
            "go:src/service.go::Service",
            "go:src/service.go::Service::Submit",
            "go:tests/service_test.go::TestService",
            "shell:src/service.sh::run_service",
        ] {
            assert!(
                units.iter().any(|unit| unit.identity == expected),
                "missing {expected}"
            );
        }
        assert!(units.iter().any(|unit| {
            unit.identity == "go:tests/service_test.go::TestService"
                && unit.exposure == ArtifactExposure::Test
        }));
    }

    #[test]
    fn native_test_identity_resolution_preserves_titles_and_rejects_duplicates() {
        let source = concat!(
            "it(\"keyword-first\", async () => {\n",
            "  expect(true).toBe(true);\n",
            "});\n",
            "test(\"other\", () => {});\n",
        );
        let tests = discover_tests("typescript", source).expect("tests");
        assert_eq!(
            tests
                .iter()
                .map(|test| test.identity.as_str())
                .collect::<Vec<_>>(),
            ["keyword-first", "other"]
        );
        assert!(tests[0].byte_end > tests[0].byte_start);
        assert_eq!(tests[0].line_start, 1);
        assert_eq!(tests[0].line_end, 3);

        let rust = "#[test]\nfn exact_test() {}\nfn helper() {}\n";
        let resolved = resolve_test("rust", rust, "exact_test").expect("Rust test");
        assert_eq!(resolved.identity, "exact_test");
        assert_eq!(resolved.line_start, 1);
        assert_eq!(resolved.line_end, 2);

        let duplicate = r#"it("duplicate", () => {}); it("duplicate", () => {});"#;
        let error = resolve_test("javascript", duplicate, "duplicate")
            .expect_err("duplicate titles must not resolve");
        assert!(error.to_string().contains("ambiguous"));

        let escaped = r#"it("unicode \u{1f600}", () => {});"#;
        let resolved =
            resolve_test("javascript", escaped, "unicode 😀").expect("Unicode test title escape");
        assert_eq!(resolved.identity, "unicode 😀");

        let surrogate = r#"it("surrogate \uD83D\uDE00", () => {});"#;
        let resolved = resolve_test("javascript", surrogate, "surrogate 😀")
            .expect("surrogate pair test title escape");
        assert_eq!(resolved.identity, "surrogate 😀");

        let non_escape = r#"it("non-escape \a \z \&", () => {});"#;
        let resolved = resolve_test("javascript", non_escape, "non-escape a z &")
            .expect("ECMAScript non-escape title");
        assert_eq!(resolved.identity, "non-escape a z &");
    }

    #[test]
    fn javascript_inventory_keeps_duplicate_test_titles_as_distinct_units() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("search.test.ts");
        fs::write(
            &path,
            "it(\"duplicate\", () => {});\nit(\"duplicate\", () => {});\n",
        )
        .unwrap();
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };

        let tests = source_symbol_units(&context, path, "typescript")
            .unwrap()
            .into_iter()
            .filter(|unit| unit.exposure == ArtifactExposure::Test)
            .collect::<Vec<_>>();
        assert_eq!(tests.len(), 2);
        assert_ne!(tests[0].identity, tests[1].identity);
        assert!(tests[0].identity.ends_with("::test::duplicate@0"));
        assert!(tests[1].identity.ends_with("::test::duplicate@1"));
    }

    #[test]
    fn openapi_semantic_diff_tracks_operation_content_and_path_renames() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("openapi.yaml");
        let context = InventoryContext {
            workspace_root: temp.path().into(),
            profile: "default".into(),
            settings: serde_yaml::Value::Null,
            excludes: vec![],
            overlays: BTreeMap::new(),
        };
        fs::write(
            &path,
            "paths:\n  /old:\n    get:\n      summary: Read user\n",
        )
        .unwrap();
        let before = openapi_operations(&context, path.clone()).unwrap();
        fs::write(
            &path,
            "paths:\n  /old:\n    get:\n      summary: Read account\n",
        )
        .unwrap();
        let modified = openapi_operations(&context, path.clone()).unwrap();
        assert!(
            semantic_diff(&before, &modified)
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Modification)
        );

        fs::write(
            &path,
            "paths:\n  /new:\n    get:\n      summary: Read account\n",
        )
        .unwrap();
        let renamed = openapi_operations(&context, path).unwrap();
        assert!(
            semantic_diff(&modified, &renamed)
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Rename)
        );

        fs::write(
            temp.path().join("openapi.yaml"),
            "paths:\n  /new:\n    delete:\n      summary: Read account\n",
        )
        .unwrap();
        let method_changed =
            openapi_operations(&context, temp.path().join("openapi.yaml")).unwrap();
        let method_changes = semantic_diff(&renamed, &method_changed);
        assert!(
            method_changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Deletion)
        );
        assert!(
            method_changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::PublicAddition)
        );
        assert!(
            !method_changes
                .iter()
                .any(|change| change.kind == SemanticChangeKind::Rename)
        );
    }

    #[test]
    fn json_schema_provider_discovers_exact_pointer_identities() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("schema.json"),
            r#"{"properties":{"user":{"type":"string"}}}"#,
        )
        .unwrap();
        let profile = InventoryProfile {
            id: "default".into(),
            providers: BTreeMap::from([(
                "json-schema".into(),
                serde_yaml::from_str("{ roots: [schema.json] }").unwrap(),
            )]),
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
        assert!(units.iter().any(|unit| {
            unit.kind == ArtifactUnitKind::SchemaNode
                && unit.identity == "json-schema:schema.json::pointer::/properties/user/type"
                && unit.exposure == ArtifactExposure::Public
        }));
    }
}
