use std::{fs, path::PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_file(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect("repository file should exist")
}

#[test]
// REQ-CORE-005
fn repository_declares_precommit_and_quality_gates() {
    let precommit = read_file(".pre-commit-config.yaml");
    let quality_script = read_file("scripts/ci/quality-gates.sh");
    let ci_workflow = read_file(".github/workflows/ci.yml");
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

    assert!(ci_workflow.contains("FEAT-QUALITY-001"));
    assert!(ci_workflow.contains("precommit:"));
    assert!(ci_workflow.contains("quality:"));
    assert!(ci_workflow.contains("actionlint:"));
    assert!(ci_workflow.contains("dependency-audit:"));
    assert!(ci_workflow.contains("dependency-review:"));
    assert!(ci_workflow.contains("installer-smoke:"));
    assert!(ci_workflow.contains("--hook-stage pre-commit"));
    assert!(ci_workflow.contains("--hook-stage pre-push"));
    assert!(ci_workflow.contains("scripts/ci/quality-gates.sh"));
    assert!(ci_workflow.contains("cargo audit"));
    assert!(ci_workflow.contains("Review dependency changes"));
    assert!(ci_workflow.contains("scripts/ci/installer-smoke.sh"));

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
    let ci_workflow = read_file(".github/workflows/ci.yml");

    assert!(coverage_script.contains("FEAT-QUALITY-001"));
    assert!(coverage_script.contains("run_coverage"));
    assert!(coverage_script.contains("LINE_THRESHOLD=100"));
    assert!(coverage_script.contains("--fail-under-lines 100"));
    assert!(coverage_script.contains("cargo llvm-cov"));

    assert!(ci_workflow.contains("coverage:"));
    assert!(ci_workflow.contains("scripts/ci/coverage.sh lcov"));
    assert!(ci_workflow.contains("cargo-llvm-cov"));
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
    let mock_registry = read_file("scripts/ci/mock_package_registry.py");
    let readme = read_file("README.md");

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
    assert!(mock_registry.contains("FEAT-INSTALL-001"));
    assert!(mock_registry.contains("build_artifacts"));

    assert!(readme.contains("install-syu.sh"));
    assert!(readme.contains("ugoite/syu"));
    assert!(readme.contains("SYU_VERSION"));
    assert!(readme.contains(&format!(
        "https://github.com/ugoite/syu/releases/download/v{current_version}/install-syu.sh"
    )));
    assert!(readme.contains("GitHub Packages"));
    assert!(!readme.contains("raw.githubusercontent.com/ugoite/syu/main/scripts/install-syu.sh"));
}

#[test]
// REQ-CORE-010
fn repository_declares_documentation_guides() {
    let current_version = env!("CARGO_PKG_VERSION");
    let readme = read_file("README.md");
    let concepts = read_file("docs/guide/concepts.md");
    let getting_started = read_file("docs/guide/getting-started.md");
    let configuration = read_file("docs/guide/configuration.md");
    let config_overview = read_file("docs/syu/config/overview.yaml");
    let config_spec = read_file("docs/syu/config/spec.yaml");
    let config_validate = read_file("docs/syu/config/validate.yaml");
    let config_runtimes = read_file("docs/syu/config/runtimes.yaml");
    let generated_config_overview = read_file("docs/generated/site-spec/config/overview.md");
    let generated_config_spec = read_file("docs/generated/site-spec/config/spec.md");
    let generated_config_validate = read_file("docs/generated/site-spec/config/validate.md");
    let generated_config_runtimes = read_file("docs/generated/site-spec/config/runtimes.md");
    let generated_site_index = read_file("docs/generated/site-spec/index.md");
    let generated_validation =
        read_file("docs/generated/site-spec/features/validation/validation.md");
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
    assert!(readme.contains("Step 0: required"));
    assert!(readme.contains("# 2. Add your spec items"));
    assert!(readme.contains("syu init"));
    assert!(readme.contains("syu init ."));
    assert!(readme.contains("syu validate"));
    assert!(readme.contains("syu browse"));
    assert!(readme.contains("syu list"));
    assert!(readme.contains("syu show"));
    assert!(readme.contains("syu app"));
    assert!(readme.contains("examples/polyglot"));
    assert!(readme.contains("CONTRIBUTING.md"));
    assert!(readme.contains("Contributing and local development"));
    assert!(readme.contains("Documentation site"));
    assert!(readme.contains("Browser app"));
    assert!(readme.contains("scripts/install-precommit.sh"));
    assert!(readme.contains("https://ugoite.github.io/syu/"));
    assert!(readme.contains("docs/syu/config/"));
    assert!(!readme.contains("cargo run -- init ."));

    assert!(concepts.contains("philosophy"));
    assert!(concepts.contains("policy"));
    assert!(concepts.contains("requirements"));
    assert!(concepts.contains("features"));
    assert!(concepts.contains("planned"));
    assert!(concepts.contains("implemented"));
    assert!(concepts.contains("Continue with these pages"));
    assert!(concepts.contains("Specification Reference"));
    assert!(getting_started.contains("New to `syu`?"));
    assert!(getting_started.contains("Start here once `syu` is installed:"));
    assert!(getting_started.contains("syu validate . --fix"));
    assert!(getting_started.contains("syu browse ."));
    assert!(getting_started.contains("syu list feature"));
    assert!(getting_started.contains("syu show REQ-CORE-015"));
    assert!(getting_started.contains("syu app ."));
    assert!(getting_started.contains("status: implemented"));
    assert!(getting_started.contains("Keep exploring"));
    assert_eq!(
        getting_started
            .matches("Follow the [end-to-end tutorial](./tutorial.md)")
            .count(),
        1
    );
    assert!(getting_started.contains("latest validation report"));
    assert!(configuration.contains("validate.default_fix"));
    assert!(configuration.contains("validate.allow_planned"));
    assert!(configuration.contains(&format!("version: {current_version}")));
    assert!(configuration.contains("docs/syu/config/overview.yaml"));
    assert!(configuration.contains("docs/syu/config/validate.yaml"));
    assert!(config_overview.contains("syu.yaml"));
    assert!(config_overview.contains("version"));
    assert!(config_spec.contains("spec.root"));
    assert!(config_validate.contains("validate.default_fix"));
    assert!(config_validate.contains("validate.require_symbol_trace_coverage"));
    assert!(config_runtimes.contains("runtimes.python.command"));
    assert!(generated_config_overview.contains("docs/syu/config/overview.yaml"));
    assert!(generated_config_overview.contains("current CLI version"));
    assert!(generated_config_spec.contains("docs/syu/config/spec.yaml"));
    assert!(generated_config_validate.contains("validate.default_fix"));
    assert!(generated_config_runtimes.contains("docs/syu/config/runtimes.yaml"));
    assert!(generated_site_index.contains("/docs/generated/site-spec/features/cli/show-list"));
    assert!(generated_site_index.contains("/docs/generated/site-spec/features/validation"));
    assert!(generated_validation.contains("docs/syu/features/validation/validation.yaml"));
    assert!(generated_validation.contains("SYU-graph-reference-001"));
    assert!(ci_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_build_action.contains("FEAT-DOCS-002"));
    assert!(docs_build_action.contains("actions/setup-node@v6"));
    assert!(docs_build_action.contains("cache-dependency-path: website/package-lock.json"));
    assert!(docs_build_action.contains("npm ci"));
    assert!(docs_build_action.contains("npm run build"));
    assert!(docs_package.contains("@docusaurus/core"));
    assert!(docs_lock.contains("\"name\": \"syu-docs\""));
    assert!(docs_lock.contains("\"lockfileVersion\":"));
    assert!(docs_package.contains("\"build\": \"docusaurus build\""));
    assert!(docs_deploy_workflow.contains("permissions:"));
    assert!(docs_deploy_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_deploy_workflow.contains("actions/configure-pages@v5"));
    assert!(docs_deploy_workflow.contains("actions/upload-pages-artifact@v4"));
    assert!(docs_deploy_workflow.contains("actions/deploy-pages@v4"));
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
    assert!(docs_css.contains(".siteHero"));
    assert!(docs_css.contains(".siteCardGrid"));
    assert!(docs_sidebars.contains("autogenerated"));
}

#[test]
// REQ-CORE-011
fn repository_declares_devcontainer_configuration() {
    let devcontainer = read_file(".devcontainer/devcontainer.json");
    assert!(devcontainer.contains("FEAT-CONTRIB-001"));
    assert!(devcontainer.contains("cargo install cargo-llvm-cov --locked"));
    assert!(devcontainer.contains("ghcr.io/devcontainers/features/python:1"));
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
    let polyglot_feature = read_file("examples/polyglot/docs/syu/features/languages/polyglot.yaml");
    let example_tests = read_file("tests/example_workspaces.rs");

    assert!(rust_example_requirement.contains("REQ-RUST-001"));
    assert!(rust_example_config.contains(&format!("version: {current_version}")));
    assert!(python_example_requirement.contains("REQ-PY-001"));
    assert!(polyglot_feature.contains("FEAT-MIX-001"));
    assert!(polyglot_feature.contains("status: implemented"));
    assert!(example_tests.contains("rust_only_example_validates"));
    assert!(example_tests.contains("python_only_example_validates"));
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
    let gitignore = read_file(".gitignore");

    assert!(contributing.contains("FEAT-CONTRIB-002"));
    assert!(contributing.contains("GitHub Flow"));
    assert!(contributing.contains("main"));
    assert!(contributing.contains(".worktrees/"));
    assert!(contributing.contains("scripts/ci/quality-gates.sh"));
    assert!(contributing.contains("scripts/install-precommit.sh"));
    assert!(contributing.contains("GitHub Pages"));
    assert!(contributing.contains("release track"));

    assert!(pr_template.contains("FEAT-CONTRIB-002"));
    assert!(pr_template.contains("scripts/ci/quality-gates.sh"));
    assert!(pr_template.contains("cargo run -- validate ."));

    assert!(bug_report.contains("FEAT-CONTRIB-002"));
    assert!(bug_report.contains("What happened?"));
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
    let docs_build_action = read_file(".github/actions/build-docs-site/action.yml");
    let docs_lock = read_file("website/package-lock.json");
    let release_artifacts = read_file(".github/workflows/release-artifacts.yml");
    let dependabot = read_file(".github/dependabot.yml");

    assert!(ci_workflow.contains("concurrency:"));
    assert!(ci_workflow.contains("cancel-in-progress: true"));
    assert!(ci_workflow.contains("permissions:"));
    assert!(ci_workflow.contains("./.github/actions/setup-rust"));
    assert!(setup_rust_action.contains("Restore Rust cache"));
    assert!(setup_rust_action.contains("Swatinem/rust-cache@v2"));
    assert!(ci_workflow.contains("taiki-e/cache-cargo-install-action@v3"));
    assert!(ci_workflow.contains("tool: cargo-llvm-cov"));
    assert!(ci_workflow.contains("tool: cargo-audit"));
    assert!(ci_workflow.contains("merge_group:"));
    assert!(ci_workflow.contains("Set up Python with pip cache"));
    assert!(ci_workflow.contains("cache: pip"));
    assert!(ci_workflow.contains("cache-dependency-path: .pre-commit-config.yaml"));
    assert!(ci_workflow.contains("cache-dependency-path: app/package-lock.json"));
    assert!(ci_workflow.contains("npm ci"));
    assert!(ci_workflow.contains("docs-site:"));
    assert!(ci_workflow.contains("./.github/actions/build-docs-site"));
    assert!(docs_build_action.contains("actions/setup-node@v6"));
    assert!(docs_build_action.contains("cache-dependency-path: website/package-lock.json"));
    assert!(docs_build_action.contains("npm ci"));
    assert!(docs_build_action.contains("npm run build"));
    assert!(docs_lock.contains("\"lockfileVersion\":"));
    assert!(codeql_workflow.contains("FEAT-QUALITY-001"));
    assert!(codeql_workflow.contains("merge_group:"));
    assert!(codeql_workflow.contains("security-events: write"));
    assert!(codeql_workflow.contains("Analyze (rust)"));
    assert!(codeql_workflow.contains("dtolnay/rust-toolchain@stable"));
    assert!(codeql_workflow.contains("Swatinem/rust-cache@v2"));
    assert!(codeql_workflow.contains("github/codeql-action/init@v4"));
    assert!(codeql_workflow.contains("github/codeql-action/autobuild@v4"));
    assert!(codeql_workflow.contains("github/codeql-action/analyze@v4"));

    assert!(release_artifacts.contains("Restore Rust cache"));
    assert!(release_artifacts.contains("Swatinem/rust-cache@v2"));

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
    let app_package = read_file("app/package.json");
    let app_source = read_file("app/src/App.tsx");
    let app_vite = read_file("app/vite.config.ts");
    let app_playwright = read_file("app/tests/browser-app.spec.ts");
    let app_wasm = read_file("app/wasm/src/lib.rs");
    let shared_core = read_file("crates/syu-core/src/lib.rs");

    assert!(ci_workflow.contains("browser-app:"));
    assert!(ci_workflow.contains("npm run build:wasm"));
    assert!(ci_workflow.contains("npm run build"));
    assert!(app_package.contains("\"vite-plus\""));
    assert!(app_package.contains("\"@playwright/test\""));
    assert!(app_source.contains("FEAT-APP-001"));
    assert!(app_source.contains("philosophy"));
    assert!(app_source.contains("requirements"));
    assert!(app_vite.contains("@tailwindcss/vite"));
    assert!(app_playwright.contains("REQ-CORE-017"));
    assert!(app_playwright.contains("FEAT-CHECK-001"));
    assert!(app_wasm.contains("FEAT-APP-001"));
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
