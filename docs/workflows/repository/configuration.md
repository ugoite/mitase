# Configuration

`mitase` v1 uses one strict `mitase/config/v1` repository configuration. Unknown fields are rejected.

The repository root `mitase.yaml` is Mitase's mature self-hosting dogfood profile, not a starter configuration. New repositories should copy the closest checked-in example and adopt one connected capability at a time; the root profile deliberately applies the strictest current v1 validation boundary to itself.

```yaml
schema: mitase/config/v1
workspace:
  spec_roots: [docs/mitase]
  excludes: [target/**, node_modules/**]
inventory:
  active_profile: default
  profiles:
    - id: default
      providers:
        rust: { mode: test, include_tests: true }
        javascript: { roots: [web/src] }
        typescript: { roots: [packages] }
        openapi: { roots: [api/openapi.yaml] }
        markdown: { roots: [docs] }
        json-schema: { roots: [schemas] }
validation:
  preset: agent-ready
  readiness:
    target: traceable
    probes:
      implemented_criteria:
        - criterion: REQ-CAPABILITY-001#criterion.spec-model
          level: work-ready
      public_entrypoints: { selection: all, level: seedable }
      changed_units: false
    limits: { max_ownership_scope_units: 64 }
  changed:
    baseline: { strategy: merge-base, against: origin/main }
    require_owned_changes: true
verification:
  runners:
    cargo-test:
      executable: cargo
      arguments: [test, -p, "{package}", "{test}", --, --exact]
```

Key fields:

- `workspace.spec_roots`: canonical v1 specification roots.
- `workspace.excludes`: paths excluded from specification and inventory discovery.
- `inventory.active_profile`: the one provider profile used to build canonical artifact identities.
- `validation.readiness.target`: the repository-wide floor.
- `validation.readiness.probes.implemented_criteria`: exact criterion-and-level pairs. A bounded list is the normal gradual-adoption path.
- `validation.readiness.probes.public_entrypoints`: set `selection: all` and the required `level` to govern every discovered public entrypoint with one exact owner and one capability exposure.
- `validation.readiness.probes.changed_units`: include changed artifact ownership in readiness when enabled.
- `validation.changed.baseline`: optional Git baseline for change and readiness comparison.
- `verification.runners`: runner identities referenced by exact verification claims. Readiness inspects their configuration but does not execute them.

Probe identity and required level are one typed configuration unit. Advance a capability only when its acceptance and behavioral verification are present; do not add catch-all ownership or planned targets to make a readiness count pass.

Inventory profiles are executable build boundaries. Rust `mode`, `features`,
`target`, and test settings determine which `cfg` artifacts are active;
conditional artifacts remain visible to semantic comparison but cannot resolve
as editable targets. JavaScript and TypeScript providers inventory exported and
private declarations, OpenAPI inventories operations, Markdown inventories
headings, and `json`, `yaml`, or `json-schema` providers inventory exact JSON
Pointer nodes. Enabling several providers unions their semantic units; it does
not select one language at the expense of another.

Cross-language dependency scope is declared in Feature contracts, not inferred
from import text. A binding with role `generated` must give each output a
`generated-from` claim containing exact source target references. Generated
artifacts remain derived context and direct edits are rejected by validation.
