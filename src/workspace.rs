// FEAT-CHECK-001
// REQ-CORE-001

use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    config::{CONFIG_FILE_NAME, SyuConfig, load_config, resolve_spec_root},
    model::{
        Feature, FeatureDocument, FeatureRegistryDocument, Philosophy, PhilosophyDocument, Policy,
        PolicyDocument, Requirement, RequirementDocument,
    },
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub spec_root: PathBuf,
    pub config: SyuConfig,
    pub philosophies: Vec<Philosophy>,
    pub policies: Vec<Policy>,
    pub requirements: Vec<Requirement>,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedDocument<T> {
    pub path: PathBuf,
    pub document: T,
}

pub fn resolve_workspace_root(root: &Path) -> Result<PathBuf> {
    let resolved = root
        .canonicalize()
        .with_context(|| format!("failed to resolve workspace root `{}`", root.display()))?;
    if resolved.is_dir() && looks_like_workspace_root(&resolved) {
        return Ok(resolved);
    }
    let search_root = if resolved.is_dir() {
        resolved.clone()
    } else {
        resolved.parent().unwrap_or(&resolved).to_path_buf()
    };

    for candidate in search_root.ancestors() {
        if candidate.join(CONFIG_FILE_NAME).is_file() {
            return Ok(candidate.to_path_buf());
        }
    }

    Ok(search_root)
}

fn looks_like_workspace_root(root: &Path) -> bool {
    let spec_root = resolve_spec_root(root, &SyuConfig::default());
    spec_root.join("philosophy").is_dir()
        && spec_root.join("policies").is_dir()
        && spec_root.join("requirements").is_dir()
        && spec_root.join("features").is_dir()
        && spec_root.join("features/features.yaml").is_file()
}

// FEAT-CHECK-001
pub fn load_workspace(root: &Path) -> Result<Workspace> {
    let root = resolve_workspace_root(root)?;
    let loaded_config = load_config(&root)?;
    let spec_root = resolve_spec_root(&root, &loaded_config.config);
    let philosophy_docs = load_philosophy_documents_with_paths(&spec_root.join("philosophy"))?;
    let policy_docs = load_policy_documents_with_paths(&spec_root.join("policies"))?;
    let requirement_docs = load_requirement_documents_with_paths(&spec_root.join("requirements"))?;
    let feature_docs = load_feature_documents_with_paths(&spec_root.join("features"))?;

    let philosophies = philosophy_docs
        .into_iter()
        .flat_map(|loaded| loaded.document.philosophies)
        .collect();
    let policies = policy_docs
        .into_iter()
        .flat_map(|loaded| loaded.document.policies)
        .collect();
    let requirements = requirement_docs
        .into_iter()
        .flat_map(|loaded| loaded.document.requirements)
        .collect();
    let features = feature_docs
        .into_iter()
        .flat_map(|loaded| loaded.document.features)
        .collect::<Vec<_>>();

    if features.is_empty() {
        bail!(
            "no feature definitions were found under `{}`",
            spec_root.join("features").display()
        );
    }

    Ok(Workspace {
        root,
        spec_root,
        config: loaded_config.config,
        philosophies,
        policies,
        requirements,
        features,
    })
}

pub(crate) fn load_philosophy_documents_with_paths(
    directory: &Path,
) -> Result<Vec<LoadedDocument<PhilosophyDocument>>> {
    let files = ensure_yaml_directory(directory, "philosophy")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("philosophy document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: PhilosophyDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(LoadedDocument { path, document });
    }
    Ok(documents)
}

pub(crate) fn load_policy_documents_with_paths(
    directory: &Path,
) -> Result<Vec<LoadedDocument<PolicyDocument>>> {
    let files = ensure_yaml_directory(directory, "policy")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("policy document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: PolicyDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(LoadedDocument { path, document });
    }
    Ok(documents)
}

pub(crate) fn load_requirement_documents_with_paths(
    directory: &Path,
) -> Result<Vec<LoadedDocument<RequirementDocument>>> {
    let files = ensure_yaml_directory_recursive(directory, "requirement")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("requirement document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: RequirementDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(LoadedDocument { path, document });
    }
    Ok(documents)
}

fn load_feature_registry(path: &Path) -> Result<FeatureRegistryDocument> {
    let label = "feature registry";
    let raw = read_yaml_text(path, label)?;
    serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {label} from `{}`", path.display()))
}

