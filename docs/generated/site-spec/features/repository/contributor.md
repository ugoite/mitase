---
title: "Contributor Experience / Contributor"
description: "Generated reference for docs/syu/features/repository/contributor.yaml"
---

> Generated from `docs/syu/features/repository/contributor.yaml`.

## Parsed content

### Category

- Contributor Experience

### Version

- 1

### Features

- **id**: FEAT-CONTRIB-001
  - **title**: Contributor devcontainer and example workspaces
  - **summary**: Provide a ready-to-use contributor container and validated example projects, including unsupported-language adoption examples.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-011
    - REQ-CORE-012
  - **implementations**:
    - **json**:
      - **file**: .devcontainer/devcontainer.json
        - **symbols**:
          - FEAT-CONTRIB-001
          - postCreateCommand
    - **shell**:
      - **file**: .devcontainer/post-create.sh
        - **symbols**:
          - FEAT-CONTRIB-001
          - install_coverage_tooling
          - install_wasm_tooling
          - install_precommit_tooling
          - main
      - **file**: .devcontainer/setup-browser-tooling.sh
        - **symbols**:
          - FEAT-CONTRIB-001
          - main
    - **rust**:
      - **file**: tests/example_workspaces.rs
        - **symbols**:
          - FEAT-CONTRIB-001
          - rust_only_example_validates
          - python_only_example_validates
          - go_only_example_validates
          - polyglot_example_validates
- **id**: FEAT-CONTRIB-002
  - **title**: Contributor workflow templates
  - **summary**: Guide GitHub Flow contributions with repository-native docs, forms, and low-noise status defaults.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-013
  - **implementations**:
    - **markdown**:
      - **file**: CONTRIBUTING.md
        - **symbols**:
          - FEAT-CONTRIB-002
          - GitHub Flow
          - .worktrees/
          - scripts/ci/pinned-npm.sh install app
          - scripts/ci/pinned-npm.sh install website
          - scripts/ci/quality-gates.sh
          - scripts/ci/validate-app.sh
          - scripts/ci/validate-website.sh
      - **file**: .github/pull_request_template.md
        - **symbols**:
          - FEAT-CONTRIB-002
          - scripts/ci/quality-gates.sh
          - cargo run -- validate .
    - **yaml**:
      - **file**: .github/ISSUE_TEMPLATE/bug_report.yml
        - **symbols**:
          - FEAT-CONTRIB-002
          - What happened?
          - Steps to reproduce
      - **file**: .github/ISSUE_TEMPLATE/feature_request.yml
        - **symbols**:
          - FEAT-CONTRIB-002
          - What problem are you trying to solve?
          - Specification impact
      - **file**: .github/ISSUE_TEMPLATE/config.yml
        - **symbols**:
          - FEAT-CONTRIB-002
          - blank_issues_enabled
          - contact_links
    - **gitignore**:
      - **file**: .gitignore
        - **symbols**:
          - FEAT-CONTRIB-002
          - /.worktrees/
- **id**: FEAT-CONTRIB-003
  - **title**: One-command pre-commit setup
  - **summary**: Install pre-commit and activate local hooks without asking contributors to assemble the commands themselves.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-013
  - **implementations**:
    - **shell**:
      - **file**: scripts/install-precommit.sh
        - **symbols**:
          - find_python
          - install_precommit
          - install_hooks
          - main

## Source YAML

```yaml
category: Contributor Experience
version: 1

features:
  - id: FEAT-CONTRIB-001
    title: Contributor devcontainer and example workspaces
    summary: Provide a ready-to-use contributor container and validated example projects, including unsupported-language adoption examples.
    status: implemented
    linked_requirements:
      - REQ-CORE-011
      - REQ-CORE-012
    implementations:
      json:
        - file: .devcontainer/devcontainer.json
          symbols:
            - FEAT-CONTRIB-001
            - postCreateCommand
      shell:
        - file: .devcontainer/post-create.sh
          symbols:
            - FEAT-CONTRIB-001
            - install_coverage_tooling
            - install_wasm_tooling
            - install_precommit_tooling
            - main
        - file: .devcontainer/setup-browser-tooling.sh
          symbols:
            - FEAT-CONTRIB-001
            - main
      rust:
        - file: tests/example_workspaces.rs
          symbols:
            - FEAT-CONTRIB-001
            - rust_only_example_validates
            - python_only_example_validates
            - go_only_example_validates
            - polyglot_example_validates

  - id: FEAT-CONTRIB-002
    title: Contributor workflow templates
    summary: Guide GitHub Flow contributions with repository-native docs, forms, and low-noise status defaults.
    status: implemented
    linked_requirements:
      - REQ-CORE-013
    implementations:
      markdown:
        - file: CONTRIBUTING.md
          symbols:
            - FEAT-CONTRIB-002
            - GitHub Flow
            - .worktrees/
            - scripts/ci/pinned-npm.sh install app
            - scripts/ci/pinned-npm.sh install website
            - scripts/ci/quality-gates.sh
            - scripts/ci/validate-app.sh
            - scripts/ci/validate-website.sh
        - file: .github/pull_request_template.md
          symbols:
            - FEAT-CONTRIB-002
            - scripts/ci/quality-gates.sh
            - cargo run -- validate .
      yaml:
        - file: .github/ISSUE_TEMPLATE/bug_report.yml
          symbols:
            - FEAT-CONTRIB-002
            - What happened?
            - Steps to reproduce
        - file: .github/ISSUE_TEMPLATE/feature_request.yml
          symbols:
            - FEAT-CONTRIB-002
            - What problem are you trying to solve?
            - Specification impact
        - file: .github/ISSUE_TEMPLATE/config.yml
          symbols:
            - FEAT-CONTRIB-002
            - blank_issues_enabled
            - contact_links
      gitignore:
        - file: .gitignore
          symbols:
            - FEAT-CONTRIB-002
            - /.worktrees/

  - id: FEAT-CONTRIB-003
    title: One-command pre-commit setup
    summary: Install pre-commit and activate local hooks without asking contributors to assemble the commands themselves.
    status: implemented
    linked_requirements:
      - REQ-CORE-013
    implementations:
      shell:
        - file: scripts/install-precommit.sh
          symbols:
            - find_python
            - install_precommit
            - install_hooks
            - main
```
