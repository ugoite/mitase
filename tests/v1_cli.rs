use assert_cmd::Command;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use tempfile::tempdir;

#[test]
fn current_workspace_validates_and_reports_configured_readiness() {
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace", ".", "--range", "HEAD...HEAD"])
        .assert()
        .success();
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["readiness", "report", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["target"], "traceable");
}

#[test]
fn public_cli_does_not_expose_work_or_task_commands() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("work "))
    );
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("task "))
    );
}

#[test]
fn public_cli_does_not_expose_transitional_validation_commands() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("plan "))
    );
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("result "))
    );
}

#[test]
fn generated_spec_reference_covers_every_source_document() {
    let index =
        fs::read_to_string("docs/reference/specification/index.md").expect("generated index");
    let mut sources = Vec::new();
    collect_spec_yaml_files(Path::new("docs/mitase"), &mut sources);
    for source in sources {
        let relative = source
            .strip_prefix("docs/mitase")
            .expect("spec source path");
        let generated =
            Path::new("docs/reference/specification").join(generated_spec_path(relative));
        let page = fs::read_to_string(&generated)
            .unwrap_or_else(|error| panic!("read {}: {error}", generated.display()));
        let source_display = source.to_string_lossy();
        assert!(
            page.contains(&format!("Generated from `{source_display}`")),
            "{} does not identify its canonical source",
            generated.display()
        );
        let doc_link = generated_spec_path(relative)
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        assert!(
            index.contains(&format!("({doc_link}.md)")),
            "generated index does not link {doc_link}"
        );
    }
}

fn generated_spec_path(relative: &Path) -> PathBuf {
    let parts: Vec<_> = relative.components().collect();
    let section = match parts.as_slice() {
        [first, ..]
            if first.as_os_str() == "philosophies"
                || first.as_os_str() == "policies"
                || first.as_os_str() == "requirements" =>
        {
            "foundations"
        }
        [first, second, ..]
            if first.as_os_str() == "features" && second.as_os_str() == "public-entrypoints" =>
        {
            "contracts"
        }
        [first, ..] if first.as_os_str() == "features" => "capabilities",
        _ => panic!("unsupported specification path: {}", relative.display()),
    };
    PathBuf::from(section)
        .join(relative.file_name().expect("spec filename"))
        .with_extension("md")
}

fn collect_spec_yaml_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("spec directory") {
        let entry = entry.expect("spec entry");
        let path = entry.path();
        if path.is_dir() {
            collect_spec_yaml_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            files.push(path);
        }
    }
}

#[test]
fn obsolete_config_shape_is_rejected() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/mitase")).unwrap();
    fs::write(
        temp.path().join("mitase.yaml"),
        "schema: mitase/config/v1\nversion: 1\nspec: { root: docs/mitase }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

#[test]
fn obsolete_pre_release_fixture_is_rejected_under_canonical_mitase_identity() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/rejected/obsolete-pre-release-v1");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());

    let config = fs::read_to_string(temp.path().join("mitase.yaml")).unwrap();
    assert!(config.contains("schema: mitase/config/v1"));

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_fixture_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).unwrap();
        }
    }
}

fn initialize_fixture_git(root: &Path) {
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "mitase-tests@example.invalid"],
        vec!["config", "user.name", "Mitase Tests"],
        vec!["add", "."],
        vec!["commit", "-qm", "fixture baseline"],
    ] {
        assert!(
            ProcessCommand::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}

fn staged_validation_fixture() -> tempfile::TempDir {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v1/valid-web-app");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());
    let config_path = temp.path().join("mitase.yaml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace(
            "require_owned_changes: true",
            "require_owned_changes: false",
        )
        .replace("require_plan: true", "require_plan: false");
    fs::write(config_path, config).unwrap();
    initialize_fixture_git(temp.path());
    temp
}

#[test]
fn staged_change_validation_uses_the_index_snapshot() {
    let temp = staged_validation_fixture();
    let feature = temp.path().join("spec/feature.yaml");
    fs::write(
        &feature,
        format!("{}\n", fs::read_to_string(&feature).unwrap()),
    )
    .unwrap();
    assert!(
        ProcessCommand::new("git")
            .args(["add", "spec/feature.yaml"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );

    fs::write(temp.path().join("mitase.yaml"), "not: [valid\n").unwrap();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .success();
}

#[test]
fn staged_change_validation_rejects_invalid_index_content_and_invalid_options() {
    let temp = staged_validation_fixture();
    let config_path = temp.path().join("mitase.yaml");
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, "not: [valid\n").unwrap();
    assert!(
        ProcessCommand::new("git")
            .args(["add", "mitase.yaml"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );
    fs::write(&config_path, original).unwrap();

    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .args(["--staged", "--baseline", "parent"])
        .assert()
        .failure();
}
