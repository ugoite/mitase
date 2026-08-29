---
name: mitase-ci-triage
description: Use when a Mitase CI job fails, the merge queue is blocked, or a local/CI process appears to hang.
---

# Mitase CI triage

This skill is for diagnosis and the smallest corrective change. It does not
authorize unrelated refactoring or a full repository test run.

## Triage order

1. Read `AGENTS.md`, `mise.toml`, and the relevant workflow.
2. Identify the exact run, job, first failing step, and commit.
3. Reproduce the same Mise task locally when possible.
4. Compare workflow pins, lockfiles, toolchain, environment, and inputs.
5. If it hangs, inspect process state, open files/locks, sockets, and the last
   visible request or log line.
6. Classify the cause as implementation, boundedness, environment, or
   infrastructure before editing.
7. Change only the owning surface and rerun the bounded reproducer.

## CI and review handoff

Report:

```text
RUN: <URL or id>
JOB/STEP: <job and first failing step>
COMMIT: <sha>
CLASSIFICATION: implementation | boundedness | environment | infrastructure
REPRODUCER: <Mise task or command>
FIX: <short description>
VALIDATION: <focused results>
LIMITATIONS: <remaining limitations>
```

Do not hide environment failures inside code changes. Keep public PR content
limited to externally safe facts.
