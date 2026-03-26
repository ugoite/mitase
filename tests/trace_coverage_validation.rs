// REQ-CORE-002

use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, cover_everything: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the graph explicit\n    product_design_principle: Every layer should be connected.\n    coding_guideline: Prefer explicit ownership.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Coverage can be enforced when needed\n    summary: Public symbols and tests may require ownership.\n    description: This fixture turns the strict coverage rule on.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    let requirement_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - covered_case\n"
    };
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Tests must stay justified\n    description: Each test should link to a requirement.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/coverage.rs\n{requirement_symbols}",
        ),
    )
    .expect("requirement");

    let feature_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - covered_api\n"
    };
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core.yaml"),
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Public APIs must stay owned\n    summary: Each public API should link to a feature.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/lib.rs\n{feature_symbols}",
        ),
    )
    .expect("feature");

    fs::write(
        root.join("src/lib.rs"),
        "/// FEAT-001\npub fn covered_api() {}\n\npub fn uncovered_api() {}\n",
    )
    .expect("source");
    fs::write(
        root.join("tests/coverage.rs"),
        "/// REQ-001\n#[test]\nfn covered_case() {}\n\n#[test]\nfn uncovered_case() {}\n",
    )
    .expect("tests");
}

#[test]
fn validate_reports_untracked_public_symbols_and_tests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "coverage gaps should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-coverage-public-001"));
    assert!(stdout.contains("SYU-coverage-test-001"));
}

#[test]
fn validate_accepts_wildcard_file_coverage() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_python_workspace(root: &Path, cover_everything: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the graph explicit\n    product_design_principle: Every layer should be connected.\n    coding_guideline: Prefer explicit ownership.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Coverage can be enforced when needed\n    summary: Public symbols and tests may require ownership.\n    description: This fixture turns the strict coverage rule on.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    let requirement_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - test_covered\n"
    };
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Tests must stay justified\n    description: Each test should link to a requirement.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      python:\n        - file: tests/test_coverage.py\n{requirement_symbols}",
        ),
    )
    .expect("requirement");

    let feature_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - covered_api\n"
    };
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core.yaml"),
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Public APIs must stay owned\n    summary: Each public API should link to a feature.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      python:\n        - file: src/api.py\n{feature_symbols}",
        ),
    )
    .expect("feature");

    fs::write(
        root.join("src/api.py"),
        "\"\"\"FEAT-001\"\"\"\n\ndef covered_api():\n    pass\n\ndef uncovered_api():\n    pass\n",
    )
    .expect("source");
    fs::write(
        root.join("tests/test_coverage.py"),
        "\"\"\"REQ-001\"\"\"\n\ndef test_covered():\n    pass\n\ndef test_uncovered():\n    pass\n",
    )
    .expect("tests");
}

#[test]
fn validate_reports_untracked_python_symbols_and_tests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_python_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "python coverage gaps should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SYU-coverage-public-001"),
        "expected public coverage error, got:\n{stdout}"
    );
    assert!(
        stdout.contains("SYU-coverage-test-001"),
        "expected test coverage error, got:\n{stdout}"
    );
}

#[test]
fn validate_accepts_wildcard_python_coverage() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_python_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn python_private_symbols_are_not_required_to_be_traced() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_python_workspace(tempdir.path(), false);

    // Add a private function that should NOT trigger a coverage error
    fs::write(
        tempdir.path().join("src/api.py"),
        "\"\"\"FEAT-001\"\"\"\n\ndef covered_api():\n    pass\n\ndef uncovered_api():\n    pass\n\ndef _private_helper():\n    pass\n",
    )
    .expect("source with private fn");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("_private_helper"),
        "private symbols should be excluded from coverage, got:\n{stdout}"
    );
}

fn write_typescript_workspace(root: &Path, cover_everything: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the graph explicit\n    product_design_principle: Every layer should be connected.\n    coding_guideline: Prefer explicit ownership.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Every symbol must be owned\n    summary: Public symbols and tests may require ownership.\n    description: This fixture turns the strict coverage rule on.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    let requirement_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - testCoveredTs\n"
    };
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Tests must stay justified\n    description: Each test should link to a requirement.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      typescript:\n        - file: tests/coverage.test.ts\n{requirement_symbols}",
        ),
    )
    .expect("requirement");

    let feature_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - coveredApi\n"
    };
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core.yaml"),
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Public APIs must stay owned\n    summary: Each public API should link to a feature.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      typescript:\n        - file: src/api.ts\n{feature_symbols}",
        ),
    )
    .expect("feature");

    fs::write(
        root.join("src/api.ts"),
        "// FEAT-001\n\nexport function coveredApi(): boolean { return true; }\nexport function uncoveredApi(): boolean { return false; }\n",
    )
    .expect("source");
    fs::write(
        root.join("tests/coverage.test.ts"),
        "// REQ-001\n\nexport function testCoveredTs(): void {}\nexport function testUncoveredTs(): void {}\n",
    )
    .expect("tests");
}

#[test]
fn validate_reports_untracked_typescript_symbols_and_tests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_typescript_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "typescript coverage gaps should fail"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SYU-coverage-public-001"),
        "expected public coverage error, got:\n{stdout}"
    );
    assert!(
        stdout.contains("SYU-coverage-test-001"),
        "expected test coverage error, got:\n{stdout}"
    );
}

#[test]
fn validate_accepts_wildcard_typescript_coverage() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_typescript_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn typescript_non_exported_symbols_are_not_required_to_be_traced() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_typescript_workspace(tempdir.path(), false);

    // Add a non-exported function that should NOT trigger a coverage error
    fs::write(
        tempdir.path().join("src/api.ts"),
        "// FEAT-001\n\nexport function coveredApi(): boolean { return true; }\nexport function uncoveredApi(): boolean { return false; }\nfunction internalHelper(): void {}\n",
    )
    .expect("source with internal fn");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("internalHelper"),
        "non-exported symbols should be excluded from coverage, got:\n{stdout}"
    );
}
