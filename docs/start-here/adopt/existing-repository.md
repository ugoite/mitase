# Existing repository

Adopt Mitase one connected capability at a time. The first goal is not repository-wide coverage; it is one implemented criterion with exact implementation ownership, an exact verification target, and current validation evidence.

The repository's Philosophy, Policy, Requirement, Criterion, and Feature
documents remain the authority for what the repository means. Mitase interprets
that specification, indexes its relationships, resolves exact Artifact
targets, and validates the evidence. Adoption does not give Mitase ownership
of planning, implementation, execution, testing, review, retries, or delivery.

## 1. Initialize, then inspect what was discovered

Start the same way as a new workspace:

```bash
mitase init .
mitase config effective .
```

`init` writes the minimal `mitase.yaml` and the spec-root marker. Repository
conventions resolve the standard spec root, excludes, inventory discovery,
validation defaults, and runner presets from the existing checkout, so `config
effective` shows what Mitase already recognizes before you write any
configuration. Add explicit settings to `mitase.yaml` only where the resolved
output misses something your first capability needs, such as a non-default
spec root or an inventory provider the discovery did not pick up.

Keep readiness off while inspecting inventory, and use the inventory subjects
from `mitase readiness report . --format json` as discovery evidence only. Do
not create a planned Feature that owns the repository, a source tree, or every
discovered file.

## 2. Connect one implemented capability

Add one Philosophy → Policy → Requirement → Feature chain. The capability is current only when all of these are true:

- the Requirement and Feature are `implemented`;
- each implementation target satisfies a specific implemented criterion;
- each implementation target has exact configured verification whose `covers` list names that target;
- ownership is exact or a bounded capability-level module/file scope;
- the criterion has current exact target and verification evidence.

A planned Feature can describe future intent, but it must not declare active ownership. Promote its status only with the implementation and verification evidence in the same reviewed change.

## 3. Turn on a bounded readiness probe

Select only the connected criterion and advance its facet deliberately:

```yaml
validation:
  preset: strict
  readiness:
    target: traceable
    probes:
      implemented_criteria:
        - criterion: REQ-FIRST-001#criterion.behavior
          level: verifiable
      changed_units: false
    limits: { max_ownership_scope_units: 64 }
  changed:
    require_owned_changes: true
```

Run:

```bash
mitase validate workspace .
mitase readiness report . --format json
```

Treat the selected criterion, its exact targets, and its verification as the readiness denominator. Unselected planned work is not evidence and does not become an owner.

## 4. Expand without weakening the boundary

Add the next capability only after the current slice stays green in CI. Move each capability through `traceable` → `seedable` → `verifiable` as its evidence becomes real.

Enable `public_entrypoints: { selection: all, level: seedable }` only after every currently discovered public entrypoint has one exact owner and exposes a behaviorally verified capability target. Once enabled, a newly exported entrypoint fails canonical workspace validation until it is governed.

Import and suggestion automation remains follow-up work. This path is intentionally manual, bounded, and evidence-preserving; it does not claim that an existing repository is self-hosted after its first slice.
