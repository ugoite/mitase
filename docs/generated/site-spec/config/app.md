---
title: "Configuration / App"
description: "Generated reference for docs/syu/config/app.yaml"
---

> Generated from `docs/syu/config/app.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Section

- app

### Precedence

- **bind**:
  - --bind
  - app.bind
  - 127.0.0.1
- **port**:
  - --port
  - app.port
  - 3000

### Items

- **key**: app.bind
  - **type**: string
  - **default**: 127.0.0.1
  - **summary**: Sets the default bind address for `syu app`.
  - **description**:
    - |
      `syu app` uses this address when `--bind` is not passed. Repositories can
      keep a checked-in default when contributors usually inspect the app through
      a shared local workflow or demo setup.
- **key**: app.port
  - **type**: integer
  - **default**: 3000
  - **summary**: Sets the default port for `syu app`.
  - **description**:
    - |
      `syu app` uses this port when `--port` is not passed. CLI flags still win
      so one-off demos and local conflicts do not require editing `syu.yaml`.

## Source YAML

```yaml
# FEAT-APP-001
category: Configuration
version: 1
section: app
precedence:
  bind:
    - --bind
    - app.bind
    - 127.0.0.1
  port:
    - --port
    - app.port
    - 3000
items:
  - key: app.bind
    type: string
    default: 127.0.0.1
    summary: Sets the default bind address for `syu app`.
    description: |
      `syu app` uses this address when `--bind` is not passed. Repositories can
      keep a checked-in default when contributors usually inspect the app through
      a shared local workflow or demo setup.
  - key: app.port
    type: integer
    default: 3000
    summary: Sets the default port for `syu app`.
    description: |
      `syu app` uses this port when `--port` is not passed. CLI flags still win
      so one-off demos and local conflicts do not require editing `syu.yaml`.
```
