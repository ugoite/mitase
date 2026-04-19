use std::{collections::HashSet, fs, path::PathBuf};

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_file(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect("repository file should exist")
}

fn read_json(path: &str) -> Value {
    serde_json::from_str(&read_file(path)).expect("repository JSON should parse")
}

#[test]
// REQ-CORE-005
fn repository_declares_precommit_and_quality_gates() {
    let precommit = read_file(".pre-commit-config.yaml");
    let install_precommit = read_file("scripts/install-precommit.sh");
    let quality_script = read_file("scripts/ci/quality-gates.sh");
    let validate_app_script = read_file("scripts/ci/validate-app.sh");
    let validate_website_script = read_file("scripts/ci/validate-website.sh");
    let install_docs_site_deps_script = read_file("scripts/ci/install-docs-site-deps.sh");
    let ci_workflow = read_file(".github/workflows/ci.yml");
    let contributing = read_file("CONTRIBUTING.md");
    let repo_config = read_file("syu.yaml");

    assert!(precommit.contains("FEAT-QUALITY-001"));
    assert!(precommit.contains("shellcheck"));
    assert!(precommit.contains("syu-validate-self"));
    assert!(precommit.contains("syu-quality-gates"));
    assert!(precommit.contains("syu-coverage-gate"));

    assert!(quality_script.contains("FEAT-QUALITY-001"));
    assert!(quality_script.contains("run_quality_gates"));
    assert!(quality_script.contains("cargo fmt --all --check"));
    assert!(quality_script.contains("cargo clippy --all-targets --all-features -- -D warnings"));
    assert!(quality_script.contains("cargo test"));
    assert!(quality_script.contains("cargo run -- validate ."));
    assert!(quality_script.contains("check-generated-docs-freshness.sh"));
    assert!(validate_app_script.contains("FEAT-QUALITY-001"));
    assert!(validate_app_script.contains("validate_app"));
    assert!(validate_app_script.contains("scripts/ci/quality-gates.sh"));
    assert!(validate_app_script.contains("check-browser-app-freshness.sh"));
    assert!(validate_app_script.contains("npm --prefix app run test:e2e"));
    assert!(validate_website_script.contains("FEAT-QUALITY-001"));
    assert!(validate_website_script.contains("validate_website"));
    assert!(validate_website_script.contains("scripts/ci/quality-gates.sh"));
    assert!(validate_website_script.contains("install-docs-site-deps.sh"));
    assert!(validate_website_script.contains("npm --prefix website run build"));
    assert!(install_docs_site_deps_script.contains("Branch switches can leave behind"));
    assert!(install_docs_site_deps_script.contains("website/node_modules"));
    assert!(install_docs_site_deps_script.contains("shutil.rmtree"));
    assert!(install_docs_site_deps_script.contains("npm --prefix website ci"));
    assert!(install_precommit.contains("site --user-base"));
    assert!(install_precommit.contains("pipx environment --value PIPX_BIN_DIR"));
    assert!(install_precommit.contains("Troubleshooting: compare"));
    assert!(install_precommit.contains("If you installed pre-commit with pipx"));
    assert!(install_precommit.contains("Checked Python user-base path:"));
    assert!(install_precommit.contains("Checked pipx bin path:"));
    assert!(install_precommit.contains("pre_commit install"));

    assert!(ci_workflow.contains("FEAT-QUALITY-001"));
    assert!(ci_workflow.contains("precommit:"));
    assert!(ci_workflow.contains("quality:"));
    assert!(ci_workflow.contains("actionlint:"));
    assert!(ci_workflow.contains("dependency-audit:"));
    assert!(ci_workflow.contains("dependency-review:"));
    assert!(ci_workflow.contains("squash-history-spec-ids:"));
    assert!(ci_workflow.contains("spec-linkage:"));
    assert!(ci_workflow.contains("installer-smoke:"));
    assert!(ci_workflow.contains("installed-binary-smoke:"));
    assert!(ci_workflow.contains("--hook-stage pre-commit"));
    assert!(ci_workflow.contains("--hook-stage pre-push"));
    assert!(ci_workflow.contains("scripts/ci/quality-gates.sh"));
    assert!(ci_workflow.contains("cargo audit"));
    assert!(ci_workflow.contains("schedule:"));
    assert!(ci_workflow.contains("0 6 * * 1"));
    assert!(ci_workflow.contains("github.event_name != 'schedule'"));
    assert!(ci_workflow.contains("Review dependency changes"));
    assert!(ci_workflow.contains("Require spec IDs in squash commit titles"));
    assert!(ci_workflow.contains("scripts/ci/check-squash-title-spec-ids.sh"));
    assert!(ci_workflow.contains("Require issue or spec IDs for self-spec changes"));
    assert!(ci_workflow.contains("scripts/ci/check-pr-spec-links.sh"));
    assert!(ci_workflow.contains("scripts/ci/installer-smoke.sh"));
    assert!(ci_workflow.contains("scripts/ci/installed-binary-smoke.sh"));

    assert!(contributing.contains("weekly schedule"));
    assert!(contributing.contains("06:00 UTC"));
    assert!(contributing.contains("cargo audit"));
    assert!(contributing.contains("npm audit"));
    assert!(contributing.contains("Contributors do **not** need to run manual audits"));
    assert!(contributing.contains("check-generated-docs-freshness.sh"));
    assert!(contributing.contains("scripts/ci/validate-app.sh"));
    assert!(contributing.contains("scripts/ci/validate-website.sh"));
    assert!(contributing.contains("docs/generated/"));
    assert!(contributing.contains("python3 -m site --user-base"));
    assert!(contributing.contains("If you installed `pre-commit` with"));
    assert!(contributing.contains("pipx environment --value PIPX_BIN_DIR"));
    assert!(contributing.contains("scripts/ci/check-browser-app-freshness.sh"));

    assert!(repo_config.contains("FEAT-CHECK-001"));
    assert!(repo_config.contains("FEAT-REPORT-001"));
    assert!(repo_config.contains("require_non_orphaned_items: true"));
    assert!(repo_config.contains("require_reciprocal_links: true"));
    assert!(repo_config.contains("require_symbol_trace_coverage: true"));
    assert!(repo_config.contains("output: docs/generated/syu-report.md"));
}

#[test]
// REQ-CORE-006
fn repository_declares_coverage_gate_at_one_hundred_percent() {
    let coverage_script = read_file("scripts/ci/coverage.sh");
    let spec_summary_script = read_file("scripts/ci/write-spec-coverage-summary.py");
    let ci_workflow = read_file(".github/workflows/ci.yml");

    assert!(coverage_script.contains("FEAT-QUALITY-001"));
    assert!(coverage_script.contains("run_coverage"));
    assert!(coverage_script.contains("LINE_THRESHOLD=100"));
    assert!(coverage_script.contains("--fail-under-lines 100"));
    assert!(coverage_script.contains("cargo llvm-cov"));
    assert!(coverage_script.contains("generate_spec_coverage_summary"));
    assert!(coverage_script.contains("target/coverage/spec-coverage-summary.md"));
    assert!(coverage_script.contains("GITHUB_STEP_SUMMARY"));

    assert!(spec_summary_script.contains("FEAT-QUALITY-001"));
    assert!(spec_summary_script.contains("Coverage by requirement and feature"));
    assert!(spec_summary_script.contains("list\", \"--with-path\", \"--format\", \"json\""));
    assert!(spec_summary_script.contains("yaml.safe_load"));
    assert!(spec_summary_script.contains("Rust implementation coverage"));

    assert!(ci_workflow.contains("coverage:"));
    assert!(ci_workflow.contains("scripts/ci/coverage.sh lcov"));
    assert!(ci_workflow.contains("cargo-llvm-cov"));
    assert!(ci_workflow.contains("target/coverage/spec-coverage-summary.md"));
}

