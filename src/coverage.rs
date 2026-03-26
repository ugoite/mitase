// FEAT-CHECK-001
// REQ-CORE-002

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

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
    if !workspace.config.validate.require_symbol_trace_coverage {
        return;
    }

    let mut targets = match discover_rust_targets(&workspace.root) {
        Ok(targets) => targets,
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    };

    match discover_python_targets(&workspace.config, &workspace.root) {
        Ok(python_targets) => targets.extend(python_targets),
        Err(issue) => {
            issues.push(*issue);
            return;
        }
    }

    match discover_typescript_targets(&workspace.root) {
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

fn discover_rust_targets(root: &Path) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let mut targets = Vec::new();
    let mut files = rust_files_under(&root.join("src"))?;
    files.extend(rust_files_under(&root.join("tests"))?);
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

fn rust_files_under(root: &Path) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_rust_files_recursive(root, &mut files).map_err(|error| {
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

fn discover_python_targets(
    config: &SyuConfig,
    root: &Path,
) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    let src_dir = root.join("src");
    let tests_dir = root.join("tests");

    let src_files = python_files_under(&src_dir)?;
    let test_files = python_files_under(&tests_dir)?;

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

fn python_files_under(root: &Path) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(root, "py", &mut files).map_err(|error| {
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

fn collect_rust_files_recursive(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    collect_files_recursive_by_extension(directory, "rs", files)
}

fn discover_typescript_targets(root: &Path) -> Result<Vec<CoverageTarget>, Box<Issue>> {
    const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

    let src_dir = root.join("src");
    let tests_dir = root.join("tests");

    let mut src_files: Vec<PathBuf> = Vec::new();
    let mut test_files: Vec<PathBuf> = Vec::new();

    for ext in TS_EXTENSIONS {
        src_files.extend(typescript_files_under(&src_dir, ext)?);
        test_files.extend(typescript_files_under(&tests_dir, ext)?);
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
        let symbols = match inspect_typescript_file(path, &contents) {
            Ok(s) => s,
            Err(_) => continue,
        };
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
        let symbols = match inspect_typescript_file(path, &contents) {
            Ok(s) => s,
            Err(_) => continue,
        };
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

fn typescript_files_under(root: &Path, extension: &str) -> Result<Vec<PathBuf>, Box<Issue>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_files_recursive_by_extension(root, extension, &mut files).map_err(|error| {
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
    directory: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files_recursive_by_extension(&path, extension, files)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            files.push(path);
        }
    }

    Ok(())
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
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use super::{
        CoverageTargetKind, collect_feature_coverage, collect_requirement_coverage,
        discover_rust_targets, normalize_relative_path, rust_files_under,
        validate_symbol_trace_coverage,
    };
    use crate::{
        config::SyuConfig,
        model::{Feature, Requirement, TraceReference},
        workspace::Workspace,
    };

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

        let targets = discover_rust_targets(tempdir.path()).expect("targets");
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
    fn rust_files_under_handles_missing_and_invalid_roots() {
        let tempdir = tempdir().expect("tempdir");
        assert!(
            rust_files_under(&tempdir.path().join("missing"))
                .expect("missing directories should be ignored")
                .is_empty()
        );

        let file_root = tempdir.path().join("not-a-dir.rs");
        fs::write(&file_root, "pub fn nope() {}\n").expect("file");
        let issue = rust_files_under(&file_root).expect_err("file roots should fail");
        assert_eq!(issue.code, "SYU-coverage-walk-001");
    }

    #[test]
    fn discover_rust_targets_reports_parse_failures() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("src");
        fs::write(tempdir.path().join("src/broken.rs"), "pub fn broken( {}\n").expect("broken");

        let issue = discover_rust_targets(tempdir.path()).expect_err("broken rust should fail");
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
}
