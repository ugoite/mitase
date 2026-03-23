---
title: "Documentation / Docs"
description: "Generated reference for docs/syu/features/documentation/docs.yaml"
---

> Generated from `docs/syu/features/documentation/docs.yaml`.

## Parsed content

### Category

- Documentation

### Version

- 1

### Features

- **id**: FEAT-DOCS-001
  - **title**: English concepts and workflow documentation
  - **summary**: Explain the four-layer model, delivery states, configuration, and command workflow in English.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-010
  - **implementations**:
    - **markdown**:
      - **file**: README.md
        - **symbols**:
          - FEAT-DOCS-001
          - syu validate
          - syu browse
          - philosophy
          - planned
      - **file**: docs/guide/concepts.md
        - **symbols**:
          - FEAT-DOCS-001
          - policy
          - requirements
          - features
          - planned
          - implemented
      - **file**: docs/guide/getting-started.md
        - **symbols**:
          - FEAT-DOCS-001
          - syu init
          - syu browse
          - syu validate
      - **file**: docs/guide/configuration.md
        - **symbols**:
          - FEAT-DOCS-001
          - validate.default_fix
          - validate.allow_planned
          - validate.require_non_orphaned_items
          - validate.require_reciprocal_links
          - validate.require_symbol_trace_coverage
          - app.bind
          - app.port
          - report.output
          - runtimes.python.command
- **id**: FEAT-DOCS-002
  - **title**: Docusaurus documentation site
  - **summary**: Render and publish the checked-in docs tree as a documentation site without maintaining a separate content source.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-010
  - **implementations**:
    - **json**:
      - **file**: website/package.json
        - **symbols**:
          - FEAT-DOCS-002
          - @docusaurus/core
          - docusaurus build
    - **javascript**:
      - **file**: website/docusaurus.config.js
        - **symbols**:
          - config
      - **file**: website/src/pages/index.js
        - **symbols**:
          - Home
    - **yaml**:
      - **file**: .github/actions/build-docs-site/action.yml
        - **symbols**:
          - actions/setup-node@v6
          - cache-dependency-path: website/package-lock.json
          - npm ci
          - npm run build
      - **file**: .github/workflows/deploy-pages.yml
        - **symbols**:
          - deploy-pages
          - actions/configure-pages@v5
          - actions/upload-pages-artifact@v4
          - actions/deploy-pages@v4

## Source YAML

```yaml
category: Documentation
version: 1

features:
  - id: FEAT-DOCS-001
    title: English concepts and workflow documentation
    summary: Explain the four-layer model, delivery states, configuration, and command workflow in English.
    status: implemented
    linked_requirements:
      - REQ-CORE-010
    implementations:
      markdown:
        - file: README.md
          symbols:
            - FEAT-DOCS-001
            - syu validate
            - syu browse
            - philosophy
            - planned
        - file: docs/guide/concepts.md
          symbols:
            - FEAT-DOCS-001
            - policy
            - requirements
            - features
            - planned
            - implemented
        - file: docs/guide/getting-started.md
          symbols:
            - FEAT-DOCS-001
            - syu init
            - syu browse
            - syu validate
        - file: docs/guide/configuration.md
          symbols:
            - FEAT-DOCS-001
            - validate.default_fix
            - validate.allow_planned
            - validate.require_non_orphaned_items
            - validate.require_reciprocal_links
            - validate.require_symbol_trace_coverage
            - app.bind
            - app.port
            - report.output
            - runtimes.python.command

  - id: FEAT-DOCS-002
    title: Docusaurus documentation site
    summary: Render and publish the checked-in docs tree as a documentation site without maintaining a separate content source.
    status: implemented
    linked_requirements:
      - REQ-CORE-010
    implementations:
      json:
        - file: website/package.json
          symbols:
            - FEAT-DOCS-002
            - "@docusaurus/core"
            - docusaurus build
      javascript:
        - file: website/docusaurus.config.js
          symbols:
            - config
        - file: website/src/pages/index.js
          symbols:
            - Home
      yaml:
        - file: .github/actions/build-docs-site/action.yml
          symbols:
            - actions/setup-node@v6
            - "cache-dependency-path: website/package-lock.json"
            - npm ci
            - npm run build
        - file: .github/workflows/deploy-pages.yml
          symbols:
            - deploy-pages
            - actions/configure-pages@v5
            - actions/upload-pages-artifact@v4
            - actions/deploy-pages@v4
```
