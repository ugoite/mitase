---
title: "Editor Integrations / Vscode"
description: "Generated reference for docs/syu/features/editor/vscode.yaml"
---

> Generated from `docs/syu/features/editor/vscode.yaml`.

## Parsed content

### Category

- Editor Integrations

### Version

- 1

### Features

- **id**: FEAT-VSCODE-001
  - **title**: VS Code navigation and diagnostics integration
  - **summary**: Surface `syu` validation diagnostics and trace/navigation workflows directly inside VS Code.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-022
  - **implementations**:
    - **javascript**:
      - **file**: editors/vscode/src/extension.js
        - **symbols**:
          - activate
          - registerCommands
          - SyuContextTreeProvider
      - **file**: editors/vscode/src/model.js
        - **symbols**:
          - loadSpecModel
          - lookupTrace
          - loadDiagnostics
    - **json**:
      - **file**: editors/vscode/package.json
        - **symbols**:
          - syu.refreshDiagnostics
          - syu.openSpecItemById
          - syu.showRelatedFilesForSpecId
          - syuContext

## Source YAML

```yaml
category: Editor Integrations
version: 1

features:
  - id: FEAT-VSCODE-001
    title: VS Code navigation and diagnostics integration
    summary: Surface `syu` validation diagnostics and trace/navigation workflows directly inside VS Code.
    status: implemented
    linked_requirements:
      - REQ-CORE-022
    implementations:
      javascript:
        - file: editors/vscode/src/extension.js
          symbols:
            - activate
            - registerCommands
            - SyuContextTreeProvider
        - file: editors/vscode/src/model.js
          symbols:
            - loadSpecModel
            - lookupTrace
            - loadDiagnostics
      json:
        - file: editors/vscode/package.json
          symbols:
            - syu.refreshDiagnostics
            - syu.openSpecItemById
            - syu.showRelatedFilesForSpecId
            - syuContext
```
