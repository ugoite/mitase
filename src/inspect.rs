// FEAT-CHECK-001
// REQ-CORE-002

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path, process::Command};
use syn::{Attribute, ImplItem, Item};
use tree_sitter::Parser;

use crate::{
    config::SyuConfig,
    language::{LanguageAdapter, adapter_for_language},
    runtime::{RuntimeKind, resolve_runtime_command},
};

const PYTHON_INSPECTOR: &str = r#"
import ast
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
source = path.read_text(encoding='utf-8')
lines = source.splitlines()
tree = ast.parse(source, filename=str(path))

def indent_for_line(line_no):
    if line_no < 1 or line_no > len(lines):
        return ""
    line = lines[line_no - 1]
    return line[: len(line) - len(line.lstrip(' \t'))]

def doc_bounds(node):
    if not getattr(node, "body", None):
        return None, None
    first = node.body[0]
    if isinstance(first, ast.Expr) and isinstance(getattr(first, "value", None), ast.Constant) and isinstance(first.value.value, str):
        return first.lineno, first.end_lineno
    return None, None

symbols = []

class Visitor(ast.NodeVisitor):
    def visit_FunctionDef(self, node):
        self.record(node)
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node):
        self.record(node)
        self.generic_visit(node)

    def visit_ClassDef(self, node):
        self.record(node)
        self.generic_visit(node)

    def record(self, node):
        doc_start, doc_end = doc_bounds(node)
        symbols.append(
            {
                "name": node.name,
                "docs": ast.get_docstring(node, clean=False) or "",
                "line": node.lineno,
                "body_line": node.body[0].lineno if getattr(node, "body", None) else node.lineno + 1,
                "body_indent": indent_for_line(node.body[0].lineno) if getattr(node, "body", None) else indent_for_line(node.lineno) + "    ",
                "doc_start_line": doc_start,
                "doc_end_line": doc_end,
            }
        )

