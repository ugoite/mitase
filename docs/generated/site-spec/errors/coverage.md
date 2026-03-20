---
title: "Coverage"
description: "Generated reference for docs/spec/errors/coverage.yaml"
---

> Generated from `docs/spec/errors/coverage.yaml`.

## Parsed content

### Genre

- Coverage Discipline

### Version

- 1

### Rules

- **code**: coverage-scan-failed
  - **severity**: error
  - **title**: Coverage inventory must be inspectable
  - **summary**: Opt-in trace coverage cannot be enforced unless `syu` can scan the workspace.
  - **description**:
    - |
      The strict trace coverage rule only means something when `syu` can read and
      parse the files it is supposed to inventory. If source or test files cannot
      be scanned, the safest behavior is to fail loudly instead of pretending the
      workspace is fully covered. This rule protects the credibility of the
      100-percent coverage claim.
- **code**: public-symbol-untracked
  - **severity**: error
  - **title**: Public API symbols must belong to at least one feature
  - **summary**: Public surface area should never appear in the repository without an owning feature.
  - **description**:
    - |
      Public functions, classes, modules, and similar API-facing symbols create
      long-term maintenance obligations. If they exist without a feature link,
      the repository has no durable explanation for why that surface area exists
      or which behavior owns it. This rule encourages deliberate API growth by
      forcing public surface area to stay attached to an explicit feature.
- **code**: test-symbol-untracked
  - **severity**: error
  - **title**: Tests must belong to at least one requirement
  - **summary**: Every test should prove some requirement instead of becoming orphaned automation.
  - **description**:
    - |
      Tests are executable claims about intended behavior. When a test has no
      requirement owner, it becomes much harder to tell whether it is obsolete,
      redundant, or still essential to the product. This rule keeps the test
      suite aligned with the specification by ensuring that each test remains
      justified by at least one requirement.

## Source YAML

```yaml
genre: Coverage Discipline
version: 1

rules:
  - code: coverage-scan-failed
    severity: error
    title: Coverage inventory must be inspectable
    summary: Opt-in trace coverage cannot be enforced unless `syu` can scan the workspace.
    description: |
      The strict trace coverage rule only means something when `syu` can read and
      parse the files it is supposed to inventory. If source or test files cannot
      be scanned, the safest behavior is to fail loudly instead of pretending the
      workspace is fully covered. This rule protects the credibility of the
      100-percent coverage claim.

  - code: public-symbol-untracked
    severity: error
    title: Public API symbols must belong to at least one feature
    summary: Public surface area should never appear in the repository without an owning feature.
    description: |
      Public functions, classes, modules, and similar API-facing symbols create
      long-term maintenance obligations. If they exist without a feature link,
      the repository has no durable explanation for why that surface area exists
      or which behavior owns it. This rule encourages deliberate API growth by
      forcing public surface area to stay attached to an explicit feature.

  - code: test-symbol-untracked
    severity: error
    title: Tests must belong to at least one requirement
    summary: Every test should prove some requirement instead of becoming orphaned automation.
    description: |
      Tests are executable claims about intended behavior. When a test has no
      requirement owner, it becomes much harder to tell whether it is obsolete,
      redundant, or still essential to the product. This rule keeps the test
      suite aligned with the specification by ensuring that each test remains
      justified by at least one requirement.
```
