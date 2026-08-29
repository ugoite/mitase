---
name: mitase-release
description: Use when changing Mitase release, packaging, installer, or publish workflows.
---

# Mitase release

Release work is a separate delivery unit from normal CI and repository
contract changes.

## Rules

- Read `docs/project/mitase-re-foundation-freeze.md` and the relevant
  `.github/workflows/release-*.yml` files first.
- Resolve the selected release tag/source revision once.
- Build every release artifact from that immutable revision.
- Keep version and action pins explicit and reproducible.
- Do not publish an incidental transitional `main` state.
- Keep packaging, installer, and publish semantics out of the normal CI lane.

## Validation

Run the narrowest release validation first, then the relevant root Mise tasks.
Report the source revision, artifact identity, and verification evidence in
the public PR without exposing private environment details.