Visitor().visit(tree)
json.dump(symbols, sys.stdout)
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInspection {
    pub docs: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PythonSymbolInspection {
    pub(crate) name: String,
    pub(crate) docs: String,
    pub(crate) line: usize,
    pub(crate) body_line: usize,
    pub(crate) body_indent: String,
    pub(crate) doc_start_line: Option<usize>,
    pub(crate) doc_end_line: Option<usize>,
}

#[derive(Debug, Clone)]
struct TypeScriptSymbolInspection {
    name: String,
    docs: String,
    line: usize,
}

#[derive(Debug, Clone)]
struct JsDocBlock {
    end_line: usize,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PythonSymbolRecord {
    name: String,
    docs: String,
    line: usize,
    body_line: usize,
    body_indent: String,
    doc_start_line: Option<usize>,
    doc_end_line: Option<usize>,
}

pub fn supports_rich_inspection(language: &str) -> bool {
    matches!(
        adapter_for_language(language).map(LanguageAdapter::canonical_name),
        Some("rust" | "python" | "typescript")
    )
}

// FEAT-CHECK-001
pub fn inspect_symbol(
    language: &str,
    config: &SyuConfig,
    path: &Path,
    contents: &str,
    symbol: &str,
) -> Result<Option<SymbolInspection>> {
    match adapter_for_language(language).map(LanguageAdapter::canonical_name) {
        Some("rust") => Ok(
            inspect_rust_symbol(contents, symbol).map(|docs| SymbolInspection {
                docs,
                line: find_rust_declaration_line(contents, symbol).unwrap_or(1),
            }),
        ),
        Some("python") => {
            Ok(
                inspect_python_symbol(config, path, symbol)?.map(|item| SymbolInspection {
                    docs: item.docs,
                    line: item.line,
                }),
            )
        }
        Some("typescript") => Ok(
            inspect_typescript_symbol(path, contents, symbol)?.map(|item| SymbolInspection {
                docs: item.docs,
                line: item.line,
            }),
        ),
        _ => Ok(None),
    }
}

// FEAT-CHECK-001
pub fn apply_symbol_doc_fix(
    language: &str,
    config: &SyuConfig,
    path: &Path,
    contents: &str,
    symbol: &str,
    required_snippets: &[String],
) -> Result<Option<String>> {
    let required = normalized_required_snippets(required_snippets);
    if required.is_empty() {
        return Ok(None);
    }

    match adapter_for_language(language).map(LanguageAdapter::canonical_name) {
        Some("rust") => fix_rust_symbol_docs(contents, symbol, &required),
        Some("python") => fix_python_symbol_docs(config, path, contents, symbol, &required),
        Some("typescript") => fix_typescript_symbol_docs(path, contents, symbol, &required),
        _ => Ok(None),
    }
}

fn inspect_rust_symbol(contents: &str, symbol: &str) -> Option<String> {
    let file = syn::parse_file(contents).ok()?;
    let mut symbols = Vec::new();
    collect_rust_items(&file.items, &mut symbols);
    symbols
        .into_iter()
        .find(|(name, _)| name == symbol)
        .map(|(_, docs)| docs)
}

fn collect_rust_items(items: &[Item], symbols: &mut Vec<(String, String)>) {
    for item in items {
        match item {
            Item::Fn(item) => symbols.push((item.sig.ident.to_string(), rust_docs(&item.attrs))),
            Item::Struct(item) => symbols.push((item.ident.to_string(), rust_docs(&item.attrs))),
            Item::Enum(item) => symbols.push((item.ident.to_string(), rust_docs(&item.attrs))),
            Item::Trait(item) => symbols.push((item.ident.to_string(), rust_docs(&item.attrs))),
            Item::Const(item) => symbols.push((item.ident.to_string(), rust_docs(&item.attrs))),
            Item::Static(item) => symbols.push((item.ident.to_string(), rust_docs(&item.attrs))),
            Item::Impl(item) => {
                for impl_item in &item.items {
                    if let ImplItem::Fn(method) = impl_item {
                        symbols.push((method.sig.ident.to_string(), rust_docs(&method.attrs)));
                    }
                }
            }
            Item::Mod(item) => {
                if let Some((_, nested_items)) = &item.content {
                    collect_rust_items(nested_items, symbols);
                }
            }
            _ => {}
        }
    }
}

fn rust_docs(attributes: &[Attribute]) -> String {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .filter_map(|attribute| match &attribute.meta {
            syn::Meta::NameValue(meta) => match &meta.value {
                syn::Expr::Lit(expr) => match &expr.lit {
                    syn::Lit::Str(value) => Some(value.value()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn inspect_python_symbol(
    config: &SyuConfig,
    path: &Path,
    symbol: &str,
) -> Result<Option<PythonSymbolInspection>> {
    let symbols = inspect_python_file(config, path)?;
    Ok(symbols.into_iter().find(|item| item.name == symbol))
}

pub(crate) fn inspect_python_file(
    config: &SyuConfig,
    path: &Path,
) -> Result<Vec<PythonSymbolInspection>> {
    inspect_python_file_with_runtime(
        resolve_runtime_command(config, RuntimeKind::Python).as_deref(),
        path,
    )
}

fn inspect_python_file_with_runtime(
    runtime: Option<&str>,
    path: &Path,
) -> Result<Vec<PythonSymbolInspection>> {
    let Some(runtime) = runtime else {
        bail!(
            "no Python runtime could be auto-detected; set `runtimes.python.command` in `syu.yaml`"
        );
    };

    let output = Command::new(runtime)
        .arg("-c")
        .arg(PYTHON_INSPECTOR)
        .arg(path)
        .output()
        .with_context(|| format!("failed to run Python inspector with `{runtime}`"))?;

    if !output.status.success() {
        bail!(
            "Python inspector failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let records: Vec<PythonSymbolRecord> = serde_json::from_slice(&output.stdout)
        .context("failed to decode Python inspection output")?;
    Ok(records
        .into_iter()
        .map(|record| PythonSymbolInspection {
            name: record.name,
            docs: record.docs,
            line: record.line,
            body_line: record.body_line,
            body_indent: record.body_indent,
            doc_start_line: record.doc_start_line,
            doc_end_line: record.doc_end_line,
        })
        .collect())
}

fn inspect_typescript_symbol(
    path: &Path,
    contents: &str,
    symbol: &str,
) -> Result<Option<TypeScriptSymbolInspection>> {
    let symbols = inspect_typescript_file(path, contents)?;
    Ok(symbols.into_iter().find(|item| item.name == symbol))
}

fn inspect_typescript_file(path: &Path, contents: &str) -> Result<Vec<TypeScriptSymbolInspection>> {
    let mut parser = Parser::new();
    let language = match path.extension().and_then(|ext| ext.to_str()) {
        Some("tsx" | "jsx") => tree_sitter_typescript::LANGUAGE_TSX,
        _ => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
    };
    parser
        .set_language(&language.into())
        .context("failed to load TypeScript grammar")?;

    let tree = parser
        .parse(contents, None)
        .ok_or_else(|| anyhow::anyhow!("failed to parse TypeScript source"))?;

    let mut symbols = Vec::new();
    collect_typescript_symbols(tree.root_node(), contents, &mut symbols);
    Ok(symbols)
}

fn collect_typescript_symbols(
    node: tree_sitter::Node<'_>,
    contents: &str,
    symbols: &mut Vec<TypeScriptSymbolInspection>,
) {
    match node.kind() {
        "function_declaration"
        | "class_declaration"
        | "interface_declaration"
        | "type_alias_declaration"
        | "method_definition"
        | "abstract_method_signature" => {
            let name = node.child_by_field_name("name").unwrap_or(node);
            symbols.push(TypeScriptSymbolInspection {
                name: node_text(name, contents).to_string(),
                docs: find_jsdoc_block(contents, node.start_position().row + 1)
                    .map(|block| block.text)
                    .unwrap_or_default(),
                line: node.start_position().row + 1,
            });
        }
        "variable_declarator" => {
            let name = node.child_by_field_name("name").unwrap_or(node);
            symbols.push(TypeScriptSymbolInspection {
                name: node_text(name, contents).to_string(),
                docs: find_jsdoc_block(contents, node.start_position().row + 1)
                    .map(|block| block.text)
                    .unwrap_or_default(),
                line: node.start_position().row + 1,
            });
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_typescript_symbols(child, contents, symbols);
        }
    }
}

fn node_text<'a>(node: tree_sitter::Node<'_>, contents: &'a str) -> &'a str {
    &contents[node.byte_range()]
}

fn fix_rust_symbol_docs(
    contents: &str,
    symbol: &str,
    required: &[String],
) -> Result<Option<String>> {
    let Some(existing) = inspect_rust_symbol(contents, symbol) else {
        return Ok(None);
    };
    let missing = missing_doc_snippets(&existing, required);
    if missing.is_empty() {
        return Ok(None);
    }

    let Some(line) = find_rust_declaration_line(contents, symbol) else {
        return Ok(None);
    };

    let mut lines = to_lines(contents);
    let indent = line_indentation(&lines[line - 1]);
    let inserts = missing
        .into_iter()
        .map(|snippet| format!("{indent}/// {snippet}"))
        .collect::<Vec<_>>();
    lines.splice(line - 1..line - 1, inserts);

    Ok(Some(join_lines(lines, contents.ends_with('\n'))))
}

fn fix_python_symbol_docs(
    config: &SyuConfig,
    path: &Path,
    contents: &str,
    symbol: &str,
    required: &[String],
) -> Result<Option<String>> {
    let Some(existing) = inspect_python_symbol(config, path, symbol)? else {
        return Ok(None);
    };
    let missing = missing_doc_snippets(&existing.docs, required);
    if missing.is_empty() {
        return Ok(None);
    }

    let merged = merged_doc_lines(&existing.docs, &missing);
    let doc_lines = render_python_docstring(&existing.body_indent, &merged);
    let mut lines = to_lines(contents);

    if let (Some(start), Some(end)) = (existing.doc_start_line, existing.doc_end_line) {
        lines.splice(start - 1..end, doc_lines);
    } else {
        lines.splice(existing.body_line - 1..existing.body_line - 1, doc_lines);
    }

    Ok(Some(join_lines(lines, contents.ends_with('\n'))))
}

fn fix_typescript_symbol_docs(
    path: &Path,
    contents: &str,
    symbol: &str,
    required: &[String],
) -> Result<Option<String>> {
    let Some(existing) = inspect_typescript_symbol(path, contents, symbol)? else {
        return Ok(None);
    };
    let missing = missing_doc_snippets(&existing.docs, required);
    if missing.is_empty() {
        return Ok(None);
    }

    let mut lines = to_lines(contents);
    let indent = line_indentation(&lines[existing.line - 1]);

    if let Some(block) = find_jsdoc_block(contents, existing.line) {
        let inserts = missing
            .into_iter()
            .map(|snippet| format!("{indent} * {snippet}"))
            .collect::<Vec<_>>();
        lines.splice(block.end_line - 1..block.end_line - 1, inserts);
    } else {
        let mut block = vec![format!("{indent}/**")];
        block.extend(
            missing
                .into_iter()
                .map(|snippet| format!("{indent} * {snippet}")),
        );
        block.push(format!("{indent} */"));
        lines.splice(existing.line - 1..existing.line - 1, block);
    }

    Ok(Some(join_lines(lines, contents.ends_with('\n'))))
}

fn find_rust_declaration_line(contents: &str, symbol: &str) -> Option<usize> {
    let escaped = regex::escape(symbol);
    let patterns = [
        format!(r"\b(?:pub(?:\([^)]*\))?\s+)?fn\s+{escaped}\b"),
        format!(r"\b(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait)\s+{escaped}\b"),
        format!(r"\b(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+{escaped}\b"),
    ];
    find_line_by_patterns(contents, &patterns)
}

fn find_line_by_patterns(contents: &str, patterns: &[String]) -> Option<usize> {
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern).ok().and_then(|regex| {
            contents
                .lines()
                .enumerate()
                .find_map(|(index, line)| regex.is_match(line).then_some(index + 1))
        })
    })
}

fn find_jsdoc_block(contents: &str, declaration_line: usize) -> Option<JsDocBlock> {
    if declaration_line <= 1 {
        return None;
    }

    let lines = contents.lines().collect::<Vec<_>>();
    let end = declaration_line - 2;
    if end >= lines.len() {
        return None;
    }

    if lines[end].trim().is_empty() {
        return None;
    }

    if !lines[end].trim_end().ends_with("*/") {
        return None;
    }

    let mut start = end;
    while start > 0 && !lines[start].trim_start().starts_with("/**") {
        start -= 1;
    }

    if !lines[start].trim_start().starts_with("/**") {
        return None;
    }

    let text = lines[start..=end]
        .iter()
        .map(|line| {
            line.trim()
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .trim_start_matches('*')
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Some(JsDocBlock {
        end_line: end + 1,
        text,
    })
}

fn missing_doc_snippets(existing_docs: &str, required: &[String]) -> Vec<String> {
    required
        .iter()
        .filter(|snippet| !existing_docs.contains(snippet.as_str()))
        .cloned()
        .collect()
}

fn normalized_required_snippets(snippets: &[String]) -> Vec<String> {
    snippets
        .iter()
        .map(|snippet| snippet.trim())
        .filter(|snippet| !snippet.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merged_doc_lines(existing_docs: &str, missing: &[String]) -> Vec<String> {
    let mut lines = existing_docs
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    lines.extend(missing.iter().cloned());
    lines
}

fn render_python_docstring(indent: &str, lines: &[String]) -> Vec<String> {
    if lines.len() == 1 {
        return vec![format!("{indent}\"\"\"{}\"\"\"", lines[0])];
    }

    let mut rendered = vec![format!("{indent}\"\"\"")];
    rendered.extend(lines.iter().map(|line| format!("{indent}{line}")));
    rendered.push(format!("{indent}\"\"\""));
    rendered
}

fn to_lines(contents: &str) -> Vec<String> {
    contents.lines().map(ToOwned::to_owned).collect()
}

fn join_lines(lines: Vec<String>, had_trailing_newline: bool) -> String {
    let mut rendered = lines.join("\n");
    if had_trailing_newline {
        rendered.push('\n');
    }
    rendered
}

fn line_indentation(line: &str) -> String {
    line.chars()
        .take_while(|character| character.is_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

    use crate::config::SyuConfig;

    use super::{
        apply_symbol_doc_fix, find_jsdoc_block, find_rust_declaration_line,
        inspect_python_file_with_runtime, inspect_rust_symbol, inspect_symbol, merged_doc_lines,
        render_python_docstring, supports_rich_inspection,
    };

    #[test]
    fn rich_inspection_supports_primary_languages() {
        assert!(supports_rich_inspection("rust"));
        assert!(supports_rich_inspection("python"));
        assert!(supports_rich_inspection("typescript"));
        assert!(!supports_rich_inspection("shell"));
    }

    #[test]
    fn rust_inspection_reads_doc_comments() {
        let source = "/// REQ-1\n/// stable docs\npub fn example() {}\n";
        let inspected = inspect_symbol(
            "rust",
            &SyuConfig::default(),
            std::path::Path::new("src/lib.rs"),
            source,
            "example",
        )
        .expect("inspection should succeed")
        .expect("symbol should exist");

        assert!(inspected.docs.contains("REQ-1"));
        assert_eq!(inspected.line, 3);
        assert_eq!(find_rust_declaration_line(source, "example"), Some(3));
    }

    #[test]
    fn inspection_returns_none_for_unsupported_languages() {
        assert_eq!(
            inspect_symbol(
                "markdown",
                &SyuConfig::default(),
                std::path::Path::new("README.md"),
                "# heading\n",
                "heading"
            )
            .expect("inspection should not fail"),
            None
        );
    }

    #[test]
    fn rust_inspection_covers_additional_item_kinds() {
        let source = r#"
use std::fmt;

#[doc(hidden)]
pub fn hidden() {}

#[doc = 123]
pub fn integer_doc() {}

#[doc = include_str!("README.md")]
pub fn macro_doc() {}

pub enum Choice {
    One,
}

pub trait Describable {
    fn describe(&self);
}

pub const LIMIT: usize = 1;
pub static DEFAULT_NAME: &str = "syu";

pub struct Container;

impl Container {
    /// method docs
    pub fn perform(&self) {}
}

pub mod nested {
    /// nested docs
    pub fn helper() {}
}

mod external;
"#;

        assert_eq!(inspect_rust_symbol(source, "hidden"), Some(String::new()));
        assert_eq!(
            inspect_rust_symbol(source, "integer_doc"),
            Some(String::new())
        );
        assert_eq!(
            inspect_rust_symbol(source, "macro_doc"),
            Some(String::new())
        );
        assert_eq!(inspect_rust_symbol(source, "Choice"), Some(String::new()));
        assert_eq!(
            inspect_rust_symbol(source, "Describable"),
            Some(String::new())
        );
        assert_eq!(inspect_rust_symbol(source, "LIMIT"), Some(String::new()));
        assert_eq!(
            inspect_rust_symbol(source, "DEFAULT_NAME"),
            Some(String::new())
        );
        assert!(
            inspect_rust_symbol(source, "perform")
                .expect("method should exist")
                .contains("method docs")
        );
        assert!(
            inspect_rust_symbol(source, "helper")
                .expect("nested function should exist")
                .contains("nested docs")
        );
    }

    #[test]
    fn rust_fix_inserts_missing_doc_lines() {
        let source = "pub fn example() {}\n";
        let updated = apply_symbol_doc_fix(
            "rust",
            &SyuConfig::default(),
            std::path::Path::new("src/lib.rs"),
            source,
            "example",
            &["REQ-1".to_string(), "Explain example".to_string()],
        )
        .expect("fix should succeed")
        .expect("fix should update source");

        assert!(updated.contains("/// REQ-1"));
        assert!(updated.contains("/// Explain example"));
    }

    #[test]
    fn rust_fix_returns_none_when_not_needed_or_not_fixable() {
        let already_documented = "/// REQ-1\npub fn example() {}\n";
        assert_eq!(
            apply_symbol_doc_fix(
                "rust",
                &SyuConfig::default(),
                std::path::Path::new("src/lib.rs"),
                already_documented,
                "example",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "rust",
                &SyuConfig::default(),
                std::path::Path::new("src/lib.rs"),
                "pub fn example() {}\n",
                "missing",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "rust",
                &SyuConfig::default(),
                std::path::Path::new("src/lib.rs"),
                "/// docs\npub fn\nexample() {}\n",
                "example",
                &["REQ-2".to_string()],
            )
            .expect("fix should succeed"),
            None
        );
    }

    #[test]
    fn python_inspection_reads_docstrings() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("module.py");
        fs::write(
            &path,
            "def sample():\n    \"\"\"REQ-1\n    stable docs\"\"\"\n    return 1\n",
        )
        .expect("python file");

        let inspected = inspect_symbol("python", &SyuConfig::default(), &path, "", "sample")
            .expect("inspection should succeed")
            .expect("symbol should exist");
        assert!(inspected.docs.contains("REQ-1"));
        assert_eq!(inspected.line, 1);
    }

    #[test]
    fn python_inspection_reports_missing_runtime_and_runtime_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("module.py");
        fs::write(&path, "def sample():\n    return 1\n").expect("python file");

        let missing_runtime = inspect_python_file_with_runtime(None, &path)
            .expect_err("missing runtime should fail")
            .to_string();
        assert!(missing_runtime.contains("no Python runtime could be auto-detected"));

        let failed_runtime = inspect_python_file_with_runtime(Some("false"), &path)
            .expect_err("failing runtime should fail")
            .to_string();
        assert!(failed_runtime.contains("Python inspector failed"));
    }

    #[test]
    fn python_inspection_reports_spawn_and_decode_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("module.py");
        fs::write(&path, "def sample():\n    return 1\n").expect("python file");

        let spawn_error =
            inspect_python_file_with_runtime(Some("definitely-missing-runtime"), &path)
                .expect_err("missing runtime binary should fail")
                .to_string();
        assert!(spawn_error.contains("failed to run Python inspector"));

        let fake_runtime = tempdir.path().join("fake-python");
        fs::write(&fake_runtime, "#!/bin/sh\necho not-json\n").expect("fake runtime should exist");
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&fake_runtime).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&fake_runtime, permissions).expect("permissions should update");
        }

        let decode_error = inspect_python_file_with_runtime(fake_runtime.to_str(), &path)
            .expect_err("invalid json should fail")
            .to_string();
        assert!(decode_error.contains("failed to decode Python inspection output"));
    }

    #[test]
    fn python_and_typescript_inspection_return_none_for_missing_symbols() {
        let tempdir = tempdir().expect("tempdir should exist");
        let python_path = tempdir.path().join("module.py");
        fs::write(&python_path, "def sample():\n    return 1\n").expect("python file");
        assert_eq!(
            inspect_symbol("python", &SyuConfig::default(), &python_path, "", "missing")
                .expect("inspection should succeed"),
            None
        );

        let ts_source = "export function sample() {\n  return 1;\n}\n";
        assert_eq!(
            inspect_symbol(
                "typescript",
                &SyuConfig::default(),
                std::path::Path::new("src/sample.ts"),
                ts_source,
                "missing",
            )
            .expect("inspection should succeed"),
            None
        );
    }

    #[test]
    fn python_fix_inserts_docstring_when_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("module.py");
        fs::write(&path, "def sample():\n    return 1\n").expect("python file");
        let source = fs::read_to_string(&path).expect("source");

        let updated = apply_symbol_doc_fix(
            "python",
            &SyuConfig::default(),
            &path,
            &source,
            "sample",
            &["REQ-1".to_string(), "Explain sample".to_string()],
        )
        .expect("fix should succeed")
        .expect("fix should update source");

        assert!(updated.contains("\"\"\""));
        assert!(updated.contains("REQ-1"));
        assert!(updated.contains("Explain sample"));
    }

    #[test]
    fn python_fix_updates_existing_docstrings_and_handles_noops() {
        let tempdir = tempdir().expect("tempdir should exist");
        let path = tempdir.path().join("module.py");
        let source = "def sample():\n    \"\"\"REQ-1\"\"\"\n    return 1\n";
        fs::write(&path, source).expect("python file");

        let updated = apply_symbol_doc_fix(
            "python",
            &SyuConfig::default(),
            &path,
            source,
            "sample",
            &["REQ-1".to_string(), "Explain sample".to_string()],
        )
        .expect("fix should succeed")
        .expect("fix should update source");
        assert!(updated.contains("Explain sample"));

        assert_eq!(
            apply_symbol_doc_fix(
                "python",
                &SyuConfig::default(),
                &path,
                source,
                "missing",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "python",
                &SyuConfig::default(),
                &path,
                source,
                "sample",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );
    }

    #[test]
    fn typescript_inspection_reads_jsdoc() {
        let source = "/**\n * REQ-1\n */\nexport function sample() {\n  return 1;\n}\n";
        let inspected = inspect_symbol(
            "typescript",
            &SyuConfig::default(),
            std::path::Path::new("src/sample.ts"),
            source,
            "sample",
        )
        .expect("inspection should succeed")
        .expect("symbol should exist");
        assert!(inspected.docs.contains("REQ-1"));
        assert_eq!(inspected.line, 4);
    }

    #[test]
    fn typescript_inspection_reads_variable_declarations() {
        let source = "/**\n * REQ-1\n */\nexport const sample = () => true;\n";
        let inspected = inspect_symbol(
            "typescript",
            &SyuConfig::default(),
            std::path::Path::new("src/sample.ts"),
            source,
            "sample",
        )
        .expect("inspection should succeed")
        .expect("symbol should exist");

        assert!(inspected.docs.contains("REQ-1"));
        assert_eq!(inspected.line, 4);
    }

    #[test]
    fn typescript_inspection_supports_tsx_files() {
        let source = "/**\n * REQ-1\n */\nexport function sample() {\n  return <div />;\n}\n";
        let inspected = inspect_symbol(
            "typescript",
            &SyuConfig::default(),
            std::path::Path::new("src/sample.tsx"),
            source,
            "sample",
        )
        .expect("inspection should succeed")
        .expect("symbol should exist");

        assert!(inspected.docs.contains("REQ-1"));
    }

    #[test]
    fn typescript_fix_inserts_jsdoc_when_missing() {
        let source = "export function sample() {\n  return 1;\n}\n";
        let updated = apply_symbol_doc_fix(
            "typescript",
            &SyuConfig::default(),
            std::path::Path::new("src/sample.ts"),
            source,
            "sample",
            &["REQ-1".to_string()],
        )
        .expect("fix should succeed")
        .expect("fix should update source");

        assert!(updated.contains("/**"));
        assert!(updated.contains(" * REQ-1"));
    }

    #[test]
    fn typescript_fix_updates_existing_jsdoc_and_handles_noops() {
        let source = "/**\n * REQ-1\n */\nexport function sample() {\n  return 1;\n}\n";

        let updated = apply_symbol_doc_fix(
            "typescript",
            &SyuConfig::default(),
            std::path::Path::new("src/sample.ts"),
            source,
            "sample",
            &["REQ-1".to_string(), "Explain sample".to_string()],
        )
        .expect("fix should succeed")
        .expect("fix should update source");
        assert!(updated.contains(" * Explain sample"));

        assert_eq!(
            apply_symbol_doc_fix(
                "typescript",
                &SyuConfig::default(),
                std::path::Path::new("src/sample.ts"),
                source,
                "missing",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "typescript",
                &SyuConfig::default(),
                std::path::Path::new("src/sample.ts"),
                source,
                "sample",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );
    }

    #[test]
    fn jsdoc_detection_requires_adjacent_block() {
        let source = "/**\n * REQ-1\n */\nconst gap = true;\nfunction sample() {}\n";
        assert!(find_jsdoc_block(source, 5).is_none());
    }

    #[test]
    fn jsdoc_detection_rejects_invalid_shapes() {
        let out_of_bounds = "function sample() {}\n";
        assert!(find_jsdoc_block(out_of_bounds, 3).is_none());

        let with_blank_line = "/**\n * REQ-1\n */\n\nfunction sample() {}\n";
        assert!(find_jsdoc_block(with_blank_line, 5).is_none());

        let not_jsdoc = "/* not jsdoc */\nfunction sample() {}\n";
        assert!(find_jsdoc_block(not_jsdoc, 2).is_none());
    }

    #[test]
    fn helper_functions_merge_doc_lines() {
        let lines = merged_doc_lines("First line", &["Second line".to_string()]);
        assert_eq!(
            lines,
            vec!["First line".to_string(), "Second line".to_string()]
        );

        let rendered = render_python_docstring("    ", &lines);
        assert!(rendered.iter().any(|line| line.contains("Second line")));
    }

    #[test]
    fn helper_functions_cover_single_line_and_noop_paths() {
        assert_eq!(
            render_python_docstring("    ", &["Only line".to_string()]),
            vec!["    \"\"\"Only line\"\"\"".to_string()]
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "markdown",
                &SyuConfig::default(),
                std::path::Path::new("README.md"),
                "# title\n",
                "sample",
                &["REQ-1".to_string()],
            )
            .expect("fix should succeed"),
            None
        );

        assert_eq!(
            apply_symbol_doc_fix(
                "rust",
                &SyuConfig::default(),
                std::path::Path::new("src/lib.rs"),
                "pub fn example() {}\n",
                "example",
                &[],
            )
            .expect("fix should succeed"),
            None
        );
    }
}
