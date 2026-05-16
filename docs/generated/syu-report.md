# syu validation report

## Status

- Result: **FAIL**
- Workspace: `.`

## Definitions

- Philosophies: 3
- Policies: 8
- Requirements: 30
- Features: 38

## Traceability

- Requirement-to-test traceability: 142/142
- Feature-to-implementation traceability: 150/150

## Issues

| Severity | Code | Rule | Subject | Location | Message |
| --- | --- | --- | --- | --- | --- |
| error | SYU-workspace-historical-001 | Deleted IDs must not be reused by default | feature FEAT-TASK-001 | docs/syu/features/cli/task.yaml | feature `FEAT-TASK-001` reuses an ID that was last known in `docs/syu/features/cli/task.yaml` at commit `94926b74ffd6f569f74df7d307778144f5789314` before it was deleted. |
| error | SYU-workspace-historical-001 | Deleted IDs must not be reused by default | feature FEAT-TASK-002 | docs/syu/features/cli/task.yaml | feature `FEAT-TASK-002` reuses an ID that was last known in `docs/syu/features/cli/task.yaml` at commit `94926b74ffd6f569f74df7d307778144f5789314` before it was deleted. |
| error | SYU-workspace-historical-001 | Deleted IDs must not be reused by default | requirement REQ-CORE-028 | docs/syu/requirements/core/workspace.yaml | requirement `REQ-CORE-028` reuses an ID that was last known in `docs/syu/requirements/core/workspace.yaml` at commit `94926b74ffd6f569f74df7d307778144f5789314` before it was deleted. |
| error | SYU-workspace-historical-001 | Deleted IDs must not be reused by default | requirement REQ-CORE-029 | docs/syu/requirements/core/workspace.yaml | requirement `REQ-CORE-029` reuses an ID that was last known in `docs/syu/requirements/core/workspace.yaml` at commit `94926b74ffd6f569f74df7d307778144f5789314` before it was deleted. |

## Referenced rules

### `SYU-workspace-historical-001` — Deleted IDs must not be reused by default

- Genre: workspace
- Severity: error
- Summary: Reintroducing an ID that was already deleted makes the historical record ambiguous.
- Description: Stable IDs are only useful when deleted values stay retired. If a new philosophy, policy, requirement, or feature reuses an ID that already existed in Git history and was later deleted, reviewers can no longer tell whether the current item is the original definition or a renamed replacement. This rule keeps the historical record trustworthy by rejecting reused deleted IDs unless a repository explicitly disables the check for a migration.

## Suggested next actions

- Choose a new ID for the replacement item, or temporarily set `validate.historical_ids.enabled: false` while intentionally migrating old data.