#[test]
// REQ-CORE-005
fn repository_keeps_node_majors_aligned_across_docs_packages_and_ci() {
    let contributing = read_file("CONTRIBUTING.md");
    let ci_workflow = read_file(".github/workflows/ci.yml");
    let codeql_workflow = read_file(".github/workflows/codeql.yml");
    let release_artifacts_workflow = read_file(".github/workflows/release-artifacts.yml");
    let app_nvmrc = read_file("app/.nvmrc");
    let website_nvmrc = read_file("website/.nvmrc");
    let app_package = read_json("app/package.json");
    let website_package = read_json("website/package.json");

    let app_major = app_nvmrc.trim();
    let website_major = website_nvmrc.trim();
    let app_next_major = app_major
        .parse::<u32>()
        .expect("app Node major should parse")
        + 1;
    let website_next_major = website_major
        .parse::<u32>()
        .expect("website Node major should parse")
        + 1;
    let app_engine = format!(">={app_major} <{app_next_major}");
    let website_engine = format!(">={website_major} <{website_next_major}");

    assert_eq!(app_major, "25");
    assert_eq!(website_major, "20");
    assert_eq!(
        app_package["engines"]["node"].as_str(),
        Some(app_engine.as_str())
    );
    assert_eq!(
        website_package["engines"]["node"].as_str(),
        Some(website_engine.as_str())
    );
    assert_eq!(app_package["packageManager"].as_str(), Some("npm@11.8.0"));
    assert_eq!(
        website_package["packageManager"].as_str(),
        Some("npm@11.8.0")
    );

    assert!(contributing.contains("use **Node 25** for `app/`"));
    assert!(contributing.contains("use **Node 20** for `website/`"));

    assert!(ci_workflow.contains("node-version: \"25\""));
    assert!(ci_workflow.contains("node-version: \"20\""));
    assert!(codeql_workflow.contains("node-version: \"25\""));
    assert!(release_artifacts_workflow.contains("node-version: \"25\""));
}

#[test]
// REQ-CORE-007
fn repository_declares_release_automation() {
    let release_please = read_file(".github/workflows/release-please.yml");
    let release_artifacts = read_file(".github/workflows/release-artifacts.yml");
    let package_script = read_file("scripts/ci/package-release.sh");
    let publish_script = read_file("scripts/ci/publish-package.sh");
    let release_notes_script = read_file("scripts/ci/release-track-notes.sh");
    let release_config = read_file("release-please-config.json");
    let manifest = read_file(".release-please-manifest.json");
    let readme = read_file("README.md");

    assert!(release_please.contains("FEAT-RELEASE-001"));
    assert!(release_please.contains("googleapis/release-please-action@v4.4.0"));
    assert!(release_please.contains("release-please:"));
    assert!(release_please.contains("target-branch: main"));
    assert!(release_please.contains("release-please skipped"));
    assert!(!release_please.contains("target-branch: alpha"));
    assert!(!release_please.contains("target-branch: beta"));

    assert!(release_artifacts.contains("FEAT-RELEASE-001"));
    assert!(release_artifacts.contains("release-artifacts"));
    assert!(release_artifacts.contains("x86_64-unknown-linux-gnu"));
    assert!(release_artifacts.contains("aarch64-apple-darwin"));
    assert!(release_artifacts.contains("x86_64-pc-windows-msvc"));
    assert!(release_artifacts.contains("PACKAGE_REPOSITORY"));
    assert!(release_artifacts.contains("publish-package.sh"));
    assert!(release_artifacts.contains("install-syu.sh"));
    assert!(release_artifacts.contains("release-notes:"));
    assert!(release_artifacts.contains("release-track-notes.sh"));
    assert!(release_artifacts.contains("scripts/ci/pinned-npm.sh install app"));
    assert!(release_artifacts.contains("npm --prefix app ci"));
    assert!(release_artifacts.contains("attestations: write"));
    assert!(release_artifacts.contains("id-token: write"));
    assert!(release_artifacts.contains("actions/attest-build-provenance@v4"));
    assert!(release_artifacts.contains("subject-path: release-artifacts/*"));

    assert!(package_script.contains("FEAT-RELEASE-001"));
    assert!(package_script.contains("package_release_artifact"));
    assert!(package_script.contains("write_sha256"));
    assert!(publish_script.contains("FEAT-RELEASE-001"));
    assert!(publish_script.contains("publish_package_artifact"));
    assert!(publish_script.contains("oras push"));
    assert!(release_notes_script.contains("FEAT-RELEASE-001"));
    assert!(release_notes_script.contains("previous_tag_name"));
    assert!(release_notes_script.contains("gh release edit"));

    assert!(release_config.contains("\"release-type\": \"rust\""));
    assert!(release_config.contains("\"package-name\": \"syu\""));
    assert!(!release_config.contains("\"skip-changelog\": true"));
    assert!(release_config.contains("\"changelog-type\": \"github\""));
    assert!(!release_config.contains("\"initial-version\""));
    assert!(manifest.contains("\".\": \"0.0.0\""));
    assert!(readme.contains("gh attestation verify"));
    assert!(readme.contains("--repo ugoite/syu"));
    assert!(
        !repo_root()
            .join("release-please-config.alpha.json")
            .exists()
    );
    assert!(!repo_root().join("release-please-config.beta.json").exists());
    assert!(
        !repo_root()
            .join(".release-please-manifest.alpha.json")
            .exists()
    );
    assert!(
        !repo_root()
            .join(".release-please-manifest.beta.json")
            .exists()
    );
    assert!(!repo_root().join("CHANGELOG.md").exists());
}

