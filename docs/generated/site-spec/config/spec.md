---
title: "Configuration / Spec"
description: "Generated reference for docs/syu/config/spec.yaml"
---

> Generated from `docs/syu/config/spec.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Section

- spec

### Items

- **key**: spec.root
  - **type**: path
  - **default**: docs/syu
  - **summary**: Controls where `syu` resolves the authored specification root.
  - **description**:
    - |
      Relative paths are resolved from the workspace root. Absolute paths are
      also supported so generated docs or external specification trees can live
      outside the repository when needed. New workspaces default to `docs/syu`.
  - **accepts**:
    - relative paths from the workspace root
    - absolute paths outside the repository

## Source YAML

```yaml
category: Configuration
version: 1
section: spec
items:
  - key: spec.root
    type: path
    default: docs/syu
    summary: Controls where `syu` resolves the authored specification root.
    description: |
      Relative paths are resolved from the workspace root. Absolute paths are
      also supported so generated docs or external specification trees can live
      outside the repository when needed. New workspaces default to `docs/syu`.
    accepts:
      - relative paths from the workspace root
      - absolute paths outside the repository
```
