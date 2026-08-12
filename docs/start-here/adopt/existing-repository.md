# Existing repository

Adopt Mitase one connected capability at a time. The first goal is not repository-wide coverage; it is one implemented criterion with exact implementation ownership, an exact verification target, and a ready canonical plan.

## 1. Inventory without claiming ownership

Create `mitase.yaml` with the real specification root and only the providers needed for the first capability. Keep readiness off while inspecting inventory:

```yaml
schema: mitase/config/v1
workspace:
  spec_roots: [docs/mitase]
  excludes: [target/**, node_modules/**]
inventory:
  active_profile: adoption
  profiles:
    - id: adoption
      providers:
        rust: { mode: test, include_tests: true }
validation:
  preset: agent-ready
  readiness:
    target: off
    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }
  changed: { require_owned_changes: false, require_plan: false }
verification:
  runners:
    cargo-test:
      adapter: cargo-libtest
      executable: cargo
      arguments: [test, -p, "{package}", "{test}", --, --exact]
work:
  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 4, max_readonly_targets: 8, max_total_bytes: 16384 }
```

Run `mitase readiness report . --format json` and use the inventory subjects as discovery evidence only. Do not create a planned Feature that owns the repository, a source tree, or every discovered file.

## 2. Connect one implemented slice

Add one Philosophy → Policy → Requirement → Feature chain. The slice is current only when all of these are true:

- the Requirement and Feature are `implemented`;
- each implementation target satisfies a specific implemented criterion;
- each implementation target has exact configured verification whose `covers` list names that target;
- ownership is exact or a bounded capability-level module/file scope;
- the criterion produces a ready canonical plan within the configured slice budgets.

A planned Feature can describe future intent, but it must not declare active ownership. Promote its status only with the implementation and verification evidence in the same reviewed change.

## 3. Turn on a bounded readiness probe

Select only the connected criterion and advance its facet deliberately:

```yaml
validation:
  preset: agent-ready
  readiness:
    target: traceable
    probes:
      implemented_criteria:
        - criterion: REQ-FIRST-001#criterion.behavior
          level: work-ready
      changed_units: false
    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }
  changed:
    require_owned_changes: true
    require_plan: true
```

Run:

```bash
mitase validate workspace .
mitase readiness report . --format json
```

Treat the selected criterion, its exact targets, and its verification as the readiness denominator. Unselected planned work is not evidence and does not become an owner.

## 4. Expand without weakening the boundary

Add the next capability only after the current slice stays green in CI. Move each capability through `traceable` → `seedable` → `work-ready` → `verifiable` → `closed-loop` as its evidence becomes real.

Enable `public_entrypoints: { selection: all, level: seedable }` only after every currently discovered public entrypoint has one exact owner, exposes a behaviorally verified capability target, and has a ready target-specific plan. Once enabled, a newly exported entrypoint fails canonical workspace validation until it is governed.

Import and suggestion automation remains follow-up work. This path is intentionally manual, bounded, and evidence-preserving; it does not claim that an existing repository is self-hosted after its first slice.
