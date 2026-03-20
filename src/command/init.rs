// REQ-CORE-009

use anyhow::{Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cli::InitArgs,
    config::{SyuConfig, render_config},
};

#[cfg(test)]
const GENERATED_PATHS: &[&str] = &[
    "syu.yaml",
    "docs/syu/philosophy/foundation.yaml",
    "docs/syu/policies/policies.yaml",
    "docs/syu/requirements/core/core.yaml",
    "docs/syu/features/features.yaml",
    "docs/syu/features/core/core.yaml",
];

// FEAT-INIT-001
pub fn run_init_command(args: &InitArgs) -> Result<i32> {
    let workspace = prepare_workspace_root(&args.workspace)?;
    let project_name = args
        .name
        .clone()
        .unwrap_or_else(|| infer_project_name(&workspace));

    let files = scaffold_files(&project_name);
    ensure_writable_targets(
        &workspace,
        files.iter().map(|(path, _)| PathBuf::from(path)),
        args.force,
    )?;

    for (relative_path, contents) in files {
        let full_path = workspace.join(&relative_path);
        let parent = full_path
            .parent()
            .expect("generated scaffold paths should always have a parent");
        fs::create_dir_all(parent)?;
        fs::write(full_path, contents)?;
    }

    println!("initialized syu workspace at {}", workspace.display());
    Ok(0)
}

fn prepare_workspace_root(path: &Path) -> Result<PathBuf> {
    if path.exists() && !path.is_dir() {
        bail!("workspace path `{}` is not a directory", path.display());
    }

    fs::create_dir_all(path)?;
    path.canonicalize().map_err(Into::into)
}

fn infer_project_name(workspace: &Path) -> String {
    workspace
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("project")
        .to_string()
}

fn ensure_writable_targets(
    workspace: &Path,
    targets: impl Iterator<Item = PathBuf>,
    force: bool,
) -> Result<()> {
    if force {
        return Ok(());
    }

    let existing: Vec<_> = targets
        .map(|target| workspace.join(target))
        .filter(|path| path.exists())
        .collect();

    if existing.is_empty() {
        return Ok(());
    }

    let paths = existing
        .iter()
        .map(|path| format!("`{}`", path.display()))
        .collect::<Vec<_>>()
        .join(", ");
    bail!("refusing to overwrite existing files without --force: {paths}");
}

fn scaffold_files(project_name: &str) -> Vec<(String, String)> {
    vec![
        (
            "syu.yaml".to_string(),
            render_default_config().expect("config template should render"),
        ),
        (
            "docs/syu/philosophy/foundation.yaml".to_string(),
            philosophy_template(project_name),
        ),
        (
            "docs/syu/policies/policies.yaml".to_string(),
            policy_template(project_name),
        ),
        (
            "docs/syu/requirements/core/core.yaml".to_string(),
            requirement_template(project_name),
        ),
        (
            "docs/syu/features/features.yaml".to_string(),
            feature_registry_template(),
        ),
        (
            "docs/syu/features/core/core.yaml".to_string(),
            feature_template(project_name),
        ),
    ]
}

fn render_default_config() -> Result<String> {
    render_config(&SyuConfig::default())
}

fn philosophy_template(project_name: &str) -> String {
    format!(
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: {project_name} should turn intent into executable agreements\n    product_design_principle: |\n      The project should keep philosophy, policy, requirements, and features\n      explicit enough that contributors can validate changes mechanically.\n    coding_guideline: |\n      Prefer stable IDs, typed data, and explicit traceability over conventions\n      that live only in contributor memory.\n    linked_policies:\n      - POL-001\n"
    )
}

fn policy_template(project_name: &str) -> String {
    format!(
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Every change in {project_name} should remain traceable\n    summary: Define rules that turn philosophy into a verifiable workflow.\n    description: |\n      A specification entry is only useful when contributors can trace it to\n      concrete requirements, features, code, and tests inside the repository.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n"
    )
}

fn requirement_template(project_name: &str) -> String {
    format!(
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Bootstrap {project_name} with a four-layer specification\n    description: |\n      The project should keep philosophy, policy, requirements, and features in\n      YAML so contributors can evolve behavior deliberately.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {{}}\n"
    )
}

fn feature_registry_template() -> String {
    format!(
        "version: \"{}\"\nupdated: \"generated by syu init\"\n\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
        SyuConfig::default().version
    )
}

