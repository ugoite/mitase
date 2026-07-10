# Configuration

`syu` v1 uses one repository config file.

```yaml
# x-release-please-start-version
version: 0.0.1-alpha.8
# x-release-please-end
schema: syu/config/v1
workspace:
  spec_roots: [docs/syu]
  artifact_roots: [src, tests]
  excludes: []
profiles: { active: [], custom: {} }
validation:
  preset: standard
  coverage:
    target: agent-ready
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
- `workspace.artifact_roots`: code and evidence roots that changed-file validation treats as owned artifacts
- `validation.changed.baseline`: optional changed-file baseline
- `validation.coverage.target`: qualitative whole-workspace item coverage target
- `work.slicing.*`: limits used by planning and context export
- `adapters.enabled`: adapters allowed for target resolution

The active root CLI does not rely on the historical `spec.root`, `report.output`, or reciprocal-link validation settings.

## Item coverage targets

Item coverage is not line coverage. A target is satisfied only when every
implemented item in the repository reaches the selected Syu benefit level.
`planned` items are reported but do not block validation, and `deprecated`
items are outside the active target.

| Target | What Syu can do with every implemented item |
| --- | --- |
| `connected` | Explain its specification anchors and graph relationships. |
| `owned` | Find explicit bindings and repository targets. |
| `agent-ready` | Produce exact, editable implementation work. This is the default. |
| `verified` | Find verification bindings and complete contract evidence. |
| `evidence-ready` | Trace intent through implementation, verification, documentation, and enforcement evidence. |

Use `evidence-ready` when the repository wants Syu's strongest delivery and
review guidance. Missing the configured target is always a validation error;
there are no per-item exclusions.
