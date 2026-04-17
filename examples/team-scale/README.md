# team-scale example

This example shows what `syu` can look like after a repository has grown beyond
the single-feature starter shape. It keeps one philosophy, two policies, three
requirements, and four features split across nested documents and traced into
separate Rust source and test files.

## What this example demonstrates

- **Team-scale growth**: multiple requirements and features are split by area
  (`auth/` and `operations/`) instead of staying in one starter document.
- **Phased adoption**: `REQ-TEAM-ADOPT-001` links to two features so an existing
  auth area can come under traceability incrementally instead of all at once.
- **Realistic file organization**: the feature registry lists multiple files and
  kinds, mirroring the way larger repositories reorganize docs as they grow.
- **Recovery drills**: the workspace stays valid in CI, but the README points to
  a few safe changes you can make to see reciprocal-link, missing-file, and
  drift recovery workflows in action.

This workspace intentionally keeps `validate.allow_planned: true` in
`syu.yaml`. It is demonstrating a migration-friendly, phased-adoption story
rather than a fully tightened end state, so treat that flag as part of the
teaching setup instead of a recommended steady-state default.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-TEAM-001` — the stable team-scale principle |
| `docs/syu/policies/team.yaml` | `POL-TEAM-001` and `POL-TEAM-002` for traceability and navigable growth |
| `docs/syu/requirements/adoption/auth.yaml` | `REQ-TEAM-ADOPT-001` — phased auth adoption |
| `docs/syu/requirements/operations/audit.yaml` | `REQ-TEAM-AUDIT-001` — audit visibility |
| `docs/syu/requirements/operations/reporting.yaml` | `REQ-TEAM-REPORT-001` — reporting stability after refactors |
| `docs/syu/features/auth/login.yaml` | `FEAT-TEAM-AUTH-LOGIN-001` |
| `docs/syu/features/auth/session.yaml` | `FEAT-TEAM-AUTH-SESSION-001` |
| `docs/syu/features/operations/audit.yaml` | `FEAT-TEAM-AUDIT-001` |
| `docs/syu/features/operations/reporting.yaml` | `FEAT-TEAM-REPORT-001` |
| `src/auth/login.rs` | login implementation owned by the auth rollout |
| `src/auth/session.rs` | follow-up session rollout implementation |
| `src/operations/audit.rs` | audit event implementation |
| `src/operations/reporting.rs` | reporting export implementation |
| `tests/auth_login.rs` | requirement trace for phased auth adoption |
| `tests/audit_visibility.rs` | requirement trace for audit visibility |
| `tests/report_exports.rs` | requirement trace for reporting exports |

## Try it

```bash
cd examples/team-scale
syu validate .
syu list feature
syu show REQ-TEAM-AUDIT-001
syu app .
```

A successful `syu validate .` produces output similar to:

```text
syu validate passed
workspace: examples/team-scale
definitions: philosophies=1 policies=2 requirements=3 features=4
traceability: requirements=3/3 features=4/4
```

## Recovery drills

Try one of these edits, run `syu validate .`, then restore the file and rerun:

1. Remove `FEAT-TEAM-AUDIT-001` from `REQ-TEAM-AUDIT-001.linked_features` to
   see reciprocal-link diagnostics on a cross-team requirement.
2. Rename `src/operations/reporting.rs` in
   `docs/syu/features/operations/reporting.yaml` to a non-existent path to see
   the refactor-drift failure mode.
3. Delete one entry from `docs/syu/features/features.yaml` to see how explicit
   feature discovery keeps reorganized documents visible.