#[test]
// REQ-CORE-008
fn repository_declares_installer_contract() {
    let current_version = env!("CARGO_PKG_VERSION");
    let installer = read_file("scripts/install-syu.sh");
    let installer_smoke = read_file("scripts/ci/installer-smoke.sh");
    let installed_binary_smoke = read_file("scripts/ci/installed-binary-smoke.sh");
    let mock_registry = read_file("scripts/ci/mock_package_registry.py");
    let readme = read_file("README.md");
    let verify_idx = readme
        .find("Recommended: verify before running")
        .expect("README should document the verify-first installer flow");
    let shortcut_idx = readme
        .find("Shortcut: run the installer directly")
        .expect("README should still document the one-line shortcut");

    assert!(installer.contains("FEAT-INSTALL-001"));
    assert!(installer.contains("resolve_repository"));
    assert!(installer.contains("resolve_target_triple"));
    assert!(installer.contains("resolve_package_repository"));
    assert!(installer.contains("resolve_package_tag"));
    assert!(installer.contains("download_package_archive"));
    assert!(installer.contains("install_syu"));
    assert!(installer.contains("SYU_REPOSITORY"));
    assert!(installer.contains("SYU_INSTALL_DIR"));
    assert!(installer.contains("SYU_PACKAGE_REPOSITORY"));
    assert!(installer.contains("ugoite/syu"));
    assert!(installer.contains("ghcr.io"));
    assert!(installer_smoke.contains("FEAT-INSTALL-001"));
    assert!(installer_smoke.contains("run_install_case"));
    assert!(installed_binary_smoke.contains("FEAT-QUALITY-001"));
    assert!(installed_binary_smoke.contains("cargo install --path"));
    assert!(installed_binary_smoke.contains("--locked"));
    assert!(installed_binary_smoke.contains("wait_for_app_url"));
    assert!(installed_binary_smoke.contains("wait_for_app_payload"));
    assert!(installed_binary_smoke.contains("print_app_diagnostics"));
    assert!(installed_binary_smoke.contains("api/app-data.json"));
    assert!(mock_registry.contains("FEAT-INSTALL-001"));
    assert!(mock_registry.contains("build_artifacts"));

    assert!(readme.contains("install-syu.sh"));
    assert!(readme.contains("ugoite/syu"));
    assert!(readme.contains("SYU_VERSION"));
    assert!(readme.contains(&format!("RELEASE=v{current_version}")));
    assert!(readme.contains("GitHub Packages"));
    assert!(readme.contains("security-sensitive environments"));
    assert!(readme.contains("checksums.sha256"));
    assert!(readme.contains("gh attestation verify"));
    assert!(readme.contains("PowerShell"));
    assert!(readme.contains("Git Bash"));
    assert!(readme.contains("checksums = 'checksums.sha256'"));
    assert!(readme.contains("Select-String -SimpleMatch $asset"));
    assert!(readme.contains("syu-x86_64-pc-windows-msvc.zip"));
    assert!(readme.contains("Get-FileHash"));
    assert!(readme.contains("LOCALAPPDATA"));
    assert!(readme.contains("If you are inside WSL"));
    assert!(!readme.contains("$asset.sha256"));
    assert!(readme.contains("syu.exe"));
    assert!(readme.contains(&format!("RELEASE=v{current_version}")));
    assert!(readme.contains("checked-in"));
    assert!(readme.contains("package track"));
    assert!(readme.contains("verifies the installer script itself"));
    assert!(readme.contains("the platform archive that the installer downloads"));
    assert!(
        verify_idx < shortcut_idx,
        "README should present the verify-first flow before the one-line shortcut"
    );
    assert!(!readme.contains("raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh"));
}

