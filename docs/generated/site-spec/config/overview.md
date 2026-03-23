---
title: "Configuration / Overview"
description: "Generated reference for docs/syu/config/overview.yaml"
---

> Generated from `docs/syu/config/overview.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Config File

- syu.yaml

### Summary

- Structured reference for the supported `syu.yaml` surface.

### Guidance

- Keep `syu.yaml` in the workspace root.
- Add new supported options under `docs/syu/config/` before expanding prose guides.
- Use `docs/guide/configuration.md` for narrative guidance and workflow examples.

### Sections

- spec.yaml
- validate.yaml
- app.yaml
- runtimes.yaml

### Items

- **key**: version
  - **type**: string
  - **default**: current CLI version
  - **summary**: Records which `syu` CLI version generated the config.
  - **description**:
    - |
      `syu init` writes the running CLI version into `syu.yaml`. When existing
      repositories still carry a legacy numeric value, `syu` continues to accept
      it while parsing the file so upgrades stay low-friction.

## Source YAML

```yaml
category: Configuration
version: 1
config_file: syu.yaml
summary: Structured reference for the supported `syu.yaml` surface.
guidance:
  - Keep `syu.yaml` in the workspace root.
  - Add new supported options under `docs/syu/config/` before expanding prose guides.
  - Use `docs/guide/configuration.md` for narrative guidance and workflow examples.
sections:
  - spec.yaml
  - validate.yaml
  - app.yaml
  - runtimes.yaml
items:
  - key: version
    type: string
    default: current CLI version
    summary: Records which `syu` CLI version generated the config.
    description: |
      `syu init` writes the running CLI version into `syu.yaml`. When existing
      repositories still carry a legacy numeric value, `syu` continues to accept
      it while parsing the file so upgrades stay low-friction.
```
