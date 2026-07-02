---
title: "Task Planning CLI / Task"
description: "Generated reference for docs/syu/features/cli/task.yaml"
---

> Generated from `docs/syu/features/cli/task.yaml`.

## Parsed content

### Category

- Task Planning CLI

### Version

- 1

### Features

- **id**: FEAT-TASK-001
  - **title**: Request artifact classification
  - **summary**: Classify captured request artifacts into requirement create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-028
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_classify_command
          - classify_request
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskClassifyArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/request-artifact-format.md
        - **symbols**:
          - syu task classify
- **id**: FEAT-TASK-002
  - **title**: Planned task scaffold preview
  - **summary**: Preview reviewable planned requirement and feature updates that keep the current add-command document and registry conventions intact before the edits are applied or committed.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-029
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_scaffold_command
          - build_scaffold_plan
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskScaffoldArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/request-artifact-format.md
        - **symbols**:
          - syu task scaffold
      - **file**: docs/guide/implementation-planning.md
        - **symbols**:
          - syu task scaffold
- **id**: FEAT-TASK-003
  - **title**: Request artifact scoping
  - **summary**: Map request artifacts onto nearby requirements, policies, philosophies, and features with signals for higher-level discussion and planned-state follow-up.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-030
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_scope_command
          - scope_request
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskScopeArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/request-artifact-format.md
        - **symbols**:
          - syu task scope
      - **file**: docs/guide/implementation-planning.md
        - **symbols**:
          - syu task scope
- **id**: FEAT-TASK-004
  - **title**: Temporary Goal Plan generation
  - **summary**: Generate and load temporary Goal Plan artifacts with goal, scope, test, coverage, and completion fields outside the persistent spec tree.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-031
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_plan_command
          - run_task_infer_command
          - build_goal_plan
          - build_diff_inferred_goal_plan
          - JsonTaskPlanOutput
          - JsonTaskPlanSourceEvidence
          - JsonTaskPlanScopeEntry
          - DiffInferenceOutcome
          - GoalPlanArtifact
          - load_goal_plan_artifact
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskPlanArgs
          - TaskInferArgs
          - TaskPlanFormat
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/request-artifact-format.md
        - **symbols**:
          - syu task plan
      - **file**: docs/guide/implementation-planning.md
        - **symbols**:
          - syu task plan
      - **file**: docs/guide/goal-plan-format.md
        - **symbols**:
          - syu.goal_plan
      - **file**: docs/guide/command-card.md
        - **symbols**:
          - syu task infer --range origin/main...HEAD
- **id**: FEAT-TASK-005
  - **title**: Goal Plan conformance checking
  - **summary**: Validate temporary Goal Plan artifacts against changed files, linked spec IDs, required tests, and declared completion commands before review.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-032
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_check_command
          - check_goal_plan
          - GoalPlanArtifact
          - load_goal_plan_artifact
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskCheckArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/goal-plan-format.md
        - **symbols**:
          - syu.goal_plan
      - **file**: docs/guide/command-card.md
        - **symbols**:
          - syu task check goal-plan.yaml --range origin/main...HEAD
- **id**: FEAT-TASK-006
  - **title**: Goal Plan test selection
  - **summary**: Turn Goal Plan test declarations into justified shell commands for CI before scoped coverage runs.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-033
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_test_select_command
          - build_task_test_selection
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskTestSelectArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatch
          - run_dispatch
    - **markdown**:
      - **file**: docs/guide/goal-plan-format.md
        - **symbols**:
          - syu.goal_plan
      - **file**: docs/guide/command-card.md
        - **symbols**:
          - syu task test-select goal-plan.yaml
- **id**: FEAT-TASK-007
  - **title**: Shared typed Work planner
  - **summary**: Resolve typed Work intent, graph impact, mutation previews, and WorkKind-specific verification through one UI-independent engine shared by CLI, automation, diff inference, and Workbench adapters.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-034
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-task-model/src/work.rs
        - **symbols**:
          - WorkKind
          - WorkOperation
          - WorkSurface
          - WorkMode
          - SourceRole
          - ImpactRole
          - WorkIntent
          - WorkImpact
          - WorkMutation
          - WorkKindProfile
          - SurfaceRequirement
          - CompletionCheck
          - resolve_work_intent
          - work_request_text
          - work_kind_profile
          - plan_work
          - goal_plan_from_work_plan
          - serialization_is_deterministic
      - **file**: crates/syu-actions/src/lib.rs
        - **symbols**:
          - plan_request_work
          - plan_request_work_with_constraints
          - plan_item_work
      - **file**: src/command/task.rs
        - **symbols**:
          - build_shared_request_work_plan
          - build_goal_plan
          - build_diff_inferred_goal_plan
      - **file**: crates/syu-workbench-server/src/workspace.rs
        - **symbols**:
          - shared_work_plan_with_options
          - shared_work_plan_input
          - build_shared_goal_plan

## Source YAML

