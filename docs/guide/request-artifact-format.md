# Request artifact format for syu task workflows

<!-- FEAT-DOCS-001 -->

Use this guide when you want a small, repeatable request record before you edit
the spec tree. If the change is already a concrete spec diff, raw spec edits are
usually enough.

## When to use it

- A request needs to be handed around before the spec changes.
- A later CLI or app flow should be able to load the same request again.
- You want to keep the intake step separate from the planning step.

## When not to use it

- You already know the exact files and IDs that must change.
- The change is tiny and one-off.
- The request only needs to live inside a PR description.

## Recommended shape

YAML keeps the artifact easy to read in the terminal and easy to reuse later:

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

## Field meanings

- **version**: keeps the format explicit when the shape changes later.
- **request**: the short statement you would otherwise paste into chat or a
  ticket.
- **context.affected_area**: the part of the repository most likely to change.
- **context.repository_constraints**: the rules that must stay true while the
  request is being planned.
- **context.linked_ids**: the IDs already known to be in scope.

The artifact is intentionally smaller than the spec itself. Once the request is
understood, move the real work into planned requirements and features with the
normal spec workflow. `syu task classify` can read the artifact and the current
graph first, `syu task scope request.yaml` can map the request onto nearby
requirements, policies, philosophies, and features, and `syu task scaffold
request.yaml` can preview the planned requirement and feature updates that
follow from that decision. `syu task plan request.yaml` can then turn the
scoped request into a temporary Goal Plan artifact that stays outside the
persistent spec tree while still keeping the implementation, test, coverage,
and completion work visible.

When the request comes from an issue template, a maintainer can copy the
problem statement into a request artifact, preserve the linked IDs and
constraints, and then run the request-driven flow above. That keeps issue
intake separate from temporary delivery planning without turning the request
note into another persistent spec layer.
