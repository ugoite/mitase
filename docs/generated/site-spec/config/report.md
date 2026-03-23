---
title: "Configuration / Report"
description: "Generated reference for docs/syu/config/report.yaml"
---

> Generated from `docs/syu/config/report.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Section

- report

### Precedence

- **output**:
  - --output
  - report.output
  - stdout

### Items

- **key**: report.output
  - **type**: string
  - **default**: stdout
  - **summary**: Sets the default Markdown output path for `syu report`.
  - **description**:
    - |
      When present, `syu report` writes to this path unless `--output` is
      passed. Relative paths are resolved from the workspace root so checked-in
      config can describe one stable repository-native report artifact.

## Source YAML

```yaml
# FEAT-REPORT-001
category: Configuration
version: 1
section: report
precedence:
  output:
    - --output
    - report.output
    - stdout
items:
  - key: report.output
    type: string
    default: stdout
    summary: Sets the default Markdown output path for `syu report`.
    description: |
      When present, `syu report` writes to this path unless `--output` is
      passed. Relative paths are resolved from the workspace root so checked-in
      config can describe one stable repository-native report artifact.
```