#[test]
// REQ-CORE-010
fn repository_declares_documentation_guides() {
    let current_version = env!("CARGO_PKG_VERSION");
    let readme = read_file("README.md");
    let concepts = read_file("docs/guide/concepts.md");
    let anti_patterns = read_file("docs/guide/spec-antipatterns.md");
    let app_guide = read_file("docs/guide/app.md");
    let examples_and_templates = read_file("docs/guide/examples-and-templates.md");
    let merge_queue_playbook = read_file("docs/guide/merge-queue-playbook.md");
    let getting_started = read_file("docs/guide/getting-started.md");
    let trace_adapter_support = read_file("docs/guide/trace-adapter-support.md");
    let vscode_guide = read_file("docs/guide/vscode-extension.md");
    let configuration = read_file("docs/guide/configuration.md");
    let config_overview = read_file("docs/syu/config/overview.yaml");
    let config_spec = read_file("docs/syu/config/spec.yaml");
    let config_validate = read_file("docs/syu/config/validate.yaml");
    let config_runtimes = read_file("docs/syu/config/runtimes.yaml");
    let generated_config_overview = read_file("docs/generated/site-spec/config/overview.md");
    let generated_config_spec = read_file("docs/generated/site-spec/config/spec.md");
    let generated_config_validate = read_file("docs/generated/site-spec/config/validate.md");
    let generated_config_runtimes = read_file("docs/generated/site-spec/config/runtimes.md");
    let generated_contributor =
        read_file("docs/generated/site-spec/features/repository/contributor.md");
    let generated_site_index = read_file("docs/generated/site-spec/index.md");
    let generated_validation =
        read_file("docs/generated/site-spec/features/validation/validation.md");
    let generated_docs_freshness = read_file("scripts/ci/check-generated-docs-freshness.sh");
    let ci_workflow = read_file(".github/workflows/ci.yml");
    let docs_deploy_workflow = read_file(".github/workflows/deploy-pages.yml");
    let docs_build_action = read_file(".github/actions/build-docs-site/action.yml");
    let docs_package = read_file("website/package.json");
    let docs_lock = read_file("website/package-lock.json");
    let docs_config = read_file("website/docusaurus.config.js");
    let docs_home = read_file("website/src/pages/index.js");
    let docs_css = read_file("website/src/css/custom.css");
    let docs_sidebars = read_file("website/sidebars.js");

    assert!(readme.contains("docs/guide/concepts.md"));
    assert!(readme.contains("## Choose your path"));
    assert!(readme.contains("docs/guide/trace-adapter-support.md"));
    assert!(readme.contains("Trace adapter matrix"));
    assert!(readme.contains("docs/guide/tutorial.md"));
    assert!(readme.contains("docs/guide/migration.md"));
    assert!(readme.contains("docs/guide/app.md"));
    assert!(readme.contains("docs/guide/troubleshooting.md"));
    assert!(readme.contains("docs/guide/spec-antipatterns.md"));
    assert!(readme.contains("docs/guide/vscode-extension.md"));
    assert!(readme.contains("shortest install-to-validate path"));
    assert!(readme.contains("do **not** already know the four-layer model"));
    assert!(readme.contains("**Getting started**"));
    assert!(readme.contains("**Migration / upgrade**"));
    assert!(readme.contains("**Visual explorer**"));
    assert!(readme.contains("new to `syu`"));
    assert!(readme.contains("already have a workspace"));
    assert!(readme.contains("10-15 minutes"));
    assert!(readme.contains("about 5 minutes"));
    assert!(readme.contains("longer walkthrough"));
    assert!(readme.contains("unblocking an existing workspace"));
    assert!(readme.contains("[Why four layers?](#why-four-layers)"));
    assert!(readme.contains("Step 0: required"));
    assert!(readme.contains("Generate a requirement stub"));
    assert!(readme.contains("add at least one `linked_policies:` entry"));
    assert!(readme.contains("`linked_requirements:` entry back to the new requirement"));
    assert!(readme.contains("scaffold any still-missing adjacent policy or"));
    assert!(readme.contains("feature documents so they link back to the new requirement."));
    assert!(readme.contains("syu init"));
    assert!(readme.contains("syu init ."));
    assert!(readme.contains("syu add"));
    assert!(readme.contains("--id-prefix"));
    assert!(readme.contains("--template docs-first"));
    assert!(readme.contains("--template rust-only"));
    assert!(readme.contains("--template go-only"));
    assert!(readme.contains("syu templates"));
    assert!(readme.contains("starter-only"));
    assert!(readme.contains("syu validate"));
    assert!(readme.contains("syu browse"));
    assert!(readme.contains("syu list"));
    assert!(readme.contains("list-shaped output"));
    assert!(readme.contains("workspace metadata, per-layer"));
    assert!(readme.contains("current validation errors in plain text"));
    assert!(readme.contains("syu show"));
    assert!(readme.contains("syu show REQ-001"));
    assert!(!readme.contains("syu show REQ-CORE-015"));
    assert!(readme.contains("syu app"));
    assert!(readme.contains("examples/csharp-fallback"));
    assert!(readme.contains("examples/go-only"));
    assert!(readme.contains("examples/polyglot"));
    assert!(readme.contains("examples/team-scale"));
    assert!(readme.contains("examples-and-templates.md"));
    assert!(readme.contains("CONTRIBUTING.md"));
    assert!(readme.contains("Contributing and local development"));
    assert!(readme.contains("Documentation site"));
    assert!(readme.contains("Browser app"));
    assert!(readme.contains("scripts/install-precommit.sh"));
    assert!(readme.contains("https://ugoite.github.io/syu/"));
    assert!(readme.contains("docs/syu/config/"));
    assert!(readme.contains("--spec-root"));
    assert!(!readme.contains("cargo run -- init ."));

    assert!(concepts.contains("philosophy"));
    assert!(concepts.contains("policy"));
    assert!(concepts.contains("requirements"));
    assert!(concepts.contains("features"));
    assert!(concepts.contains("planned"));
    assert!(concepts.contains("implemented"));
    assert!(concepts.contains("Continue with these pages"));
    assert!(concepts.contains("spec anti-patterns guide"));
    assert!(concepts.contains("Specification Reference"));
    assert!(anti_patterns.contains("bad-but-valid"));
    assert!(anti_patterns.contains("Philosophy that changes every sprint"));
    assert!(anti_patterns.contains("Policy that only repeats another layer"));
    assert!(anti_patterns.contains("When to merge, split, or rename spec items"));
    assert!(anti_patterns.contains("green-but-messy spec"));
    assert!(app_guide.contains("Status badge"));
    assert!(app_guide.contains("README chooser on GitHub"));
    assert!(app_guide.contains("## Search shortcuts"));
    assert!(app_guide.contains("ArrowDown"));
    assert!(app_guide.contains("Escape"));
    assert!(app_guide.contains("the item's YAML `status:` field"));
    assert!(app_guide.contains("../../website/static/img/app-guide-overview.png"));
    assert!(!app_guide.contains("](/img/"));
    assert!(!app_guide.contains("`planned`, `implemented`, or `deprecated`"));
    assert!(getting_started.contains("New to `syu`?"));
    assert!(getting_started.contains("Need a different level of guidance?"));
    assert!(getting_started.contains("README quick start"));
    assert!(
        getting_started.contains("[trace adapter capability matrix](./trace-adapter-support.md)")
    );
    assert!(
        getting_started
            .contains("https://github.com/ugoite/syu/blob/main/README.md#choose-your-path")
    );
    assert!(getting_started.contains(
        "https://github.com/ugoite/syu/blob/main/README.md#is-syu-right-for-this-repository"
    ));
    assert!(getting_started.contains("narrated first-run path"));
    assert!(getting_started.contains("first workspace setup explained step by"));
    assert!(getting_started.contains("first manual editing step"));
    assert!(getting_started.contains("compact command card"));
    assert!(getting_started.contains("install-syu.sh"));
    assert!(getting_started.contains("checksums.sha256"));
    assert!(getting_started.contains("security-sensitive environments"));
    assert!(getting_started.contains("README installer verification flow"));
    assert!(getting_started.contains("SYU_VERSION=alpha"));
    assert!(getting_started.contains("PowerShell"));
    assert!(getting_started.contains("Git Bash"));
    assert!(getting_started.contains("README PowerShell install flow"));
    assert!(getting_started.contains("syu-x86_64-pc-windows-msvc.zip"));
    assert!(getting_started.contains("LOCALAPPDATA"));
    assert!(getting_started.contains("If you are inside WSL"));
    assert!(!getting_started.contains("$asset.sha256"));
    assert!(getting_started.contains("current checked-in release"));
    assert!(getting_started.contains("latest published alpha"));
    assert!(getting_started.contains("--template docs-first"));
    assert!(getting_started.contains("--template rust-only"));
    assert!(getting_started.contains("--template go-only"));
    assert!(getting_started.contains("syu templates"));
    assert!(getting_started.contains("--id-prefix"));
    assert!(getting_started.contains("syu validate . --fix"));
    assert!(getting_started.contains("syu browse ."));
    assert!(getting_started.contains("emitted as JSON for automation"));
    assert!(getting_started.contains("workspace metadata, per-layer"));
    assert!(getting_started.contains("current validation errors in plain text"));
    assert!(getting_started.contains("syu list feature"));
    assert!(getting_started.contains("syu show REQ-001"));
    assert!(getting_started.contains("syu app ."));
    assert!(getting_started.contains("install-syu.sh"));
    assert!(getting_started.contains("SYU_VERSION=alpha"));
    assert!(getting_started.contains("--spec-root"));
    assert!(getting_started.contains("Requirements are discovered"));
    assert!(getting_started.contains("implementation claims should stay deliberate"));
    assert!(getting_started.contains(&format!("version: {current_version}")));
    assert!(getting_started.contains("kind: core"));
    assert!(getting_started.contains("freshly initialized project will not have them yet"));
    assert!(generated_docs_freshness.contains("write_sha256"));
    assert!(generated_docs_freshness.contains("sha256sum or shasum is required"));
    assert!(generated_docs_freshness.contains("shasum -a 256"));
    assert!(getting_started.contains("https://ugoite.github.io/syu/docs/generated/site-spec"));
    assert!(getting_started.contains("https://ugoite.github.io/syu/docs/generated/syu-report"));
    assert!(getting_started.contains("status: implemented"));
    assert!(getting_started.contains("Keep exploring"));
    assert!(getting_started.contains("examples/rust-only"));
    assert!(getting_started.contains("examples/python-only"));
    assert!(getting_started.contains("examples/csharp-fallback"));
    assert!(getting_started.contains("examples/go-only"));
    assert!(getting_started.contains("examples/polyglot"));
    assert!(
        getting_started.contains("[examples and templates guide](./examples-and-templates.md)")
    );
    assert!(
        getting_started
            .matches("Follow the [end-to-end tutorial](./tutorial.md)")
            .count()
            >= 2
    );
    assert!(getting_started.contains("[troubleshooting](./troubleshooting.md)"));
    assert!(getting_started.contains("live [validation report]"));
    assert!(vscode_guide.contains("syu Context"));
    assert!(vscode_guide.contains("syu validate . --format json"));
    assert!(vscode_guide.contains("Trace active file"));
    assert!(vscode_guide.contains("syu.binaryPath"));
    let tutorial = read_file("docs/guide/tutorial.md");
    assert!(tutorial.contains("Want a different entry point?"));
    assert!(tutorial.contains("[getting started](./getting-started.md)"));
    assert!(tutorial.contains("[troubleshooting](./troubleshooting.md)"));
    assert!(tutorial.contains("starter registry entry"));
    assert!(tutorial.contains("Only add another `files` entry"));
    assert!(trace_adapter_support.contains("# Trace adapter capability matrix"));
    assert!(trace_adapter_support.contains("validate.require_symbol_trace_coverage"));
    assert!(trace_adapter_support.contains("TypeScript / JavaScript"));
    assert!(trace_adapter_support.contains("Gitignore"));
    assert!(trace_adapter_support.contains("| Go | `go`, `golang`, `gotest` / `.go` |"));
    assert!(examples_and_templates.contains("starter templates"));
    assert!(examples_and_templates.contains("checked-in examples"));
    assert!(examples_and_templates.contains("examples/csharp-fallback"));
    assert!(examples_and_templates.contains("examples/docs-first"));
    assert!(examples_and_templates.contains("`syu init . --template docs-first`"));
    assert!(examples_and_templates.contains("`syu init . --template rust-only`"));
    assert!(examples_and_templates.contains("`syu init . --template go-only`"));
    assert!(examples_and_templates.contains("examples/go-only"));
    assert!(examples_and_templates.contains("examples/polyglot"));
    assert!(examples_and_templates.contains("examples/team-scale"));
    assert!(merge_queue_playbook.contains("merge_group"));
    assert!(merge_queue_playbook.contains("gh api graphql"));
    assert!(merge_queue_playbook.contains("autoMergeRequest"));
    assert!(merge_queue_playbook.contains("reviewDecision"));
    assert!(merge_queue_playbook.contains("AWAITING_CHECKS"));
    assert!(merge_queue_playbook.contains("All comments must be resolved"));
    assert!(merge_queue_playbook.contains("gh pr merge 123 --auto --squash"));
    assert!(merge_queue_playbook.contains("gh-readonly-queue/main/pr-123-<sha>"));
    assert!(configuration.contains("validate.default_fix"));
    assert!(configuration.contains("trace-adapter-support.md"));
    assert!(configuration.contains("validate.allow_planned"));
    assert!(configuration.contains("Rust, Python, Go, Java, and TypeScript/JavaScript"));
    assert!(configuration.contains("--spec-root"));
    assert!(configuration.contains(&format!("version: {current_version}")));
    assert!(configuration.contains("docs/syu/config/overview.yaml"));
    assert!(configuration.contains("docs/syu/config/validate.yaml"));
    assert!(config_overview.contains("syu.yaml"));
    assert!(config_overview.contains("version"));
    assert!(config_spec.contains("spec.root"));
    assert!(config_validate.contains("validate.default_fix"));
    assert!(config_validate.contains("validate.require_symbol_trace_coverage"));
    assert!(config_validate.contains("Rust, Python, Go, Java, and TypeScript/JavaScript"));
    assert!(config_runtimes.contains("runtimes.python.command"));
    assert!(generated_config_overview.contains("docs/syu/config/overview.yaml"));
    assert!(generated_config_overview.contains("current CLI version"));
    assert!(generated_config_spec.contains("docs/syu/config/spec.yaml"));
    assert!(generated_config_validate.contains("validate.default_fix"));
    assert!(
        generated_config_validate.contains("Rust, Python, Go, Java, and TypeScript/JavaScript")
    );
    assert!(generated_config_validate.contains("array&lt;path&gt;"));
    assert!(generated_config_runtimes.contains("docs/syu/config/runtimes.yaml"));
    assert!(generated_contributor.contains("Closes #123"));
    assert!(generated_site_index.contains("features/cli/show-list"));
    assert!(generated_site_index.contains("features/validation"));
    assert!(generated_validation.contains("docs/syu/features/validation/validation.yaml"));
    assert!(generated_validation.contains("SYU-graph-reference-001"));
    assert!(
        generated_validation
            .contains("Rust, Python, Go, Java, and TypeScript/JavaScript source and test files")
    );
    assert!(generated_docs_freshness.contains("FEAT-QUALITY-001"));
    assert!(generated_docs_freshness.contains("check_generated_docs_freshness"));
    assert!(generated_docs_freshness.contains("python3 scripts/generate-site-docs.py"));
    assert!(generated_docs_freshness.contains("docs/generated/syu-report.md"));
    assert!(generated_docs_freshness.contains("git --no-pager diff --stat -- docs/generated"));
    assert!(ci_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_build_action.contains("FEAT-DOCS-002"));
    assert!(docs_build_action.contains("actions/setup-node@v6"));
    assert!(docs_build_action.contains("cache-dependency-path: website/package-lock.json"));
    assert!(docs_build_action.contains("install-docs-site-deps.sh"));
    assert!(docs_build_action.contains("npm run build"));
    assert!(docs_package.contains("@docusaurus/core"));
    assert!(docs_lock.contains("\"name\": \"syu-docs\""));
    assert!(docs_lock.contains("\"lockfileVersion\":"));
    assert!(docs_package.contains("\"build\": \"docusaurus build\""));
    assert!(docs_deploy_workflow.contains("permissions:"));
    assert!(docs_deploy_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_deploy_workflow.contains("actions/configure-pages@v6"));
    assert!(docs_deploy_workflow.contains("actions/upload-pages-artifact@v4"));
    assert!(docs_deploy_workflow.contains("actions/deploy-pages@v5"));
    assert!(docs_deploy_workflow.contains("github-pages"));
    assert!(docs_config.contains("FEAT-DOCS-002"));
    assert!(docs_config.contains("routeBasePath: 'docs'"));
    assert!(docs_config.contains("projectName: 'syu'"));
    assert!(docs_config.contains("Concepts"));
    assert!(docs_config.contains("Validation report"));
    assert!(docs_config.contains("/docs/generated/site-spec/features/documentation/docs"));
    assert!(docs_home.contains("Four specification layers"));
    assert!(docs_home.contains("Common journeys"));
    assert!(docs_home.contains("Stay close to checked-in source"));
    assert!(docs_home.contains("Follow a full tutorial"));
    assert!(docs_home.contains("Troubleshoot a broken workspace"));
    assert!(docs_home.contains("Stay in VS Code"));
    assert!(docs_css.contains(".siteHero"));
    assert!(docs_css.contains(".siteCardGrid"));
    assert!(docs_sidebars.contains("autogenerated"));
    let troubleshooting = read_file("docs/guide/troubleshooting.md");
    assert!(troubleshooting.contains("[End-to-end tutorial](./tutorial.md)"));
    assert!(
        troubleshooting.contains("[trace adapter capability matrix](./trace-adapter-support.md)")
    );
    assert!(troubleshooting.contains("[spec anti-patterns guide](./spec-antipatterns.md)"));
}