```yaml
category: Task Planning CLI
version: 1

features:
  - id: FEAT-TASK-001
    title: Request artifact classification
    summary: Classify captured request artifacts into requirement create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.
    status: implemented
    linked_requirements:
      - REQ-CORE-028
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_classify_command
            - classify_request
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskClassifyArgs
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/request-artifact-format.md
          symbols:
            - syu task classify
  - id: FEAT-TASK-002
    title: Planned task scaffold preview
    summary: Preview reviewable planned requirement and feature updates that keep the current add-command document and registry conventions intact before the edits are applied or committed.
    status: implemented
    linked_requirements:
      - REQ-CORE-029
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_scaffold_command
            - build_scaffold_plan
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskScaffoldArgs
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/request-artifact-format.md
          symbols:
            - syu task scaffold
        - file: docs/guide/implementation-planning.md
          symbols:
            - syu task scaffold
  - id: FEAT-TASK-003
    title: Request artifact scoping
    summary: Map request artifacts onto nearby requirements, policies, philosophies, and features with signals for higher-level discussion and planned-state follow-up.
    status: implemented
    linked_requirements:
      - REQ-CORE-030
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_scope_command
            - scope_request
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskScopeArgs
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/request-artifact-format.md
          symbols:
            - syu task scope
        - file: docs/guide/implementation-planning.md
          symbols:
            - syu task scope
  - id: FEAT-TASK-004
    title: Temporary Goal Plan generation
    summary: Generate and load temporary Goal Plan artifacts with goal, scope, test, coverage, and completion fields outside the persistent spec tree.
    status: implemented
    linked_requirements:
      - REQ-CORE-031
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_plan_command
            - run_task_infer_command
            - build_goal_plan
            - build_diff_inferred_goal_plan
            - JsonTaskPlanOutput
            - JsonTaskPlanSourceEvidence
            - JsonTaskPlanScopeEntry
            - DiffInferenceOutcome
            - GoalPlanArtifact
            - load_goal_plan_artifact
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskPlanArgs
            - TaskInferArgs
            - TaskPlanFormat
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/request-artifact-format.md
          symbols:
            - syu task plan
        - file: docs/guide/implementation-planning.md
          symbols:
            - syu task plan
        - file: docs/guide/goal-plan-format.md
          symbols:
            - syu.goal_plan
        - file: docs/guide/command-card.md
          symbols:
            - syu task infer --range origin/main...HEAD
  - id: FEAT-TASK-005
    title: Goal Plan conformance checking
    summary: Validate temporary Goal Plan artifacts against changed files, linked spec IDs, required tests, and declared completion commands before review.
    status: implemented
    linked_requirements:
      - REQ-CORE-032
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_check_command
            - check_goal_plan
            - GoalPlanArtifact
            - load_goal_plan_artifact
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskCheckArgs
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/goal-plan-format.md
          symbols:
            - syu.goal_plan
        - file: docs/guide/command-card.md
          symbols:
            - syu task check goal-plan.yaml --range origin/main...HEAD
  - id: FEAT-TASK-006
    title: Goal Plan test selection
    summary: Turn Goal Plan test declarations into justified shell commands for CI before scoped coverage runs.
    status: implemented
    linked_requirements:
      - REQ-CORE-033
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_test_select_command
            - build_task_test_selection
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskTestSelectArgs
        - file: src/lib.rs
          symbols:
            - dispatch
            - run_dispatch
      markdown:
        - file: docs/guide/goal-plan-format.md
          symbols:
            - syu.goal_plan
        - file: docs/guide/command-card.md
          symbols:
            - syu task test-select goal-plan.yaml
  - id: FEAT-TASK-007
    title: Shared typed Work planner
    summary: Resolve typed Work intent, graph impact, mutation previews, and WorkKind-specific verification through one UI-independent engine shared by CLI, automation, diff inference, and Workbench adapters.
    status: implemented
    linked_requirements:
      - REQ-CORE-034
    implementations:
      rust:
        - file: crates/syu-task-model/src/work.rs
          symbols:
            - WorkKind
            - WorkOperation
            - WorkSurface
            - WorkMode
            - SourceRole
            - ImpactRole
            - WorkIntent
            - WorkImpact
            - WorkMutation
            - WorkKindProfile
            - SurfaceRequirement
            - CompletionCheck
            - resolve_work_intent
            - work_request_text
            - work_kind_profile
            - plan_work
            - goal_plan_from_work_plan
            - serialization_is_deterministic
        - file: crates/syu-actions/src/lib.rs
          symbols:
            - plan_request_work
            - plan_request_work_with_constraints
            - plan_item_work
        - file: src/command/task.rs
          symbols:
            - build_shared_request_work_plan
            - build_goal_plan
            - build_diff_inferred_goal_plan
        - file: crates/syu-workbench-server/src/workspace.rs
          symbols:
            - shared_work_plan_with_options
            - shared_work_plan_input
            - build_shared_goal_plan
```
