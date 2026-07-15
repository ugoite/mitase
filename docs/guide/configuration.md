# Configuration

`syu` v1 uses one repository config file.

```yaml
# x-release-please-start-version
version: 0.0.1-alpha.8
# x-release-please-end
schema: syu/config/v1
workspace:
  spec_roots: [docs/syu]
  excludes: []
profiles: { active: [], custom: {} }
validation:
  preset: standard
  deny_warnings: false
  rules: {}
  changed:
    require_owned_changes: false
work:
  slicing:
    max_editable_files: 4
    max_editable_symbols: 8
    max_verification_targets: 4
    max_readonly_targets: 8
    max_total_bytes: 16384
  context:
    include_parent_principles: false
    include_parent_rules: false
adapters: { enabled: [rust, typescript, javascript, shell, python, go, java, ruby, csharp, markdown, yaml, json] }
```

Key fields:

- `workspace.spec_roots`: v1 spec document roots
- `workspace.excludes`: paths excluded from specification and inventory discovery
- `validation.changed.baseline`: optional changed-file baseline
- `work.slicing.*`: limits used by planning and context export
- `adapters.enabled`: adapters allowed for target resolution

The active root CLI does not rely on the historical `spec.root`, `report.output`, or reciprocal-link validation settings.
