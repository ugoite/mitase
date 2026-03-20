---
title: "Agent Skills / Skills"
description: "Generated reference for docs/syu/features/skills.yaml"
---

> Generated from `docs/syu/features/skills.yaml`.

## Parsed content

### Category

- Agent Skills

### Version

- 1

### Features

- **id**: FEAT-SKILLS-001
  - **title**: Checked-in syu maintainer skill
  - **summary**: Provide an agent skill that teaches repeatable syu workflows for inspecting, updating, validating, and reporting on a repository.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-016
  - **implementations**:
    - **markdown**:
      - **file**: skills/README.md
        - **symbols**:
          - SKILL.md
          - Anthropics Skills
      - **file**: skills/syu-maintainer/SKILL.md
        - **symbols**:
          - name: syu-maintainer
          - syu validate .
          - syu report . --output docs/generated/syu-report.md
      - **file**: README.md
        - **symbols**:
          - skills/syu-maintainer/SKILL.md
          - Agent skill

## Source YAML

```yaml
category: Agent Skills
version: 1

features:
  - id: FEAT-SKILLS-001
    title: Checked-in syu maintainer skill
    summary: Provide an agent skill that teaches repeatable syu workflows for inspecting, updating, validating, and reporting on a repository.
    status: implemented
    linked_requirements:
      - REQ-CORE-016
    implementations:
      markdown:
        - file: skills/README.md
          symbols:
            - SKILL.md
            - Anthropics Skills
        - file: skills/syu-maintainer/SKILL.md
          symbols:
            - "name: syu-maintainer"
            - syu validate .
            - syu report . --output docs/generated/syu-report.md
        - file: README.md
          symbols:
            - skills/syu-maintainer/SKILL.md
            - Agent skill
```
