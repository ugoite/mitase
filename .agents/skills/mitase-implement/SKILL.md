---
name: mitase-implement
description: Use when implementing Mitase code, documentation, repository workflow, or delivery-contract changes.
---

# Mitase implementation

Use the smallest workflow that satisfies the requested delivery unit. Keep
authoritative specification data portable and preserve `AGENTS.md`.

## Workflow

1. Read the owning surface and the relevant freeze/ADR/specification.
2. Confirm branch, worktree, exact base, and pre-existing changes.
3. Discover an existing Issue or create the required public Issue before the
   implementation.
4. Define the smallest coherent unit. Split independent invariants when they
   can be reviewed independently; keep coupled changes together when an
   intermediate state would not be valid.
5. Make the smallest change that satisfies the Issue.
6. Run focused validation, then broaden only when the changed surface or merge
   gate requires it.
7. Review the diff, publish the branch, and use the repository PR/CI flow.

## Product boundary

Repository tooling may create worktrees, Issues, pull requests, review
records, CI runs, and merges. Those abilities must never become Mitase product
responsibilities. Do not add compatibility aliases or migration layers for
the pre-v1 Work model unless a future Issue explicitly requires one.

## Review convergence

The primary implementer owns the review ledger. A reviewer must report the
commit it actually reviewed using:

```text
REVIEW_BASE: <commit>
REVIEWED_FROM: <previous REVIEWED_HEAD, or none for the first review>
REVIEWED_HEAD: <commit>
REVIEW_SCOPE: <scope>
REVIEWED_INVARIANTS: <invariants checked>
REVIEWED_CHECKS: <checks and CI evidence considered>
VERDICT: APPROVE | CHANGE_REQUEST
CARRIED_BLOCKERS: <none or each prior blocker with RESOLVED | STILL_OPEN>
NEW_BLOCKERS: <none or complete list>
FOLLOW_UPS: <Issue links or none>
EVIDENCE: <focused tests/checks>
LIMITATIONS: <environment limitations>
```

The first review covers the cumulative scope. Later reviews are delta-first
from the prior reviewed head and then recheck all affected invariants. Every
carried blocker must be marked `RESOLVED` or `STILL_OPEN`. `CHANGE_REQUEST` is
reserved for a problem that makes merge unsafe or impossible; unrelated
improvements become follow-up Issues.
