# generic example

This example matches the default `syu init .` starter shape.

It shows the smallest checked-in workspace that still keeps the four layers
explicit, plus one shell implementation trace and one markdown acceptance
check, so you can inspect the neutral starter before you scaffold the same
shape in your own repository.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-001` linked to `PHIL-001` |
| `docs/syu/requirements/core/core.yaml` | `REQ-001` with a markdown-backed acceptance check |
| `docs/syu/features/core/core.yaml` | `FEAT-001` with a shell implementation trace |
| `scripts/bootstrap-generic.sh` | shell implementation containing `bootstrap_generic_workspace` |
| `README.md` | markdown-backed acceptance checklist |

## Try it

```bash
cd examples/generic
syu validate .
syu list requirement
syu show REQ-001
syu browse .
```

## GenericAcceptanceChecklist

- `REQ-001` keeps the neutral starter explicit without adding language-specific
  scaffolding yet.
- `FEAT-001` traces the bootstrap shell function `bootstrap_generic_workspace`.
- The example stays small while still demonstrating one concrete implementation
  and one concrete check.
