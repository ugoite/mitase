---
title: "Doctor Command / Doctor"
description: "Generated reference for docs/syu/features/cli/doctor.yaml"
---

> Generated from `docs/syu/features/cli/doctor.yaml`.

## Parsed content

### Category

- Doctor Command

### Version

- 1

### Features

- **id**: FEAT-DOCTOR-001
  - **title**: Contributor readiness doctor command
  - **summary**: Report the current Rust, Node, npm, dependency-install, and Playwright readiness state before local contributor checks begin.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-026
  - **implementations**:
    - **rust**:
      - **file**: src/command/doctor.rs
        - **symbols**:
          - *
      - **file**: src/cli.rs
        - **symbols**:
          - FEAT-DOCTOR-001
          - Commands
          - DoctorArgs
      - **file**: src/lib.rs
        - **symbols**:
          - run_dispatch
          - dispatch
    - **markdown**:
      - **file**: CONTRIBUTING.md
        - **symbols**:
          - FEAT-DOCTOR-001
          - syu doctor .
      - **file**: README.md
        - **symbols**:
          - FEAT-DOCTOR-001
          - syu doctor .

## Source YAML

```yaml
category: Doctor Command
version: 1

features:
  - id: FEAT-DOCTOR-001
    title: Contributor readiness doctor command
    summary: Report the current Rust, Node, npm, dependency-install, and Playwright readiness state before local contributor checks begin.
    status: implemented
    linked_requirements:
      - REQ-CORE-026
    implementations:
      rust:
        - file: src/command/doctor.rs
          symbols:
            - "*"
        - file: src/cli.rs
          symbols:
            - FEAT-DOCTOR-001
            - Commands
            - DoctorArgs
        - file: src/lib.rs
          symbols:
            - run_dispatch
            - dispatch
      markdown:
        - file: CONTRIBUTING.md
          symbols:
            - FEAT-DOCTOR-001
            - syu doctor .
        - file: README.md
          symbols:
            - FEAT-DOCTOR-001
            - syu doctor .
```
