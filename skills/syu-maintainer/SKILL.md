---
name: syu-maintainer
description: Maintain a syu-driven repository by updating the layered spec, preserving adjacent links, running validate/report, and refreshing generated artifacts when the spec changes.
---

# syu maintainer

<!-- FEAT-SKILLS-001 -->

Use this skill when you need to make or review changes in a repository that uses
`syu` for specification-driven development.

## Goals

- Keep philosophy, policy, requirement, and feature changes connected.
- Update implementation and test traceability instead of leaving spec edits as
  disconnected YAML.
- Run the repository checks that keep `syu` trustworthy.
- Refresh generated artifacts such as the checked-in validation report when the
  spec or validation output changes.

## Workflow

1. Inspect the current layered model before editing.
   - Run `syu` or `syu browse .` to see counts, linked definitions, and current
     validation errors.
   - Read the relevant files under `docs/syu/`.
2. When changing intent, update adjacent layers together.
   - Philosophy should link to policy.
   - Policy should link to requirement.
   - Requirement should link to feature.
   - Features should trace to code, tests, docs, scripts, or workflows that
     actually implement the behavior.
3. Keep validation rules in mind.
   - `syu validate .` should stay green unless you are intentionally working
     through a broken intermediate state.
   - If rule-backed errors appear, read the rule code, title, summary, and
     description before changing files.
4. Refresh generated artifacts when relevant.
   - Run `syu report . --output docs/generated/syu-report.md` after spec or
     validation changes.
   - Regenerate docs derived from `docs/syu/` when spec files change.
5. Finish by running the repository quality checks and summarizing what changed.

## Commands

```bash
syu
syu browse .
syu validate .
syu report . --output docs/generated/syu-report.md
scripts/ci/quality-gates.sh
```

## Guidelines

- Prefer small, reviewable edits that keep the repository in a releasable state.
- Do not add orphan philosophy, policy, requirement, or feature entries.
- If the repository enables strict symbol ownership, make sure public APIs belong
  to features and tests belong to requirements.
- Treat docs, CI, release scripts, and contributor tooling as part of the
  product contract when they are linked from the spec.
