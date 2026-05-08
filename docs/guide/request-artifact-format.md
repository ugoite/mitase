# Request artifact format for syu task workflows

<!-- FEAT-DOCS-001 -->

Use this guide when you want a small, repeatable request record before you start
editing the spec tree. If the request is already a concrete diff, raw spec edits
are usually enough.

## Recommended shape

YAML keeps the request easy to read in the terminal and easy to reuse in later
tooling:

```yaml
version: 1
request: >
  Expand syu validate --fix so it repairs the safe trace and ownership hygiene
  cases that are already accepted by the validator.
context:
  affected_area: validation
  repository_constraints:
    - keep current CLI flags stable
    - preserve text and JSON output
    - stay compatible with the existing quality gates
  linked_ids:
    - FEAT-CHECK-001
```

## What each field is for

- **version**: keeps the artifact format explicit when the shape changes later.
- **request**: the short statement you would otherwise paste into chat or a
  ticket.
- **context.affected_area**: the part of the repository most likely to change.
- **context.repository_constraints**: the rules that must stay true while the
  request is being planned.
- **context.linked_ids**: the IDs already known to be in scope.

## When to use it

- When a request needs to be handed around before the spec is edited.
- When a later CLI or app flow should be able to load the same request again.
- When you want to keep the intake step separate from the spec rewrite step.

## When not to use it

- When you already know the exact files and IDs that need to change.
- When the change is tiny and one-off.
- When the request is only useful as an inline note inside the PR description.

The request artifact is intentionally smaller than the spec itself. Once the
request is understood, move the real work into planned requirements and features
with the planning guide.