pub(crate) fn load_feature_documents_with_paths(
    feature_root: &Path,
) -> Result<Vec<LoadedDocument<FeatureDocument>>> {
    let registry_path = feature_root.join("features.yaml");
    let registry = load_feature_registry(&registry_path)?;
    if registry.files.is_empty() {
        bail!(
            "feature registry `{}` does not declare any feature files",
            registry_path.display()
        );
    }

    let mut documents = Vec::new();
    for file in registry.files {
        let path = resolve_feature_document_path(feature_root, &file.file)?;
        let document = load_feature_document(&path, &file.kind)?;
        documents.push(LoadedDocument { path, document });
    }
    Ok(documents)
}

fn load_feature_document(path: &Path, kind: &str) -> Result<FeatureDocument> {
    let label = format!("feature document `{}` ({kind})", path.display());
    let raw = read_yaml_text(path, &label)?;
    serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {label} from `{}`", path.display()))
}

fn ensure_yaml_directory(directory: &Path, label: &str) -> Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        bail!("missing {label} directory `{}`", directory.display());
    }

    let mut files = yaml_file_paths(directory)?;
    if files.is_empty() {
        bail!("no YAML files found in `{}`", directory.display());
    }

    files.sort();
    Ok(files)
}

fn ensure_yaml_directory_recursive(directory: &Path, label: &str) -> Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        bail!("missing {label} directory `{}`", directory.display());
    }

    let mut files = yaml_file_paths_recursive(directory)?;
    if files.is_empty() {
        bail!("no YAML files found in `{}`", directory.display());
    }

    files.sort();
    Ok(files)
}

fn yaml_file_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory `{}`", directory.display()))?;
    let mut files = Vec::new();

    for entry in entries {
        let path = entry?.path();
        if is_yaml_path(&path) {
            files.push(path);
        }
    }

    Ok(files)
}

fn yaml_file_paths_recursive(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_yaml_file_paths(directory, &mut files)?;
    Ok(files)
}

fn collect_yaml_file_paths(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory `{}`", directory.display()))?;

    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml_file_paths(&path, files)?;
        } else if is_yaml_path(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_yaml_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

fn resolve_feature_document_path(feature_root: &Path, relative_path: &Path) -> Result<PathBuf> {
    if relative_path.is_absolute() {
        bail!(
            "feature registry entry must use a relative path inside `{}`: `{}`",
            feature_root.display(),
            relative_path.display()
        );
    }

    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!(
            "feature registry entry must stay within `{}`: `{}`",
            feature_root.display(),
            relative_path.display()
        );
    }

    if !is_yaml_path(relative_path) {
        bail!(
            "feature registry entry must point to a YAML file: `{}`",
            relative_path.display()
        );
    }

    Ok(feature_root.join(relative_path))
}

