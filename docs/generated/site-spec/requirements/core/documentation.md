---
title: "Core Documentation / Documentation"
description: "Generated reference for docs/syu/requirements/core/documentation.yaml"
---

> Generated from `docs/syu/requirements/core/documentation.yaml`.

## Parsed content

### Category

- Core Documentation

### Prefix

- REQ-CORE

### Requirements

- **id**: REQ-CORE-010
  - **title**: Provide English documentation for concepts, workflows, and the docs site
  - **description**:
    - |
      The repository MUST explain philosophy, policy, requirements, and
      features in English, describe why each layer exists, document how to
      define them, explain `planned` / `implemented` delivery states, show how
      to use `init`, `browse`, `validate`, `report`, config, and autofix, orient
      first-time users with actionable root CLI help, and publish the checked-in
      `docs/` tree through a Docusaurus site.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-002
    - POL-005
  - **linked_features**:
    - FEAT-DOCS-001
    - FEAT-DOCS-002
  - **tests**:
    - **rust**:
      - **file**: tests/help_command.rs
        - **symbols**:
          - root_help_includes_start_here_guidance
          - workspace_help_uses_current_directory_default_consistently
          - init_help_lists_starter_templates
          - init_help_mentions_custom_spec_roots
          - validate_help_lists_temporary_config_overrides
      - **file**: tests/repository_quality.rs
        - **symbols**:
          - repository_declares_documentation_guides
      - **file**: tests/site_docs_generator.rs
        - **symbols**:
          - site_docs_generator_accepts_absolute_spec_roots_outside_repo
- **id**: REQ-CORE-016
  - **title**: Ship an agent skill for maintaining syu-driven repositories
  - **description**:
    - |
      The repository MUST include at least one checked-in agent skill inspired
      by Anthropics Skills that teaches an agent how to work with `syu`
      repositories. The skill SHOULD explain when to inspect the layered model,
      how to update philosophy / policy / requirement / feature links, which
      commands to run to validate and regenerate reports, and how to keep
      changes reviewable instead of treating YAML as disconnected prose.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
    - POL-006
  - **linked_features**:
    - FEAT-SKILLS-001
  - **tests**:
    - **rust**:
      - **file**: tests/repository_quality.rs
        - **symbols**:
          - repository_ships_agent_skill

## Source YAML

```yaml
category: Core Documentation
prefix: REQ-CORE
requirements:
  - id: REQ-CORE-010
    title: Provide English documentation for concepts, workflows, and the docs site
    description: |
      The repository MUST explain philosophy, policy, requirements, and
      features in English, describe why each layer exists, document how to
      define them, explain `planned` / `implemented` delivery states, show how
      to use `init`, `browse`, `validate`, `report`, config, and autofix, orient
      first-time users with actionable root CLI help, and publish the checked-in
      `docs/` tree through a Docusaurus site.
    priority: medium
    status: implemented
    linked_policies:
      - POL-002
      - POL-005
    linked_features:
      - FEAT-DOCS-001
      - FEAT-DOCS-002
    tests:
      rust:
        - file: tests/help_command.rs
          symbols:
            - root_help_includes_start_here_guidance
            - workspace_help_uses_current_directory_default_consistently
            - init_help_lists_starter_templates
            - init_help_mentions_custom_spec_roots
            - validate_help_lists_temporary_config_overrides
        - file: tests/repository_quality.rs
          symbols:
            - repository_declares_documentation_guides
        - file: tests/site_docs_generator.rs
          symbols:
            - site_docs_generator_accepts_absolute_spec_roots_outside_repo
  - id: REQ-CORE-016
    title: Ship an agent skill for maintaining syu-driven repositories
    description: |
      The repository MUST include at least one checked-in agent skill inspired
      by Anthropics Skills that teaches an agent how to work with `syu`
      repositories. The skill SHOULD explain when to inspect the layered model,
      how to update philosophy / policy / requirement / feature links, which
      commands to run to validate and regenerate reports, and how to keep
      changes reviewable instead of treating YAML as disconnected prose.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
      - POL-006
    linked_features:
      - FEAT-SKILLS-001
    tests:
      rust:
        - file: tests/repository_quality.rs
          symbols:
            - repository_ships_agent_skill
```
