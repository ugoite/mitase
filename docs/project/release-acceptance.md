---
title: "Release train acceptance"
description: "The release-line gates for Mitase 0.1.x dogfood and the v0.2 cutover."
sidebar_position: 5
---

# Release train acceptance

Mitase's next patch line is the 0.1.x dogfood release. It keeps the semantic
core and makes the new authoring and configuration frontends observable in the
real Mitase corpus. The later 0.2 release candidate closes the canonical v1
authoring entrance.

The release candidate workflow runs
[`check-release-acceptance.sh`](../../scripts/ci/check-release-acceptance.sh)
after `mise run ci`. This is repository release tooling. It does not add
release, planning, execution, test-running, or mutation responsibilities to
the Mitase executable.

## Release-line gates

| Release line | Normal source policy | Required acceptance |
| --- | --- | --- |
| `0.1.x` | Dual-source | v2 self-hosted dogfood, v1 fixture loading, canonical graph regression, and exact artifact-resolution regression |
| `0.2.x` including RCs | v2-only | v1 rejection from `check` and `validate`, actionable `MITASE-SOURCE-001`, read-only `migrate`, canonical graph and artifact-resolution regressions, and the frozen architecture boundary |

The 0.1.x candidate is built from the exact merged source SHA. Its workspace
version and installer default identify the next patch release; the candidate
workflow records the same source SHA and artifact digests in the immutable
candidate manifest.

The 0.2 gate uses `fixtures/v1/valid-web-app` as a deliberately small v1
source. Both normal workspace commands must reject it and point to
`mitase migrate <source> --stdout`. The migration check only reads the source
and verifies that it emits `schema: mitase/authoring/v2`.

Canonical graph preservation is covered by the checked-in digest regression in
`tests/v1_cli.rs`, which compares the normalized v2 corpus with the pre-v2
canonical baseline. The same release gate checks the self-hosted exact target
resolution baseline. These checks verify meaning and repository identity
without introducing a second semantic model.

Promotion remains separate: `release-publish` downloads and verifies the
candidate identified by its run ID and candidate ID, then publishes those exact
bytes. It never rebuilds from `main` or a mutable tag. See the [release
candidate identity contract](./release-candidate-contract.md).