#[test]
// REQ-CORE-022
fn repository_ships_vscode_extension() {
    let extension_package = read_file("editors/vscode/package.json");
    let extension_lock = read_file("editors/vscode/package-lock.json");
    let extension_readme = read_file("editors/vscode/README.md");
    let extension_launch = read_file("editors/vscode/.vscode/launch.json");
    let extension_entry = read_file("editors/vscode/src/extension.js");
    let extension_model = read_file("editors/vscode/src/model.js");
    let extension_tests = read_file("editors/vscode/test/model.test.js");
    let gitignore = read_file(".gitignore");
    let readme = read_file("README.md");

    assert!(extension_package.contains("\"syu.refreshDiagnostics\""));
    assert!(extension_package.contains("\"syu.showTraceForActiveFile\""));
    assert!(extension_package.contains("\"syu.openSpecItemById\""));
    assert!(extension_package.contains("\"syu.showRelatedFilesForSpecId\""));
    assert!(extension_package.contains("\"syuContext\""));
    assert!(extension_package.contains("\"yaml\""));
    assert!(extension_lock.contains("\"name\": \"syu-vscode\""));
    assert!(extension_lock.contains("\"yaml\""));
    assert!(extension_readme.contains("Problems panel"));
    assert!(extension_readme.contains("syu Context"));
    assert!(extension_readme.contains("Extension Development Host"));
    assert!(extension_launch.contains("\"extensionHost\""));
    assert!(extension_entry.contains("FEAT-VSCODE-001"));
    assert!(extension_entry.contains("SyuContextTreeProvider"));
    assert!(extension_entry.contains("refreshDiagnostics"));
    assert!(extension_model.contains("lookupTrace"));
    assert!(extension_model.contains("loadDiagnostics"));
    assert!(extension_tests.contains("REQ-CORE-022"));
    assert!(extension_tests.contains("lookupTrace"));
    assert!(gitignore.contains("editors/vscode/node_modules"));
    assert!(readme.contains("## VS Code extension"));
}