fn read_yaml_text(path: &Path, label: &str) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read {label} from `{}`", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use super::{
        ensure_yaml_directory, ensure_yaml_directory_recursive, load_feature_document,
        load_feature_registry, load_philosophy_documents_with_paths,
        load_policy_documents_with_paths, load_requirement_documents_with_paths, load_workspace,
        read_yaml_text, resolve_feature_document_path, resolve_workspace_root, yaml_file_paths,
        yaml_file_paths_recursive,
    };

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    #[test]
    fn load_workspace_reads_passing_fixture() {
        let workspace = load_workspace(&fixture_root("passing")).expect("fixture should load");
        assert_eq!(workspace.philosophies.len(), 1);
        assert_eq!(workspace.policies.len(), 2);
        assert_eq!(workspace.requirements.len(), 5);
        assert_eq!(workspace.features.len(), 5);
    }

    #[test]
    fn load_workspace_reads_nested_requirement_and_feature_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements/core")).expect("dir");
        fs::create_dir_all(spec_root.join("features/core")).expect("dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: PHIL-1\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n    linked_policies:\n      - POL-1\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL-1\n    title: T\n    summary: S\n    description: D\n    linked_philosophies:\n      - PHIL-1\n    linked_requirements:\n      - REQ-1\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-1\n    linked_features:\n      - FEAT-1\n    tests: {}\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.1'\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/core/core.yaml"),
            "category: Core\nversion: 1\nfeatures:\n  - id: FEAT-1\n    title: T\n    summary: S\n    status: planned\n    linked_requirements:\n      - REQ-1\n    implementations: {}\n",
        )
        .expect("write");

        let workspace = load_workspace(tempdir.path()).expect("nested workspace should load");
        assert_eq!(workspace.requirements.len(), 1);
        assert_eq!(workspace.features.len(), 1);
        assert_eq!(workspace.requirements[0].id, "REQ-1");
        assert_eq!(workspace.features[0].id, "FEAT-1");
    }

    #[test]
    fn resolve_workspace_root_discovers_parent_config_from_child_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let nested = workspace_root.join("src/nested");
        fs::create_dir_all(&nested).expect("nested dir");
        fs::write(workspace_root.join("syu.yaml"), "version: 1\n").expect("config");

        let resolved =
            resolve_workspace_root(&nested).expect("parent workspace root should resolve");
        assert_eq!(
            resolved,
            workspace_root
                .canonicalize()
                .expect("workspace root should canonicalize")
        );
    }

    #[test]
    fn resolve_workspace_root_discovers_parent_config_from_workspace_file_path() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let source_file = workspace_root.join("src/lib.rs");
        fs::create_dir_all(source_file.parent().expect("parent dir")).expect("source dir");
        fs::write(workspace_root.join("syu.yaml"), "version: 1\n").expect("config");
        fs::write(&source_file, "pub fn demo() {}\n").expect("source file");

        let resolved =
            resolve_workspace_root(&source_file).expect("parent workspace root should resolve");
        assert_eq!(
            resolved,
            workspace_root
                .canonicalize()
                .expect("workspace root should canonicalize")
        );
    }

    #[test]
    fn resolve_workspace_root_keeps_explicit_root_when_config_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        fs::create_dir_all(&workspace_root).expect("workspace dir");

        let resolved =
            resolve_workspace_root(&workspace_root).expect("workspace root should resolve");
        assert_eq!(
            resolved,
            workspace_root
                .canonicalize()
                .expect("workspace root should canonicalize")
        );
    }

    #[test]
    fn resolve_workspace_root_ignores_ancestor_default_spec_root_without_config() {
        let tempdir = tempdir().expect("tempdir should exist");
        let parent = tempdir.path().join("parent");
        let nested = parent.join("child/frontend");
        fs::create_dir_all(parent.join("docs/syu/features")).expect("spec dir");
        fs::create_dir_all(&nested).expect("nested dir");

        let resolved =
            resolve_workspace_root(&nested).expect("workspace root should resolve safely");
        assert_eq!(
            resolved,
            nested
                .canonicalize()
                .expect("nested path should canonicalize")
        );
    }

    #[test]
    fn load_workspace_discovers_parent_workspace_from_child_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let nested = workspace_root.join("frontend");
        let spec_root = workspace_root.join("docs/syu");
        fs::create_dir_all(&nested).expect("nested dir");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements/core")).expect("dir");
        fs::create_dir_all(spec_root.join("features/core")).expect("dir");
        fs::write(
            workspace_root.join("syu.yaml"),
            format!(
                "version: {}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\napp:\n  bind: 127.0.0.1\n  port: 3000\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("config");
        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: PHIL-1\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n    linked_policies:\n      - POL-1\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL-1\n    title: T\n    summary: S\n    description: D\n    linked_philosophies:\n      - PHIL-1\n    linked_requirements:\n      - REQ-1\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-1\n    linked_features:\n      - FEAT-1\n    tests: {}\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.1'\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/core/core.yaml"),
            "category: Core\nversion: 1\nfeatures:\n  - id: FEAT-1\n    title: T\n    summary: S\n    status: planned\n    linked_requirements:\n      - REQ-1\n    implementations: {}\n",
        )
        .expect("write");

        let workspace =
            load_workspace(&nested).expect("nested workspace path should discover the root");
        assert_eq!(
            workspace.root,
            workspace_root
                .canonicalize()
                .expect("workspace root should canonicalize")
        );
        assert_eq!(workspace.requirements[0].id, "REQ-1");
        assert_eq!(workspace.features[0].id, "FEAT-1");
    }

    #[test]
    fn load_workspace_fails_for_missing_root() {
        let error = load_workspace(Path::new("/definitely/missing/syu-workspace"))
            .expect_err("missing root should fail");
        assert!(
            error
                .to_string()
                .contains("failed to resolve workspace root")
        );
    }

    #[test]
    fn load_workspace_fails_when_feature_registry_is_empty() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("dir");
        fs::create_dir_all(spec_root.join("features")).expect("dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: P\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL\n    title: T\n    summary: S\n    description: D\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: implemented\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.1'\nfiles: []\n",
        )
        .expect("write");

        let error = load_workspace(tempdir.path()).expect_err("empty registry should fail");
        assert!(
            error
                .to_string()
                .contains("does not declare any feature files")
        );
    }

    #[test]
    fn load_workspace_fails_when_philosophy_directory_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("dir");
        fs::create_dir_all(spec_root.join("features")).expect("dir");

        let error = load_workspace(tempdir.path()).expect_err("missing philosophy should fail");
        assert!(error.to_string().contains("missing philosophy directory"));
    }

    #[test]
    fn load_workspace_fails_when_feature_registry_is_invalid() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("dir");
        fs::create_dir_all(spec_root.join("features")).expect("dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: P\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL\n    title: T\n    summary: S\n    description: D\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: implemented\n",
        )
        .expect("write");
        fs::write(spec_root.join("features/features.yaml"), "version: [\n").expect("write");

        let error = load_workspace(tempdir.path()).expect_err("invalid registry should fail");
        assert!(
            error
                .to_string()
                .contains("failed to parse feature registry")
        );
    }

    #[test]
    fn load_workspace_fails_when_feature_document_is_invalid() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("dir");
        fs::create_dir_all(spec_root.join("features")).expect("dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: P\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL\n    title: T\n    summary: S\n    description: D\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: implemented\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.1'\nfiles:\n  - kind: broken\n    file: broken.yaml\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/broken.yaml"),
            "category: Broken\nversion: [\n",
        )
        .expect("write");

        let error =
            load_workspace(tempdir.path()).expect_err("invalid feature document should fail");
        assert!(
            error
                .to_string()
                .contains("failed to parse feature document")
        );
    }

    #[test]
    fn load_workspace_fails_when_feature_documents_have_no_features() {
        let tempdir = tempdir().expect("tempdir should exist");
        let spec_root = tempdir.path().join("docs/syu");
        fs::create_dir_all(spec_root.join("philosophy")).expect("dir");
        fs::create_dir_all(spec_root.join("policies")).expect("dir");
        fs::create_dir_all(spec_root.join("requirements")).expect("dir");
        fs::create_dir_all(spec_root.join("features")).expect("dir");

        fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nphilosophies:\n  - id: PHIL-1\n    title: T\n    product_design_principle: A\n    coding_guideline: B\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\npolicies:\n  - id: POL-1\n    title: T\n    summary: S\n    description: D\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: T\n    description: D\n    priority: high\n    status: implemented\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/features.yaml"),
            "version: '0.1'\nfiles:\n  - kind: empty\n    file: empty.yaml\n",
        )
        .expect("write");
        fs::write(
            spec_root.join("features/empty.yaml"),
            "category: Empty\nversion: 1\nfeatures: []\n",
        )
        .expect("write");

        let error = load_workspace(tempdir.path()).expect_err("empty feature docs should fail");
        assert!(
            error
                .to_string()
                .contains("no feature definitions were found")
        );
    }

    #[test]
    fn load_document_helpers_report_missing_or_invalid_inputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        let philosophy_dir = tempdir.path().join("philosophy");
        let policy_dir = tempdir.path().join("policies");
        let requirement_dir = tempdir.path().join("requirements");
        let missing_dir = tempdir.path().join("missing");
        fs::create_dir_all(&philosophy_dir).expect("dir");
        fs::create_dir_all(&policy_dir).expect("dir");
        fs::create_dir_all(&requirement_dir).expect("dir");

        fs::write(philosophy_dir.join("broken.yaml"), "category: [\n").expect("write");
        fs::write(policy_dir.join("broken.yaml"), "category: [\n").expect("write");
        fs::write(requirement_dir.join("broken.yaml"), "category: [\n").expect("write");

        assert!(load_philosophy_documents_with_paths(&missing_dir).is_err());
        assert!(load_policy_documents_with_paths(&missing_dir).is_err());
        assert!(load_requirement_documents_with_paths(&missing_dir).is_err());

        assert!(load_philosophy_documents_with_paths(&philosophy_dir).is_err());
        assert!(load_policy_documents_with_paths(&policy_dir).is_err());
        assert!(load_requirement_documents_with_paths(&requirement_dir).is_err());
    }

    #[test]
    fn feature_file_helpers_report_invalid_inputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        let registry = tempdir.path().join("features.yaml");
        let feature = tempdir.path().join("feature.yaml");
        let missing = tempdir.path().join("missing.yaml");

        fs::write(&registry, "version: [\n").expect("write");
        fs::write(&feature, "category: Broken\nversion: [\n").expect("write");

        assert!(load_feature_registry(&missing).is_err());
        assert!(load_feature_registry(&registry).is_err());
        assert!(load_feature_document(&missing, "broken").is_err());
        assert!(load_feature_document(&feature, "broken").is_err());
    }

    #[test]
    fn resolve_feature_document_path_rejects_escaping_paths() {
        let feature_root = Path::new("/tmp/syu/features");

        let absolute = resolve_feature_document_path(feature_root, Path::new("/tmp/escape.yaml"))
            .expect_err("absolute paths should fail");
        assert!(absolute.to_string().contains("must use a relative path"));

        let parent_dir = resolve_feature_document_path(feature_root, Path::new("../escape.yaml"))
            .expect_err("parent traversal should fail");
        assert!(parent_dir.to_string().contains("must stay within"));

        let not_yaml = resolve_feature_document_path(feature_root, Path::new("nested/escape.txt"))
            .expect_err("non-yaml files should fail");
        assert!(not_yaml.to_string().contains("must point to a YAML file"));
    }

    #[test]
    fn ensure_yaml_directory_fails_for_missing_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let error = ensure_yaml_directory(&tempdir.path().join("missing"), "philosophy")
            .expect_err("missing directory should fail");

        assert!(error.to_string().contains("missing philosophy directory"));
    }

    #[test]
    fn ensure_yaml_directory_fails_for_empty_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let directory = tempdir.path().join("docs/syu/philosophy");
        fs::create_dir_all(&directory).expect("directory should exist");

        let error = ensure_yaml_directory(&directory, "philosophy")
            .expect_err("empty directory should fail");

        assert!(error.to_string().contains("no YAML files found"));
    }

    #[test]
    fn ensure_yaml_directory_recursive_fails_for_empty_directory() {
        let tempdir = tempdir().expect("tempdir should exist");
        let directory = tempdir.path().join("docs/syu/requirements");
        fs::create_dir_all(&directory).expect("directory should exist");

        let error = ensure_yaml_directory_recursive(&directory, "requirement")
            .expect_err("empty directory should fail");

        assert!(error.to_string().contains("no YAML files found"));
    }

    #[test]
    fn read_yaml_text_reads_existing_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let file = tempdir.path().join("doc.yaml");
        fs::write(&file, "hello: world\n").expect("yaml file should be written");

        let raw = read_yaml_text(&file, "fixture").expect("yaml text should load");
        assert_eq!(raw, "hello: world\n");
    }

    #[test]
    fn read_yaml_text_reports_missing_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let error = read_yaml_text(&tempdir.path().join("missing.yaml"), "fixture")
            .expect_err("missing file should fail");
        assert!(error.to_string().contains("failed to read fixture"));
    }

    #[test]
    fn yaml_file_paths_only_collect_yaml_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join("a.yaml"), "a: 1\n").expect("write");
        fs::write(tempdir.path().join("b.yml"), "b: 2\n").expect("write");
        fs::write(tempdir.path().join("c.txt"), "c\n").expect("write");

        let mut files = yaml_file_paths(tempdir.path()).expect("yaml files should be listed");
        files.sort();

        let names: Vec<_> = files
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();
        assert_eq!(names, vec!["a.yaml", "b.yml"]);
    }

    #[test]
    fn yaml_file_paths_recursive_collect_nested_yaml_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::create_dir_all(tempdir.path().join("nested/deeper")).expect("dirs");
        fs::write(tempdir.path().join("a.yaml"), "a: 1\n").expect("write");
        fs::write(tempdir.path().join("nested/b.yml"), "b: 2\n").expect("write");
        fs::write(tempdir.path().join("nested/deeper/c.txt"), "c\n").expect("write");

        let mut files =
            yaml_file_paths_recursive(tempdir.path()).expect("yaml files should be listed");
        files.sort();

        let names: Vec<_> = files
            .iter()
            .map(|path| {
                path.strip_prefix(tempdir.path())
                    .expect("relative path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(names, vec!["a.yaml", "nested/b.yml"]);
    }

    #[test]
    fn yaml_file_paths_reports_missing_directories() {
        let error = yaml_file_paths(Path::new("/definitely/missing-syu-directory"))
            .expect_err("missing directory should fail");
        assert!(error.to_string().contains("failed to read directory"));
    }
}
