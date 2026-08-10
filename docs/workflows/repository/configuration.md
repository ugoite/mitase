# Configuration

`mitase` v1 uses one strict `mitase/config/v1` repository configuration. Unknown fields are rejected.

The repository root `mitase.yaml` is Mitase's mature self-hosting dogfood profile, not a starter configuration. New repositories should copy the closest checked-in example and adopt one connected capability at a time; the root profile deliberately applies the strictest current v1 slice boundary to itself.

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
        - criterion: REQ-WORK-001#criterion.exact-slice
          level: work-ready
      public_entrypoints: { selection: all, level: seedable }
      changed_units: false
    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }
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
- `validation.readiness.probes.implemented_criteria`: exact criterion-and-level pairs. A bounded list is the normal gradual-adoption path.
- `validation.readiness.probes.public_entrypoints`: set `selection: all` and the required `level` to govern every discovered public entrypoint with one exact owner, one capability exposure, and a ready target-specific plan.
- `validation.readiness.probes.changed_units`: include changed artifact ownership in readiness when enabled.
- `validation.changed.baseline`: optional Git baseline for change and readiness comparison.
- `verification.runners`: executable templates used by exact verification claims. Readiness can prove exact execution only for supported runners.
- `work.slicing.*`: hard limits used by planning and context export.

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
from import text. A contract source and its provider/consumer participants
become readonly context whenever one participant is editable. A binding with
role `generated` must give each output a `generated-from` claim containing
exact source target references. Planning makes those outputs derived context:
an implementation tool can edit the source, but it cannot directly edit the
generated artifact.
