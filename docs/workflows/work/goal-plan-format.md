# Transitional Work plan format

Work plans are execution artifacts owned by external repository tooling. They
are retained temporarily so the Work runtime can be removed in focused steps;
the Mitase CLI does not create, approve, execute, retry, or finalize them.

The active planner output is `mitase/work-plan/v1`.

A work plan contains:

- the typed Work origin and its linked origin closure
- plan basis and workspace fingerprint
- execution mode (`isolated-slices` in v1)
- ready, blocked, or split-recovery candidate status
- execution slices
- slice budgets
- completion checks

External tooling owns plan inspection, slice selection, context export,
approval, delivery, and the exact `{ plan_digest, slice_id }` execution
identity. Use
`mitase/work-plan/v1` only as the transitional planner output format.