fn feature_template(project_name: &str) -> String {
    format!(
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Bootstrap the {project_name} spec workspace\n    summary: Provide a starter structure that contributors can extend.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {{}}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::cli::InitArgs;

    use super::{
        GENERATED_PATHS, ensure_writable_targets, infer_project_name, run_init_command,
        scaffold_files,
    };

    #[test]
    fn infer_project_name_uses_workspace_directory() {
        assert_eq!(
            infer_project_name(std::path::Path::new("/tmp/example-project")),
            "example-project"
        );
    }

    #[test]
    fn scaffold_files_include_all_expected_templates() {
        let files = scaffold_files("demo");
        let paths: Vec<_> = files.into_iter().map(|(path, _)| path).collect();
        assert_eq!(paths.len(), GENERATED_PATHS.len());
        for expected in GENERATED_PATHS {
            assert!(paths.iter().any(|path| path == expected));
        }
    }

    #[test]
    fn ensure_writable_targets_reports_existing_files_without_force() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::write(tempdir.path().join("syu.yaml"), "existing").expect("file should exist");

        let error = ensure_writable_targets(
            tempdir.path(),
            [std::path::PathBuf::from("syu.yaml")].into_iter(),
            false,
        )
        .expect_err("overwrite should be rejected");

        assert!(error.to_string().contains("refusing to overwrite"));
    }

    #[test]
    fn init_command_creates_scaffolded_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let args = InitArgs {
            workspace: workspace.clone(),
            name: Some("demo".to_string()),
            force: false,
        };

        let code = run_init_command(&args).expect("init should succeed");
        assert_eq!(code, 0);

        for path in GENERATED_PATHS {
            assert!(
                workspace.join(path).exists(),
                "missing generated file: {path}"
            );
        }
    }

    #[test]
    fn init_command_honors_force_overwrite() {
        let tempdir = tempdir().expect("tempdir should exist");
        fs::create_dir_all(tempdir.path()).expect("workspace dir");
        fs::write(tempdir.path().join("syu.yaml"), "old").expect("old config");

        let args = InitArgs {
            workspace: tempdir.path().to_path_buf(),
            name: Some("forced".to_string()),
            force: true,
        };

        run_init_command(&args).expect("force init should succeed");
        let config =
            fs::read_to_string(tempdir.path().join("syu.yaml")).expect("config should exist");
        assert!(config.contains("default_fix: false"));
        assert!(config.contains("allow_planned: true"));
        assert!(config.contains(crate::config::current_cli_version()));
    }

    #[test]
    fn scaffold_files_default_to_planned_status() {
        let files = scaffold_files("demo");
        let requirement = files
            .iter()
            .find(|(path, _)| path == "docs/syu/requirements/core/core.yaml")
            .expect("requirement template");
        let feature = files
            .iter()
            .find(|(path, _)| path == "docs/syu/features/core/core.yaml")
            .expect("feature template");

        assert!(requirement.1.contains("status: planned"));
        assert!(feature.1.contains("status: planned"));
    }

    #[test]
    fn init_command_rejects_file_workspaces() {
        let tempdir = tempdir().expect("tempdir should exist");
        let file_path = tempdir.path().join("workspace-file");
        fs::write(&file_path, "not a directory").expect("file should exist");

        let error = run_init_command(&InitArgs {
            workspace: file_path.clone(),
            name: None,
            force: false,
        })
        .expect_err("file workspace should be rejected");

        assert!(error.to_string().contains(&format!(
            "workspace path `{}` is not a directory",
            file_path.display()
        )));
    }

    #[test]
    fn init_command_reports_directory_creation_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        fs::write(workspace.join("docs"), "blocking file").expect("blocking file should exist");

        let error = run_init_command(&InitArgs {
            workspace,
            name: Some("demo".to_string()),
            force: false,
        })
        .expect_err("directory creation should fail");

        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn init_command_reports_file_write_failures() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        fs::create_dir_all(workspace.join("docs/syu/features/core/core.yaml"))
            .expect("blocking directory should exist");

        let error = run_init_command(&InitArgs {
            workspace,
            name: Some("demo".to_string()),
            force: true,
        })
        .expect_err("file write should fail");

        assert!(!error.to_string().is_empty());
    }
}
