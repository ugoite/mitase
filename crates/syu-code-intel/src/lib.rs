#![forbid(unsafe_code)]
use anyhow::{Result, bail};
use proc_macro2::{LineColumn, Span};
use sha2::{Digest, Sha256};
use syn::spanned::Spanned;

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

#[derive(Clone)]
struct Candidate {
    kind: &'static str,
    identity: String,
    span: Span,
}

#[derive(Clone, Copy)]
enum BraceScanMode {
    CLike,
    Shell,
}

pub fn resolve_symbol(adapter: &str, source: &str, name: &str) -> Result<SymbolResolution> {
    match adapter {
        "rust" => resolve_rust(source, name),
        "typescript" => resolve_typescript(source, name),
        "shell" => resolve_shell(source, name),
        "python" => resolve_python(source, name),
        "go" => resolve_go(source, name),
        _ => bail!("adapter {adapter} does not support symbol selectors"),
    }
}

fn resolve_typescript(source: &str, name: &str) -> Result<SymbolResolution> {
    for prefix in ["function ", "class "] {
        if let Ok(found) = resolve_keyword_definition(source, name, prefix) {
            return Ok(found);
        }
    }
    resolve_assignment_block(source, name, &["const ", "let ", "var "])
}

fn resolve_rust(source: &str, name: &str) -> Result<SymbolResolution> {
    let file = syn::parse_file(source)?;

    struct Finder<'a> {
        name: &'a str,
        candidates: Vec<Candidate>,
        module_path: Vec<String>,
        impl_path: Vec<String>,
    }

    impl Finder<'_> {
        fn scoped_identity(&self, leaf: &str) -> String {
            self.module_path
                .iter()
                .chain(self.impl_path.iter())
                .chain(std::iter::once(&leaf.to_string()))
                .cloned()
                .collect::<Vec<_>>()
                .join("::")
        }
    }

    impl<'ast> syn::visit::Visit<'ast> for Finder<'_> {
        fn visit_item_mod(&mut self, value: &'ast syn::ItemMod) {
            self.module_path.push(value.ident.to_string());
            syn::visit::visit_item_mod(self, value);
            self.module_path.pop();
        }

        fn visit_item_impl(&mut self, value: &'ast syn::ItemImpl) {
            if let syn::Type::Path(path) = &*value.self_ty
                && let Some(segment) = path.path.segments.last()
            {
                self.impl_path.push(segment.ident.to_string());
                syn::visit::visit_item_impl(self, value);
                self.impl_path.pop();
                return;
            }
            syn::visit::visit_item_impl(self, value);
        }

        fn visit_item_fn(&mut self, value: &'ast syn::ItemFn) {
            if value.sig.ident == self.name {
                self.candidates.push(Candidate {
                    kind: "function",
                    identity: self.scoped_identity(&value.sig.ident.to_string()),
                    span: value.span(),
                });
            }
        }

        fn visit_impl_item_fn(&mut self, value: &'ast syn::ImplItemFn) {
            if value.sig.ident == self.name {
                self.candidates.push(Candidate {
                    kind: "method",
                    identity: self.scoped_identity(&value.sig.ident.to_string()),
                    span: value.span(),
                });
            }
        }

        fn visit_item_struct(&mut self, value: &'ast syn::ItemStruct) {
            if value.ident == self.name {
                self.candidates.push(Candidate {
                    kind: "struct",
                    identity: self.scoped_identity(&value.ident.to_string()),
                    span: value.span(),
                });
            }
        }

        fn visit_item_enum(&mut self, value: &'ast syn::ItemEnum) {
            if value.ident == self.name {
                self.candidates.push(Candidate {
                    kind: "enum",
                    identity: self.scoped_identity(&value.ident.to_string()),
                    span: value.span(),
                });
            }
        }

        fn visit_item_trait(&mut self, value: &'ast syn::ItemTrait) {
            if value.ident == self.name {
                self.candidates.push(Candidate {
                    kind: "trait",
                    identity: self.scoped_identity(&value.ident.to_string()),
                    span: value.span(),
                });
            }
        }
    }

    let mut finder = Finder {
        name,
        candidates: Vec::new(),
        module_path: Vec::new(),
        impl_path: Vec::new(),
    };
    syn::visit::Visit::visit_file(&mut finder, &file);

    match finder.candidates.as_slice() {
        [] => bail!("symbol {name} has no Rust definition"),
        [candidate] => resolution_from_span(source, candidate),
        _ => bail!("symbol {name} is ambiguous in Rust source"),
    }
}

