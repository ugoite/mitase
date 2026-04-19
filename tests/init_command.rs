use assert_cmd::cargo::CommandCargoExt;
use serde_yaml::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
// REQ-CORE-009
fn init_command_bootstraps_a_workspace_that_validate_accepts() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--name")
        .arg("demo")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(workspace.join("syu.yaml").exists());
    assert!(workspace.join("docs/syu/features/core/core.yaml").exists());

    let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
    let parsed_config: Value = serde_yaml::from_str(&config).expect("config should be valid yaml");
    let requirement = fs::read_to_string(workspace.join("docs/syu/requirements/core/core.yaml"))
        .expect("requirement should exist");
    let feature = fs::read_to_string(workspace.join("docs/syu/features/core/core.yaml"))
        .expect("feature should exist");

    assert!(config.contains(env!("CARGO_PKG_VERSION")));
    assert_eq!(parsed_config["app"]["bind"].as_str(), Some("127.0.0.1"));
    assert_eq!(parsed_config["app"]["port"].as_u64(), Some(3000));
    assert_eq!(
        parsed_config["validate"]["require_reciprocal_links"].as_bool(),
        Some(true)
    );
    assert!(requirement.contains("status: planned"));
    assert!(feature.contains("status: planned"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
    assert!(String::from_utf8_lossy(&validate.stdout).contains("syu validate passed"));
}

#[test]
// FEAT-INIT-007
fn init_command_interactive_requires_a_terminal() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--interactive")
        .output()
        .expect("init should run");

    assert!(
        !init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        String::from_utf8_lossy(&init.stderr)
            .contains("`syu init --interactive` requires a terminal")
    );
    assert!(
        !workspace.exists(),
        "non-terminal interactive init should fail before creating the workspace"
    );
}

#[test]
// REQ-CORE-009
fn init_help_lists_go_only_template_as_a_supported_starter() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg("--help")
        .output()
        .expect("help should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--template <TEMPLATE>"));
    assert!(stdout.contains("go-only"));
    assert!(stdout.contains("syu init . --template go-only"));
}

#[test]
// REQ-CORE-009
fn init_command_bootstraps_language_templates_that_validate_accept() {
    for (template, requirement_path, feature_path, requirement_id, feature_id) in [
        (
            "docs-first",
            "docs/syu/requirements/core/docs.yaml",
            "docs/syu/features/documentation/docs.yaml",
            "REQ-DOCS-001",
            "FEAT-DOCS-001",
        ),
        (
            "rust-only",
            "docs/syu/requirements/core/rust.yaml",
            "docs/syu/features/languages/rust.yaml",
            "REQ-RUST-001",
            "FEAT-RUST-001",
        ),
        (
            "python-only",
            "docs/syu/requirements/core/python.yaml",
            "docs/syu/features/languages/python.yaml",
            "REQ-PY-001",
            "FEAT-PY-001",
        ),
        (
            "go-only",
            "docs/syu/requirements/core/go.yaml",
            "docs/syu/features/languages/go.yaml",
            "REQ-GO-001",
            "FEAT-GO-001",
        ),
        (
            "polyglot",
            "docs/syu/requirements/core/polyglot.yaml",
            "docs/syu/features/languages/polyglot.yaml",
            "REQ-MIX-001",
            "FEAT-MIX-001",
        ),
    ] {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path().join(template);

        let init = Command::cargo_bin("syu")
            .expect("binary should build")
            .arg("init")
            .arg(&workspace)
            .arg("--template")
            .arg(template)
            .output()
            .expect("init should run");

        assert!(
            init.status.success(),
            "template={template}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&init.stdout),
            String::from_utf8_lossy(&init.stderr)
        );
        assert!(
            workspace.join(requirement_path).exists(),
            "missing {requirement_path}"
        );
        assert!(
            workspace.join(feature_path).exists(),
            "missing {feature_path}"
        );

        let requirement =
            fs::read_to_string(workspace.join(requirement_path)).expect("requirement should exist");
        let feature =
            fs::read_to_string(workspace.join(feature_path)).expect("feature should exist");
        assert!(
            requirement.contains(requirement_id),
            "missing {requirement_id}"
        );
        assert!(feature.contains(feature_id), "missing {feature_id}");
        if template == "docs-first" {
            assert!(workspace.join("scripts/publish-docs.sh").exists());
            assert!(workspace.join("config/navigation.yaml").exists());
            assert!(requirement.contains("status: implemented"));
            assert!(requirement.contains("DocsFirstAcceptanceChecklist"));
            assert!(feature.contains("status: implemented"));
            assert!(feature.contains("publish_release_notes"));
            #[cfg(unix)]
            {
                let mode = fs::metadata(workspace.join("scripts/publish-docs.sh"))
                    .expect("script metadata should exist")
                    .permissions()
                    .mode();
                assert_ne!(
                    mode & 0o111,
                    0,
                    "docs-first starter script should be executable"
                );
            }
        } else if template == "go-only" {
            assert!(workspace.join("go.mod").exists(), "missing go.mod");
            assert!(workspace.join("go/app.go").exists(), "missing go/app.go");
            assert!(
                workspace.join("go/app_test.go").exists(),
                "missing go/app_test.go"
            );
            let go_mod = fs::read_to_string(workspace.join("go.mod")).expect("go.mod should exist");
            assert!(go_mod.contains("module example.com/go-only"));
            assert!(go_mod.contains("go 1.19"));
            assert!(requirement.contains("status: implemented"));
            assert!(requirement.contains("TestGoRequirement"));
            assert!(feature.contains("status: implemented"));
            assert!(feature.contains("GoFeatureImpl"));
        } else {
            assert!(requirement.contains("status: planned"));
            assert!(feature.contains("status: planned"));
        }

        let validate = Command::cargo_bin("syu")
            .expect("binary should build")
            .arg("validate")
            .arg(&workspace)
            .output()
            .expect("validate should run");

        assert!(
            validate.status.success(),
            "template={template}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&validate.stdout),
            String::from_utf8_lossy(&validate.stderr)
        );
    }
}

