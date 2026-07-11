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
    artifact_ownership: agent-ready
    spec_fulfillment: agent-ready
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
- `validation.coverage.artifact_ownership`: artifact-to-spec coverage target
- `validation.coverage.spec_fulfillment`: spec-to-artifact coverage target
- `work.slicing.*`: limits used by planning and context export
- `adapters.enabled`: adapters allowed for target resolution

The active root CLI does not rely on the historical `spec.root`, `report.output`, or reciprocal-link validation settings.

## Bidirectional coverage targets

Coverage is not line coverage. Both fields are required and each evaluates its
own full-repository denominator: `artifact_ownership` discovers every
addressable implementation symbol and test; `spec_fulfillment` evaluates every
delivery anchor in the spec graph. `workspace.excludes` is the only way to
remove generated or otherwise out-of-scope artifacts from the first denominator.

| Target | Artifact ownership | Spec fulfillment |
| --- | --- | --- |
| `connected` | A binding refers to the symbol or test. | The anchor has a valid graph relation. |
| `owned` | One canonical implementation or verification binding owns it. | Required implementation and verification bindings exist. |
| `agent-ready` | An exact canonical identity, responsibility, facet, and criterion relation make it editable. | Bound targets resolve to exact inventory identities. |
| `verified` | Its criterion has the opposite implementation/test evidence. | Every implemented criterion is implemented and tested. |
| `evidence-ready` | Its criterion reaches policy enforcement and philosophy evidence. | The full intent-to-evidence chain closes. |

`off` is explicit and useful while incrementally adopting Syu; it still keeps
the two coverage directions visible in the configuration. Missing either
configured target is always a validation error. A newly added
unowned function or test therefore changes the denominator and fails validation.
