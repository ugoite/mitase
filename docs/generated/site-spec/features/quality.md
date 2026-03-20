---
title: "Quality Automation / Quality"
description: "Generated reference for docs/syu/features/quality.yaml"
---

> Generated from `docs/syu/features/quality.yaml`.

## Parsed content

### Category

- Quality Automation

### Version

- 1

### Features

- **id**: FEAT-QUALITY-001
  - **title**: Repository quality automation
  - **summary**: Keep self-validation, dependency hygiene, code scanning, and merge-queue-safe CI execution fast enough to run on every pull request.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-005
    - REQ-CORE-006
    - REQ-CORE-014
  - **implementations**:
    - **shell**:
      - **file**: scripts/ci/quality-gates.sh
        - **symbols**:
          - run_quality_gates
      - **file**: scripts/ci/coverage.sh
        - **symbols**:
          - configure_llvm_tools
          - run_coverage
    - **yaml**:
      - **file**: .pre-commit-config.yaml
        - **symbols**:
          - syu-validate-self
          - syu-quality-gates
          - syu-coverage-gate
      - **file**: .github/workflows/ci.yml
        - **symbols**:
          - merge_group
          - precommit
          - quality
          - coverage
          - actionlint
          - Restore Rust cache
          - Set up Python with pip cache
          - Review dependency changes
      - **file**: .github/workflows/codeql.yml
        - **symbols**:
          - merge_group
          - Analyze (rust)
          - github/codeql-action/init@v3
      - **file**: .github/dependabot.yml
        - **symbols**:
          - FEAT-QUALITY-001
          - package-ecosystem
          - target-branch

## Source YAML

```yaml
category: Quality Automation
version: 1

features:
  - id: FEAT-QUALITY-001
    title: Repository quality automation
    summary: Keep self-validation, dependency hygiene, code scanning, and merge-queue-safe CI execution fast enough to run on every pull request.
    status: implemented
    linked_requirements:
      - REQ-CORE-005
      - REQ-CORE-006
      - REQ-CORE-014
    implementations:
      shell:
        - file: scripts/ci/quality-gates.sh
          symbols:
            - run_quality_gates
        - file: scripts/ci/coverage.sh
          symbols:
            - configure_llvm_tools
            - run_coverage
      yaml:
        - file: .pre-commit-config.yaml
          symbols:
            - syu-validate-self
            - syu-quality-gates
            - syu-coverage-gate
        - file: .github/workflows/ci.yml
          symbols:
            - merge_group
            - precommit
            - quality
            - coverage
            - actionlint
            - Restore Rust cache
            - Set up Python with pip cache
            - Review dependency changes
        - file: .github/workflows/codeql.yml
          symbols:
            - merge_group
            - Analyze (rust)
            - github/codeql-action/init@v3
        - file: .github/dependabot.yml
          symbols:
            - FEAT-QUALITY-001
            - package-ecosystem
            - target-branch
```