#[test]
// REQ-CORE-011
fn repository_declares_devcontainer_configuration() {
    let devcontainer = read_file(".devcontainer/devcontainer.json");
    let post_create = read_file(".devcontainer/post-create.sh");
    let browser_setup = read_file(".devcontainer/setup-browser-tooling.sh");
    assert!(devcontainer.contains("FEAT-CONTRIB-001"));
    assert!(devcontainer.contains("bash .devcontainer/post-create.sh"));
    assert!(devcontainer.contains("ghcr.io/devcontainers/features/python:1"));
    assert!(post_create.contains("FEAT-CONTRIB-001"));
    assert!(post_create.contains("cargo install cargo-llvm-cov --locked"));
    assert!(post_create.contains("cargo install wasm-pack --locked"));
    assert!(post_create.contains("scripts/install-precommit.sh"));
    assert!(post_create.contains("CONTRIBUTING.md#local-checks"));
    assert!(post_create.contains("bash .devcontainer/setup-browser-tooling.sh"));
    assert!(post_create.contains("stay opt-in"));
    assert!(browser_setup.contains("FEAT-CONTRIB-001"));
    assert!(browser_setup.contains("npm --prefix app ci"));
    assert!(browser_setup.contains("playwright install --with-deps chromium"));
    assert!(browser_setup.contains("local app builds"));
}

#[test]
// REQ-CORE-012
fn repository_ships_example_workspaces() {
    let current_version = env!("CARGO_PKG_VERSION");
    let rust_example_requirement =
        read_file("examples/rust-only/docs/syu/requirements/core/rust.yaml");
    let rust_example_config = read_file("examples/rust-only/syu.yaml");
    let python_example_requirement =
        read_file("examples/python-only/docs/syu/requirements/core/python.yaml");
    let csharp_fallback_requirement =
        read_file("examples/csharp-fallback/docs/syu/requirements/core/csharp.yaml");
    let csharp_fallback_config = read_file("examples/csharp-fallback/syu.yaml");
    let csharp_fallback_readme = read_file("examples/csharp-fallback/README.md");
    let docs_first_requirement =
        read_file("examples/docs-first/docs/syu/requirements/core/docs.yaml");
    let docs_first_readme = read_file("examples/docs-first/README.md");
    let go_example_requirement = read_file("examples/go-only/docs/syu/requirements/core/go.yaml");
    let go_example_readme = read_file("examples/go-only/README.md");
    let polyglot_feature = read_file("examples/polyglot/docs/syu/features/languages/polyglot.yaml");
    let example_tests = read_file("tests/example_workspaces.rs");

    assert!(rust_example_requirement.contains("REQ-RUST-001"));
    assert!(rust_example_config.contains(&format!("version: {current_version}")));
    assert!(python_example_requirement.contains("REQ-PY-001"));
    assert!(csharp_fallback_requirement.contains("REQ-CSHARP-001"));
    assert!(csharp_fallback_config.contains(&format!("version: {current_version}")));
    assert!(csharp_fallback_readme.contains("CsharpFallbackAcceptanceChecklist"));
    assert!(csharp_fallback_readme.contains("SYU-trace-language-001"));
    assert!(docs_first_requirement.contains("REQ-DOCS-001"));
    assert!(docs_first_requirement.contains("DocsFirstAcceptanceChecklist"));
    assert!(docs_first_readme.contains("syu init --template docs-first"));
    assert!(go_example_requirement.contains("REQ-GO-001"));
    assert!(go_example_readme.contains("TestGoRequirement"));
    assert!(go_example_readme.contains("GoFeatureImpl"));
    assert!(polyglot_feature.contains("FEAT-MIX-001"));
    assert!(polyglot_feature.contains("status: implemented"));
    assert!(example_tests.contains("docs_first_example_validates"));
    assert!(example_tests.contains("csharp_fallback_example_validates"));
    assert!(example_tests.contains("rust_only_example_validates"));
    assert!(example_tests.contains("python_only_example_validates"));
    assert!(example_tests.contains("go_only_example_validates"));
    assert!(example_tests.contains("polyglot_example_validates"));
}

