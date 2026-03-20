---
title: "Report Command / Report"
description: "Generated reference for docs/syu/features/cli/report.yaml"
---

> Generated from `docs/syu/features/cli/report.yaml`.

## Parsed content

### Category

- Report Command

### Version

- 1

### Features

- **id**: FEAT-REPORT-001
  - **title**: Markdown report generation
  - **summary**: Render validation results as Markdown and optionally write a file.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-004
  - **implementations**:
    - **rust**:
      - **file**: src/command/report.rs
        - **symbols**:
          - run_report_command
      - **file**: src/report.rs
        - **symbols**:
          - render_markdown_report

## Source YAML

```yaml
category: Report Command
version: 1

features:
  - id: FEAT-REPORT-001
    title: Markdown report generation
    summary: Render validation results as Markdown and optionally write a file.
    status: implemented
    linked_requirements:
      - REQ-CORE-004
    implementations:
      rust:
        - file: src/command/report.rs
          symbols:
            - run_report_command
        - file: src/report.rs
          symbols:
            - render_markdown_report
```
