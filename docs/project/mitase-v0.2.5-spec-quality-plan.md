---
title: "Mitase v0.2.5 spec quality plan"
description: "Staged delivery plan for Meaningful Specification Quality (Q001-Q005)."
sidebar_position: 5
---

# Mitase v0.2.5 spec quality plan

**Status:** active delivery plan, not a completion claim.
**Scope:** `ugoite/mitase` v0.2.5 implementation; Ugoite adoption follows only
after the official immutable release.
**Source of truth:** `docs/project/mitase-re-foundation-freeze.md`, applicable
ADRs, and `AGENTS.md`.

## Goal

Promote specifications where distinct decisions, promises, and implementation
responsibilities live in meaningful positions. Detect structurally unnatural
but formally valid specifications with evidence-backed diagnostics along two
axes:

1. **Normative Redundancy:** the same promise is repeated across layers/items.
2. **Responsibility Cohesion:** bundled conditions and responsibilities lack
   shared meaning.

## Frozen boundaries

- Keep `Philosophy → Policy → Requirement → Criterion → Feature → Binding →
  Artifact` plus `Verification Claim → Verifier / Test / Artifact`.
- Normal authoring source stays `mitase/authoring/v2` only. No new spec kind,
  no `authoring/v3`, no mandatory Policy/Philosophy, no mandatory Facet.
- Mitase reads, validates, interprets, and reports. It does not run LLM
  semantic review, generate or patch code, execute tests, own workflows,
  operate CI, or auto-fix.
- Structural suspicion is not semantic error. Target existence, test execution
  and success, and whether a test proves intent are distinct facts.
- No consumer-specific vocabulary in generic detection logic.

## Baseline discipline

- Mitase is pre-v1: prefer the ideal current design over compatibility with
  old internal forms unless explicitly required.
- The `v0.2.2` semantic digest fixture stays frozen. Self-spec wording changes
  (for example authoring principles) require either whitelisted semantic-diff
  verification or an ADR-approved snapshot migration. M-01 therefore defers
  self-spec item edits and ships guidance as normative docs.
- Record the v0.2.4 base SHA, current base SHA, measured Q001–Q005 JSON,
  counts, timing, errors, and regression-gate results before implementation.
  Do not hard-code preliminary sample counts as test expectations.

## Staged delivery

Each stage is a small serial PR merged before the next worktree starts.

| PR | Scope | Merge condition |
| --- | --- | --- |
| M-01 | Principles, Q001–Q005 contract, plan baseline, self-spec change policy | Spec review plus frozen-regression handling agreed; docs-only |
| M-02 | Deterministic checks Q001/Q005 with CLI/JSON/LSP parity | True positives fire, negative controls stay silent |
| M-03 | Graph checks Q003/Q004 on direct relations | Cohesion/duplication cases plus scale regression |
| M-04 | Composite heuristic Q002, dedup, docs, release notes | False-positive review complete, old gates green |
| M-05 | 0.2.5 version finalization and release-grade confirmation | Immutable candidate can proceed |

Implementation notes:

- New quality checks live in a read-only `mitase-validation` quality module of
  pure functions, invoked after graph validation only when prerequisites hold.
- Register `MITASE-QUALITY-*` rule metadata, keep `phase_for_rule` mapping to
  `graph`, use stable sort and shared diagnostic locations/evidence.
- Quality findings are Warnings (Q001–Q004) or Info (Q005); existing errors
  are unchanged and existing `mitase check` success conditions do not change
  on Warning-only output.
- Q001/Q004/Q005 target zero false positives on deterministic cases; Q002/Q003
  must explain each firing and ignore intentional negative controls.
- Release stop conditions: unexplained semantic-graph regression updates,
  mass false positives on healthy Facet structures, public-API breakage from
  quality warnings, or a failing release-grade gate.

## Formal release

M-01–M-05 merge at a recorded SHA, then follow the existing
`release-candidate` / `release-publish` contract without adding a new release
command: bump the workspace version, dispatch the candidate workflow with the
merged source SHA, verify the candidate manifest and four target archives,
promote the identical bytes, and verify the public tag.

## Ugoite adoption (after publication only)

1. Pin only the published v0.2.5 artifacts by measured SHA-256; never connect
   `main` HEAD or an unpublished candidate to production CI.
2. Triage quality findings with stable fingerprints (rule, subject, reference,
   related anchor), keep reasonable exceptions with design reasons, and gate
   only new Q001–Q004 fingerprints in CI.
3. Refactor in small reviewed PRs without changing current meaning,
   evidence structure, or observable behavior.
4. Roll back to the prior pin on release or upgrade failure; never overwrite a
   published tag/artifact.