#[test]
// REQ-CORE-013
fn repository_declares_contribution_workflow_assets() {
    let contributing = read_file("CONTRIBUTING.md");
    let pr_template = read_file(".github/pull_request_template.md");
    let bug_report = read_file(".github/ISSUE_TEMPLATE/bug_report.yml");
    let feature_request = read_file(".github/ISSUE_TEMPLATE/feature_request.yml");
    let issue_config = read_file(".github/ISSUE_TEMPLATE/config.yml");
    let squash_title_script = read_file("scripts/ci/check-squash-title-spec-ids.sh");
    let pr_link_script = read_file("scripts/ci/check-pr-spec-links.sh");
    let gitignore = read_file(".gitignore");
    let shared_merge_queue_guidance = "Use a GitHub closing keyword (`Closes #123`, `Fixes #123`, or `Resolves #123`) when this PR implements an issue so the issue closes automatically after the merge queue lands the change on `main`.";

    assert!(contributing.contains("FEAT-CONTRIB-002"));
    assert!(contributing.contains("GitHub Flow"));
    assert!(contributing.contains("main"));
    assert!(contributing.contains("docs/guide/concepts.md"));
    assert!(contributing.contains("philosophy -> policies -> requirements -> features"));
    assert!(contributing.contains("spec edits under"));
    assert!(contributing.contains("Run branch 1 for every change"));
    assert!(contributing.contains("Docs-only edits outside"));
    assert!(contributing.contains("README.md"));
    assert!(contributing.contains("docs/guide/"));
    assert!(contributing.contains("docs/generated/site-spec/"));
    assert!(contributing.contains(".worktrees/"));
    assert!(contributing.contains("scripts/ci/quality-gates.sh"));
    assert!(contributing.contains("scripts/ci/check-generated-docs-freshness.sh"));
    assert!(contributing.contains("docs/generated/"));
    assert!(contributing.contains("scripts/ci/check-browser-app-freshness.sh"));
    assert!(contributing.contains("scripts/ci/pinned-npm.sh install app"));
    assert!(contributing.contains("GitHub uses the PR title as the squash commit headline"));
    assert!(contributing.contains("requirement/feature coverage summary"));
    assert!(contributing.contains("Linked issue or specification"));
    assert!(contributing.contains("Closes #123"));
    assert!(contributing.contains("merge queue lands the change on `main`"));
    assert!(contributing.contains(shared_merge_queue_guidance));
    assert!(contributing.contains("app/dist"));
    assert!(contributing.contains("bash .devcontainer/setup-browser-tooling.sh"));
    assert!(contributing.contains("npm run build:wasm"));
    assert!(contributing.contains("npm run check"));
    assert!(contributing.contains("npx --prefix app playwright install --with-deps chromium"));
    assert!(contributing.contains("npm --prefix app run test:e2e"));
    assert!(contributing.contains("app/playwright.config.ts"));
    assert!(contributing.contains("install-docs-site-deps.sh"));
    assert!(contributing.contains("npm --prefix website ci"));
    assert!(contributing.contains("npm --prefix website run start"));
    assert!(contributing.contains("npm --prefix website run build"));
    assert!(contributing.contains(".github/actions/build-docs-site"));
    assert!(contributing.contains("scripts/install-precommit.sh"));
    assert!(contributing.contains("devcontainer/Codespaces post-create step"));
    assert!(contributing.contains(".devcontainer/post-create.sh"));
    assert!(contributing.contains("cargo-llvm-cov"));
    assert!(contributing.contains("wasm-pack"));
    assert!(contributing.contains("Playwright Chromium"));
    assert!(contributing.contains("local browser-app work"));
    assert!(contributing.contains("does **not** install `website/` docs-site dependencies"));
    assert!(contributing.contains("GitHub Pages"));
    assert!(contributing.contains("release track"));
    assert!(contributing.contains("merge queue playbook"));

    assert!(pr_template.contains("FEAT-CONTRIB-002"));
    assert!(pr_template.contains("scripts/ci/quality-gates.sh"));
    assert!(pr_template.contains("cargo run -- validate ."));
    assert!(pr_template.contains("requirement or feature IDs"));
    assert!(pr_template.contains("include the same IDs in the PR title"));
    assert!(pr_template.contains("preserves them in `git log`"));
    assert!(pr_template.contains("Closes #123"));
    assert!(pr_template.contains("automatically after the merge queue lands the change on `main`"));
    assert!(pr_template.contains(shared_merge_queue_guidance));

    assert!(squash_title_script.contains("FEAT-CONTRIB-002"));
    assert!(squash_title_script.contains("GitHub squash merges use the PR title"));
    assert!(squash_title_script.contains("local git history traceable"));
    assert!(pr_template.contains("Linked issue or specification"));

    assert!(pr_link_script.contains("FEAT-CONTRIB-002"));
    assert!(pr_link_script.contains("docs/syu/"));
    assert!(pr_link_script.contains("Linked issue or specification"));

    assert!(bug_report.contains("FEAT-CONTRIB-002"));
    assert!(bug_report.contains("What happened?"));
    assert!(bug_report.contains("What area is affected?"));
    assert!(bug_report.contains("CLI or runtime behavior"));
    assert!(bug_report.contains("Required for CLI or runtime bugs"));
    assert!(bug_report.contains("Steps to reproduce"));

    assert!(feature_request.contains("FEAT-CONTRIB-002"));
    assert!(feature_request.contains("What problem are you trying to solve?"));
    assert!(feature_request.contains("Specification impact"));

    assert!(issue_config.contains("FEAT-CONTRIB-002"));
    assert!(issue_config.contains("blank_issues_enabled: false"));
    assert!(issue_config.contains("contact_links"));

    assert!(gitignore.contains("FEAT-CONTRIB-002"));
    assert!(gitignore.contains("/.worktrees/"));
}

#[test]
// REQ-CORE-014
fn repository_declares_dependency_hygiene_and_ci_caching() {
    let ci_workflow = read_file(".github/workflows/ci.yml");
    let setup_rust_action = read_file(".github/actions/setup-rust/action.yml");
    let codeql_workflow = read_file(".github/workflows/codeql.yml");
    let merge_queue_checks = read_file(".github/merge-queue-checks.json");
    let docs_build_action = read_file(".github/actions/build-docs-site/action.yml");
    let docs_lock = read_file("website/package-lock.json");
    let release_artifacts = read_file(".github/workflows/release-artifacts.yml");
    let browser_app_freshness = read_file("scripts/ci/check-browser-app-freshness.sh");
    let dependabot = read_file(".github/dependabot.yml");

    assert!(ci_workflow.contains("concurrency:"));
    assert!(ci_workflow.contains("cancel-in-progress: true"));
    assert!(ci_workflow.contains("permissions:"));
    assert!(ci_workflow.contains("./.github/actions/setup-rust"));
    assert!(setup_rust_action.contains("actions/setup-node@v6"));
    assert!(setup_rust_action.contains("cache-dependency-path: app/package-lock.json"));
    assert!(setup_rust_action.contains("tool: wasm-pack"));
    assert!(setup_rust_action.contains("Restore Rust cache"));
    assert!(setup_rust_action.contains("Swatinem/rust-cache@v2"));
    assert!(ci_workflow.contains("taiki-e/cache-cargo-install-action@v3"));
    assert!(ci_workflow.contains("tool: cargo-llvm-cov"));
    assert!(ci_workflow.contains("tool: cargo-audit"));
    assert!(ci_workflow.contains("tool: wasm-pack"));
    assert!(release_artifacts.contains("libc6-dev-arm64-cross"));
    assert!(ci_workflow.contains("merge_group:"));
    assert!(ci_workflow.contains("check-msrv:"));
    assert!(ci_workflow.contains("Set up Python with pip cache"));
    assert!(ci_workflow.contains("cache: pip"));
    assert!(ci_workflow.contains("cache-dependency-path: .pre-commit-config.yaml"));
    assert!(ci_workflow.contains("cache-dependency-path: app/package-lock.json"));
    assert!(browser_app_freshness.contains("npm ci"));
    assert!(ci_workflow.contains("docs-site:"));
    assert!(ci_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_build_action.contains("actions/setup-node@v6"));
    assert!(docs_build_action.contains("cache-dependency-path: website/package-lock.json"));
    assert!(docs_build_action.contains("install-docs-site-deps.sh"));
    assert!(docs_build_action.contains("npm run build"));
    assert!(docs_lock.contains("\"lockfileVersion\":"));
    assert!(codeql_workflow.contains("FEAT-QUALITY-001"));
    assert!(codeql_workflow.contains("merge_group:"));
    assert!(codeql_workflow.contains("security-events: write"));
    assert!(codeql_workflow.contains("Analyze (rust)"));
    assert!(codeql_workflow.contains("actions/setup-node@v6"));
    assert!(codeql_workflow.contains("dtolnay/rust-toolchain@stable"));
    assert!(codeql_workflow.contains("Swatinem/rust-cache@v2"));
    assert!(codeql_workflow.contains("tool: wasm-pack"));
    assert!(codeql_workflow.contains("github/codeql-action/init@v4"));
    assert!(codeql_workflow.contains("github/codeql-action/autobuild@v4"));
    assert!(codeql_workflow.contains("github/codeql-action/analyze@v4"));
    let merge_queue_manifest: serde_json::Value = serde_json::from_str(&merge_queue_checks)
        .expect("merge queue manifest should be valid JSON");
    let required_checks = merge_queue_manifest["required_checks"]
        .as_array()
        .expect("merge queue manifest should declare required checks");
    let contexts: Vec<&str> = required_checks
        .iter()
        .map(|entry| {
            entry["context"]
                .as_str()
                .expect("merge queue context should be a string")
        })
        .collect();
    let unique_contexts: HashSet<&str> = contexts.iter().copied().collect();

    assert_eq!(merge_queue_manifest["version"], 1);
    assert_eq!(contexts.len(), unique_contexts.len());
    assert!(
        required_checks
            .iter()
            .any(|entry| entry["workflow"] == "ci")
    );
    assert!(
        required_checks
            .iter()
            .any(|entry| entry["workflow"] == "codeql")
    );
    assert!(contexts.contains(&"precommit"));
    assert!(contexts.contains(&"MSRV check (1.88)"));
    assert!(contexts.contains(&"Analyze (rust)"));
    assert!(!contexts.contains(&"dependency-review"));
    assert!(
        required_checks
            .iter()
            .any(|entry| entry["job_id"] == "check-msrv")
    );
    assert!(
        required_checks
            .iter()
            .any(|entry| entry["job_id"] == "analyze")
    );

    assert!(release_artifacts.contains("Restore Rust cache"));
    assert!(release_artifacts.contains("Swatinem/rust-cache@v2"));
    assert!(release_artifacts.contains("actions/setup-node@v6"));
    assert!(release_artifacts.contains("tool: wasm-pack"));

    assert!(dependabot.contains("FEAT-QUALITY-001"));
    assert!(dependabot.contains("package-ecosystem: cargo"));
    assert!(dependabot.contains("package-ecosystem: github-actions"));
    assert!(dependabot.matches("package-ecosystem: npm").count() >= 2);
    assert!(dependabot.contains("target-branch: main"));
    assert!(dependabot.contains("rust-crates"));
    assert!(dependabot.contains("github-actions"));
    assert!(dependabot.contains("directory: /website"));
    assert!(dependabot.contains("directory: /app"));
    assert!(dependabot.contains("docs-site-npm"));
    assert!(dependabot.contains("browser-app-npm"));
}