#[test]
// REQ-CORE-009
fn init_command_bootstraps_a_custom_spec_root_that_validate_accepts() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--name")
        .arg("demo")
        .arg("--spec-root")
        .arg("spec/contracts")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(workspace.join("syu.yaml").exists());
    assert!(
        workspace
            .join("spec/contracts/features/core/core.yaml")
            .exists()
    );

    let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
    let parsed_config: Value = serde_yaml::from_str(&config).expect("config should be valid yaml");
    assert_eq!(
        parsed_config["spec"]["root"].as_str(),
        Some("spec/contracts")
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(stdout.contains(&format!("{}/", workspace.join("spec/contracts").display())));
    assert!(stdout.contains("spec/contracts/philosophy/foundation.yaml"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
    assert!(String::from_utf8_lossy(&validate.stdout).contains("syu validate passed"));
}

#[test]
// REQ-CORE-009
fn init_command_combines_custom_spec_roots_with_language_templates() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--template")
        .arg("rust-only")
        .arg("--spec-root")
        .arg("spec/contracts")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        workspace
            .join("spec/contracts/requirements/core/rust.yaml")
            .exists()
    );
    assert!(
        workspace
            .join("spec/contracts/features/languages/rust.yaml")
            .exists()
    );

    let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
    let parsed_config: Value = serde_yaml::from_str(&config).expect("config should be valid yaml");
    assert_eq!(
        parsed_config["spec"]["root"].as_str(),
        Some("spec/contracts")
    );

    let registry = fs::read_to_string(workspace.join("spec/contracts/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(registry.contains("kind: rust"));
    assert!(registry.contains("file: languages/rust.yaml"));

    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(stdout.contains("spec/contracts/requirements/core/rust.yaml"));
    assert!(stdout.contains("spec/contracts/features/languages/rust.yaml"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
// REQ-CORE-009
fn init_command_bootstraps_project_specific_id_prefixes_that_validate_accept() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--id-prefix")
        .arg("store")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let philosophy = fs::read_to_string(workspace.join("docs/syu/philosophy/foundation.yaml"))
        .expect("philosophy should exist");
    let policy = fs::read_to_string(workspace.join("docs/syu/policies/policies.yaml"))
        .expect("policy should exist");
    let requirement = fs::read_to_string(workspace.join("docs/syu/requirements/core/core.yaml"))
        .expect("requirement should exist");
    let feature = fs::read_to_string(workspace.join("docs/syu/features/core/core.yaml"))
        .expect("feature should exist");

    assert!(philosophy.contains("id: PHIL-STORE-001"));
    assert!(policy.contains("id: POL-STORE-001"));
    assert!(requirement.contains("prefix: REQ-STORE"));
    assert!(requirement.contains("id: REQ-STORE-001"));
    assert!(feature.contains("id: FEAT-STORE-001"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
// REQ-CORE-009
fn init_command_allows_layer_specific_id_prefix_overrides() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--template")
        .arg("rust-only")
        .arg("--id-prefix")
        .arg("store")
        .arg("--philosophy-prefix")
        .arg("phil-guiding")
        .arg("--policy-prefix")
        .arg("pol-governance")
        .arg("--requirement-prefix")
        .arg("req-auth")
        .arg("--feature-prefix")
        .arg("feat-auth")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let philosophy = fs::read_to_string(workspace.join("docs/syu/philosophy/foundation.yaml"))
        .expect("philosophy should exist");
    let policy = fs::read_to_string(workspace.join("docs/syu/policies/policies.yaml"))
        .expect("policy should exist");
    let requirement = fs::read_to_string(workspace.join("docs/syu/requirements/core/rust.yaml"))
        .expect("requirement should exist");
    let feature = fs::read_to_string(workspace.join("docs/syu/features/languages/rust.yaml"))
        .expect("feature should exist");

    assert!(philosophy.contains("id: PHIL-GUIDING-001"));
    assert!(philosophy.contains("linked_policies:\n      - POL-GOVERNANCE-001"));
    assert!(policy.contains("id: POL-GOVERNANCE-001"));
    assert!(requirement.contains("prefix: REQ-AUTH"));
    assert!(requirement.contains("id: REQ-AUTH-001"));
    assert!(feature.contains("id: FEAT-AUTH-001"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
// REQ-CORE-009
fn init_command_rejects_full_prefixes_as_shared_stems() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--id-prefix")
        .arg("REQ-STORE")
        .output()
        .expect("init should run");

    assert!(
        !init.status.success(),
        "init should reject full typed stems"
    );
    assert!(String::from_utf8_lossy(&init.stderr).contains("--id-prefix"));
}

#[test]
// REQ-CORE-009
fn init_command_requires_force_when_generated_files_exist() {
    let tempdir = tempdir().expect("tempdir should exist");
    fs::write(
        tempdir.path().join("syu.yaml"),
        format!("version: {}\n", env!("CARGO_PKG_VERSION")),
    )
    .expect("config should exist");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(tempdir.path())
        .output()
        .expect("init should run");

    assert!(!init.status.success(), "init should refuse overwrite");
    assert!(String::from_utf8_lossy(&init.stderr).contains("refusing to overwrite"));
}

#[test]
// REQ-CORE-009
fn init_command_prints_workspace_aware_next_steps_for_explicit_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo workspace");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    let workspace_arg = format!("'{}'", workspace.display());
    assert!(stdout.contains(&format!("Run `syu validate {workspace_arg}`")));
    assert!(stdout.contains(&format!("Run `syu browse {workspace_arg}`")));
    assert!(stdout.contains(&format!("`syu app {workspace_arg}`")));
    assert!(stdout.contains("Run `syu templates` before another `syu init`"));
    assert!(stdout.contains(&format!("{}/", workspace.join("docs/syu").display())));
}

#[test]
// REQ-CORE-009
fn init_command_rejects_spec_roots_outside_the_workspace() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--spec-root")
        .arg("../spec")
        .output()
        .expect("init should run");

    assert!(
        !init.status.success(),
        "init should reject invalid spec roots"
    );
    assert!(String::from_utf8_lossy(&init.stderr).contains("--spec-root"));
}
