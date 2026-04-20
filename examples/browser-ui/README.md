# browser-ui example

This example demonstrates a **frontend-first workspace** with a traced browser UI
flow using **React** and **TypeScript**. It shows how to link requirements and
features to interactive component code and tests.

This is the first checked-in example focused on a real frontend user story
rather than just backend or CLI tracing.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-UI-001` — frontend tracing philosophy |
| `docs/syu/policies/policies.yaml` | `POL-UI-001` linked to `PHIL-UI-001` |
| `docs/syu/requirements/frontend/counter.yaml` | `REQ-UI-001` with test trace to `Counter.test.tsx` |
| `docs/syu/features/ui/counter.yaml` | `FEAT-UI-001` with implementation trace to `Counter.tsx` |
| `src/components/Counter.tsx` | React component with traced feature implementation |
| `src/components/Counter.test.tsx` | Test file with traced requirement verification |

## Try it

```bash
cd examples/browser-ui
syu validate .
syu list requirement
syu show REQ-UI-001
syu app .
```

A successful `syu validate .` produces output similar to:

```
syu validate passed
workspace: examples/browser-ui
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 traces validated; features=1/1 traces validated
```

## What this example demonstrates

### Frontend tracing pattern

- **Component-level traces** — `FEAT-UI-001` traces to the `Counter` component
  exported from `Counter.tsx`
- **Test traces** — `REQ-UI-001` traces to the `test_counter_increments` test
  function that validates the counter behavior
- **JSDoc comments** — TypeScript uses `/** … */` doc comments to satisfy
  `doc_contains` constraints, just like Rust uses `///` and Python uses `"""`

### Real user flow

The example models a complete interactive flow:

1. **State management** — `useState` hook manages the counter value
2. **User interaction** — button click triggers increment
3. **UI update** — React re-renders with new count
4. **Test coverage** — test simulates clicks and verifies display updates

This is more than a "hello world" — it's a practical pattern you can copy for
your own frontend feature work.

### File layout

The example uses a typical frontend structure:

```
src/
  components/
    Counter.tsx          # component implementation
    Counter.test.tsx     # component tests
```

This shows how to organize UI code in a way that scales for larger frontend
projects.

## Key things to notice

- **TypeScript runtime** — `syu.yaml` declares `runtimes.node.command: auto` so
  syu can parse TypeScript files and extract symbols
- **Reciprocal links** — `REQ-UI-001` and `FEAT-UI-001` reference each other in
  both directions, just like the other examples
- **Frontend-first IDs** — the `REQ-UI` and `FEAT-UI` prefixes make it clear
  these are browser UI concerns, not backend or CLI features

## Compared to other examples

| Example | Best for |
|---------|----------|
| `rust-only` | Rust-first backends |
| `python-only` | Python services or scripts |
| `go-only` | Go microservices |
| `polyglot` | Mixed-language repositories |
| **`browser-ui`** | **Frontend React/TypeScript work** |

If you're evaluating syu for a product UI team, **start here**.