fn resolution_from_span(source: &str, candidate: &impl CandidateView) -> Result<SymbolResolution> {
    let start = line_column_to_byte(source, candidate.start())?;
    let end = line_column_to_byte(source, candidate.end())?;
    Ok(build(
        &candidate.identity(),
        candidate.kind(),
        source,
        start,
        end,
        candidate.start().line,
        candidate.end().line,
    ))
}

trait CandidateView {
    fn identity(&self) -> String;
    fn kind(&self) -> &str;
    fn start(&self) -> LineColumn;
    fn end(&self) -> LineColumn;
}

impl CandidateView for Candidate {
    fn identity(&self) -> String {
        self.identity.clone()
    }

    fn kind(&self) -> &str {
        self.kind
    }

    fn start(&self) -> LineColumn {
        self.span.start()
    }

    fn end(&self) -> LineColumn {
        self.span.end()
    }
}

fn line_column_to_byte(source: &str, location: LineColumn) -> Result<usize> {
    let mut byte = 0usize;
    let mut line = 1usize;
    for segment in source.split_inclusive('\n') {
        if line == location.line {
            let prefix = segment
                .chars()
                .take(location.column)
                .map(char::len_utf8)
                .sum::<usize>();
            return Ok(byte + prefix);
        }
        byte += segment.len();
        line += 1;
    }
    if location.line == line {
        return Ok(source.len());
    }
    bail!("definition range is outside source")
}

fn resolve_keyword_definition(source: &str, name: &str, prefix: &str) -> Result<SymbolResolution> {
    let mut matches = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_comment_line(trimmed) {
            continue;
        }
        let Some(keyword_index) = trimmed.find(prefix) else {
            continue;
        };
        if keyword_index > 0 && !trimmed[..keyword_index].ends_with("export ") {
            continue;
        }
        let remainder = &trimmed[keyword_index + prefix.len()..];
        if !starts_with_symbol(remainder, name) {
            continue;
        }
        matches.push((line_index, trimmed.to_string()));
    }
    match matches.as_slice() {
        [] => bail!("symbol {name} has no definition"),
        [_single] => {}
        _ => bail!("symbol {name} is ambiguous in source"),
    }
    let (line_index, line) = matches.into_iter().next().expect("one match");
    let start = line_start_byte(source, line_index);
    let (end, line_end) = if line.contains('{') {
        block_from_brace(source, start, BraceScanMode::CLike)?
    } else {
        (start + line.len(), line_index + 1)
    };
    Ok(build(
        name,
        "definition",
        source,
        start,
        end,
        line_index + 1,
        line_end,
    ))
}

fn resolve_assignment_block(
    source: &str,
    name: &str,
    prefixes: &[&str],
) -> Result<SymbolResolution> {
    let mut matches = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_comment_line(trimmed) {
            continue;
        }
        for prefix in prefixes {
            if !trimmed.starts_with(prefix) {
                continue;
            }
            let remainder = &trimmed[prefix.len()..];
            if !starts_with_symbol(remainder, name) {
                continue;
            }
            matches.push((line_index, trimmed.to_string()));
        }
    }
    match matches.as_slice() {
        [] => bail!("symbol {name} has no definition"),
        [_single] => {}
        _ => bail!("symbol {name} is ambiguous in source"),
    }
    let (line_index, line) = matches.into_iter().next().expect("one match");
    let start = line_start_byte(source, line_index);
    let (end, line_end) = if line.contains('{') {
        block_from_brace(source, start, BraceScanMode::CLike)?
    } else {
        (start + line.len(), line_index + 1)
    };
    Ok(build(
        name,
        "definition",
        source,
        start,
        end,
        line_index + 1,
        line_end,
    ))
}

