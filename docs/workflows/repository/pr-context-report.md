---
title: "PR Context Report"
description: "Trace two committed snapshots to their declared specification responsibilities and verification evidence."
---

# PR Context Report

`mitase report pr` builds a read-only review context from two commits already
available in one Git repository. It reads the exact base and head snapshots;
uncommitted changes are excluded and Mitase never fetches missing commits.

```bash
mitase report pr --base origin/main --head HEAD --format markdown
mitase report pr --base <base-sha> --head <head-sha> --format json
```

JSON uses the `mitase/cli/v1` envelope and identifies its report payload with
`mitase/pr-context-report/v1`. JSON and Markdown render the same typed report.
Items and their evidence are sorted deterministically.

The report lists changed files and resolvable symbols, direct Binding owners,
upstream Criterion/Policy/Philosophy relations, always-review items, declared
verification assessments on both snapshots, and evidence gaps. A removed
currently valid claim is marked as lost. Invalid declarations that were
already invalid at base are not described as lost evidence.

Optional always-review rules belong in `mitase.yaml`:

```yaml
schema: mitase/config/v1
review:
  always:
    - id: frontend-review
      paths: ["web/src/**"]
      items: [POL-006, PHIL-INTERACTION-001]
```

Each listed item must resolve in the current specification graph. A matching
rule contributes an `always` display reason with the rule ID and changed path;
it does not create a new graph relation or count as direct impact.

The report describes declarations and resolvable repository evidence. A valid
Verification Claim does not mean its test ran or passed. The command does not
execute verifiers, modify specifications, or decide whether a Philosophy or
Policy is substantively satisfied. Reviewers make those judgments.
