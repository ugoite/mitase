// FEAT-CHECK-001
// REQ-CORE-001

use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::{SyuConfig, load_config, resolve_spec_root},
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

// FEAT-CHECK-001
pub fn load_workspace(root: &Path) -> Result<Workspace> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve workspace root `{}`", root.display()))?;
    let loaded_config = load_config(&root)?;
    let spec_root = resolve_spec_root(&root, &loaded_config.config);
    let philosophy_docs = load_philosophy_documents(&spec_root.join("philosophy"))?;
    let policy_docs = load_policy_documents(&spec_root.join("policies"))?;
    let requirement_docs = load_requirement_documents(&spec_root.join("requirements"))?;

    let feature_root = spec_root.join("features");
    let registry_path = feature_root.join("features.yaml");
    let registry = load_feature_registry(&registry_path)?;
    if registry.files.is_empty() {
        bail!(
            "feature registry `{}` does not declare any feature files",
            registry_path.display()
        );
    }

    let mut features = Vec::new();
    for file in registry.files {
        let path = feature_root.join(&file.file);
        let document = load_feature_document(&path, &file.kind)?;
        features.extend(document.features);
    }

    if features.is_empty() {
        bail!(
            "no feature definitions were found under `{}`",
            feature_root.display()
        );
    }

    Ok(Workspace {
        root,
        spec_root,
        config: loaded_config.config,
        philosophies: philosophy_docs
            .into_iter()
            .flat_map(|document| document.philosophies)
            .collect(),
        policies: policy_docs
            .into_iter()
            .flat_map(|document| document.policies)
            .collect(),
        requirements: requirement_docs
            .into_iter()
            .flat_map(|document| document.requirements)
            .collect(),
        features,
    })
}

fn load_philosophy_documents(directory: &Path) -> Result<Vec<PhilosophyDocument>> {
    let files = ensure_yaml_directory(directory, "philosophy")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("philosophy document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: PhilosophyDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(document);
    }
    Ok(documents)
}

fn load_policy_documents(directory: &Path) -> Result<Vec<PolicyDocument>> {
    let files = ensure_yaml_directory(directory, "policy")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("policy document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: PolicyDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(document);
    }
    Ok(documents)
}

fn load_requirement_documents(directory: &Path) -> Result<Vec<RequirementDocument>> {
    let files = ensure_yaml_directory(directory, "requirement")?;
    let mut documents = Vec::new();
    for path in files {
        let label = format!("requirement document `{}`", path.display());
        let raw = read_yaml_text(&path, &label)?;
        let document: RequirementDocument = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse {label} from `{}`", path.display()))?;
        documents.push(document);
    }
    Ok(documents)
}

fn load_feature_registry(path: &Path) -> Result<FeatureRegistryDocument> {
    let label = "feature registry";
    let raw = read_yaml_text(path, label)?;
    serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {label} from `{}`", path.display()))
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

fn yaml_file_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory `{}`", directory.display()))?;
    let mut files = Vec::new();

    for entry in entries {
        let path = entry?.path();
        if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml")
        ) {
            files.push(path);
        }
    }

    Ok(files)
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
        ensure_yaml_directory, load_feature_document, load_feature_registry,
        load_philosophy_documents, load_policy_documents, load_requirement_documents,
        load_workspace, read_yaml_text, yaml_file_paths,
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
        assert_eq!(workspace.requirements.len(), 3);
        assert_eq!(workspace.features.len(), 3);
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

        assert!(load_philosophy_documents(&missing_dir).is_err());
        assert!(load_policy_documents(&missing_dir).is_err());
        assert!(load_requirement_documents(&missing_dir).is_err());

        assert!(load_philosophy_documents(&philosophy_dir).is_err());
        assert!(load_policy_documents(&policy_dir).is_err());
        assert!(load_requirement_documents(&requirement_dir).is_err());
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
    fn yaml_file_paths_reports_missing_directories() {
        let error = yaml_file_paths(Path::new("/definitely/missing-syu-directory"))
            .expect_err("missing directory should fail");
        assert!(error.to_string().contains("failed to read directory"));
    }
}
