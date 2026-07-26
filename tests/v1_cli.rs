use assert_cmd::Command;
use std::{fs, path::Path, process::Command as ProcessCommand};
use syu_work_model::WorkPlan;
use tempfile::tempdir;

#[test]
fn current_workspace_validates_and_reports_configured_readiness() {
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "workspace", "."])
        .assert()
        .success();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["readiness", "report", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["target"], "traceable");
}

#[test]
fn generated_spec_reference_covers_every_source_document() {
    let index = fs::read_to_string("docs/generated/site-spec/index.md").expect("generated index");
    for entry in fs::read_dir("docs/syu").expect("spec directory") {
        let entry = entry.expect("spec entry");
        let source = entry.path();
        if source.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let stem = source
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("spec stem");
        let generated = Path::new("docs/generated/site-spec").join(format!("{stem}.md"));
        let page = fs::read_to_string(&generated)
            .unwrap_or_else(|error| panic!("read {}: {error}", generated.display()));
        let source_display = source.to_string_lossy();
        assert!(
            page.contains(&format!("Generated from `{source_display}`")),
            "{} does not identify its canonical source",
            generated.display()
        );
        assert!(
            index.contains(&format!("({stem})")),
            "generated index does not link {stem}"
        );
    }
}

#[test]
fn obsolete_config_shape_is_rejected() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/syu")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        "schema: syu/config/v1\nversion: 1\nspec: { root: docs/syu }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

#[test]
fn workbench_projection_does_not_implicitly_plan() {
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args([
            "workbench",
            "project",
            "--workspace",
            ".",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let projection: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(projection["work"]["request"].is_null());
    assert!(projection["work"]["plan"].is_null());
    assert_eq!(projection["diagnostics"]["validation"]["state"], "not_run");
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
        vec!["config", "user.email", "syu-tests@example.invalid"],
        vec!["config", "user.name", "Syu Tests"],
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
    let config_path = temp.path().join("syu.yaml");
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

    fs::write(temp.path().join("syu.yaml"), "not: [valid\n").unwrap();
    Command::cargo_bin("syu")
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
    let config_path = temp.path().join("syu.yaml");
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, "not: [valid\n").unwrap();
    assert!(
        ProcessCommand::new("git")
            .args(["add", "syu.yaml"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );
    fs::write(&config_path, original).unwrap();

    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .args(["--staged", "--baseline", "parent"])
        .assert()
        .failure();
}

fn run_cli_post_state_flow(out_of_scope: bool) -> bool {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v1/valid-workbench-flow");
    let temp = tempdir().unwrap();
    let artifacts = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());
    initialize_fixture_git(temp.path());

    let plan_path = artifacts.path().join("plan.yaml");
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(temp.path().join("work.yaml"))
        .args(["--out"])
        .arg(&plan_path)
        .args(["--workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "plan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: WorkPlan = serde_yaml::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    let slice = plan
        .slices
        .iter()
        .find(|slice| !slice.verification_targets.is_empty())
        .unwrap()
        .id
        .clone();

    let context_path = artifacts.path().join("context.yaml");
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "export-context", "--plan"])
        .arg(&plan_path)
        .args(["--slice"])
        .arg(&slice)
        .args(["--workspace"])
        .arg(temp.path())
        .args(["--out"])
        .arg(&context_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "context export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    if out_of_scope {
        fs::write(
            temp.path().join("src/unrelated.rs"),
            "pub const UNRELATED: bool = true;\n",
        )
        .unwrap();
    } else {
        fs::write(
            temp.path().join("src/lib.rs"),
            "pub fn behavior() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();
    }

    let receipt_path = artifacts.path().join("receipt.yaml");
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "verify", "--plan"])
        .arg(&plan_path)
        .args(["--slice"])
        .arg(&slice)
        .args(["--workspace"])
        .arg(temp.path())
        .args(["--out"])
        .arg(&receipt_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "verification failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "result"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan_path)
        .args(["--receipt"])
        .arg(&receipt_path)
        .args(["--format", "json"])
        .output()
        .unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid result output: {error}; status={} stdout={} stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
    let result_is_valid = report["status"] == "complete";
    if out_of_scope {
        assert!(
            !output.status.success(),
            "out-of-scope result unexpectedly passed"
        );
        assert!(!report["blockers"].as_array().unwrap().is_empty());
    } else {
        assert!(
            output.status.success(),
            "unexpected result status: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(result_is_valid, "in-scope result had error diagnostics");
    }
    output.status.success()
}

#[test]
fn cli_post_state_flow_accepts_editable_change_and_rejects_out_of_scope_change() {
    assert!(run_cli_post_state_flow(false));
    assert!(!run_cli_post_state_flow(true));
}
