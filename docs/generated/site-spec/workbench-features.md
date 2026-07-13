---
title: "Workbench implementation / Workbench Features"
description: "Generated reference for docs/syu/workbench-features.yaml"
---

> Generated from `docs/syu/workbench-features.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- features

### Namespace

- workbench

### Category

- Workbench implementation

### Features

- **id**: FEAT-WORKBENCH-SELF-HOSTING-001
  - **title**: Workbench self-hosting
  - **summary**: Keep the repository's implementation, tooling, documentation, and fixture ownership explicit.
  - **status**: planned
  - **bindings**:
    - **id**: self-hosting-metadata
      - **role**: configuration
      - **facet**: tooling
      - **responsibility**: Own repository metadata, CI policy, and maintainer tooling as declared file subjects.
      - **owns**:
        - **id**: selfhost-github-issue-template-bug-report-yml
          - **adapter**: declared
          - **path**: .github/ISSUE_TEMPLATE/bug_report.yml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-github-pull-request-template-md
          - **adapter**: declared
          - **path**: .github/pull_request_template.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-github-workflows-quality-yml
          - **adapter**: declared
          - **path**: .github/workflows/quality.yml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-contributing-md
          - **adapter**: declared
          - **path**: CONTRIBUTING.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-cargo-lock
          - **adapter**: declared
          - **path**: Cargo.lock
          - **selector**:
            - **kind**: file
        - **id**: selfhost-cargo-toml
          - **adapter**: declared
          - **path**: Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-readme-md
          - **adapter**: declared
          - **path**: README.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-scripts-ci-check-workbench-contract-py
          - **adapter**: declared
          - **path**: scripts/ci/check-workbench-contract.py
          - **selector**:
            - **kind**: file
        - **id**: selfhost-scripts-ci-quality-gates-sh
          - **adapter**: declared
          - **path**: scripts/ci/quality-gates.sh
          - **selector**:
            - **kind**: file
        - **id**: selfhost-scripts-ci-workbench-visual-sh
          - **adapter**: declared
          - **path**: scripts/ci/workbench-visual.sh
          - **selector**:
            - **kind**: file
        - **id**: selfhost-scripts-dev-validate-changed-sh
          - **adapter**: declared
          - **path**: scripts/dev/validate-changed.sh
          - **selector**:
            - **kind**: file
        - **id**: selfhost-skills-syu-maintainer-skill-md
          - **adapter**: declared
          - **path**: skills/syu-maintainer/SKILL.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-syu-yaml
          - **adapter**: declared
          - **path**: syu.yaml
          - **selector**:
            - **kind**: file
      - **targets**:
        - **id**: syu-config
          - **adapter**: declared
          - **path**: syu.yaml
          - **selector**:
            - **kind**: marker
            - **value**: schema: syu/config/v1
    - **id**: self-hosting-frontend
      - **role**: configuration
      - **facet**: tooling
      - **responsibility**: Own browser assets and UI crate modules as separated frontend units.
      - **owns**:
        - **id**: retired-workbench-app-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/app.js
          - **selector**:
            - **kind**: file
        - **id**: retired-workbench-projection-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/projection.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-api-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-components-action-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-components-diagnostic-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/components/diagnostic.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-components-editor-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/components/editor.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-components-readiness-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/components/readiness.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-components-target-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/components/target.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-i18n-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/i18n.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-main-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-diagnostics-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-readiness-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-scope-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-settings-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/settings.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-specifications-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-pages-work-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-router-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-js-state-js
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-locales-en-json
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/locales/en.json
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-locales-ja-json
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/locales/ja.json
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-assets-workbench-html
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-src-pages-rs
          - **adapter**: declared
          - **path**: crates/syu-app-ui/src/pages.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-app-ui-src-shell-rs
          - **adapter**: declared
          - **path**: crates/syu-app-ui/src/shell.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-editors-vscode-readme-md
          - **adapter**: declared
          - **path**: editors/vscode/README.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-editors-vscode-src-model-js
          - **adapter**: declared
          - **path**: editors/vscode/src/model.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-editors-vscode-test-model-test-js
          - **adapter**: declared
          - **path**: editors/vscode/test/model.test.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-editors-vscode-src-model-js-code
          - **adapter**: javascript
          - **path**: editors/vscode/src/model.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: selfhost-editors-vscode-test-model-test-js-code
          - **adapter**: javascript
          - **path**: editors/vscode/test/model.test.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: selfhost-website-src-pages-index-js
          - **adapter**: declared
          - **path**: website/src/pages/index.js
          - **selector**:
            - **kind**: file
        - **id**: selfhost-website-src-pages-index-js-code
          - **adapter**: javascript
          - **path**: website/src/pages/index.js
          - **selector**:
            - **kind**: module
            - **name**: *
      - **targets**:
        - **id**: frontend-capability-anchor
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: symbol
            - **name**: actionCapability
          - **claims**:
            - (empty list)
    - **id**: self-hosting-crates
      - **role**: configuration
      - **facet**: tooling
      - **responsibility**: Own crate and CLI module units that are not covered by a more specific Workbench feature.
      - **owns**:
        - **id**: selfhost-crates-syu-code-intel-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-diagnostics-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-diagnostics/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-diagnostics-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-inventory-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-inventory/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-inventory-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-planner-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-planner/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-planner-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-project-model-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-project-model/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-project-model-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-project-model/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-spec-model-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-validation-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-validation/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-validation-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-validation-src-readiness-rs
          - **adapter**: declared
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-work-model-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-workbench-server-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-workbench-server/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-workbench-server-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-workspace-cargo-toml
          - **adapter**: declared
          - **path**: crates/syu-workspace/Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-crates-syu-workspace-src-lib-rs
          - **adapter**: declared
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-add-rs
          - **adapter**: declared
          - **path**: src/command/add.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-audit-rs
          - **adapter**: declared
          - **path**: src/command/audit.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-browse-rs
          - **adapter**: declared
          - **path**: src/command/browse.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-check-rs
          - **adapter**: declared
          - **path**: src/command/check.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-completion-rs
          - **adapter**: declared
          - **path**: src/command/completion.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-doctor-rs
          - **adapter**: declared
          - **path**: src/command/doctor.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-explain-rs
          - **adapter**: declared
          - **path**: src/command/explain.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-init-rs
          - **adapter**: declared
          - **path**: src/command/init.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-issue-text-rs
          - **adapter**: declared
          - **path**: src/command/issue_text.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-list-rs
          - **adapter**: declared
          - **path**: src/command/list.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-log-rs
          - **adapter**: declared
          - **path**: src/command/log.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-lookup-rs
          - **adapter**: declared
          - **path**: src/command/lookup.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-mod-rs
          - **adapter**: declared
          - **path**: src/command/mod.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-prompt-rs
          - **adapter**: declared
          - **path**: src/command/prompt.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-relate-rs
          - **adapter**: declared
          - **path**: src/command/relate.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-report-rs
          - **adapter**: declared
          - **path**: src/command/report.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-search-rs
          - **adapter**: declared
          - **path**: src/command/search.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-show-rs
          - **adapter**: declared
          - **path**: src/command/show.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-task-rs
          - **adapter**: declared
          - **path**: src/command/task.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-templates-rs
          - **adapter**: declared
          - **path**: src/command/templates.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-trace-rs
          - **adapter**: declared
          - **path**: src/command/trace.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-command-workbench-rs
          - **adapter**: declared
          - **path**: src/command/workbench.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-src-lib-rs
          - **adapter**: declared
          - **path**: src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-root-build-module
          - **adapter**: rust
          - **path**: build.rs
          - **selector**:
            - **kind**: module
            - **name**: build
        - **id**: selfhost-desktop-build-module
          - **adapter**: rust
          - **path**: crates/syu-desktop/build.rs
          - **selector**:
            - **kind**: module
            - **name**: build
        - **id**: selfhost-desktop-lib-module
          - **adapter**: rust
          - **path**: crates/syu-desktop/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: selfhost-desktop-main-module
          - **adapter**: rust
          - **path**: crates/syu-desktop/src/main.rs
          - **selector**:
            - **kind**: module
            - **name**: main
      - **targets**:
        - **id**: desktop-capability-anchor
          - **adapter**: rust
          - **path**: crates/syu-desktop/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: desktop_action_bridge
          - **claims**:
            - (empty list)
    - **id**: self-hosting-documentation
      - **role**: configuration
      - **facet**: tooling
      - **responsibility**: Own documentation and generated-site artifacts as explicit documentation units.
      - **owns**:
        - **id**: selfhost-docs-generated-site-spec-capabilities-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/capabilities.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-delivery-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/delivery.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-foundation-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/foundation.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-index-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/index.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-planner-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/planner.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-public-entrypoints-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/public-entrypoints.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-work-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/work.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-workbench-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/workbench.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-generated-site-spec-workbench-features-md
          - **adapter**: declared
          - **path**: docs/generated/site-spec/workbench-features.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-command-card-md
          - **adapter**: declared
          - **path**: docs/guide/command-card.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-configuration-md
          - **adapter**: declared
          - **path**: docs/guide/configuration.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-examples-and-templates-md
          - **adapter**: declared
          - **path**: docs/guide/examples-and-templates.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-existing-repository-md
          - **adapter**: declared
          - **path**: docs/guide/existing-repository.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-migration-md
          - **adapter**: declared
          - **path**: docs/guide/migration.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-spec-antipatterns-md
          - **adapter**: declared
          - **path**: docs/guide/spec-antipatterns.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-guide-vscode-extension-md
          - **adapter**: declared
          - **path**: docs/guide/vscode-extension.md
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-capabilities-yaml
          - **adapter**: declared
          - **path**: docs/syu/capabilities.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-delivery-yaml
          - **adapter**: declared
          - **path**: docs/syu/delivery.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-foundation-yaml
          - **adapter**: declared
          - **path**: docs/syu/foundation.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-planner-yaml
          - **adapter**: declared
          - **path**: docs/syu/planner.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-public-entrypoints-yaml
          - **adapter**: declared
          - **path**: docs/syu/public-entrypoints.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-work-yaml
          - **adapter**: declared
          - **path**: docs/syu/work.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-workbench-features-yaml
          - **adapter**: declared
          - **path**: docs/syu/workbench-features.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-docs-syu-workbench-yaml
          - **adapter**: declared
          - **path**: docs/syu/workbench.yaml
          - **selector**:
            - **kind**: file
      - **targets**:
        - **id**: documentation-capability-anchor
          - **adapter**: declared
          - **path**: docs/generated/site-spec/index.md
          - **selector**:
            - **kind**: marker
            - **value**: Specification Reference
          - **claims**:
            - (empty list)
    - **id**: self-hosting-tests-fixtures
      - **role**: configuration
      - **facet**: tooling
      - **responsibility**: Own test, fixture, and example artifacts as explicit verification-support units.
      - **owns**:
        - **id**: selfhost-examples-browser-ui-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/browser-ui/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-browser-ui-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/browser-ui/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-browser-ui-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/browser-ui/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-browser-ui-syu-yaml
          - **adapter**: declared
          - **path**: examples/browser-ui/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-csharp-fallback-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/csharp-fallback/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-csharp-fallback-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/csharp-fallback/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-csharp-fallback-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/csharp-fallback/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-csharp-fallback-syu-yaml
          - **adapter**: declared
          - **path**: examples/csharp-fallback/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-docs-first-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/docs-first/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-docs-first-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/docs-first/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-docs-first-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/docs-first/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-docs-first-syu-yaml
          - **adapter**: declared
          - **path**: examples/docs-first/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-generic-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/generic/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-generic-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/generic/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-generic-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/generic/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-generic-syu-yaml
          - **adapter**: declared
          - **path**: examples/generic/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-go-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/go-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-go-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/go-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-go-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/go-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-go-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/go-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-java-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/java-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-java-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/java-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-java-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/java-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-java-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/java-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-polyglot-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/polyglot/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-polyglot-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/polyglot/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-polyglot-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/polyglot/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-polyglot-syu-yaml
          - **adapter**: declared
          - **path**: examples/polyglot/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-python-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/python-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-python-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/python-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-python-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/python-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-python-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/python-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-ruby-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/ruby-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-ruby-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/ruby-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-ruby-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/ruby-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-ruby-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/ruby-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-rust-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/rust-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-rust-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/rust-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-rust-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/rust-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-rust-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/rust-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-team-scale-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/team-scale/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-team-scale-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/team-scale/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-team-scale-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/team-scale/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-team-scale-syu-yaml
          - **adapter**: declared
          - **path**: examples/team-scale/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-typescript-only-docs-syu-features-core-core-yaml
          - **adapter**: declared
          - **path**: examples/typescript-only/docs/syu/features/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-typescript-only-docs-syu-policies-policies-yaml
          - **adapter**: declared
          - **path**: examples/typescript-only/docs/syu/policies/policies.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-typescript-only-docs-syu-requirements-core-core-yaml
          - **adapter**: declared
          - **path**: examples/typescript-only/docs/syu/requirements/core/core.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-examples-typescript-only-syu-yaml
          - **adapter**: declared
          - **path**: examples/typescript-only/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-rejected-obsolete-pre-release-v1-spec-yaml
          - **adapter**: declared
          - **path**: fixtures/rejected/obsolete-pre-release-v1/spec.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-rejected-obsolete-pre-release-v1-syu-yaml
          - **adapter**: declared
          - **path**: fixtures/rejected/obsolete-pre-release-v1/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-v1-invalid-unknown-field-syu-yaml
          - **adapter**: declared
          - **path**: fixtures/v1/invalid-unknown-field/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-v1-valid-web-app-spec-feature-yaml
          - **adapter**: declared
          - **path**: fixtures/v1/valid-web-app/spec/feature.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-v1-valid-web-app-spec-requirement-yaml
          - **adapter**: declared
          - **path**: fixtures/v1/valid-web-app/spec/requirement.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-fixtures-v1-valid-web-app-syu-yaml
          - **adapter**: declared
          - **path**: fixtures/v1/valid-web-app/syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: selfhost-tests-v1-cli-rs
          - **adapter**: declared
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-tests-workbench-smoke-rs
          - **adapter**: declared
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: file
        - **id**: selfhost-tests-workbench-visual-rs
          - **adapter**: declared
          - **path**: tests/workbench_visual.rs
          - **selector**:
            - **kind**: file
      - **targets**:
        - **id**: fixture-capability-anchor
          - **adapter**: declared
          - **path**: fixtures/v1/valid-web-app/syu.yaml
          - **selector**:
            - **kind**: marker
            - **value**: schema: syu/config/v1
          - **claims**:
            - (empty list)
- **id**: FEAT-WORKBENCH-SERVER-001
  - **title**: Workbench server
  - **summary**: Expose canonical read, work, validation, edit, and security APIs.
  - **status**: implemented
  - **bindings**:
    - **id**: server
      - **role**: implementation
      - **facet**: server
      - **responsibility**: Serve canonical Workbench HTTP routes and mutation guards.
      - **targets**:
        - **id**: mutation-guard
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: mutation_guard
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-005#criterion.secure-local-server
      - **owns**:
        - **id**: workbench-server-module
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
- **id**: FEAT-WORKBENCH-PROJECTION-001
  - **title**: Workbench projection
  - **summary**: Project canonical readiness, work, scope, specification, and diagnostics state.
  - **status**: implemented
  - **bindings**:
    - **id**: projection
      - **role**: implementation
      - **facet**: projection
      - **responsibility**: Build the canonical Workbench projection from indexed server data.
      - **targets**:
        - **id**: project
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
    - **id**: projection-contract
      - **role**: contract-source
      - **facet**: projection
      - **responsibility**: Define the canonical projection boundary consumed by verification.
      - **targets**:
        - **id**: project-contract
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project
      - **owns**:
        - **id**: workspace-module
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: inventory-module
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: validation-module
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: readiness-module
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: module
            - **name**: readiness
        - **id**: readiness-file
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: file
        - **id**: diagnostics-module
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: code-intel-module
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: branch-scope-module
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/branch_scope.rs
          - **selector**:
            - **kind**: module
            - **name**: branch_scope
        - **id**: project-model-module
          - **adapter**: rust
          - **path**: crates/syu-project-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: domain-module
          - **adapter**: rust
          - **path**: crates/syu-domain/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
  - **contracts**:
    - **id**: projection-verification
      - **kind**: function
      - **source**: FEAT-WORKBENCH-PROJECTION-001#binding.projection-contract/target.project-contract
      - **participants**:
        - **target**: FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
          - **role**: producer
        - **target**: FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
          - **role**: verifier
        - **target**: FEAT-WORKBENCH-VERIFICATION-001#binding.test-exposure/target.server-e2e
          - **role**: verifier
      - **guarantees**:
        - REQ-WORKBENCH-001#criterion.canonical-projection
- **id**: FEAT-WORKBENCH-WORK-UI-001
  - **title**: Workbench work UI
  - **summary**: Drive the WorkRequest to result-validation journey from the Work page.
  - **status**: implemented
  - **bindings**:
    - **id**: work
      - **role**: implementation
      - **facet**: work
      - **responsibility**: Plan and validate a bounded Workbench work session.
      - **targets**:
        - **id**: plan
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_plan
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
      - **owns**:
        - **id**: planner-module
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: work-model-module
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: cli-module
          - **adapter**: rust
          - **path**: src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: cli-main-module
          - **adapter**: rust
          - **path**: src/main.rs
          - **selector**:
            - **kind**: module
            - **name**: main
        - **id**: cli-main-file
          - **adapter**: rust
          - **path**: src/main.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-module
          - **adapter**: rust
          - **path**: src/lsp/mod.rs
          - **selector**:
            - **kind**: module
            - **name**: mod
        - **id**: lsp-file
          - **adapter**: rust
          - **path**: src/lsp/mod.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-handlers-module
          - **adapter**: rust
          - **path**: src/lsp/handlers.rs
          - **selector**:
            - **kind**: module
            - **name**: handlers
        - **id**: lsp-handlers-file
          - **adapter**: rust
          - **path**: src/lsp/handlers.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-protocol-module
          - **adapter**: rust
          - **path**: src/lsp/protocol.rs
          - **selector**:
            - **kind**: module
            - **name**: protocol
        - **id**: lsp-protocol-file
          - **adapter**: rust
          - **path**: src/lsp/protocol.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-server-module
          - **adapter**: rust
          - **path**: src/lsp/server.rs
          - **selector**:
            - **kind**: module
            - **name**: server
        - **id**: lsp-server-file
          - **adapter**: rust
          - **path**: src/lsp/server.rs
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-SPEC-EDITOR-001
  - **title**: Workbench specification editor
  - **summary**: Preview and apply structured specification and configuration edits transactionally.
  - **status**: implemented
  - **bindings**:
    - **id**: editor
      - **role**: implementation
      - **facet**: editor
      - **responsibility**: Apply validated structured edits through the Workbench transaction boundary.
      - **targets**:
        - **id**: specification-apply
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_specification_apply
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-003#criterion.transactional-spec-edit
        - **id**: config-apply
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_config_apply
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-004#criterion.transactional-config-edit
      - **owns**:
        - **id**: spec-model-module
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: app-ui-components-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/components.rs
          - **selector**:
            - **kind**: module
            - **name**: components
        - **id**: app-ui-components-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/components.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-document-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/document.rs
          - **selector**:
            - **kind**: module
            - **name**: document
        - **id**: app-ui-document-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/document.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-lib-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: app-ui-pages-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/pages.rs
          - **selector**:
            - **kind**: module
            - **name**: pages
        - **id**: app-ui-pages-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/pages.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-shell-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/shell.rs
          - **selector**:
            - **kind**: module
            - **name**: shell
        - **id**: app-ui-shell-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/shell.rs
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-NAVIGATION-001
  - **title**: Workbench navigation
  - **summary**: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  - **status**: implemented
  - **bindings**:
    - **id**: navigation
      - **role**: implementation
      - **facet**: tooling
      - **responsibility**: Implement browser navigation and accessibility behavior as typed server DTO consumers.
      - **targets**:
        - **id**: html-navigation
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-page="work"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: javascript-navigation
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindRouter
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: keyboard-navigation
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindKeyboardTabs
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: accessibility-attributes
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-i18n-aria="a11y.main_pages"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
      - **owns**:
        - **id**: assets-i18n-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/i18n.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-api-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-action-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-diagnostic-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/diagnostic.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-editor-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/editor.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-readiness-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-target-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/target.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-js-i18n-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/i18n.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-main-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-router-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-state-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-diagnostics-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-readiness-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-scope-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-settings-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/settings.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-specifications-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-work-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: html-file
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-VERIFICATION-001
  - **title**: Workbench verification tests
  - **summary**: Exercise the real Workbench HTTP server and exact verification targets.
  - **status**: implemented
  - **bindings**:
    - **id**: verification-harness
      - **role**: implementation
      - **facet**: verification
      - **responsibility**: Maintain the executable Workbench HTTP verification harness.
      - **targets**:
        - **id**: verification-endpoint
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_verify
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
      - **owns**:
        - **id**: cli-test-module
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: module
            - **name**: v1_cli
        - **id**: workbench-command-test-module
          - **adapter**: rust
          - **path**: tests/workbench_command.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_command
        - **id**: workbench-smoke-test-module
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_smoke
        - **id**: workbench-visual-test-module
          - **adapter**: rust
          - **path**: tests/workbench_visual.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_visual
    - **id**: test-exposure
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Expose every executable Workbench test as an exact verification target.
      - **targets**:
        - **id**: server-e2e
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_http_closed_loop_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
                - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_http_closed_loop_flow
        - **id**: server-transport
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_http_server_transport_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
                - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_http_server_transport_flow
        - **id**: command-help
          - **adapter**: rust
          - **path**: tests/workbench_command.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_help_lists_browser_launch_options
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
              - **covers**:
                - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_command
                  - **test**: workbench_help_lists_browser_launch_options
        - **id**: smoke-projection
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_projection_is_server_owned_and_starts_not_run
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_smoke
                  - **test**: workbench_projection_is_server_owned_and_starts_not_run
        - **id**: smoke-module-contract
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: rendered_workbench_uses_external_module_assets_and_specifications_route
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_smoke
                  - **test**: rendered_workbench_uses_external_module_assets_and_specifications_route
        - **id**: smoke-dto-contract
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: browser_modules_render_dtos_without_model_inference
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_smoke
                  - **test**: browser_modules_render_dtos_without_model_inference
        - **id**: smoke-keyboard
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_tabs_are_keyboard_navigable
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_smoke
                  - **test**: workbench_tabs_are_keyboard_navigable
        - **id**: visual-dom
          - **adapter**: rust
          - **path**: tests/workbench_visual.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_rendered_dom_uses_projection_driven_placeholders
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_visual
                  - **test**: workbench_rendered_dom_uses_projection_driven_placeholders

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-SELF-HOSTING-001
  title: Workbench self-hosting
  summary: Keep the repository's implementation, tooling, documentation, and fixture ownership explicit.
  status: planned
  bindings:
  - id: self-hosting-metadata
    role: configuration
    facet: tooling
    responsibility: Own repository metadata, CI policy, and maintainer tooling as declared file subjects.
    owns:
    - id: selfhost-github-issue-template-bug-report-yml
      adapter: declared
      path: .github/ISSUE_TEMPLATE/bug_report.yml
      selector:
        kind: file
    - id: selfhost-github-pull-request-template-md
      adapter: declared
      path: .github/pull_request_template.md
      selector:
        kind: file
    - id: selfhost-github-workflows-quality-yml
      adapter: declared
      path: .github/workflows/quality.yml
      selector:
        kind: file
    - id: selfhost-contributing-md
      adapter: declared
      path: CONTRIBUTING.md
      selector:
        kind: file
    - id: selfhost-cargo-lock
      adapter: declared
      path: Cargo.lock
      selector:
        kind: file
    - id: selfhost-cargo-toml
      adapter: declared
      path: Cargo.toml
      selector:
        kind: file
    - id: selfhost-readme-md
      adapter: declared
      path: README.md
      selector:
        kind: file
    - id: selfhost-scripts-ci-check-workbench-contract-py
      adapter: declared
      path: scripts/ci/check-workbench-contract.py
      selector:
        kind: file
    - id: selfhost-scripts-ci-quality-gates-sh
      adapter: declared
      path: scripts/ci/quality-gates.sh
      selector:
        kind: file
    - id: selfhost-scripts-ci-workbench-visual-sh
      adapter: declared
      path: scripts/ci/workbench-visual.sh
      selector:
        kind: file
    - id: selfhost-scripts-dev-validate-changed-sh
      adapter: declared
      path: scripts/dev/validate-changed.sh
      selector:
        kind: file
    - id: selfhost-skills-syu-maintainer-skill-md
      adapter: declared
      path: skills/syu-maintainer/SKILL.md
      selector:
        kind: file
    - id: selfhost-syu-yaml
      adapter: declared
      path: syu.yaml
      selector:
        kind: file
    targets:
    - id: syu-config
      adapter: declared
      path: syu.yaml
      selector:
        kind: marker
        value: 'schema: syu/config/v1'
  - id: self-hosting-frontend
    role: configuration
    facet: tooling
    responsibility: Own browser assets and UI crate modules as separated frontend units.
    owns:
    - id: retired-workbench-app-js
      adapter: declared
      path: crates/syu-app-ui/assets/app.js
      selector:
        kind: file
    - id: retired-workbench-projection-js
      adapter: declared
      path: crates/syu-app-ui/assets/projection.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-api-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-components-action-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/components/action.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-components-diagnostic-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/components/diagnostic.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-components-editor-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/components/editor.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-components-readiness-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/components/readiness.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-components-target-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/components/target.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-i18n-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/i18n.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-main-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-diagnostics-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-readiness-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/readiness.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-scope-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/scope.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-settings-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/settings.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-specifications-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-pages-work-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-router-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-js-state-js
      adapter: declared
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-locales-en-json
      adapter: declared
      path: crates/syu-app-ui/assets/locales/en.json
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-locales-ja-json
      adapter: declared
      path: crates/syu-app-ui/assets/locales/ja.json
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-assets-workbench-html
      adapter: declared
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-src-lib-rs
      adapter: declared
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-src-pages-rs
      adapter: declared
      path: crates/syu-app-ui/src/pages.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-app-ui-src-shell-rs
      adapter: declared
      path: crates/syu-app-ui/src/shell.rs
      selector:
        kind: file
    - id: selfhost-editors-vscode-readme-md
      adapter: declared
      path: editors/vscode/README.md
      selector:
        kind: file
    - id: selfhost-editors-vscode-src-model-js
      adapter: declared
      path: editors/vscode/src/model.js
      selector:
        kind: file
    - id: selfhost-editors-vscode-test-model-test-js
      adapter: declared
      path: editors/vscode/test/model.test.js
      selector:
        kind: file
    - id: selfhost-editors-vscode-src-model-js-code
      adapter: javascript
      path: editors/vscode/src/model.js
      selector:
        kind: module
        name: '*'
    - id: selfhost-editors-vscode-test-model-test-js-code
      adapter: javascript
      path: editors/vscode/test/model.test.js
      selector:
        kind: module
        name: '*'
    - id: selfhost-website-src-pages-index-js
      adapter: declared
      path: website/src/pages/index.js
      selector:
        kind: file
    - id: selfhost-website-src-pages-index-js-code
      adapter: javascript
      path: website/src/pages/index.js
      selector:
        kind: module
        name: '*'
    targets:
    - id: frontend-capability-anchor
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/action.js
      selector:
        kind: symbol
        name: actionCapability
      claims: []
  - id: self-hosting-crates
    role: configuration
    facet: tooling
    responsibility: Own crate and CLI module units that are not covered by a more specific Workbench feature.
    owns:
    - id: selfhost-crates-syu-code-intel-src-lib-rs
      adapter: declared
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-diagnostics-cargo-toml
      adapter: declared
      path: crates/syu-diagnostics/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-diagnostics-src-lib-rs
      adapter: declared
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-inventory-cargo-toml
      adapter: declared
      path: crates/syu-inventory/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-inventory-src-lib-rs
      adapter: declared
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-planner-cargo-toml
      adapter: declared
      path: crates/syu-planner/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-planner-src-lib-rs
      adapter: declared
      path: crates/syu-planner/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-project-model-cargo-toml
      adapter: declared
      path: crates/syu-project-model/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-project-model-src-lib-rs
      adapter: declared
      path: crates/syu-project-model/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-spec-model-src-lib-rs
      adapter: declared
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-validation-cargo-toml
      adapter: declared
      path: crates/syu-validation/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-validation-src-lib-rs
      adapter: declared
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-validation-src-readiness-rs
      adapter: declared
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-work-model-src-lib-rs
      adapter: declared
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-workbench-server-cargo-toml
      adapter: declared
      path: crates/syu-workbench-server/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-workbench-server-src-lib-rs
      adapter: declared
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: file
    - id: selfhost-crates-syu-workspace-cargo-toml
      adapter: declared
      path: crates/syu-workspace/Cargo.toml
      selector:
        kind: file
    - id: selfhost-crates-syu-workspace-src-lib-rs
      adapter: declared
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: file
    - id: selfhost-src-command-add-rs
      adapter: declared
      path: src/command/add.rs
      selector:
        kind: file
    - id: selfhost-src-command-audit-rs
      adapter: declared
      path: src/command/audit.rs
      selector:
        kind: file
    - id: selfhost-src-command-browse-rs
      adapter: declared
      path: src/command/browse.rs
      selector:
        kind: file
    - id: selfhost-src-command-check-rs
      adapter: declared
      path: src/command/check.rs
      selector:
        kind: file
    - id: selfhost-src-command-completion-rs
      adapter: declared
      path: src/command/completion.rs
      selector:
        kind: file
    - id: selfhost-src-command-doctor-rs
      adapter: declared
      path: src/command/doctor.rs
      selector:
        kind: file
    - id: selfhost-src-command-explain-rs
      adapter: declared
      path: src/command/explain.rs
      selector:
        kind: file
    - id: selfhost-src-command-init-rs
      adapter: declared
      path: src/command/init.rs
      selector:
        kind: file
    - id: selfhost-src-command-issue-text-rs
      adapter: declared
      path: src/command/issue_text.rs
      selector:
        kind: file
    - id: selfhost-src-command-list-rs
      adapter: declared
      path: src/command/list.rs
      selector:
        kind: file
    - id: selfhost-src-command-log-rs
      adapter: declared
      path: src/command/log.rs
      selector:
        kind: file
    - id: selfhost-src-command-lookup-rs
      adapter: declared
      path: src/command/lookup.rs
      selector:
        kind: file
    - id: selfhost-src-command-mod-rs
      adapter: declared
      path: src/command/mod.rs
      selector:
        kind: file
    - id: selfhost-src-command-prompt-rs
      adapter: declared
      path: src/command/prompt.rs
      selector:
        kind: file
    - id: selfhost-src-command-relate-rs
      adapter: declared
      path: src/command/relate.rs
      selector:
        kind: file
    - id: selfhost-src-command-report-rs
      adapter: declared
      path: src/command/report.rs
      selector:
        kind: file
    - id: selfhost-src-command-search-rs
      adapter: declared
      path: src/command/search.rs
      selector:
        kind: file
    - id: selfhost-src-command-show-rs
      adapter: declared
      path: src/command/show.rs
      selector:
        kind: file
    - id: selfhost-src-command-task-rs
      adapter: declared
      path: src/command/task.rs
      selector:
        kind: file
    - id: selfhost-src-command-templates-rs
      adapter: declared
      path: src/command/templates.rs
      selector:
        kind: file
    - id: selfhost-src-command-trace-rs
      adapter: declared
      path: src/command/trace.rs
      selector:
        kind: file
    - id: selfhost-src-command-workbench-rs
      adapter: declared
      path: src/command/workbench.rs
      selector:
        kind: file
    - id: selfhost-src-lib-rs
      adapter: declared
      path: src/lib.rs
      selector:
        kind: file
    - id: selfhost-root-build-module
      adapter: rust
      path: build.rs
      selector:
        kind: module
        name: build
    - id: selfhost-desktop-build-module
      adapter: rust
      path: crates/syu-desktop/build.rs
      selector:
        kind: module
        name: build
    - id: selfhost-desktop-lib-module
      adapter: rust
      path: crates/syu-desktop/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: selfhost-desktop-main-module
      adapter: rust
      path: crates/syu-desktop/src/main.rs
      selector:
        kind: module
        name: main
    targets:
    - id: desktop-capability-anchor
      adapter: rust
      path: crates/syu-desktop/src/lib.rs
      selector:
        kind: symbol
        name: desktop_action_bridge
      claims: []
  - id: self-hosting-documentation
    role: configuration
    facet: tooling
    responsibility: Own documentation and generated-site artifacts as explicit documentation units.
    owns:
    - id: selfhost-docs-generated-site-spec-capabilities-md
      adapter: declared
      path: docs/generated/site-spec/capabilities.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-delivery-md
      adapter: declared
      path: docs/generated/site-spec/delivery.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-foundation-md
      adapter: declared
      path: docs/generated/site-spec/foundation.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-index-md
      adapter: declared
      path: docs/generated/site-spec/index.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-planner-md
      adapter: declared
      path: docs/generated/site-spec/planner.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-public-entrypoints-md
      adapter: declared
      path: docs/generated/site-spec/public-entrypoints.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-work-md
      adapter: declared
      path: docs/generated/site-spec/work.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-workbench-md
      adapter: declared
      path: docs/generated/site-spec/workbench.md
      selector:
        kind: file
    - id: selfhost-docs-generated-site-spec-workbench-features-md
      adapter: declared
      path: docs/generated/site-spec/workbench-features.md
      selector:
        kind: file
    - id: selfhost-docs-guide-command-card-md
      adapter: declared
      path: docs/guide/command-card.md
      selector:
        kind: file
    - id: selfhost-docs-guide-configuration-md
      adapter: declared
      path: docs/guide/configuration.md
      selector:
        kind: file
    - id: selfhost-docs-guide-examples-and-templates-md
      adapter: declared
      path: docs/guide/examples-and-templates.md
      selector:
        kind: file
    - id: selfhost-docs-guide-existing-repository-md
      adapter: declared
      path: docs/guide/existing-repository.md
      selector:
        kind: file
    - id: selfhost-docs-guide-migration-md
      adapter: declared
      path: docs/guide/migration.md
      selector:
        kind: file
    - id: selfhost-docs-guide-spec-antipatterns-md
      adapter: declared
      path: docs/guide/spec-antipatterns.md
      selector:
        kind: file
    - id: selfhost-docs-guide-vscode-extension-md
      adapter: declared
      path: docs/guide/vscode-extension.md
      selector:
        kind: file
    - id: selfhost-docs-syu-capabilities-yaml
      adapter: declared
      path: docs/syu/capabilities.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-delivery-yaml
      adapter: declared
      path: docs/syu/delivery.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-foundation-yaml
      adapter: declared
      path: docs/syu/foundation.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-planner-yaml
      adapter: declared
      path: docs/syu/planner.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-public-entrypoints-yaml
      adapter: declared
      path: docs/syu/public-entrypoints.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-work-yaml
      adapter: declared
      path: docs/syu/work.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-workbench-features-yaml
      adapter: declared
      path: docs/syu/workbench-features.yaml
      selector:
        kind: file
    - id: selfhost-docs-syu-workbench-yaml
      adapter: declared
      path: docs/syu/workbench.yaml
      selector:
        kind: file
    targets:
    - id: documentation-capability-anchor
      adapter: declared
      path: docs/generated/site-spec/index.md
      selector:
        kind: marker
        value: Specification Reference
      claims: []
  - id: self-hosting-tests-fixtures
    role: configuration
    facet: tooling
    responsibility: Own test, fixture, and example artifacts as explicit verification-support units.
    owns:
    - id: selfhost-examples-browser-ui-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/browser-ui/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-browser-ui-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/browser-ui/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-browser-ui-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/browser-ui/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-browser-ui-syu-yaml
      adapter: declared
      path: examples/browser-ui/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-csharp-fallback-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/csharp-fallback/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-csharp-fallback-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/csharp-fallback/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-csharp-fallback-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/csharp-fallback/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-csharp-fallback-syu-yaml
      adapter: declared
      path: examples/csharp-fallback/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-docs-first-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/docs-first/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-docs-first-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/docs-first/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-docs-first-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/docs-first/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-docs-first-syu-yaml
      adapter: declared
      path: examples/docs-first/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-generic-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/generic/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-generic-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/generic/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-generic-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/generic/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-generic-syu-yaml
      adapter: declared
      path: examples/generic/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-go-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/go-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-go-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/go-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-go-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/go-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-go-only-syu-yaml
      adapter: declared
      path: examples/go-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-java-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/java-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-java-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/java-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-java-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/java-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-java-only-syu-yaml
      adapter: declared
      path: examples/java-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-polyglot-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/polyglot/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-polyglot-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/polyglot/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-polyglot-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/polyglot/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-polyglot-syu-yaml
      adapter: declared
      path: examples/polyglot/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-python-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/python-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-python-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/python-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-python-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/python-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-python-only-syu-yaml
      adapter: declared
      path: examples/python-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-ruby-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/ruby-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-ruby-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/ruby-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-ruby-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/ruby-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-ruby-only-syu-yaml
      adapter: declared
      path: examples/ruby-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-rust-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/rust-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-rust-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/rust-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-rust-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/rust-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-rust-only-syu-yaml
      adapter: declared
      path: examples/rust-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-team-scale-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/team-scale/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-team-scale-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/team-scale/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-team-scale-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/team-scale/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-team-scale-syu-yaml
      adapter: declared
      path: examples/team-scale/syu.yaml
      selector:
        kind: file
    - id: selfhost-examples-typescript-only-docs-syu-features-core-core-yaml
      adapter: declared
      path: examples/typescript-only/docs/syu/features/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-typescript-only-docs-syu-policies-policies-yaml
      adapter: declared
      path: examples/typescript-only/docs/syu/policies/policies.yaml
      selector:
        kind: file
    - id: selfhost-examples-typescript-only-docs-syu-requirements-core-core-yaml
      adapter: declared
      path: examples/typescript-only/docs/syu/requirements/core/core.yaml
      selector:
        kind: file
    - id: selfhost-examples-typescript-only-syu-yaml
      adapter: declared
      path: examples/typescript-only/syu.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-rejected-obsolete-pre-release-v1-spec-yaml
      adapter: declared
      path: fixtures/rejected/obsolete-pre-release-v1/spec.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-rejected-obsolete-pre-release-v1-syu-yaml
      adapter: declared
      path: fixtures/rejected/obsolete-pre-release-v1/syu.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-v1-invalid-unknown-field-syu-yaml
      adapter: declared
      path: fixtures/v1/invalid-unknown-field/syu.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-v1-valid-web-app-spec-feature-yaml
      adapter: declared
      path: fixtures/v1/valid-web-app/spec/feature.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-v1-valid-web-app-spec-requirement-yaml
      adapter: declared
      path: fixtures/v1/valid-web-app/spec/requirement.yaml
      selector:
        kind: file
    - id: selfhost-fixtures-v1-valid-web-app-syu-yaml
      adapter: declared
      path: fixtures/v1/valid-web-app/syu.yaml
      selector:
        kind: file
    - id: selfhost-tests-v1-cli-rs
      adapter: declared
      path: tests/v1_cli.rs
      selector:
        kind: file
    - id: selfhost-tests-workbench-smoke-rs
      adapter: declared
      path: tests/workbench_smoke.rs
      selector:
        kind: file
    - id: selfhost-tests-workbench-visual-rs
      adapter: declared
      path: tests/workbench_visual.rs
      selector:
        kind: file
    targets:
    - id: fixture-capability-anchor
      adapter: declared
      path: fixtures/v1/valid-web-app/syu.yaml
      selector:
        kind: marker
        value: 'schema: syu/config/v1'
      claims: []
- id: FEAT-WORKBENCH-SERVER-001
  title: Workbench server
  summary: Expose canonical read, work, validation, edit, and security APIs.
  status: implemented
  bindings:
  - id: server
    role: implementation
    facet: server
    responsibility: Serve canonical Workbench HTTP routes and mutation guards.
    targets:
    - id: mutation-guard
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: mutation_guard
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-005#criterion.secure-local-server
    owns:
    - id: workbench-server-module
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: module
        name: lib
- id: FEAT-WORKBENCH-PROJECTION-001
  title: Workbench projection
  summary: Project canonical readiness, work, scope, specification, and diagnostics state.
  status: implemented
  bindings:
  - id: projection
    role: implementation
    facet: projection
    responsibility: Build the canonical Workbench projection from indexed server data.
    targets:
    - id: project
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: project
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
  - id: projection-contract
    role: contract-source
    facet: projection
    responsibility: Define the canonical projection boundary consumed by verification.
    targets:
    - id: project-contract
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: project
    owns:
    - id: workspace-module
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: inventory-module
      adapter: rust
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: validation-module
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: readiness-module
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: module
        name: readiness
    - id: readiness-file
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: file
    - id: diagnostics-module
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: code-intel-module
      adapter: rust
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: branch-scope-module
      adapter: rust
      path: crates/syu-code-intel/src/branch_scope.rs
      selector:
        kind: module
        name: branch_scope
    - id: project-model-module
      adapter: rust
      path: crates/syu-project-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: domain-module
      adapter: rust
      path: crates/syu-domain/src/lib.rs
      selector:
        kind: module
        name: lib
  contracts:
  - id: projection-verification
    kind: function
    source: FEAT-WORKBENCH-PROJECTION-001#binding.projection-contract/target.project-contract
    participants:
    - target: FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
      role: producer
    - target: FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
      role: verifier
    - target: FEAT-WORKBENCH-VERIFICATION-001#binding.test-exposure/target.server-e2e
      role: verifier
    guarantees:
    - REQ-WORKBENCH-001#criterion.canonical-projection
- id: FEAT-WORKBENCH-WORK-UI-001
  title: Workbench work UI
  summary: Drive the WorkRequest to result-validation journey from the Work page.
  status: implemented
  bindings:
  - id: work
    role: implementation
    facet: work
    responsibility: Plan and validate a bounded Workbench work session.
    targets:
    - id: plan
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_plan
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-002#criterion.work-session
    owns:
    - id: planner-module
      adapter: rust
      path: crates/syu-planner/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: work-model-module
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: cli-module
      adapter: rust
      path: src/lib.rs
      selector:
        kind: module
        name: lib
    - id: cli-main-module
      adapter: rust
      path: src/main.rs
      selector:
        kind: module
        name: main
    - id: cli-main-file
      adapter: rust
      path: src/main.rs
      selector:
        kind: file
    - id: lsp-module
      adapter: rust
      path: src/lsp/mod.rs
      selector:
        kind: module
        name: mod
    - id: lsp-file
      adapter: rust
      path: src/lsp/mod.rs
      selector:
        kind: file
    - id: lsp-handlers-module
      adapter: rust
      path: src/lsp/handlers.rs
      selector:
        kind: module
        name: handlers
    - id: lsp-handlers-file
      adapter: rust
      path: src/lsp/handlers.rs
      selector:
        kind: file
    - id: lsp-protocol-module
      adapter: rust
      path: src/lsp/protocol.rs
      selector:
        kind: module
        name: protocol
    - id: lsp-protocol-file
      adapter: rust
      path: src/lsp/protocol.rs
      selector:
        kind: file
    - id: lsp-server-module
      adapter: rust
      path: src/lsp/server.rs
      selector:
        kind: module
        name: server
    - id: lsp-server-file
      adapter: rust
      path: src/lsp/server.rs
      selector:
        kind: file
- id: FEAT-WORKBENCH-SPEC-EDITOR-001
  title: Workbench specification editor
  summary: Preview and apply structured specification and configuration edits transactionally.
  status: implemented
  bindings:
  - id: editor
    role: implementation
    facet: editor
    responsibility: Apply validated structured edits through the Workbench transaction boundary.
    targets:
    - id: specification-apply
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_specification_apply
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-003#criterion.transactional-spec-edit
    - id: config-apply
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_config_apply
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-004#criterion.transactional-config-edit
    owns:
    - id: spec-model-module
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: app-ui-components-module
      adapter: rust
      path: crates/syu-app-ui/src/components.rs
      selector:
        kind: module
        name: components
    - id: app-ui-components-file
      adapter: rust
      path: crates/syu-app-ui/src/components.rs
      selector:
        kind: file
    - id: app-ui-document-module
      adapter: rust
      path: crates/syu-app-ui/src/document.rs
      selector:
        kind: module
        name: document
    - id: app-ui-document-file
      adapter: rust
      path: crates/syu-app-ui/src/document.rs
      selector:
        kind: file
    - id: app-ui-lib-module
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: app-ui-pages-module
      adapter: rust
      path: crates/syu-app-ui/src/pages.rs
      selector:
        kind: module
        name: pages
    - id: app-ui-pages-file
      adapter: rust
      path: crates/syu-app-ui/src/pages.rs
      selector:
        kind: file
    - id: app-ui-shell-module
      adapter: rust
      path: crates/syu-app-ui/src/shell.rs
      selector:
        kind: module
        name: shell
    - id: app-ui-shell-file
      adapter: rust
      path: crates/syu-app-ui/src/shell.rs
      selector:
        kind: file
- id: FEAT-WORKBENCH-NAVIGATION-001
  title: Workbench navigation
  summary: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  status: implemented
  bindings:
  - id: navigation
    role: implementation
    facet: tooling
    responsibility: Implement browser navigation and accessibility behavior as typed server DTO consumers.
    targets:
    - id: html-navigation
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-page="work"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: javascript-navigation
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindRouter
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: keyboard-navigation
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindKeyboardTabs
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: accessibility-attributes
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-i18n-aria="a11y.main_pages"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    owns:
    - id: assets-i18n-module
      adapter: javascript
      path: crates/syu-app-ui/assets/i18n.js
      selector:
        kind: module
        name: '*'
    - id: assets-api-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: module
        name: '*'
    - id: assets-action-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/action.js
      selector:
        kind: module
        name: '*'
    - id: assets-diagnostic-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/diagnostic.js
      selector:
        kind: module
        name: '*'
    - id: assets-editor-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/editor.js
      selector:
        kind: module
        name: '*'
    - id: assets-readiness-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-target-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/target.js
      selector:
        kind: module
        name: '*'
    - id: assets-js-i18n-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/i18n.js
      selector:
        kind: module
        name: '*'
    - id: assets-main-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: module
        name: '*'
    - id: assets-router-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: module
        name: '*'
    - id: assets-state-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-diagnostics-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-readiness-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-scope-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/scope.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-settings-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/settings.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-specifications-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-work-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector:
        kind: module
        name: '*'
    - id: html-file
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: file
- id: FEAT-WORKBENCH-VERIFICATION-001
  title: Workbench verification tests
  summary: Exercise the real Workbench HTTP server and exact verification targets.
  status: implemented
  bindings:
  - id: verification-harness
    role: implementation
    facet: verification
    responsibility: Maintain the executable Workbench HTTP verification harness.
    targets:
    - id: verification-endpoint
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_verify
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
    owns:
    - id: cli-test-module
      adapter: rust
      path: tests/v1_cli.rs
      selector:
        kind: module
        name: v1_cli
    - id: workbench-command-test-module
      adapter: rust
      path: tests/workbench_command.rs
      selector:
        kind: module
        name: workbench_command
    - id: workbench-smoke-test-module
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: module
        name: workbench_smoke
    - id: workbench-visual-test-module
      adapter: rust
      path: tests/workbench_visual.rs
      selector:
        kind: module
        name: workbench_visual
  - id: test-exposure
    role: verification
    facet: verification
    responsibility: Expose every executable Workbench test as an exact verification target.
    targets:
    - id: server-e2e
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: tests::workbench_http_closed_loop_flow
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
        runner:
          runner: cargo-test
          arguments:
            package: syu-workbench-server
            test: tests::workbench_http_closed_loop_flow
    - id: server-transport
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: tests::workbench_http_server_transport_flow
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
        runner:
          runner: cargo-test
          arguments:
            package: syu-workbench-server
            test: tests::workbench_http_server_transport_flow
    - id: command-help
      adapter: rust
      path: tests/workbench_command.rs
      selector:
        kind: symbol
        name: workbench_help_lists_browser_launch_options
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-002#criterion.work-session
        covers:
        - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_command
            test: workbench_help_lists_browser_launch_options
    - id: smoke-projection
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: workbench_projection_is_server_owned_and_starts_not_run
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_smoke
            test: workbench_projection_is_server_owned_and_starts_not_run
    - id: smoke-module-contract
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: rendered_workbench_uses_external_module_assets_and_specifications_route
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_smoke
            test: rendered_workbench_uses_external_module_assets_and_specifications_route
    - id: smoke-dto-contract
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: browser_modules_render_dtos_without_model_inference
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_smoke
            test: browser_modules_render_dtos_without_model_inference
    - id: smoke-keyboard
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: workbench_tabs_are_keyboard_navigable
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_smoke
            test: workbench_tabs_are_keyboard_navigable
    - id: visual-dom
      adapter: rust
      path: tests/workbench_visual.rs
      selector:
        kind: symbol
        name: workbench_rendered_dom_uses_projection_driven_placeholders
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_visual
            test: workbench_rendered_dom_uses_projection_driven_placeholders
```
