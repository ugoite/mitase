# Implementation planning with syu

<!-- FEAT-DOCS-001 -->

Use this guide when a new request is ready to become planned spec work before
the implementation starts. If you already know the exact IDs and files, jump to
[reviewer workflow](./reviewer-workflow.md). If you only have a request note,
start with [request artifact format](./request-artifact-format.md).

For request-driven delivery work, the planning artifact is a temporary Goal
Plan. It is intentionally separate from the persistent `philosophy` / `policy`
/ `requirement` / `feature` model and can live in a workspace-local task file,
an issue body, a PR body, or a CI artifact.

## When to use it

- A request has enough detail to turn into planned requirements or features.
- You need to review policies or philosophy before editing YAML.
- The work should stay reviewable before code lands.

## When not to use it

- You already have a concrete spec diff.
- The change is a tiny one-off fix.
- You only need a request note and not the planning steps yet.

## 1. Capture the request

Keep the request short enough that one person can still read it without losing
the thread. The request should say what outcome is wanted, what area of the
repository is likely affected, and which constraints must stay true.

## 2. Find the closest existing spec surface

Use the existing navigation commands to find the best anchor before you edit
anything:

```bash
syu task scope request.yaml
syu task classify request.yaml
syu task scaffold request.yaml
syu show FEAT-CHECK-001
syu relate FEAT-CHECK-001
syu browse .
syu search validation --kind requirement
```

At this point, decide whether the request belongs to an existing requirement or
feature, whether it needs a new one, and whether the scope result suggests a
policy or philosophy review before you touch feature-specific YAML.

## 3. Decide whether to create, expand, or delete a requirement

- **Create** a requirement when the request adds durable behavior that no
  current requirement explains.
- **Expand** a requirement when the request changes the scope or acceptance
  criteria of existing work without changing the core intent.
- **Delete** a requirement only when the request has been fully superseded and
  every linked feature, test, and reference can move cleanly to a new owner.

When you create or expand, keep the new `planned` state explicit until the code
and traces exist. When you delete, remove stale references first and then rerun
validation so the graph stays honest.

## 4. Review the policy and philosophy context

Before editing the spec, use `syu relate` on the owning item and check the
surrounding policies and philosophy. That keeps the request aligned with the
repository's higher-level intent instead of adding a one-off exception.

If the request would weaken an existing policy, make that change explicit in the
spec instead of hiding it inside implementation notes.

## 5. Turn the request into planned spec edits

Use `syu add` when you need new planned requirements or features, then edit the
generated YAML so the new work is reviewable:

```bash
syu add requirement REQ-PLANNING-001
syu add feature FEAT-PLANNING-001 --kind planning
```

If you already have a request artifact and want a reviewable preview before you
edit YAML by hand, use the task workflow first:

```bash
syu task scope request.yaml
syu task classify request.yaml
syu task scope request.yaml
syu task scaffold request.yaml
```

The resulting Goal Plan should stay outside `spec.root` unless you explicitly
choose to materialize it somewhere else for delivery tooling. By default,
`syu validate .` only loads the persistent spec tree under the configured
`spec.root`, so temporary planning files such as `.syu/tasks/*.yaml` do not
become part of the long-lived graph.

Keep the planned items small. If one request is trying to introduce multiple
behaviors, split the request before you split the implementation.

## 6. Implement and validate

Once the spec shape is stable, add the code or docs that satisfy it, then trace
the concrete files and run validation:

```bash
syu trace src/command/check.rs --symbol run_check_command
syu log FEAT-CHECK-001 --kind implementation --path src/command
syu validate .
```

That final pass should confirm the request, the spec, and the implementation all
agree before the work moves into review.