#[test]
// REQ-CORE-017
fn repository_ships_browser_app() {
    let ci_workflow = read_file(".github/workflows/ci.yml");
    let build_script = read_file("build.rs");
    let app_gitignore = read_file("app/.gitignore");
    let app_package = read_file("app/package.json");
    let app_source = read_file("app/src/App.tsx");
    let app_vite = read_file("app/vite.config.ts");
    let app_playwright = read_file("app/tests/browser-app.spec.ts");
    let app_wasm = read_file("app/wasm/src/lib.rs");
    let bundle_freshness = read_file("scripts/ci/check-browser-app-freshness.sh");
    let pinned_npm = read_file("scripts/ci/pinned-npm.sh");
    let readme = read_file("README.md");
    let shared_core = read_file("crates/syu-core/src/lib.rs");

    assert!(ci_workflow.contains("browser-app:"));
    assert!(ci_workflow.contains("Build browser app bundle"));
    assert!(ci_workflow.contains("scripts/ci/pinned-npm.sh install app"));
    assert!(ci_workflow.contains("scripts/ci/pinned-npm.sh install website"));
    assert!(ci_workflow.contains("scripts/ci/check-browser-app-freshness.sh"));
    assert!(ci_workflow.contains("if: success()"));
    assert!(
        ci_workflow.contains(
            "browser-app-dist-run-${{ github.run_id }}-attempt-${{ github.run_attempt }}"
        )
    );
    assert!(build_script.contains("syu-app-dist"));
    assert!(build_script.contains("scripts/ci/pinned-npm.sh install app"));
    assert!(build_script.contains("browser app dependencies are not ready"));
    assert!(build_script.contains("build:wasm"));
    assert!(build_script.contains("--outDir"));
    assert!(build_script.contains("shared_core_dir"));
    assert!(build_script.contains("scripts"));
    assert!(build_script.contains("remove_dir_if_exists"));
    assert!(!build_script.contains("install browser app dependencies with `npm ci`"));
    assert!(app_package.contains("\"packageManager\": \"npm@11.8.0\""));
    assert!(app_package.contains("\"vite-plus\""));
    assert!(app_package.contains("\"@playwright/test\""));
    assert!(app_gitignore.contains("dist"));
    assert!(app_gitignore.contains("src/wasm"));
    assert!(app_source.contains("FEAT-APP-001"));
    assert!(app_source.contains("philosophy"));
    assert!(app_source.contains("requirements"));
    assert!(app_vite.contains("@tailwindcss/vite"));
    assert!(app_playwright.contains("REQ-CORE-017"));
    assert!(app_playwright.contains("FEAT-CHECK-001"));
    assert!(app_wasm.contains("FEAT-APP-001"));
    assert!(bundle_freshness.contains("FEAT-QUALITY-001"));
    assert!(bundle_freshness.contains("ensure_app_dependencies"));
    assert!(bundle_freshness.contains("clear_generated_browser_outputs"));
    assert!(bundle_freshness.contains("pinned-npm.sh"));
    assert!(bundle_freshness.contains("npm ci"));
    assert!(bundle_freshness.contains("check_browser_app_freshness"));
    assert!(bundle_freshness.contains("npm run build:wasm"));
    assert!(bundle_freshness.contains("npm run build"));
    assert!(bundle_freshness.contains("rm -rf src/wasm dist"));
    assert!(bundle_freshness.contains("Browser app Wasm bridge was not regenerated"));
    assert!(!bundle_freshness.contains("[[ -d node_modules ]]"));
    assert!(pinned_npm.contains("FEAT-QUALITY-001"));
    assert!(pinned_npm.contains("packageManager"));
    assert!(pinned_npm.contains("npm install --global"));
    assert!(pinned_npm.contains("Run 'scripts/ci/pinned-npm.sh install"));
    assert!(readme.contains("generates the embedded"));
    assert!(readme.contains("check-browser-app-freshness.sh"));
    assert!(readme.contains("regenerates the local"));
    assert!(readme.contains("app/dist"));
    assert!(shared_core.contains("FEAT-APP-001"));
}

#[test]
// REQ-CORE-016
fn repository_ships_agent_skill() {
    let readme = read_file("README.md");
    let skills_index = read_file("skills/README.md");
    let skill = read_file("skills/syu-maintainer/SKILL.md");

    assert!(readme.contains("Agent skill"));
    assert!(readme.contains("skills/syu-maintainer/SKILL.md"));
    assert!(skills_index.contains("Anthropics Skills"));
    assert!(skills_index.contains("SKILL.md"));
    assert!(skill.contains("name: syu-maintainer"));
    assert!(skill.contains("syu validate ."));
    assert!(skill.contains("syu report . --output docs/generated/syu-report.md"));
    assert!(skill.contains("scripts/ci/quality-gates.sh"));
}
