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
  - **summary**: Render validation results as Markdown and optionally write a file or use a checked-in default output path.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-004
  - **implementations**:
    - **rust**:
      - **file**: src/cli.rs
        - **symbols**:
          - ReportArgs
      - **file**: src/config.rs
        - **symbols**:
          - ReportConfig
      - **file**: src/command/report.rs
        - **symbols**:
          - run_report_command
      - **file**: src/report.rs
        - **symbols**:
          - render_markdown_report
    - **yaml**:
      - **file**: syu.yaml
        - **symbols**:
          - FEAT-REPORT-001
          - docs/generated/syu-report.md
      - **file**: docs/syu/config/report.yaml
        - **symbols**:
          - FEAT-REPORT-001
          - report.output

## Source YAML

```yaml
category: Report Command
version: 1

features:
  - id: FEAT-REPORT-001
    title: Markdown report generation
    summary: Render validation results as Markdown and optionally write a file or use a checked-in default output path.
    status: implemented
    linked_requirements:
      - REQ-CORE-004
    implementations:
      rust:
        - file: src/cli.rs
          symbols:
            - ReportArgs
        - file: src/config.rs
          symbols:
            - ReportConfig
        - file: src/command/report.rs
          symbols:
            - run_report_command
        - file: src/report.rs
          symbols:
            - render_markdown_report
      yaml:
        - file: syu.yaml
          symbols:
            - FEAT-REPORT-001
            - docs/generated/syu-report.md
        - file: docs/syu/config/report.yaml
          symbols:
            - FEAT-REPORT-001
            - report.output
```
