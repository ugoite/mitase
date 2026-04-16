# Adopting `syu` in an existing repository

<!-- FEAT-DOCS-001 -->

This guide is for repositories that already have code, docs, tests, and git
history. Do not try to pretend the repository is greenfield. Start from what is
already true, write one thin connected slice of the spec, then tighten
validation as the graph becomes trustworthy.

Want a different entry point?

- Use [getting started](./getting-started.md) when you are creating a new
  workspace and want the first `syu init .` path explained.
- Use [the tutorial](./tutorial.md) when you want a full from-scratch example.
- Jump to [troubleshooting](./troubleshooting.md) if a partial adoption is
  already blocked on validation errors.

## 1. Pick a spec home without reorganising the repository

`syu` does not require a brand-new repository. Add a `syu.yaml` at the workspace
root and point `spec.root` at the directory that fits your current layout.
Treat the example below as schematic: keep the fields, but let `syu init` or a
freshly generated `syu.yaml` supply the concrete `version` string for the CLI
release you are actually running:

```yaml
version: <current syu CLI version>
spec:
  root: docs/syu
validate:
  default_fix: false
  allow_planned: true
  require_non_orphaned_items: false
  require_reciprocal_links: false
  require_symbol_trace_coverage: false
```

Use `docs/syu` if you do not already have a better convention. If the repository
already keeps design material under `docs/spec`, `spec/contracts`, or another
stable path, keep that layout and set `spec.root` explicitly instead of moving
files just to match the default.

For the first slice, create only the directories you need:

```text
docs/syu/
  philosophy/
  policies/
  requirements/
  features/
```

Features also need the explicit registry file at
`<spec.root>/features/features.yaml`. Requirements are discovered by walking the
tree, but feature documents stay deliberate and reviewable through that short
registry.

## 2. Start from current repository truth

The fastest adoption path is not inventing new prose. Mine the material the
repository already trusts:

- architecture notes, ADRs, design docs, and product docs
- compliance checklists, runbooks, and support expectations
- public APIs, CLI commands, or user-visible workflows
- existing tests that already prove an obligation

Choose one subsystem or workflow that people already understand. Good first
slices are narrow and important: authentication, upload/download, billing
events, release delivery, or another boundary that already has clear code and
tests.

Do **not** start by writing fifty disconnected requirements. `syu` is strongest
when each adopted area forms a small connected graph quickly.

## 3. Decide whether philosophy or requirements come first

Start with the clearest source of truth in the existing repository:

| If the repository already has... | Start with... | Why |
|---|---|---|
| Stable product principles, ADRs, or architecture values | philosophy → policy | The repository already knows the *why* and needs that captured before you enumerate obligations. |
| Strong compliance rules, acceptance criteria, or SLO-style checks | requirement → policy | The repository already knows the obligations, so capture them first and then connect them upward. |
| Shipped behavior with clear tests and owners, but weak design docs | feature → requirement | The implementation truth is already present; document the behavior, then explain which requirement it satisfies. |

Whichever layer you start from, close the first slice before scaling out:

1. one philosophy
2. one policy
3. one requirement
4. one feature

That thin chain teaches the repository how links, IDs, and validation behave
without forcing a whole-program rewrite.

## 4. Write the first connected slice from existing material

For an existing repository, a good first slice usually looks like this:

- **philosophy**: a stable value already visible in architecture or product docs
- **policy**: the engineering rule that operationalises that value
- **requirement**: one concrete obligation a test can prove
- **feature**: the shipped capability or module that satisfies that requirement

Keep the text close to current terminology. `syu` should explain the repository
more clearly, not force a parallel vocabulary that nobody else uses.

When you add the first feature document, remember the feature registry:

```yaml
version: 0.0.1-alpha.7
files:
  - kind: adoption
    file: adoption/core.yaml
```

Place that in `<spec.root>/features/features.yaml`, then store the matching
feature YAML at `<spec.root>/features/adoption/core.yaml` (or another
repository-relative path that reads naturally for the subsystem).

## 5. Phase validation in on purpose

Existing repositories usually need a ratchet, not a switch-flip.

### Phase 1 — allow partial adoption

Keep these settings relaxed while the graph is still being backfilled:

- `validate.allow_planned: true`
- `validate.require_non_orphaned_items: false`
- `validate.require_reciprocal_links: false`
- `validate.require_symbol_trace_coverage: false`

This lets you land a partial spec without pretending the whole repository is
fully modeled yet.

### Phase 2 — tighten the graph

Once each adopted area has real adjacent links, turn these back on:

- `validate.require_non_orphaned_items: true`
- `validate.require_reciprocal_links: true`

At that point, new spec work must join the graph cleanly instead of accreting as
isolated notes.

### Phase 3 — tighten code ownership

Turn on `validate.require_symbol_trace_coverage: true` only after the repository
is ready for public APIs and tests to stay fully owned by the spec. This is
usually a later step for mature teams, not the first adoption milestone.

For one-off experiments, prefer CLI overrides such as
`syu validate . --require-reciprocal-links=false` before committing a permanent
config relaxation.

## 6. Map existing code to requirements and features

When you move from prose to code ownership:

- map **requirements** to obligations that tests can prove
- map **features** to capabilities, modules, commands, or APIs that implement
  those obligations
- keep IDs stable and visible in doc comments so the link is intentional

For already-shipped behavior, mark the requirement and feature as
`status: implemented` only when you can name the current test and
implementation symbols truthfully. If a spec item is accepted but you are not
ready to claim traces yet, leave it `planned`.

Use `syu validate . --fix` only after you have decided the owning IDs. Autofix
is good at inserting required doc-comment snippets; it does not choose the right
graph links for you.

If one small file is intentionally owned by one requirement or feature, you can
use `symbols: ['*']` instead of enumerating every public symbol by hand.

## 7. Use templates and examples as references, not as an in-place rewrite

You do not need to run `syu init` inside the existing repository just to get a
shape to copy. Borrow the closest reference material instead:

- [Rust-only example](https://github.com/ugoite/syu/tree/main/examples/rust-only)
- [Python-only example](https://github.com/ugoite/syu/tree/main/examples/python-only)
- [Polyglot example](https://github.com/ugoite/syu/tree/main/examples/polyglot)

Those examples show starter file layout, naming style, and trace structure
without asking you to replace established repository conventions.

If you still want to inspect generated starter files, run `syu init` with the
closest template in a disposable directory or clone, then copy only the parts
that fit your repository. Treat the template as reference material, not as a
command that gets to redefine your history.

## 8. Ratchet one area at a time

A healthy adoption sequence usually looks like this:

1. commit `syu.yaml`
2. land one connected slice for one subsystem
3. validate that slice repeatedly
4. add traces for the code and tests you already trust
5. tighten graph rules
6. expand to the next subsystem

That stays aligned with `syu`'s goal: specification-driven development that can
fit an existing repository incrementally instead of taking it over all at once.
