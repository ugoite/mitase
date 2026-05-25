# Goal plan format for temporary syu delivery artifacts

<!-- FEAT-DOCS-002 -->

Use this guide when a request has already been scoped and you need a temporary
delivery artifact that stays outside the persistent spec tree. Goal Plans are
planning artifacts, not another long-lived spec layer.

The normal path is request-driven: classify the request, scope it against the
current graph, scaffold the planned edits, and then turn that result into a
Goal Plan. If implementation already happened, `syu task infer` can rebuild a
provisional Goal Plan from the diff and the current traces.

## When to use it

- A request already has a clear implementation goal, scope, and test plan.
- You want a structured artifact that can live in a task file, PR body, issue
  body, CI artifact, or another temporary delivery location.
- You need a reviewable artifact before the implementation starts.
- You want the PR-scoped test selection and coverage expectation recorded
  separately from the repository's full integration gates.

## When not to use it

- You still need to classify or scope the request against the current spec
  graph.
- You want to add durable intent to philosophy, policy, requirement, or feature
  YAML.
- The change is already a direct spec diff.

## Request-driven flow

When the request still needs shaping, use the planning commands in order:

1. `syu task classify request.yaml`
2. `syu task scope request.yaml`
3. `syu task scaffold request.yaml`
4. `syu task plan request.yaml`
5. implementation
6. `syu task check goal-plan.yaml --range origin/main...HEAD`

That flow keeps the intake, the spec adjacency check, the reviewable preview,
and the temporary Goal Plan separate from one another.

## Diff-inferred fallback

If the implementation already exists and you need a provisional plan instead of
request-driven planning, use:

1. `syu task infer --range origin/main...HEAD`
2. `syu task check goal-plan.yaml --range origin/main...HEAD`

The inferred plan should record the changed files, traced owners, and
confidence level that shaped the result, but it still remains temporary.

## Recommended shape

```yaml
version: 1
kind: syu.goal_plan

source:
  mode: request_driven
  request_artifact: request.yaml
  confidence: medium
  evidence:
    changed_files:
      - src/command/task.rs
    traced_requirements:
      - REQ-CORE-030
    traced_features:
      - FEAT-TASK-003

goal:
  id: GOAL-001
  title: Keep temporary planning explicit
  statement: Capture the implementation plan without turning it into a fifth spec layer.
  inferred: false
  non_goals:
    - Add a persistent task tree under spec.root

spec_mapping:
  persistent_items:
    philosophies: [PHIL-001]
    policies: [POL-001]
    requirements: [REQ-CORE-030]
    features: [FEAT-TASK-003]
  spec_updates:
    required: false
    expected_updates: []

implementation_plan:
  scope:
    include:
      - src/command/task.rs
    exclude:
      - docs/syu/**
  steps:
    - add a Goal Plan model
    - document the temporary artifact locations

test_plan:
  selection_mode: affected
  required_tests:
    rust:
      - file: tests/task_command.rs
        symbols:
          - task_plan_generates_goal_from_request
  suggested_tests: {}

coverage:
  mode: changed_lines
  threshold: 100
  include:
    - src/command/task.rs
  exclude: []

completion:
  must_pass:
    - syu validate .
```

## Field meanings

- **version**: keeps the format explicit when the shape changes later.
- **kind**: identifies the artifact as `syu.goal_plan`.
- **source.mode**: distinguishes request-driven planning from diff-inferred
  planning.
- **source.request_artifact**: points at the request artifact when one exists.
- **source.range**: records the diff range used for inferred plans.
- **source.confidence**: records how certain an inferred plan is.
- **source.evidence**: records the changed files and traced IDs that shaped a diff-inferred plan.
- **goal**: states the implementation goal, the short title, and the non-goals.
- **goal.inferred**: marks a goal that came from diff inference rather than request-driven planning.
- **spec_mapping**: records which persistent spec items the plan may touch and
  whether spec updates are expected.
- **implementation_plan**: lists the bounded file or symbol scope and the steps
  to complete the work.
- **test_plan**: records the test selection mode plus required and suggested
  checks, including file and symbol references when a plan needs explicit test
  coverage.
- **coverage**: states the coverage expectation for the changed surface. This
  is the PR-scoped coverage target, not the repository's full integration
  validation.
- **completion**: lists the checks that must pass before the work is complete.

Goal Plans are intentionally temporary. They may live in `.syu/tasks/*.yaml`,
`target/syu/*.json`, a GitHub issue or PR body, or a CI artifact. They do not
need to live under `spec.root`, and `syu validate .` should not require them to
exist.

Scoped tests and scoped coverage are a preview of the PR's justified risk
boundaries. The merge queue and the main branch still own the full integration
gate, so a Goal Plan should never imply that `syu task test-select` or
coverage selection replaces the repository-wide validation pass.