fn resolve_go(source: &str, name: &str) -> Result<SymbolResolution> {
    let mut matches = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_comment_line(trimmed) {
            continue;
        }
        if let Some(remainder) = trimmed.strip_prefix("func ") {
            let candidate = remainder.trim_start();
            let matches_name = starts_with_symbol(candidate, name)
                || candidate.find(')').is_some_and(|index| {
                    starts_with_symbol(candidate[index + 1..].trim_start(), name)
                });
            if matches_name {
                matches.push((line_index, trimmed.to_string()));
            }
        }
    }
    match matches.as_slice() {
        [] => bail!("symbol {name} has no definition"),
        [_single] => {}
        _ => bail!("symbol {name} is ambiguous in source"),
    }
    let (line_index, _line) = matches.into_iter().next().expect("one match");
    let start = line_start_byte(source, line_index);
    let (end, line_end) = block_from_brace(source, start, BraceScanMode::CLike)?;
    Ok(build(
        name,
        "definition",
        source,
        start,
        end,
        line_index + 1,
        line_end,
    ))
}

fn resolve_shell(source: &str, name: &str) -> Result<SymbolResolution> {
    let mut matches = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_comment_line(trimmed) {
            continue;
        }
        let plain = starts_with_symbol(trimmed, name)
            && trimmed[name.len()..].trim_start().starts_with("()");
        let function_form = trimmed
            .strip_prefix("function ")
            .is_some_and(|rest| starts_with_symbol(rest.trim_start(), name));
        if plain || function_form {
            matches.push((line_index, trimmed.to_string()));
        }
    }
    match matches.as_slice() {
        [] => bail!("symbol {name} has no definition"),
        [_single] => {}
        _ => bail!("symbol {name} is ambiguous in source"),
    }
    let (line_index, _line) = matches.into_iter().next().expect("one match");
    let start = line_start_byte(source, line_index);
    let (end, line_end) = block_from_brace(source, start, BraceScanMode::Shell)?;
    Ok(build(
        name,
        "definition",
        source,
        start,
        end,
        line_index + 1,
        line_end,
    ))
}

fn resolve_python(source: &str, name: &str) -> Result<SymbolResolution> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut matches = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if is_comment_line(trimmed) {
            continue;
        }
        let is_match = trimmed
            .strip_prefix("def ")
            .is_some_and(|rest| starts_with_symbol(rest, name))
            || trimmed
                .strip_prefix("class ")
                .is_some_and(|rest| starts_with_symbol(rest, name));
        if is_match {
            matches.push(line_index);
        }
    }
    match matches.as_slice() {
        [] => bail!("symbol {name} has no definition"),
        [_single] => {}
        _ => bail!("symbol {name} is ambiguous in source"),
    }
    let mut start_line = matches[0];
    while start_line > 0 && lines[start_line - 1].trim_start().starts_with('@') {
        start_line -= 1;
    }
    let start_indent = indentation(lines[matches[0]]);
    let mut end_line = lines.len();
    for (line_index, line) in lines.iter().enumerate().skip(matches[0] + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indentation(line) <= start_indent {
            end_line = line_index;
            break;
        }
    }
    let start = line_start_byte(source, start_line);
    let end = if end_line >= lines.len() {
        source.len()
    } else {
        line_start_byte(source, end_line)
    };
    Ok(build(
        name,
        "definition",
        source,
        start,
        end,
        start_line + 1,
        end_line.max(start_line + 1),
    ))
}

fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//") || trimmed.starts_with('#')
}

fn starts_with_symbol(input: &str, name: &str) -> bool {
    input.strip_prefix(name).is_some_and(|rest| {
        rest.is_empty() || (!rest.starts_with(char::is_alphanumeric) && !rest.starts_with('_'))
    })
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn line_start_byte(source: &str, line_index: usize) -> usize {
    source
        .lines()
        .take(line_index)
        .map(|line| line.len() + 1)
        .sum()
}

fn block_from_brace(source: &str, start: usize, mode: BraceScanMode) -> Result<(usize, usize)> {
    let mut depth = 0usize;
    let mut seen_open = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let mut heredoc: Option<(String, bool)> = None;
    let mut line_end = source[..start].lines().count().max(1);
    let chars = source[start..].char_indices().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let (offset, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, value)| *value);
        if ch == '\n' {
            line_end += 1;
            line_comment = false;
            if let Some((marker, strip_tabs)) = heredoc.as_ref() {
                let line_start = offset + ch.len_utf8();
                let remainder = &source[start + line_start..];
                let line_end_offset = remainder
                    .find('\n')
                    .map(|next_newline| line_start + next_newline)
                    .unwrap_or(source[start..].len());
                let next_line = &source[start + line_start..start + line_end_offset];
                let candidate = if *strip_tabs {
                    next_line.trim_end().trim_start_matches('\t')
                } else {
                    next_line.trim_end()
                };
                if candidate == marker {
                    heredoc = None;
                }
            }
            index += 1;
            continue;
        }
        if heredoc.is_some() {
            index += 1;
            continue;
        }
        if escape {
            escape = false;
            index += 1;
            continue;
        }
        if line_comment {
            index += 1;
            continue;
        }
        if block_comment {
            if ch == '*' && next == Some('/') {
                block_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        match ch {
            '\\' if in_single || in_double || in_backtick => {
                escape = true;
            }
            '\'' if !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_single && !in_double && matches!(mode, BraceScanMode::CLike) => {
                in_backtick = !in_backtick;
            }
            '/' if !in_single
                && !in_double
                && !in_backtick
                && matches!(mode, BraceScanMode::CLike) =>
            {
                if next == Some('/') {
                    line_comment = true;
                    index += 2;
                    continue;
                }
                if next == Some('*') {
                    block_comment = true;
                    index += 2;
                    continue;
                }
            }
            '#' if !in_single
                && !in_double
                && !in_backtick
                && matches!(mode, BraceScanMode::Shell) =>
            {
                line_comment = true;
                index += 1;
                continue;
            }
            '<' if !in_single
                && !in_double
                && !in_backtick
                && matches!(mode, BraceScanMode::Shell) =>
            {
                if next == Some('<') {
                    let mut marker_start = index + 2;
                    let mut strip_tabs = false;
                    if chars.get(marker_start).map(|(_, value)| *value) == Some('-') {
                        strip_tabs = true;
                        marker_start += 1;
                    }
                    while chars
                        .get(marker_start)
                        .map(|(_, value)| value.is_whitespace() && *value != '\n')
                        .unwrap_or(false)
                    {
                        marker_start += 1;
                    }
                    let mut marker_end = marker_start;
                    while chars
                        .get(marker_end)
                        .map(|(_, value)| !value.is_whitespace())
                        .unwrap_or(false)
                    {
                        marker_end += 1;
                    }
                    if marker_end > marker_start {
                        let start_offset = chars[marker_start].0;
                        let end_offset = chars
                            .get(marker_end)
                            .map(|(value, _)| *value)
                            .unwrap_or(source[start..].len());
                        let mut marker =
                            source[start + start_offset..start + end_offset].to_string();
                        if (marker.starts_with('\'') && marker.ends_with('\''))
                            || (marker.starts_with('"') && marker.ends_with('"'))
                        {
                            marker = marker[1..marker.len() - 1].to_string();
                        }
                        heredoc = Some((marker, strip_tabs));
                    }
                }
            }
            '{' if !in_single && !in_double && !in_backtick => {
                depth += 1;
                seen_open = true;
            }
            '}' if !in_single && !in_double && !in_backtick && depth > 0 => {
                depth -= 1;
                if seen_open && depth == 0 {
                    return Ok((start + offset + ch.len_utf8(), line_end));
                }
            }
            _ => {}
        }
        index += 1;
    }
    if seen_open {
        return Ok((source.len(), source.lines().count().max(1)));
    }
    bail!("definition block not found")
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
    fn rust_definition_covers_function_body() {
        let r = resolve_symbol("rust", "fn exact() {\n    let x = 1;\n}\n", "exact").unwrap();
        assert!(r.excerpt.contains("let x = 1;"));
        assert_eq!(r.line_start, 1);
        assert_eq!(r.line_end, 3);
    }

    #[test]
    fn rust_ambiguous_symbol_errors() {
        let error = resolve_symbol(
            "rust",
            "fn exact() {}\nmod nested { pub fn exact() {} }\n",
            "exact",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ambiguous"));
    }

    #[test]
    fn typescript_definition_covers_function_body() {
        let r = resolve_symbol(
            "typescript",
            "export function submitLogin() {\n  return true;\n}\n",
            "submitLogin",
        )
        .unwrap();
        assert!(r.excerpt.contains("return true;"));
        assert_eq!(r.line_end, 3);
    }

    #[test]
    fn typescript_definition_ignores_comments_and_template_literals() {
        let r = resolve_symbol(
            "typescript",
            "export function submitLogin() {\n  const tpl = `value } still string`;\n  // }\n  /* } */\n  return tpl;\n}\n",
            "submitLogin",
        )
        .unwrap();
        assert!(r.excerpt.contains("return tpl;"));
        assert_eq!(r.line_end, 6);
    }

    #[test]
    fn go_definition_ignores_raw_strings() {
        let r = resolve_symbol(
            "go",
            "package sample\n\nfunc runTask() {\n    query := `value } still string`\n    println(query)\n}\n",
            "runTask",
        )
        .unwrap();
        assert!(r.excerpt.contains("println(query)"));
        assert_eq!(r.line_end, 5);
    }

    #[test]
    fn shell_definition_ignores_heredoc_body() {
        let r = resolve_symbol(
            "shell",
            "run_task() {\n  cat <<'EOF'\n}\nEOF\n  echo done\n}\n",
            "run_task",
        )
        .unwrap();
        assert!(r.excerpt.contains("echo done"));
        assert_eq!(r.line_end, 6);
    }

    #[test]
    fn shell_definition_handles_quoted_heredoc_and_following_function() {
        let r = resolve_symbol(
            "shell",
            "run_task() {\n  cat <<'EOF'\n}\nEOF\n}\n\nanother_function() {\n  echo later\n}\n",
            "run_task",
        )
        .unwrap();
        assert!(!r.excerpt.contains("another_function"));
        assert_eq!(r.line_end, 5);
    }

    #[test]
    fn shell_definition_handles_tab_stripped_heredoc_markers() {
        let r = resolve_symbol(
            "shell",
            "run_task() {\n  cat <<-EOF\n\t}\n\tEOF\n  echo done\n}\n",
            "run_task",
        )
        .unwrap();
        assert!(r.excerpt.contains("echo done"));
        assert_eq!(r.line_end, 6);
    }

    #[test]
    fn python_definition_includes_decorators_and_body() {
        let r = resolve_symbol(
            "python",
            "@decorator\n\
def run_task():\n\
    value = 1\n\
    return value\n",
            "run_task",
        )
        .unwrap();
        assert!(r.excerpt.contains("@decorator"));
        assert!(r.line_end >= 2);
    }
}
