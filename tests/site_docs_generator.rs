use std::{fs, path::Path, path::PathBuf, process::Command};

use serde_json::json;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn posix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
/// REQ-CORE-010
fn site_docs_generator_accepts_absolute_spec_roots_outside_repo() {
    let repo = tempdir().expect("temp repo should exist");
    let external = tempdir().expect("external spec root should exist");

    let temp_repo = repo.path();
    let external_spec_root = external.path().join("syu-spec");
    let external_feature = external_spec_root.join("features").join("features.yaml");

    fs::create_dir_all(temp_repo.join("scripts")).expect("scripts directory should exist");
    fs::create_dir_all(
        external_feature
            .parent()
            .expect("feature parent should exist"),
    )
    .expect("external feature directory should exist");

    fs::copy(
        repo_root().join("scripts/generate-site-docs.py"),
        temp_repo.join("scripts/generate-site-docs.py"),
    )
    .expect("generator script should copy");

    fs::write(
        &external_feature,
        "version: 1\nfeatures:\n  - id: FEAT-EXT-001\n    title: External feature\n",
    )
    .expect("external feature spec should write");

    let config = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "spec": {
            "root": posix_path(&external_spec_root),
        }
    });
    fs::write(
        temp_repo.join("syu.yaml"),
        serde_yaml::to_string(&config).expect("config should serialize"),
    )
    .expect("config should write");

    let output = Command::new("python3")
        .arg(temp_repo.join("scripts/generate-site-docs.py"))
        .current_dir(temp_repo)
        .output()
        .expect("generator should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated_feature =
        fs::read_to_string(temp_repo.join("docs/generated/site-spec/features/features.md"))
            .expect("generated feature page should exist");
    let generated_index = fs::read_to_string(temp_repo.join("docs/generated/site-spec/index.md"))
        .expect("generated index should exist");

    assert!(generated_feature.contains(&posix_path(&external_feature)));
    assert!(generated_index.contains(&posix_path(&external_spec_root)));
}
