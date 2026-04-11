// REQ-CORE-009
// FEAT-INIT-003
// FEAT-INIT-002

use anyhow::{Result, bail};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    cli::{InitArgs, OutputFormat},
    command::shell_quote_path,
    config::{SyuConfig, render_config},
    coverage::normalize_relative_path,
};

const DEFAULT_SPEC_ROOT: &str = "docs/syu";

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
    let spec_root = resolve_init_spec_root(args.spec_root.as_deref())?;

    let files = scaffold_files(&project_name, &spec_root);
    ensure_writable_targets(
        &workspace,
        files.iter().map(|(path, _)| PathBuf::from(path)),
        args.force,
    )?;

    for (relative_path, contents) in &files {
        let full_path = workspace.join(relative_path);
        let parent = full_path
            .parent()
            .expect("generated scaffold paths should always have a parent");
        fs::create_dir_all(parent)?;
        fs::write(full_path, contents)?;
    }

    let created_paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();

    match args.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "workspace": workspace.display().to_string(),
                    "created_files": created_paths,
                })
            );
        }
        OutputFormat::Text => {
            let workspace_arg = shell_quote_path(&workspace);
            let absolute_spec_root = workspace.join(&spec_root);
            println!("initialized syu workspace at {}", workspace.display());
            println!();
            println!("Created files:");
            for path in &created_paths {
                println!("  {path}");
            }
            println!();
            println!("What to do next:");
            println!(
                "  1. Edit the spec files in {}/ to describe your project",
                absolute_spec_root.display()
            );
            println!(
                "     - {}  (core principles)",
                path_label(&spec_root.join("philosophy/foundation.yaml"))
            );
            println!(
                "     - {}  (governance rules)",
                path_label(&spec_root.join("policies/policies.yaml"))
            );
            println!(
                "     - {}  (concrete requirements)",
                path_label(&spec_root.join("requirements/core/core.yaml"))
            );
            println!(
                "     - {}  (feature definitions)",
                path_label(&spec_root.join("features/core/core.yaml"))
            );
            println!("  2. Run `syu validate {workspace_arg}` to check your spec for consistency");
            println!(
                "  3. Run `syu browse {workspace_arg}` for terminal exploration, or `syu app {workspace_arg}` for the browser UI"
            );
            println!("  4. Commit the generated files to version control");
        }
    }

    Ok(0)
}

fn prepare_workspace_root(path: &Path) -> Result<PathBuf> {
    if path.exists() && !path.is_dir() {
        bail!("workspace path `{}` is not a directory", path.display());
    }

    fs::create_dir_all(path)?;
    path.canonicalize().map_err(Into::into)
}

fn resolve_init_spec_root(spec_root: Option<&Path>) -> Result<PathBuf> {
    let raw = spec_root.unwrap_or_else(|| Path::new(DEFAULT_SPEC_ROOT));
    let normalized = normalize_relative_path(raw);

    if normalized.as_os_str().is_empty()
        || normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "`--spec-root` must stay within the workspace root as a relative path: `{}`",
            raw.display()
        );
    }

    Ok(PathBuf::from(path_label(&normalized)))
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

fn scaffold_files(project_name: &str, spec_root: &Path) -> Vec<(String, String)> {
    vec![
        (
            "syu.yaml".to_string(),
            render_default_config(spec_root).expect("config template should render"),
        ),
        (
            path_label(&spec_root.join("philosophy/foundation.yaml")),
            philosophy_template(project_name),
        ),
        (
            path_label(&spec_root.join("policies/policies.yaml")),
            policy_template(project_name),
        ),
        (
            path_label(&spec_root.join("requirements/core/core.yaml")),
            requirement_template(project_name),
        ),
        (
            path_label(&spec_root.join("features/features.yaml")),
            feature_registry_template(),
        ),
        (
            path_label(&spec_root.join("features/core/core.yaml")),
            feature_template(project_name),
        ),
    ]
}

fn render_default_config(spec_root: &Path) -> Result<String> {
    let mut config = SyuConfig::default();
    config.spec.root = spec_root.to_path_buf();
    render_config(&config)
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        DEFAULT_SPEC_ROOT, GENERATED_PATHS, ensure_writable_targets, infer_project_name,
        resolve_init_spec_root, run_init_command, scaffold_files,
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
        let files = scaffold_files("demo", std::path::Path::new(DEFAULT_SPEC_ROOT));
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
            spec_root: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
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
            spec_root: None,
            force: true,
            format: crate::cli::OutputFormat::Text,
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
        let files = scaffold_files("demo", std::path::Path::new(DEFAULT_SPEC_ROOT));
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
            spec_root: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
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
            spec_root: None,
            force: false,
            format: crate::cli::OutputFormat::Text,
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
            spec_root: None,
            force: true,
            format: crate::cli::OutputFormat::Text,
        })
        .expect_err("file write should fail");

        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn init_command_json_format_includes_created_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join("demo");
        let args = InitArgs {
            workspace: workspace.clone(),
            name: Some("demo".to_string()),
            spec_root: None,
            force: false,
            format: crate::cli::OutputFormat::Json,
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
    fn resolve_init_spec_root_normalizes_relative_paths() {
        assert_eq!(
            resolve_init_spec_root(Some(std::path::Path::new("./spec/./contracts")))
                .expect("spec root should normalize"),
            std::path::PathBuf::from("spec/contracts")
        );
        assert_eq!(
            resolve_init_spec_root(None).expect("default spec root should resolve"),
            std::path::PathBuf::from(DEFAULT_SPEC_ROOT)
        );
    }

    #[test]
    fn resolve_init_spec_root_rejects_paths_outside_workspace() {
        let parent = resolve_init_spec_root(Some(std::path::Path::new("../spec")))
            .expect_err("parent paths should fail");
        assert!(parent.to_string().contains("--spec-root"));

        let absolute = resolve_init_spec_root(Some(std::path::Path::new("/tmp/spec")))
            .expect_err("absolute paths should fail");
        assert!(absolute.to_string().contains("--spec-root"));
    }
}
