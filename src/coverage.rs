// FEAT-CHECK-001
// REQ-CORE-002

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use regex::Regex;
use syn::{Attribute, ImplItem, Item, Visibility};

use crate::{
    config::SyuConfig,
    inspect::{inspect_python_file, inspect_typescript_file},
    model::{Feature, Issue, Requirement, TraceReference},
    workspace::Workspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoverageTargetKind {
    PublicSymbol,
    TestSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoverageTarget {
    file: PathBuf,
    symbol: String,
    kind: CoverageTargetKind,
}

#[derive(Debug, Default, Clone)]
struct CoverageMap {
    explicit_symbols: BTreeMap<PathBuf, BTreeSet<String>>,
    wildcard_files: BTreeSet<PathBuf>,
}

pub fn validate_symbol_trace_coverage(workspace: &Workspace, issues: &mut Vec<Issue>) {
    validate_symbol_trace_coverage_with(
        workspace,
        issues,
        discover_rust_targets,
        discover_python_targets,
        discover_go_targets,
        discover_java_targets,
        discover_typescript_targets,
    );
}

fn validate_symbol_trace_coverage_with(
    workspace: &Workspace,
    issues: &mut Vec<Issue>,
    rust_discovery: impl Fn(&SyuConfig, &Path) -> Result<Vec<CoverageTarget>, Box<Issue>>,
    python_discovery: impl Fn(&SyuConfig, &Path) -> Result<Vec<CoverageTarget>, Box<Issue>>,
    go_discovery: impl Fn(&SyuConfig, &Path) -> Result<Vec<CoverageTarget>, Box<Issue>>,
    java_discovery: impl Fn(&SyuConfig, &Path) -> Result<Vec<CoverageTarget>, Box<Issue>>,
    ts_discovery: impl Fn(&SyuConfig, &Path) -> Result<Vec<CoverageTarget>, Box<Issue>>,
) {
    if !workspace.config.validate.require_symbol_trace_coverage {
        return;
    }

    let mut targets = match rust_discovery(&workspace.config, &workspace.root) {
        Ok(targets) => targets,
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    };

    match python_discovery(&workspace.config, &workspace.root) {
        Ok(python_targets) => targets.extend(python_targets),
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    }

    match go_discovery(&workspace.config, &workspace.root) {
        Ok(go_targets) => targets.extend(go_targets),
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    }

    match java_discovery(&workspace.config, &workspace.root) {
        Ok(java_targets) => targets.extend(java_targets),
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    }

    match ts_discovery(&workspace.config, &workspace.root) {
        Ok(ts_targets) => targets.extend(ts_targets),
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    }

    let feature_coverage = collect_feature_coverage(&workspace.features);
    let requirement_coverage = collect_requirement_coverage(&workspace.requirements);

    for target in targets {
        match target.kind {
            CoverageTargetKind::PublicSymbol
                if !feature_coverage.covers(&target.file, &target.symbol) =>
            {
                issues.push(Issue::error(
                    "SYU-coverage-public-001",
                    format!("public symbol {}", target.symbol),
                    Some(format!("{}::{}", target.file.display(), target.symbol)),
                    format!(
                        "Public symbol `{}` in `{}` is not traced by any feature implementation.",
                        target.symbol,
                        target.file.display()
                    ),
                    Some(format!(
                        "Add `{}` to a feature implementation trace for `{}` or use `*` to cover that file.",
                        target.symbol,
                        target.file.display()
                    )),
                ));
            }
            CoverageTargetKind::TestSymbol
                if !requirement_coverage.covers(&target.file, &target.symbol) =>
            {
                issues.push(Issue::error(
                    "SYU-coverage-test-001",
                    format!("test {}", target.symbol),
                    Some(format!("{}::{}", target.file.display(), target.symbol)),
                    format!(
                        "Test `{}` in `{}` is not traced by any requirement.",
                        target.symbol,
                        target.file.display()
                    ),
                    Some(format!(
                        "Add `{}` to a requirement test trace for `{}` or use `*` to cover that file.",
                        target.symbol,
                        target.file.display()
                    )),
                ));
            }
            _ => {}
        }
    }
}

fn collect_feature_coverage(features: &[Feature]) -> CoverageMap {
    let mut coverage = CoverageMap::default();
    for feature in features {
        collect_trace_map_coverage(&feature.implementations, &mut coverage);
    }
    coverage
}

fn collect_requirement_coverage(requirements: &[Requirement]) -> CoverageMap {
    let mut coverage = CoverageMap::default();
    for requirement in requirements {
        collect_trace_map_coverage(&requirement.tests, &mut coverage);
    }
    coverage
}

fn collect_trace_map_coverage(
    references_by_language: &BTreeMap<String, Vec<TraceReference>>,
    coverage: &mut CoverageMap,
) {
    for references in references_by_language.values() {
        for reference in references {
            if reference.file.as_os_str().is_empty() {
                continue;
            }

            let path = normalize_relative_path(&reference.file);
            let has_wildcard = reference.symbols.iter().any(|symbol| symbol.trim() == "*");

            if has_wildcard {
                coverage.wildcard_files.insert(path);
                continue;
            }

            let entry = coverage.explicit_symbols.entry(path).or_default();
            for symbol in reference
                .symbols
                .iter()
                .map(|symbol| symbol.trim())
                .filter(|symbol| !symbol.is_empty())
            {
                entry.insert(symbol.to_string());
            }
        }
    }
}

impl CoverageMap {
    fn covers(&self, file: &Path, symbol: &str) -> bool {
        let normalized = normalize_relative_path(file);
        self.wildcard_files.contains(&normalized)
            || self
                .explicit_symbols
                .get(&normalized)
                .is_some_and(|symbols| symbols.contains(symbol))
    }
}

fn normalized_symbol_trace_coverage_ignored_paths(config: &SyuConfig) -> BTreeSet<PathBuf> {
    config
        .validate
        .symbol_trace_coverage_ignored_paths
        .iter()
        .filter_map(|path| {
            let normalized = normalize_relative_path(path);
            (!normalized.as_os_str().is_empty()).then_some(normalized)
        })
        .collect()
}

fn discover_rust_targets(
    config: &SyuConfig,
    root: &Path,
) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);
    let mut targets = Vec::new();
    let mut files = rust_files_under(root, &root.join("src"), &ignored_paths)?;
    files.extend(rust_files_under(root, &root.join("tests"), &ignored_paths)?);
    files.sort();

    for path in files {
        let contents = fs::read_to_string(&path).map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-read-001",
                "trace coverage inventory",
                Some(path.display().to_string()),
                format!("Failed to read `{}` while building trace coverage inventory: {error}", path.display()),
                Some("Fix the unreadable file or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;

        let file = syn::parse_file(&contents).map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-parse-001",
                "trace coverage inventory",
                Some(path.display().to_string()),
                format!("Failed to parse `{}` while building trace coverage inventory: {error}", path.display()),
                Some("Fix the Rust syntax error or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;

        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();
        collect_rust_targets(&file.items, &relative, false, &mut targets);
    }

    targets.sort_by(|left, right| {
        (
            left.file.as_os_str(),
            left.symbol.as_str(),
            match left.kind {
                CoverageTargetKind::PublicSymbol => 0,
                CoverageTargetKind::TestSymbol => 1,
            },
        )
            .cmp(&(
                right.file.as_os_str(),
                right.symbol.as_str(),
                match right.kind {
                    CoverageTargetKind::PublicSymbol => 0,
                    CoverageTargetKind::TestSymbol => 1,
                },
            ))
    });
    targets.dedup();

    Ok(targets)
}

fn collect_rust_targets(
    items: &[Item],
    relative_path: &Path,
    in_cfg_test_module: bool,
    targets: &mut Vec<CoverageTarget>,
) {
    for item in items {
        let item_cfg_test = in_cfg_test_module || attrs_have_cfg_test(item_attributes(item));

        match item {
            Item::Fn(function) => {
                if !item_cfg_test && is_public(&function.vis) {
                    targets.push(CoverageTarget {
                        file: relative_path.to_path_buf(),
                        symbol: function.sig.ident.to_string(),
                        kind: CoverageTargetKind::PublicSymbol,
                    });
                }

                if attrs_have_test(item_attributes(item)) {
                    targets.push(CoverageTarget {
                        file: relative_path.to_path_buf(),
                        symbol: function.sig.ident.to_string(),
                        kind: CoverageTargetKind::TestSymbol,
                    });
                }
            }
            Item::Struct(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Enum(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Trait(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Const(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Static(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Type(item) if !item_cfg_test && is_public(&item.vis) => {
                targets.push(CoverageTarget {
                    file: relative_path.to_path_buf(),
                    symbol: item.ident.to_string(),
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
            Item::Mod(item) => {
                if !item_cfg_test && is_public(&item.vis) {
                    targets.push(CoverageTarget {
                        file: relative_path.to_path_buf(),
                        symbol: item.ident.to_string(),
                        kind: CoverageTargetKind::PublicSymbol,
                    });
                }

                if let Some((_, nested)) = &item.content {
                    collect_rust_targets(nested, relative_path, item_cfg_test, targets);
                }
            }
            Item::Impl(item) if !item_cfg_test => {
                for impl_item in &item.items {
                    if let ImplItem::Fn(method) = impl_item
                        && is_public(&method.vis)
                    {
                        targets.push(CoverageTarget {
                            file: relative_path.to_path_buf(),
                            symbol: method.sig.ident.to_string(),
                            kind: CoverageTargetKind::PublicSymbol,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn rust_files_under(
    workspace_root: &Path,
    root: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_rust_files_recursive(workspace_root, root, ignored_paths, &mut files).map_err(
        |error| {
            Box::new(Issue::error(
                "SYU-coverage-walk-001",
                "trace coverage inventory",
                Some(root.display().to_string()),
                format!("Failed to walk `{}` while building trace coverage inventory: {error}", root.display()),
                Some("Fix the directory layout or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        },
    )?;
    Ok(files)
}

fn discover_python_targets(
    config: &SyuConfig,
    root: &Path,
) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);

    let src_files = python_files_under(root, &src_dir, &ignored_paths)?;
    let test_files = python_files_under(root, &tests_dir, &ignored_paths)?;

    if src_files.is_empty() && test_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();

    for path in &src_files {
        let symbols = match inspect_python_file(config, path) {
            Ok(symbols) => symbols,
            Err(_) => continue,
        };
        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();
        for symbol in symbols {
            if !symbol.name.starts_with('_') {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol: symbol.name,
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
        }
    }

    for path in &test_files {
        let symbols = match inspect_python_file(config, path) {
            Ok(symbols) => symbols,
            Err(_) => continue,
        };
        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();
        for symbol in symbols {
            if symbol.name.starts_with("test_") || symbol.name.starts_with("Test") {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol: symbol.name,
                    kind: CoverageTargetKind::TestSymbol,
                });
            }
        }
    }

    targets.sort_by(|left, right| {
        (
            left.file.as_os_str(),
            left.symbol.as_str(),
            match left.kind {
                CoverageTargetKind::PublicSymbol => 0,
                CoverageTargetKind::TestSymbol => 1,
            },
        )
            .cmp(&(
                right.file.as_os_str(),
                right.symbol.as_str(),
                match right.kind {
                    CoverageTargetKind::PublicSymbol => 0,
                    CoverageTargetKind::TestSymbol => 1,
                },
            ))
    });
    targets.dedup();

    Ok(targets)
}

fn python_files_under(
    workspace_root: &Path,
    root: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(workspace_root, root, "py", ignored_paths, &mut files)
        .map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-walk-001",
                "trace coverage inventory",
                Some(root.display().to_string()),
                format!("Failed to walk `{}` while building trace coverage inventory: {error}", root.display()),
                Some("Fix the directory layout or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;
    Ok(files)
}

fn collect_rust_files_recursive(
    workspace_root: &Path,
    directory: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    collect_files_recursive_by_extension(workspace_root, directory, "rs", ignored_paths, files)
}

fn discover_typescript_targets(
    config: &SyuConfig,
    root: &Path,
) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);

    let mut src_files: Vec<PathBuf> = Vec::new();
    let mut test_files: Vec<PathBuf> = Vec::new();

    for ext in TS_EXTENSIONS {
        let src_matches = typescript_files_under(root, &src_dir, ext, &ignored_paths)?;
        let test_matches = typescript_files_under(root, &tests_dir, ext, &ignored_paths)?;
        src_files.extend(src_matches);
        test_files.extend(test_matches);
    }

    if src_files.is_empty() && test_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();

    for path in &src_files {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let symbols = inspect_typescript_file(path, &contents).unwrap_or_default();
        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();
        for symbol in symbols {
            if symbol.is_exported {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol: symbol.name,
                    kind: CoverageTargetKind::PublicSymbol,
                });
            }
        }
    }

    for path in &test_files {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let symbols = inspect_typescript_file(path, &contents).unwrap_or_default();
        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();
        for symbol in symbols {
            if symbol.name.starts_with("test") || symbol.name.starts_with("Test") {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol: symbol.name,
                    kind: CoverageTargetKind::TestSymbol,
                });
            }
        }
    }

    targets.sort_by(|left, right| {
        (
            left.file.as_os_str(),
            left.symbol.as_str(),
            match left.kind {
                CoverageTargetKind::PublicSymbol => 0,
                CoverageTargetKind::TestSymbol => 1,
            },
        )
            .cmp(&(
                right.file.as_os_str(),
                right.symbol.as_str(),
                match right.kind {
                    CoverageTargetKind::PublicSymbol => 0,
                    CoverageTargetKind::TestSymbol => 1,
                },
            ))
    });
    targets.dedup();

    Ok(targets)
}

fn discover_go_targets(config: &SyuConfig, root: &Path) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);
    let mut files = go_files_under(root, &root.join("src"), &ignored_paths)?;
    files.extend(go_files_under(root, &root.join("tests"), &ignored_paths)?);
    files.sort();

    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for path in files {
        let contents = fs::read_to_string(&path).map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-read-001",
                "trace coverage inventory",
                Some(path.display().to_string()),
                format!(
                    "Failed to read `{}` while building trace coverage inventory: {error}",
                    path.display()
                ),
                Some("Fix the unreadable file or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;

        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();

        if is_go_test_file(&path) {
            for symbol in collect_go_test_symbols(&contents) {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol,
                    kind: CoverageTargetKind::TestSymbol,
                });
            }
            continue;
        }

        for symbol in collect_go_public_symbols(&contents) {
            targets.push(CoverageTarget {
                file: relative.clone(),
                symbol,
                kind: CoverageTargetKind::PublicSymbol,
            });
        }
    }

    targets.sort_by(|left, right| {
        (
            left.file.as_os_str(),
            left.symbol.as_str(),
            match left.kind {
                CoverageTargetKind::PublicSymbol => 0,
                CoverageTargetKind::TestSymbol => 1,
            },
        )
            .cmp(&(
                right.file.as_os_str(),
                right.symbol.as_str(),
                match right.kind {
                    CoverageTargetKind::PublicSymbol => 0,
                    CoverageTargetKind::TestSymbol => 1,
                },
            ))
    });
    targets.dedup();

    Ok(targets)
}

fn collect_go_public_symbols(contents: &str) -> Vec<String> {
    let function_regex = Regex::new(
        r"^\s*func\s+(?:\([^)]*\)\s*)?(?P<name>[A-Z][A-Za-z0-9_]*)\s*(?:\[[^\n\r\]]+\])?\s*\(",
    )
    .expect("Go function regex should compile");
    let type_regex = Regex::new(r"^\s*type\s+(?P<name>[A-Z][A-Za-z0-9_]*)\b")
        .expect("Go type regex should compile");
    let value_regex = Regex::new(r"^\s*(?:const|var)\s+(?P<name>[A-Z][A-Za-z0-9_]*)\b")
        .expect("Go value regex should compile");
    let block_start_regex =
        Regex::new(r"^\s*(?:const|var)\s*\(\s*$").expect("Go block regex should compile");
    let block_value_regex = Regex::new(r"^\s*(?P<name>[A-Z][A-Za-z0-9_]*)\b")
        .expect("Go block value regex should compile");
    let block_end_regex = Regex::new(r"^\s*\)\s*$").expect("Go block end regex should compile");

    let mut symbols = Vec::new();
    let mut in_value_block = false;

    for line in contents.lines() {
        if in_value_block {
            if block_end_regex.is_match(line) {
                in_value_block = false;
                continue;
            }
            if let Some(captures) = block_value_regex.captures(line) {
                symbols.push(captures["name"].to_string());
            }
            continue;
        }

        if block_start_regex.is_match(line) {
            in_value_block = true;
            continue;
        }

        if let Some(captures) = function_regex.captures(line) {
            symbols.push(captures["name"].to_string());
            continue;
        }
        if let Some(captures) = type_regex.captures(line) {
            symbols.push(captures["name"].to_string());
            continue;
        }
        if let Some(captures) = value_regex.captures(line) {
            symbols.push(captures["name"].to_string());
        }
    }

    symbols.dedup();
    symbols
}

fn collect_go_test_symbols(contents: &str) -> Vec<String> {
    let regex =
        Regex::new(r"(?m)^\s*func\s+(?P<name>(?:Test|Benchmark|Fuzz|Example)[A-Za-z0-9_]*)\s*\(")
            .expect("Go test symbol regex should compile");
    regex
        .captures_iter(contents)
        .filter_map(|captures| {
            captures
                .name("name")
                .map(|symbol| symbol.as_str().to_string())
        })
        .collect()
}

fn go_files_under(
    workspace_root: &Path,
    root: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(workspace_root, root, "go", ignored_paths, &mut files)
        .map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-walk-001",
                "trace coverage inventory",
                Some(root.display().to_string()),
                format!(
                    "Failed to walk `{}` while building trace coverage inventory: {error}",
                    root.display()
                ),
                Some("Fix the directory layout or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;
    Ok(files)
}

fn is_go_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_test.go"))
}

fn discover_java_targets(
    config: &SyuConfig,
    root: &Path,
) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(config);
    let mut files = java_files_under(root, &root.join("src"), &ignored_paths)?;
    files.extend(java_files_under(root, &root.join("tests"), &ignored_paths)?);
    files.sort();

    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for path in files {
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        let relative = path
            .strip_prefix(root)
            .expect("scanned file should remain under the workspace root")
            .to_path_buf();

        if is_java_test_file(&path, &contents) {
            for symbol in collect_java_test_symbols(&contents) {
                targets.push(CoverageTarget {
                    file: relative.clone(),
                    symbol,
                    kind: CoverageTargetKind::TestSymbol,
                });
            }
            continue;
        }

        for symbol in collect_java_public_symbols(&contents) {
            targets.push(CoverageTarget {
                file: relative.clone(),
                symbol,
                kind: CoverageTargetKind::PublicSymbol,
            });
        }
    }

    targets.sort_by(|left, right| {
        (
            left.file.as_os_str(),
            left.symbol.as_str(),
            match left.kind {
                CoverageTargetKind::PublicSymbol => 0,
                CoverageTargetKind::TestSymbol => 1,
            },
        )
            .cmp(&(
                right.file.as_os_str(),
                right.symbol.as_str(),
                match right.kind {
                    CoverageTargetKind::PublicSymbol => 0,
                    CoverageTargetKind::TestSymbol => 1,
                },
            ))
    });
    targets.dedup();

    Ok(targets)
}

fn collect_java_public_symbols(contents: &str) -> Vec<String> {
    let type_regex = Regex::new(
        r"(?m)^\s*public\s+(?:static\s+)?(?:sealed\s+|non-sealed\s+|abstract\s+|final\s+)?(?:class|interface|enum|record)\s+(?P<name>[A-Z][A-Za-z0-9_]*)\b",
    )
    .expect("Java type regex should compile");
    let method_regex = Regex::new(
        r"(?m)^\s*public\s+(?:static\s+)?(?:final\s+)?(?:synchronized\s+)?(?:abstract\s+)?(?:native\s+)?(?:strictfp\s+)?(?:<[^>{}]+>\s*)?(?:[\w\[\]<>?,]+\s+)+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )
    .expect("Java method regex should compile");
    let constructor_regex =
        Regex::new(r"(?m)^\s*public\s+(?:<[^>{}]+>\s*)?(?P<name>[A-Z][A-Za-z0-9_]*)\s*\(")
            .expect("Java constructor regex should compile");
    let field_regex = Regex::new(
        r"(?m)^\s*public\s+(?:static\s+)?(?:final\s+)?(?:transient\s+)?(?:volatile\s+)?(?:[\w\[\]<>?,]+\s+)+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:=|;)",
    )
    .expect("Java field regex should compile");

    let mut symbols = BTreeSet::new();
    for captures in type_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    for captures in method_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    for captures in constructor_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    for captures in field_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    for symbol in collect_java_public_interface_members(contents) {
        symbols.insert(symbol);
    }
    symbols.into_iter().collect()
}

fn collect_java_test_symbols(contents: &str) -> Vec<String> {
    let annotation_regex = Regex::new(
        r"(?ms)@(?:[\w.]+\.)?Test(?:\s*\([^)]*\))?\s*(?:(?:\r?\n\s*)*@[\w.]+(?:\s*\([^)]*\))?\s*)*(?:\r?\n\s*)*(?:public|protected|private)?\s*(?:static\s+)?(?:final\s+)?(?:<[^>{}]+>\s*)?(?:void|[\w\[\]<>?,]+)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )
    .expect("Java @Test regex should compile");
    let legacy_regex = Regex::new(r"(?m)^\s*public\s+void\s+(?P<name>test[A-Za-z0-9_]*)\s*\(")
        .expect("Java legacy test regex should compile");

    let mut symbols = BTreeSet::new();
    for captures in annotation_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    for captures in legacy_regex.captures_iter(contents) {
        if let Some(name) = captures.name("name") {
            symbols.insert(name.as_str().to_string());
        }
    }
    symbols.into_iter().collect()
}

fn collect_java_public_interface_members(contents: &str) -> Vec<String> {
    let interface_start_regex = Regex::new(
        r"(?m)^\s*public\s+(?:sealed\s+|non-sealed\s+)?interface\s+[A-Z][A-Za-z0-9_]*\b[^{]*\{",
    )
    .expect("Java interface regex should compile");
    let method_regex = Regex::new(
        r"^\s*(?:public\s+)?(?:default\s+|static\s+|abstract\s+|strictfp\s+|synchronized\s+)*(?:<[^>{}]+>\s*)?(?:[\w\[\]<>?,]+\s+)+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )
    .expect("Java interface method regex should compile");
    let field_regex = Regex::new(
        r"^\s*(?:public\s+)?(?:static\s+)?(?:final\s+)?(?:transient\s+)?(?:volatile\s+)?(?:[\w\[\]<>?,]+\s+)+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:=|;)",
    )
    .expect("Java interface field regex should compile");

    let mut symbols = BTreeSet::new();
    for mat in interface_start_regex.find_iter(contents) {
        let body_start = mat.end() - 1;
        let Some(body_end) = find_matching_brace(contents, body_start) else {
            continue;
        };
        let body = &contents[body_start + 1..body_end];
        let top_level = collect_java_top_level_lines(body);
        for line in top_level.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("private ") || trimmed.is_empty() {
                continue;
            }
            if let Some(captures) = method_regex.captures(trimmed) {
                if let Some(name) = captures.name("name") {
                    symbols.insert(name.as_str().to_string());
                }
                continue;
            }
            if let Some(captures) = field_regex.captures(trimmed)
                && let Some(name) = captures.name("name")
            {
                symbols.insert(name.as_str().to_string());
            }
        }
    }

    symbols.into_iter().collect()
}

fn find_matching_brace(contents: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0;
    for (offset, ch) in contents[open_index..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_java_top_level_lines(body: &str) -> String {
    let mut depth = 0_i32;
    let mut top_level = String::new();
    for line in body.lines() {
        if depth == 0 {
            top_level.push_str(line);
            top_level.push('\n');
        }
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' if depth > 0 => depth -= 1,
                _ => {}
            }
        }
    }
    top_level
}

fn java_files_under(
    workspace_root: &Path,
    root: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(workspace_root, root, "java", ignored_paths, &mut files)
        .map_err(|error| {
            Box::new(Issue::error(
                "SYU-coverage-walk-001",
                "trace coverage inventory",
                Some(root.display().to_string()),
                format!(
                    "Failed to walk `{}` while building trace coverage inventory: {error}",
                    root.display()
                ),
                Some("Fix the directory layout or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
            ))
        })?;
    Ok(files)
}

fn is_java_test_file(path: &Path, contents: &str) -> bool {
    let junit_annotation_regex =
        Regex::new(r"@(?:[\w.]+\.)?Test\b").expect("Java test annotation regex should compile");
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("Test.java") || name.ends_with("Tests.java"))
        || junit_annotation_regex.is_match(contents)
        || contents.contains("extends TestCase")
}

fn typescript_files_under(
    workspace_root: &Path,
    root: &Path,
    extension: &str,
    ignored_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(
        workspace_root,
        root,
        extension,
        ignored_paths,
        &mut files,
    )
    .map_err(|error| {
        Box::new(Issue::error(
            "SYU-coverage-walk-001",
            "trace coverage inventory",
            Some(root.display().to_string()),
            format!(
                "Failed to walk `{}` while building trace coverage inventory: {error}",
                root.display()
            ),
            Some("Fix the directory layout or disable `validate.require_symbol_trace_coverage` until the workspace can be scanned.".to_string()),
        ))
    })?;
    Ok(files)
}

fn collect_files_recursive_by_extension(
    workspace_root: &Path,
    directory: &Path,
    extension: &str,
    ignored_paths: &BTreeSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            if should_skip_generated_directory(workspace_root, &path, ignored_paths) {
                continue;
            }
            let next = path;
            let recurse = collect_files_recursive_by_extension;
            recurse(workspace_root, &next, extension, ignored_paths, files)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some(extension) {
            continue;
        }

        files.push(path);
    }

    Ok(())
}

fn should_skip_generated_directory(
    workspace_root: &Path,
    path: &Path,
    ignored_paths: &BTreeSet<PathBuf>,
) -> bool {
    path.strip_prefix(workspace_root)
        .ok()
        .map(normalize_relative_path)
        .is_some_and(|relative| ignored_paths.contains(&relative))
}

pub(crate) fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(segment) => normalized.push(segment),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        _ => &[],
    }
}

fn attrs_have_cfg_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        matches!(
            &attribute.meta,
            syn::Meta::List(list)
                if attribute.path().is_ident("cfg") && list.tokens.to_string().contains("test")
        )
    })
}

fn attrs_have_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use super::{
        CoverageTargetKind, collect_feature_coverage, collect_files_recursive_by_extension,
        collect_go_public_symbols, collect_go_test_symbols, collect_java_public_symbols,
        collect_java_test_symbols, collect_requirement_coverage, discover_go_targets,
        discover_java_targets, discover_python_targets, discover_rust_targets,
        discover_typescript_targets, go_files_under, java_files_under, normalize_relative_path,
        normalized_symbol_trace_coverage_ignored_paths, python_files_under, rust_files_under,
        typescript_files_under, validate_symbol_trace_coverage,
        validate_symbol_trace_coverage_with,
    };
    use crate::{
        config::SyuConfig,
        model::{Feature, Issue, Requirement, TraceReference},
        workspace::Workspace,
    };

    fn no_targets(
        _config: &SyuConfig,
        _root: &Path,
    ) -> Result<Vec<super::CoverageTarget>, Box<Issue>> {
        Ok(Vec::new())
    }

    fn no_java_targets(
        _config: &SyuConfig,
        _root: &Path,
    ) -> Result<Vec<super::CoverageTarget>, Box<Issue>> {
        Ok(Vec::new())
    }

    #[test]
    fn discover_rust_targets_collects_public_symbols_and_tests() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::create_dir_all(tempdir.path().join("src/nested")).expect("nested src");
        fs::create_dir_all(tempdir.path().join("tests")).expect("tests");
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "use std::fmt;\n\
             pub fn public_api() {}\n\
             pub struct PublicStruct;\n\
             pub enum PublicEnum { Ready }\n\
             pub trait PublicTrait { fn describe(&self); }\n\
             pub const LIMIT: usize = 1;\n\
             pub static LABEL: &str = \"ok\";\n\
             pub type Alias = usize;\n\
             pub mod nested { pub fn nested_api() {} }\n\
             pub struct ImplType;\n\
             impl ImplType { pub fn create() -> Self { Self } }\n\
             #[cfg(test)] pub fn hidden_in_test() {}\n\
             #[cfg(test)] mod tests { #[test] fn unit_case() {} }\n",
        )
        .expect("source");
        fs::write(
            tempdir.path().join("src/nested/inner.rs"),
            "pub fn nested_file_api() {}\n",
        )
        .expect("nested source");
        fs::write(
            tempdir.path().join("tests/integration.rs"),
            "fn helper() {}\n#[test] fn integration_case() {}\n",
        )
        .expect("integration");

        let targets =
            discover_rust_targets(&SyuConfig::default(), tempdir.path()).expect("targets");
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "public_api"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "PublicStruct"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "PublicEnum"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "PublicTrait"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "LIMIT"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "LABEL"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "Alias"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "nested"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "create"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/lib.rs")
                && target.symbol == "unit_case"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("tests/integration.rs")
                && target.symbol == "integration_case"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/nested/inner.rs")
                && target.symbol == "nested_file_api"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(!targets.iter().any(|target| target.symbol == "helper"));
        assert!(
            !targets
                .iter()
                .any(|target| target.symbol == "hidden_in_test")
        );
    }

    #[test]
    fn coverage_maps_honor_explicit_and_wildcard_symbols() {
        let feature_coverage = collect_feature_coverage(&[Feature {
            id: "FEAT-1".to_string(),
            title: "Title".to_string(),
            summary: "Summary".to_string(),
            status: "implemented".to_string(),
            linked_requirements: vec!["REQ-1".to_string()],
            implementations: BTreeMap::from([(
                "rust".to_string(),
                vec![
                    TraceReference {
                        file: PathBuf::from("src/api.rs"),
                        symbols: vec!["public_api".to_string()],
                        doc_contains: Vec::new(),
                    },
                    TraceReference {
                        file: PathBuf::from("src/cli.rs"),
                        symbols: vec!["*".to_string()],
                        doc_contains: Vec::new(),
                    },
                    TraceReference {
                        file: PathBuf::new(),
                        symbols: vec!["ignored".to_string()],
                        doc_contains: Vec::new(),
                    },
                ],
            )]),
        }]);

        let requirement_coverage = collect_requirement_coverage(&[Requirement {
            id: "REQ-1".to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            priority: "high".to_string(),
            status: "implemented".to_string(),
            linked_policies: vec!["POL-1".to_string()],
            linked_features: vec!["FEAT-1".to_string()],
            tests: BTreeMap::from([(
                "rust".to_string(),
                vec![
                    TraceReference {
                        file: PathBuf::from("tests/api.rs"),
                        symbols: vec!["*".to_string()],
                        doc_contains: Vec::new(),
                    },
                    TraceReference {
                        file: PathBuf::from("./tests/../tests/helpers.rs"),
                        symbols: vec![" ".to_string(), "helper_case".to_string()],
                        doc_contains: Vec::new(),
                    },
                ],
            )]),
        }]);

        assert!(feature_coverage.covers(Path::new("src/api.rs"), "public_api"));
        assert!(feature_coverage.covers(Path::new("src/cli.rs"), "browse"));
        assert!(!feature_coverage.covers(Path::new("src/api.rs"), "missing"));

        assert!(requirement_coverage.covers(Path::new("tests/api.rs"), "integration_case"));
        assert!(requirement_coverage.covers(Path::new("tests/helpers.rs"), "helper_case"));
    }

    #[test]
    fn discover_java_targets_collects_public_symbols_and_tests() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::create_dir_all(tempdir.path().join("tests")).expect("tests");
        fs::write(
            tempdir.path().join("src/TraceService.java"),
            "public class TraceService {\n    public static final String TRACE_LABEL = \"ok\";\n    public void start() {}\n    void helper() {}\n}\npublic interface TraceApi {}\npublic enum TraceState { READY }\npublic record TraceRecord(String value) {}\n",
        )
        .expect("java source");
        fs::write(
            tempdir.path().join("src/TraceServiceTest.java"),
            "import org.junit.jupiter.api.Test;\n\nclass TraceServiceTest {\n    @Test\n    void reqTraceJavaTest() {}\n}\n",
        )
        .expect("java src test");
        fs::write(
            tempdir.path().join("tests/TraceabilityTest.java"),
            "import org.junit.Test;\n\npublic class TraceabilityTest {\n    @Test\n    public void reqTraceJavaIntegration() {}\n\n    public void testLegacyStyle() {}\n}\n",
        )
        .expect("java tests");

        let targets =
            discover_java_targets(&SyuConfig::default(), tempdir.path()).expect("targets");
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "TraceService"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "TraceApi"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "TraceState"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "TraceRecord"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "start"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceService.java")
                && target.symbol == "TRACE_LABEL"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/TraceServiceTest.java")
                && target.symbol == "reqTraceJavaTest"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("tests/TraceabilityTest.java")
                && target.symbol == "reqTraceJavaIntegration"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("tests/TraceabilityTest.java")
                && target.symbol == "testLegacyStyle"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
        assert!(!targets.iter().any(|target| target.symbol == "helper"));
    }

    #[test]
    fn discover_java_targets_skips_unreadable_java_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).expect("src dir");
        let readable = src_dir.join("Owned.java");
        let unreadable = src_dir.join("Hidden.java");
        fs::write(&readable, "public class Owned {}\n").expect("readable java file");
        fs::write(&unreadable, "public class Hidden {}\n").expect("unreadable java file");

        let mut perm = fs::metadata(&unreadable).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&unreadable, perm).expect("set unreadable");

        let targets =
            discover_java_targets(&SyuConfig::default(), tempdir.path()).expect("targets");

        let mut restore = fs::metadata(&unreadable).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&unreadable, restore).expect("restore");

        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/Owned.java")
                && target.symbol == "Owned"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(
            !targets
                .iter()
                .any(|target| { target.file == Path::new("src/Hidden.java") })
        );
    }

    #[test]
    fn collect_java_public_symbols_covers_interface_members_without_public_modifiers() {
        let symbols = collect_java_public_symbols(
            "public interface FeatureTrace {\n    void featureTraceJava();\n    String EXPORTED_NAME = \"ok\";\n    default void defaultHelper() {\n        if (true) {\n            featureTraceJava();\n        }\n    }\n    private void hiddenHelper() {}\n}\n",
        );

        assert_eq!(
            symbols,
            vec![
                "EXPORTED_NAME",
                "FeatureTrace",
                "defaultHelper",
                "featureTraceJava"
            ]
        );
    }

    #[test]
    fn collect_java_public_symbols_covers_constructors_and_fields() {
        let symbols = collect_java_public_symbols(
            "public class FeatureTrace {\n    public static final String TRACE_LABEL = \"ok\";\n    public FeatureTrace() {}\n}\n",
        );

        assert_eq!(symbols, vec!["FeatureTrace", "TRACE_LABEL"]);
    }

    #[test]
    fn collect_java_test_symbols_covers_stacked_annotations() {
        let symbols = collect_java_test_symbols(
            "import org.junit.jupiter.api.DisplayName;\nimport org.junit.jupiter.api.Tag;\nimport org.junit.jupiter.api.Test;\n\nclass TraceabilityTest {\n    @Test\n    @DisplayName(\"stacked\")\n    @Tag(\"coverage\")\n    void untrackedStackedTest() {}\n}\n",
        );

        assert_eq!(symbols, vec!["untrackedStackedTest"]);
    }

    #[test]
    fn collect_java_test_symbols_covers_fully_qualified_test_annotations() {
        let symbols = collect_java_test_symbols(
            "class TraceabilityTest {\n    @org.junit.jupiter.api.Test\n    void qualifiedTest() {}\n}\n",
        );

        assert_eq!(symbols, vec!["qualifiedTest"]);
    }

    #[test]
    fn collect_java_public_symbols_covers_nested_public_static_types() {
        let symbols = collect_java_public_symbols(
            "public class FeatureTrace {\n    public static final class NestedFeature {}\n}\n",
        );

        assert_eq!(symbols, vec!["FeatureTrace", "NestedFeature"]);
    }

    #[test]
    fn collect_java_public_symbols_skips_unclosed_interface_bodies() {
        let symbols = collect_java_public_symbols(
            "public interface BrokenTrace {\n    void missingBrace();\n",
        );

        assert_eq!(symbols, vec!["BrokenTrace"]);
    }

    #[test]
    fn rust_files_under_handles_missing_and_invalid_roots() {
        let tempdir = tempdir().expect("tempdir");
        let ignored_paths = BTreeSet::new();
        assert!(
            rust_files_under(
                tempdir.path(),
                &tempdir.path().join("missing"),
                &ignored_paths
            )
            .expect("missing directories should be ignored")
            .is_empty()
        );

        let file_root = tempdir.path().join("not-a-dir.rs");
        fs::write(&file_root, "pub fn nope() {}\n").expect("file");
        let issue = rust_files_under(tempdir.path(), &file_root, &ignored_paths)
            .expect_err("file roots should fail");
        assert_eq!(issue.code, "SYU-coverage-walk-001");
    }

    #[test]
    fn file_discovery_skips_configured_repository_relative_generated_paths() {
        let tempdir = tempdir().expect("tempdir");
        let src_root = tempdir.path().join("src");
        let app_root = tempdir.path().join("app/dist");
        let target_root = tempdir.path().join("target");
        fs::create_dir_all(src_root.join("nested")).expect("nested");
        fs::create_dir_all(src_root.join("build")).expect("build");
        fs::create_dir_all(src_root.join("target")).expect("nested target");
        fs::create_dir_all(&app_root).expect("app dist");
        fs::create_dir_all(&target_root).expect("root target");

        let keep_root = src_root.join("lib.rs");
        let keep_nested = src_root.join("nested/mod.rs");
        let keep_build = src_root.join("build/authored.rs");
        let keep_target = src_root.join("target/authored.rs");
        fs::write(&keep_root, "pub fn keep_root() {}\n").expect("keep root");
        fs::write(&keep_nested, "pub fn keep_nested() {}\n").expect("keep nested");
        fs::write(&keep_build, "pub fn keep_build() {}\n").expect("keep build");
        fs::write(&keep_target, "pub fn keep_target() {}\n").expect("keep target");
        fs::write(app_root.join("generated.rs"), "pub fn ignored_dist() {}\n").expect("dist file");
        fs::write(
            target_root.join("generated.rs"),
            "pub fn ignored_target() {}\n",
        )
        .expect("target file");

        let mut files = Vec::new();
        let ignored_paths = BTreeSet::from([PathBuf::from("app/dist"), PathBuf::from("target")]);
        collect_files_recursive_by_extension(
            tempdir.path(),
            tempdir.path(),
            "rs",
            &ignored_paths,
            &mut files,
        )
        .expect("discovery should succeed");
        files.sort();

        assert_eq!(files, vec![keep_build, keep_root, keep_nested, keep_target]);
    }

    #[test]
    fn discover_rust_targets_reports_parse_failures() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::write(tempdir.path().join("src/broken.rs"), "pub fn broken( {}\n").expect("broken");

        let issue = discover_rust_targets(&SyuConfig::default(), tempdir.path())
            .expect_err("broken rust should fail");
        assert_eq!(issue.code, "SYU-coverage-parse-001");
    }

    #[cfg(unix)]
    #[test]
    fn validate_symbol_trace_coverage_reports_unreadable_sources() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).expect("src");
        let unreadable = src_dir.join("hidden.rs");
        fs::write(&unreadable, "pub fn hidden() {}\n").expect("source");

        let mut permissions = fs::metadata(&unreadable).expect("metadata").permissions();
        let original_mode = permissions.mode();
        permissions.set_mode(0o000);
        fs::set_permissions(&unreadable, permissions).expect("set unreadable");

        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage(&workspace, &mut issues);

        let mut restore = fs::metadata(&unreadable).expect("metadata").permissions();
        restore.set_mode(original_mode);
        fs::set_permissions(&unreadable, restore).expect("restore");

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-coverage-read-001");
    }

    #[test]
    fn normalize_relative_path_handles_dot_parent_and_absolute_segments() {
        assert_eq!(
            normalize_relative_path(Path::new("./src/../src/lib.rs")),
            PathBuf::from("src/lib.rs")
        );
        assert_eq!(
            normalize_relative_path(Path::new("/tmp/coverage.rs")),
            PathBuf::from("/tmp/coverage.rs")
        );
        assert_eq!(
            normalize_relative_path(Path::new("../spec/trace.rs")),
            PathBuf::from("../spec/trace.rs")
        );
    }

    #[test]
    fn collect_go_public_symbols_covers_values_blocks_and_generic_functions() {
        let symbols = collect_go_public_symbols(
            "package trace\n\n\
             func CoveredAPI() {}\n\
             func GenericAPI[T any]() {}\n\
             type PublicThing interface { Run() }\n\
             var ExportedConfig = \"ok\"\n\
             const ExportedFlag = true\n\
             const (\n\
                 ExportedBlockFlag = true\n\
                 hiddenBlockFlag = false\n\
             )\n\
             var (\n\
                 ExportedBlockConfig = \"ok\"\n\
                 hiddenBlockConfig = \"no\"\n\
             )\n\
             func hiddenAPI() {}\n",
        );

        assert_eq!(
            symbols,
            vec![
                "CoveredAPI",
                "GenericAPI",
                "PublicThing",
                "ExportedConfig",
                "ExportedFlag",
                "ExportedBlockFlag",
                "ExportedBlockConfig",
            ]
        );
    }

    #[test]
    fn collect_go_test_symbols_covers_all_go_entry_points() {
        let symbols = collect_go_test_symbols(
            "package trace\n\n\
             import \"testing\"\n\n\
             func TestCovered(t *testing.T) {}\n\
             func BenchmarkCovered(b *testing.B) {}\n\
             func FuzzCovered(f *testing.F) {}\n\
             func ExampleCovered() {}\n\
             func helperCase(t *testing.T) {}\n",
        );

        assert_eq!(
            symbols,
            vec![
                "TestCovered",
                "BenchmarkCovered",
                "FuzzCovered",
                "ExampleCovered",
            ]
        );
    }

    #[test]
    fn go_files_under_returns_empty_for_nonexistent_dir() {
        let tempdir = tempdir().expect("tempdir");
        let missing = tempdir.path().join("nonexistent");
        let result = go_files_under(tempdir.path(), &missing, &BTreeSet::new())
            .expect("should return empty ok");
        assert!(result.is_empty());
    }

    #[test]
    fn go_files_under_errors_on_file_path() {
        let tempdir = tempdir().expect("tempdir");
        let src = tempdir.path().join("src");
        fs::write(&src, "not a directory").expect("write blocking file");
        let result = go_files_under(tempdir.path(), &src, &BTreeSet::new());
        assert!(result.is_err());
    }

    #[test]
    fn discover_go_targets_collects_exported_symbols_and_tests() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::create_dir_all(tempdir.path().join("tests")).expect("tests");
        fs::write(
            tempdir.path().join("src/api.go"),
            "package trace\n\n\
             func FeatureTraceGo() {}\n\
             func GenericAPI[T any]() {}\n\
             type PublicThing interface { Run() }\n\
             const (\n\
                 ExportedBlockFlag = true\n\
             )\n",
        )
        .expect("go source");
        fs::write(
            tempdir.path().join("tests/api_test.go"),
            "package trace\n\n\
             import \"testing\"\n\n\
             func TestTraceCoverage(t *testing.T) {}\n\
             func BenchmarkTraceCoverage(b *testing.B) {}\n",
        )
        .expect("go tests");

        let targets = discover_go_targets(&SyuConfig::default(), tempdir.path()).expect("targets");
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/api.go")
                && target.symbol == "FeatureTraceGo"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/api.go")
                && target.symbol == "GenericAPI"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/api.go")
                && target.symbol == "ExportedBlockFlag"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(targets.iter().any(|target| {
            target.file == Path::new("tests/api_test.go")
                && target.symbol == "BenchmarkTraceCoverage"
                && target.kind == CoverageTargetKind::TestSymbol
        }));
    }

    #[test]
    fn discover_go_targets_returns_empty_without_go_files() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        let result =
            discover_go_targets(&SyuConfig::default(), tempdir.path()).expect("should succeed");
        assert!(result.is_empty());
    }

    #[test]
    fn java_files_under_errors_on_file_path() {
        let tempdir = tempdir().expect("tempdir");
        let src = tempdir.path().join("src");
        fs::write(&src, "not a directory").expect("write blocking file");
        let result = java_files_under(tempdir.path(), &src, &BTreeSet::new());
        assert!(result.is_err());
    }

    #[test]
    fn discover_java_targets_skips_ignored_paths() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::create_dir_all(
            tempdir
                .path()
                .join("tests/fixtures/workspaces/passing/java"),
        )
        .expect("fixture java dir");
        fs::write(
            tempdir.path().join("src/Owned.java"),
            "public class Owned { public void covered() {} }\n",
        )
        .expect("java source");
        fs::write(
            tempdir
                .path()
                .join("tests/fixtures/workspaces/passing/java/FeatureTrace.java"),
            "public class FeatureTrace { public void featureTraceJava() {} }\n",
        )
        .expect("fixture java source");

        let mut config = SyuConfig::default();
        config.validate.symbol_trace_coverage_ignored_paths =
            vec![PathBuf::from("tests/fixtures/workspaces")];

        let ignored_paths = normalized_symbol_trace_coverage_ignored_paths(&config);
        let files = java_files_under(
            tempdir.path(),
            &tempdir.path().join("tests"),
            &ignored_paths,
        )
        .expect("java files");
        assert!(files.is_empty());

        let targets = discover_java_targets(&config, tempdir.path()).expect("targets");
        assert!(targets.iter().any(|target| {
            target.file == Path::new("src/Owned.java")
                && target.symbol == "Owned"
                && target.kind == CoverageTargetKind::PublicSymbol
        }));
        assert!(!targets.iter().any(|target| {
            target.file == Path::new("tests/fixtures/workspaces/passing/java/FeatureTrace.java")
        }));
    }

    #[test]
    fn python_files_under_returns_empty_for_nonexistent_dir() {
        let tempdir = tempdir().expect("tempdir");
        let missing = tempdir.path().join("nonexistent");
        let result = python_files_under(tempdir.path(), &missing, &BTreeSet::new())
            .expect("should return empty ok");
        assert!(result.is_empty());
    }

    #[test]
    fn python_files_under_collects_py_files() {
        let tempdir = tempdir().expect("tempdir");
        fs::write(tempdir.path().join("a.py"), "def func(): pass\n").expect("write");
        let result = python_files_under(tempdir.path(), tempdir.path(), &BTreeSet::new())
            .expect("should succeed");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn typescript_files_under_returns_empty_for_nonexistent_dir() {
        let tempdir = tempdir().expect("tempdir");
        let missing = tempdir.path().join("nonexistent");
        let result = typescript_files_under(tempdir.path(), &missing, "ts", &BTreeSet::new())
            .expect("should return empty ok");
        assert!(result.is_empty());
    }

    #[test]
    fn discover_python_targets_returns_empty_without_py_files() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        let config = SyuConfig::default();
        let result = discover_python_targets(&config, tempdir.path()).expect("should succeed");
        assert!(result.is_empty());
    }

    #[test]
    fn validate_symbol_trace_coverage_reports_python_scan_failure() {
        let tempdir = tempdir().expect("tempdir");
        // Write a FILE at the src path so collect_files_recursive_by_extension fails
        fs::write(tempdir.path().join("src"), "not a directory").expect("blocking file");

        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage(&workspace, &mut issues);
        assert!(!issues.is_empty());
    }

    #[test]
    fn discover_typescript_targets_returns_empty_without_ts_files() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        let result = discover_typescript_targets(&SyuConfig::default(), tempdir.path())
            .expect("should succeed");
        assert!(result.is_empty());
    }

    #[test]
    fn python_files_under_errors_on_file_path() {
        let tempdir = tempdir().expect("tempdir");
        let src = tempdir.path().join("src");
        // Write a regular FILE where a directory is expected
        fs::write(&src, "not a directory").expect("write blocking file");
        let result = python_files_under(tempdir.path(), &src, &BTreeSet::new());
        assert!(result.is_err());
    }

    #[test]
    fn typescript_files_under_errors_on_file_path() {
        let tempdir = tempdir().expect("tempdir");
        let src = tempdir.path().join("src");
        fs::write(&src, "not a directory").expect("write blocking file");
        let result = typescript_files_under(tempdir.path(), &src, "ts", &BTreeSet::new());
        assert!(result.is_err());
    }

    #[test]
    fn discover_python_targets_skips_unreadable_py_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).expect("src dir");
        let py_file = src_dir.join("mod.py");
        fs::write(&py_file, "def public_fn(): pass\n").expect("py file");

        // Make the .py file unreadable so inspect_python_file fails
        let mut perm = fs::metadata(&py_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&py_file, perm).expect("set unreadable");

        let config = SyuConfig::default();
        let result = discover_python_targets(&config, tempdir.path()).expect("should succeed");

        let mut restore = fs::metadata(&py_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&py_file, restore).expect("restore");

        // Unreadable file is silently skipped
        assert!(result.is_empty());
    }

    #[test]
    fn discover_python_targets_skips_unreadable_test_py_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let tests_dir = tempdir.path().join("tests");
        fs::create_dir_all(&tests_dir).expect("tests dir");
        let py_file = tests_dir.join("test_mod.py");
        fs::write(&py_file, "def test_something(): pass\n").expect("py file");

        let mut perm = fs::metadata(&py_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&py_file, perm).expect("set unreadable");

        let config = SyuConfig::default();
        let result = discover_python_targets(&config, tempdir.path()).expect("should succeed");

        let mut restore = fs::metadata(&py_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&py_file, restore).expect("restore");

        assert!(result.is_empty());
    }

    #[test]
    fn discover_go_targets_reports_unreadable_go_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).expect("src dir");
        let go_file = src_dir.join("api.go");
        fs::write(&go_file, "package trace\n\nfunc FeatureTraceGo() {}\n").expect("go file");

        let mut perm = fs::metadata(&go_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&go_file, perm).expect("set unreadable");

        let result = discover_go_targets(&SyuConfig::default(), tempdir.path());

        let mut restore = fs::metadata(&go_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&go_file, restore).expect("restore");

        assert!(result.is_err());
    }

    #[test]
    fn discover_go_targets_reports_unreadable_go_test_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let tests_dir = tempdir.path().join("tests");
        fs::create_dir_all(&tests_dir).expect("tests dir");
        let go_file = tests_dir.join("api_test.go");
        fs::write(
            &go_file,
            "package trace\n\nimport \"testing\"\n\nfunc TestTraceCoverage(t *testing.T) {}\n",
        )
        .expect("go test file");

        let mut perm = fs::metadata(&go_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&go_file, perm).expect("set unreadable");

        let result = discover_go_targets(&SyuConfig::default(), tempdir.path());

        let mut restore = fs::metadata(&go_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&go_file, restore).expect("restore");

        assert!(result.is_err());
    }

    #[test]
    fn discover_typescript_targets_skips_unreadable_ts_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).expect("src dir");
        let ts_file = src_dir.join("app.ts");
        fs::write(&ts_file, "export function hello() {}\n").expect("ts file");

        let mut perm = fs::metadata(&ts_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&ts_file, perm).expect("set unreadable");

        let result = discover_typescript_targets(&SyuConfig::default(), tempdir.path())
            .expect("should succeed");

        let mut restore = fs::metadata(&ts_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&ts_file, restore).expect("restore");

        assert!(result.is_empty());
    }

    #[test]
    fn discover_typescript_targets_skips_unreadable_test_ts_files() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempdir().expect("tempdir");
        let tests_dir = tempdir.path().join("tests");
        fs::create_dir_all(&tests_dir).expect("tests dir");
        let ts_file = tests_dir.join("app.test.ts");
        fs::write(&ts_file, "export function testSomething() {}\n").expect("ts file");

        let mut perm = fs::metadata(&ts_file).expect("meta").permissions();
        let mode = perm.mode();
        perm.set_mode(0o000);
        fs::set_permissions(&ts_file, perm).expect("set unreadable");

        let result = discover_typescript_targets(&SyuConfig::default(), tempdir.path())
            .expect("should succeed");

        let mut restore = fs::metadata(&ts_file).expect("meta").permissions();
        restore.set_mode(mode);
        fs::set_permissions(&ts_file, restore).expect("restore");

        assert!(result.is_empty());
    }

    #[test]
    fn validate_with_python_discovery_error_records_issue_and_returns() {
        let tempdir = tempdir().expect("tempdir");
        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage_with(
            &workspace,
            &mut issues,
            no_targets,
            |_config, _root| {
                Err(Box::new(crate::model::Issue::error(
                    "SYU-coverage-walk-001",
                    "trace coverage inventory",
                    None,
                    "injected python discovery error".to_string(),
                    None,
                )))
            },
            no_targets,
            no_java_targets,
            discover_typescript_targets,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-coverage-walk-001");
    }

    #[test]
    fn validate_with_go_discovery_error_records_issue_and_returns() {
        let tempdir = tempdir().expect("tempdir");
        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage_with(
            &workspace,
            &mut issues,
            |_config, _root| Ok(Vec::new()),
            |_config, _root| Ok(Vec::new()),
            |_config, _root| {
                Err(Box::new(crate::model::Issue::error(
                    "SYU-coverage-walk-001",
                    "trace coverage inventory",
                    None,
                    "injected go discovery error".to_string(),
                    None,
                )))
            },
            discover_java_targets,
            discover_typescript_targets,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-coverage-walk-001");
    }

    #[test]
    fn validate_with_java_discovery_error_records_issue_and_returns() {
        let tempdir = tempdir().expect("tempdir");
        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage_with(
            &workspace,
            &mut issues,
            |_config, _root| Ok(Vec::new()),
            |_config, _root| Ok(Vec::new()),
            |_config, _root| Ok(Vec::new()),
            |_config, _root| {
                Err(Box::new(crate::model::Issue::error(
                    "SYU-coverage-walk-001",
                    "trace coverage inventory",
                    None,
                    "injected java discovery error".to_string(),
                    None,
                )))
            },
            discover_typescript_targets,
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-coverage-walk-001");
    }

    #[test]
    fn validate_with_typescript_discovery_error_records_issue_and_returns() {
        let tempdir = tempdir().expect("tempdir");
        let mut config = SyuConfig::default();
        config.validate.require_symbol_trace_coverage = true;
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config,
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let mut issues = Vec::new();
        validate_symbol_trace_coverage_with(
            &workspace,
            &mut issues,
            no_targets,
            no_targets,
            no_targets,
            no_java_targets,
            |_config, _root| {
                Err(Box::new(crate::model::Issue::error(
                    "SYU-coverage-walk-001",
                    "trace coverage inventory",
                    None,
                    "injected typescript discovery error".to_string(),
                    None,
                )))
            },
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "SYU-coverage-walk-001");
    }
}
