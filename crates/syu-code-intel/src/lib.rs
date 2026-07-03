#![forbid(unsafe_code)]
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolResolution {
    pub identity: String,
    pub kind: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub excerpt: String,
    pub excerpt_hash: String,
}

pub fn resolve_symbol(adapter: &str, source: &str, name: &str) -> Result<SymbolResolution> {
    match adapter {
        "rust" => resolve_rust(source, name),
        "typescript" => resolve_text_definition(
            source,
            name,
            &[
                "function ",
                "class ",
                "const ",
                "let ",
                "interface ",
                "type ",
            ],
        ),
        "shell" => resolve_text_definition(source, name, &["function ", "()"]),
        "python" => resolve_text_definition(source, name, &["def ", "class "]),
        "go" => resolve_text_definition(source, name, &["func ", "type "]),
        _ => bail!("adapter {adapter} does not support symbol selectors"),
    }
}

fn resolve_rust(source: &str, name: &str) -> Result<SymbolResolution> {
    let file = syn::parse_file(source)?;
    struct Finder<'a> {
        name: &'a str,
        found: Option<(&'static str, String)>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Finder<'_> {
        fn visit_item_fn(&mut self, value: &'ast syn::ItemFn) {
            if value.sig.ident == self.name {
                self.found = Some(("function", format!("fn {}", value.sig.ident)));
            }
            syn::visit::visit_item_fn(self, value);
        }
        fn visit_item_struct(&mut self, value: &'ast syn::ItemStruct) {
            if value.ident == self.name {
                self.found = Some(("struct", format!("struct {}", value.ident)));
            }
        }
        fn visit_item_enum(&mut self, value: &'ast syn::ItemEnum) {
            if value.ident == self.name {
                self.found = Some(("enum", format!("enum {}", value.ident)));
            }
        }
        fn visit_item_trait(&mut self, value: &'ast syn::ItemTrait) {
            if value.ident == self.name {
                self.found = Some(("trait", format!("trait {}", value.ident)));
            }
        }
    }
    let mut finder = Finder { name, found: None };
    syn::visit::Visit::visit_file(&mut finder, &file);
    let needle = finder
        .found
        .ok_or_else(|| anyhow::anyhow!("symbol {name} has no Rust definition"))?;
    resolution_from_needle(source, name, needle.0, &needle.1)
}

fn resolve_text_definition(
    source: &str,
    name: &str,
    prefixes: &[&str],
) -> Result<SymbolResolution> {
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        for prefix in prefixes {
            let matches = if *prefix == "()" {
                trimmed.starts_with(&format!("{name}()"))
            } else {
                trimmed.contains(&format!("{prefix}{name}"))
            };
            if matches {
                let start = source.lines().take(line_index).map(|l| l.len() + 1).sum();
                let end = start + line.len();
                return Ok(build(
                    name,
                    "definition",
                    source,
                    start,
                    end,
                    line_index + 1,
                    line_index + 1,
                ));
            }
        }
    }
    bail!(
        "symbol {name} has no {adapter} definition",
        adapter = "language-aware"
    )
}
fn resolution_from_needle(
    source: &str,
    name: &str,
    kind: &str,
    needle: &str,
) -> Result<SymbolResolution> {
    let start = source
        .find(needle)
        .ok_or_else(|| anyhow::anyhow!("definition range for {name} not found"))?;
    let line_start = source[..start].lines().count().max(1);
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset);
    Ok(build(
        name, kind, source, start, end, line_start, line_start,
    ))
}
fn build(
    name: &str,
    kind: &str,
    source: &str,
    start: usize,
    end: usize,
    line_start: usize,
    line_end: usize,
) -> SymbolResolution {
    let excerpt = source[start..end].to_string();
    let mut hash = Sha256::new();
    hash.update(excerpt.as_bytes());
    SymbolResolution {
        identity: name.into(),
        kind: kind.into(),
        byte_start: start,
        byte_end: end,
        line_start,
        line_end,
        excerpt,
        excerpt_hash: format!("sha256:{:x}", hash.finalize()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_and_calls_are_not_definitions() {
        assert!(
            resolve_symbol(
                "rust",
                "// fn hidden() {}\nfn caller(){ hidden(); }",
                "hidden"
            )
            .is_err()
        );
    }
    #[test]
    fn rust_definition_has_exact_range() {
        let r = resolve_symbol("rust", "fn exact() {}\n", "exact").unwrap();
        assert_eq!(r.excerpt, "fn exact() {}");
    }
}
