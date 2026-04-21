# Reviewer workflow: trace, relate, and log together

<!-- FEAT-DOCS-001 -->

Use this guide when a pull request already exists and you want one concrete
review loop that connects specification intent, traced code, and recent Git
history.

`syu`'s reviewer flow works best when you keep three questions in order:

1. **What is this change supposed to satisfy?**
2. **Which files and symbols currently claim that work?**
3. **What changed recently in those traced paths?**

The commands below answer those questions with `show`/`relate`, `trace`,
`explain`, and `log`.

If you review from the terminal often, generate shell completions once so spec
IDs and subcommands stay close at hand:

```bash
syu completion bash > ~/.local/share/bash-completion/completions/syu
```

## Example review target

This repository already ships a good self-hosted example in the validation
command feature:

- feature: `FEAT-CHECK-001`
- implementation file: `src/command/check.rs`
- implementation symbol: `run_check_command`

You can follow the same flow in any repository by swapping in your own spec ID,
file path, and symbol name.

If you only need the short-form command reminder while you review, keep the
[command card](./command-card.md) open alongside this guide.

## 1. Start from the spec item under review

Open the feature or requirement that the PR says it changed:

```bash
syu show FEAT-CHECK-001
```

Use `syu show` first when the PR description already names the exact ID. This
gives you the title, summary, linked requirements, and declared traces before
you jump into code.

If the PR only gives you a keyword, search first:

```bash
syu search validation --kind feature
```

## 2. Expand the surrounding context

Once you know the ID, inspect the nearby graph:

```bash
syu relate FEAT-CHECK-001
```

`syu relate` is the quickest reviewer command for answering questions like:

- which requirement does this feature satisfy?
- which policy or philosophy is above it?
- which files and symbols are traced today?
- are there obvious graph gaps such as missing reciprocal links?

Use this output to decide whether the PR still matches the connected policy and
requirement context, or whether it is changing the behavior in a way that
should have updated adjacent YAML too.

When you want the same selector flexibility but a more opinionated summary, run
`syu explain TARGET`. It keeps the ID/path/symbol entry points from `syu
relate`, then turns the result into a focused assessment with the connected
chain, traces in scope, and obvious gaps that still need review.

For bigger spec edits or cleanup PRs, add a quick heuristic audit pass:

```bash
syu audit .
```

That summary is useful when you want review notes about likely overlap,
policy drift, or policy text that no longer turns into concrete requirements.

## 3. Jump from code back to the owning spec

When review starts in a file diff instead of a spec ID, reverse the direction:

```bash
syu trace src/command/check.rs --symbol run_check_command
```

Use `syu trace` when you are already staring at code and need to answer:

- which feature owns this symbol?
- which requirement owns the related tests?
- does this file participate in multiple traced spec items?

That makes it easier to spot a common review smell: code changed in a traced
symbol, but the linked requirement or feature in the PR body is incomplete.

## 4. Pull the recent history for the same spec surface

After you know the owning ID and traced files, inspect their recent Git history:

```bash
syu log FEAT-CHECK-001 --kind implementation --path src/command
syu log REQ-CORE-017 --include-related --merge-base-ref origin/main
```

Use `syu log` when review needs historical context:

- has this area changed repeatedly in the same way?
- did a recent commit rename the traced file or symbol?
- is the PR fixing a regression in a path that already has relevant history?
- do I need the linked requirement/feature surface, not just one selected ID?
- what changed on this review branch since it diverged from main?

Treat `syu log` as history for the **currently traced** surface, not proof that
the whole PR diff is covered. A newly added implementation or test file can be
missing from this history slice if the trace mapping was never updated, so pair
an empty or too-small log result with the PR diff and `syu trace`/`syu relate`
before concluding that review coverage is complete.

For requirement-oriented review, swap to definition or test history instead:

```bash
syu log REQ-CORE-017 --kind definition
syu log REQ-CORE-017 --kind test
```

## 5. Close the loop with a focused validation pass

If the PR changes spec files, traced paths, or link structure, use the normal
validation commands as a focused review view over the full repository result:

```bash
syu validate . --genre trace
syu validate . --id FEAT-CHECK-001
```

Use `--genre trace` when you want trace-specific failures first. Use `--id`
when the review is anchored on one concrete requirement or feature and you want
the output filtered down to that item after the full workspace validation run.
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

## When to choose a different entry point

- Start with [getting started](./getting-started.md) when you are still learning
  the command names themselves.
- Start with the [browser app guide](./app.md) when review is easier in a visual
  graph than in terminal output.
- Start with [troubleshooting](./troubleshooting.md) when validation is already
  failing and you need rule-by-rule repair guidance more than review workflow
  advice.
