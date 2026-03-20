---
title: "Configuration / Runtimes"
description: "Generated reference for docs/syu/config/runtimes.yaml"
---

> Generated from `docs/syu/config/runtimes.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Section

- runtimes

### Items

- **key**: runtimes.python.command
  - **type**: string
  - **default**: auto
  - **summary**: Chooses the Python executable used for Python inspection.
  - **description**:
    - |
      Use `auto` to let `syu` try `python3` and then `python`. Set an explicit
      executable path or command name when the repository needs a predictable
      runtime in local or CI environments.
- **key**: runtimes.node.command
  - **type**: string
  - **default**: auto
  - **summary**: Reserves the Node.js command for runtime-backed integrations.
  - **description**:
    - |
      The TypeScript inspector is currently bundled, but keeping the runtime
      configurable makes future Node-backed integrations easier to adopt without
      surprising repositories.

## Source YAML

```yaml
category: Configuration
version: 1
section: runtimes
items:
  - key: runtimes.python.command
    type: string
    default: auto
    summary: Chooses the Python executable used for Python inspection.
    description: |
      Use `auto` to let `syu` try `python3` and then `python`. Set an explicit
      executable path or command name when the repository needs a predictable
      runtime in local or CI environments.
  - key: runtimes.node.command
    type: string
    default: auto
    summary: Reserves the Node.js command for runtime-backed integrations.
    description: |
      The TypeScript inspector is currently bundled, but keeping the runtime
      configurable makes future Node-backed integrations easier to adopt without
      surprising repositories.
```
