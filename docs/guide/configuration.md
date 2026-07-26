# Configuration

`syu` v1 uses one strict `syu/config/v1` repository configuration. Unknown fields are rejected.

```yaml
schema: syu/config/v1
workspace:
  spec_roots: [docs/syu]
  excludes: [target/**, node_modules/**]
inventory:
  active_profile: default
  profiles:
    - id: default
      providers:
        rust: { mode: test, include_tests: true }
validation:
  preset: agent-ready
  readiness:
    target: traceable
    scopes: { planner: work-ready }
    probes:
      implemented_criteria: "REQ-WORK-001#criterion.exact-slice"
      changed_units: false
    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_seed: 4 }
  changed:
    baseline: { strategy: merge-base, against: origin/main }
    require_owned_changes: true
    require_plan: true
verification:
  runners:
    cargo-test:
      executable: cargo
      arguments: [test, -p, "{package}", "{test}", --, --exact]
work:
  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 4, max_readonly_targets: 8, max_total_bytes: 16384 }
```

Key fields:

- `workspace.spec_roots`: canonical v1 specification roots.
- `workspace.excludes`: paths excluded from specification and inventory discovery.
- `inventory.active_profile`: the one provider profile used to build canonical artifact identities.
- `validation.readiness.target`: the repository-wide floor.
- `validation.readiness.scopes`: stricter capability/facet stages layered over that floor.
- `validation.readiness.probes.implemented_criteria`: `all` or a comma-separated exact criterion set. A bounded set is the normal gradual-adoption path.
- `validation.readiness.probes.public_entrypoints`: set to `all` to require every discovered public entrypoint to have exact governance and a ready target-specific plan.
- `validation.readiness.probes.changed_units`: include changed artifact ownership in readiness when enabled.
- `validation.changed.baseline`: optional Git baseline for change and readiness comparison.
- `verification.runners`: executable templates used by exact verification claims. Readiness can prove exact execution only for supported runners.
- `work.slicing.*`: hard limits used by planning and context export.

Scope names are also binding facets when criterion readiness is selected. Advance a capability only when its acceptance and verification are present; do not add catch-all ownership or planned targets to make a readiness count pass.
