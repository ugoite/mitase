# Reviewer workflow: trace, relate, and log together

<!-- FEAT-DOCS-001 -->

Use this guide when a pull request already exists and you want one concrete
review loop that connects specification intent, executable work planning, and
the current validation surface.

`syu`'s reviewer flow works best when you keep three questions in order:

1. **What is this change supposed to satisfy?**
2. **Which files and symbols currently claim that work?**
3. **What changed recently in those traced paths?**

The current CLI keeps that loop centered on `validate`, `work`, and
`workbench project`.

If the work starts as a request note instead of a concrete diff, use
[request artifact format](./request-artifact-format.md) first so the intake
stays small and predictable.

## Example review target

This repository already ships a good self-hosted example in the work-planning
surface:

- requirement: `REQ-WORK-001`
- feature: `FEAT-WORK-001`
- implementation file: `crates/syu-planner/src/lib.rs`
- verification file: `crates/syu-spec-model/src/lib.rs`

You can follow the same flow in any repository by swapping in your own spec ID,
file path, and symbol name.

If you only need the short-form command reminder while you review, keep the
[command card](./command-card.md) open alongside this guide.

## 1. Start from the current repository state

Run the canonical validator first:

```bash
syu validate .
```

Use `--range origin/main...HEAD` when the review is anchored on one branch
diff instead of the full repository:

```bash
syu validate . --range origin/main...HEAD
```

This gives you the current rule failures, changed-impact failures, and any
work-plan integrity failures before you inspect one specific slice.

## 2. Materialize the executable work plan

When the review includes a concrete request artifact, generate the exact work
plan and inspect the saved plan:

```bash
syu work plan --request work.yaml --out plan.yaml --workspace .
syu work show --plan plan.yaml
```

This is the fastest way to verify which editable, verification, and readonly
targets the planner believes are in scope.

## 3. Re-validate the saved plan

Run validation against the saved plan so the planner and validator are checked
together:

```bash
syu validate . --plan plan.yaml
```

If the plan has multiple slices and you want to inspect one slice artifact
directly, export its context pack:

```bash
syu work export-context --plan plan.yaml --slice <slice-id> --workspace .
```

## 4. Inspect the product projection

If the change is about the Workbench or VS Code surface, inspect the current
server payload instead of reading stale screenshots or old docs:

```bash
syu workbench project --workspace . --format json
```
It is a review-focused view over the collected result, not a smaller or faster
validation scope.

When review only needs the YAML-side graph and document consistency, use
`syu validate . --spec-only` to skip traced source enforcement until you are
ready to bring code and test evidence back into scope.

## Fast reviewer playbook

Use this sequence as the default review loop:

```bash
syu show FEAT-CHECK-001
syu relate FEAT-CHECK-001
syu trace src/command/check.rs --symbol run_check_command
syu log FEAT-CHECK-001 --kind implementation --path src/command
syu validate . --id FEAT-CHECK-001
```

That sequence keeps the review grounded in checked-in YAML, then confirms the
claimed code evidence, then pulls the recent history that helps you judge
whether the current change still fits the surrounding intent.

## Git integration story: what syu supports today

`syu` does not try to replace your Git host, branch policy, or merge queue. The
current contract is narrower and more deliberate: it projects the checked-in
spec graph onto the Git history that already exists so review can stay anchored
on the same requirement/feature ownership the repository validates.

Today that means:

- `syu trace` and `syu relate` answer **what owns this code right now**.
- `syu log` answers **how the currently traced surface changed in Git**.
- `syu validate . --id ...` answers **whether the checked-in graph still agrees
  with the repository after this review target changed**.

That separation matters for merge-queue review too. When a branch waits in the
queue, you still need to reason about the current traced surface rather than
assuming the PR description stayed complete after rebases or queue updates.
`syu log --merge-base-ref origin/main` is the quickest way to ask for the
history slice that matters since the review branch diverged from main.

Use scoped Goal Plan tests and coverage to keep the PR reviewable, but treat
the merge queue and main branch as the full integration gate. A PR can be
small enough for focused validation and still require the repository-wide
checks before it is considered landed.

## Merge-queue and long-lived branch review

Use this variant when the branch has been rebased repeatedly, queued for merge,
or grown beyond one tidy diff:

```bash
syu relate FEAT-CHECK-001
syu trace src/command/check.rs --symbol run_check_command
syu log FEAT-CHECK-001 --kind implementation --merge-base-ref origin/main
syu validate . --id FEAT-CHECK-001
```

That flow keeps one reviewer question per command:

1. **What is the intended scope today?** → `syu relate`
2. **Which traced files/symbols are actually in play?** → `syu trace`
3. **What changed on this branch since it split from main?** → `syu log --merge-base-ref`
4. **Did the checked-in spec graph keep up?** → `syu validate . --id ...`

## Where the workflow is still intentionally manual

Some Git-aware review work is still outside the current CLI contract:

- choosing which PR or merge-queue entry to review first
- comparing two competing topic branches against each other
- deciding whether a broad PR should have been split into multiple spec-scoped changes
- reviewing newly added files that were never traced in the checked-in graph

When you hit those limits, use the normal PR diff together with `syu trace` and
`syu validate`. `syu` is intentionally strongest when the repository already has
usable trace coverage, not when review must infer intent from Git alone.

## Where reviewer tooling can grow next

The current model leaves room for richer Git-aware reviewer help without
changing the core philosophy:

- summarize spec ownership for a Git range instead of one ID at a time
- flag changed files in a branch that still lack requirement/feature ownership
- offer stronger merge-queue views for review branches that were rebased or re-enqueued many times

Those are natural follow-ons because they keep the same principle: Git history
becomes more useful when `syu` can project the checked-in spec graph onto it,
not when the tool invents a second workflow beside Git.

## When to choose a different entry point

- Start with [getting started](./getting-started.md) when you are still learning
  the command names themselves.
- Start with `syu browse .` when review is easier in a visual graph than in
  terminal output.
- Start with [troubleshooting](./troubleshooting.md) when validation is already
  failing and you need rule-by-rule repair guidance more than review workflow
  advice.
