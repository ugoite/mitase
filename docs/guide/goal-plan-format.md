# Goal plan format for temporary syu delivery artifacts

<!-- FEAT-DOCS-002 -->

Use this guide when a request has already been scoped and you need a temporary
delivery artifact that stays outside the persistent spec tree. Goal Plans are
planning artifacts, not another long-lived spec layer.

## When to use it

- A request already has a clear implementation goal, scope, and test plan.
- You want a structured artifact that can live in a task file, PR body, issue
  body, CI artifact, or another temporary delivery location.
- You need a reviewable artifact before the implementation starts.

## When not to use it

- You still need to classify or scope the request against the current spec
  graph.
- You want to add durable intent to philosophy, policy, requirement, or feature
  YAML.
- The change is already a direct spec diff.

## Recommended shape

```yaml
version: 1
kind: syu.goal_plan

source:
  mode: request_driven
  request_artifact: request.yaml

goal:
  id: GOAL-001
  title: Keep temporary planning explicit
  statement: Capture the implementation plan without turning it into a fifth spec layer.
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
- **goal**: states the implementation goal, the short title, and the non-goals.
- **spec_mapping**: records which persistent spec items the plan may touch and
  whether spec updates are expected.
- **implementation_plan**: lists the bounded file or symbol scope and the steps
  to complete the work.
- **test_plan**: records the test selection mode plus required and suggested
  checks, including file and symbol references when a plan needs explicit test
  coverage.
- **coverage**: states the coverage expectation for the changed surface.
- **completion**: lists the checks that must pass before the work is complete.

Goal Plans are intentionally temporary. They may live in `.syu/tasks/*.yaml`,
`target/syu/*.json`, a GitHub issue or PR body, or a CI artifact. They do not
need to live under `spec.root`, and `syu validate .` should not require them to
exist.
