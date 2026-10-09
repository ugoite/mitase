---
title: "Mitase 0.2.5 release notes"
description: "Evidence-backed quality diagnostics for structurally unnatural specifications."
sidebar_position: 10
---

# Mitase 0.2.5 release notes

Mitase 0.2.5 adds Meaningful Specification Quality: five diagnostics that
detect formally valid specifications whose layers and items do not carry
distinct meaning. The checks are read-only interpretation; they never run
tests, modify the workspace, or apply fixes.

## New diagnostics

All five run in the Graph phase in both Standard and Strict presets:

| Code | Name | Severity |
| --- | --- | --- |
| `MITASE-QUALITY-001` | Layer Echo | Warning |
| `MITASE-QUALITY-002` | One-off Policy | Warning |
| `MITASE-QUALITY-003` | Fragmented Requirement | Warning |
| `MITASE-QUALITY-004` | Duplicate Obligation | Warning |
| `MITASE-QUALITY-005` | Redundant Rule Description | Info |

Quality findings never change the success condition of `mitase check` on
their own: a workspace reporting only quality findings still passes, and
every pre-existing error keeps its meaning. Filter the quality-only subset
by the `MITASE-QUALITY-` code prefix in text, JSON, and LSP output.

## Impact on existing repositories

Upgrading the validator may surface new Warning or Info findings on a
previously green specification. That is expected and not a regression: the
findings point at repeated promises across layers (normative redundancy) or
bundled responsibilities without a shared change reason (responsibility
cohesion). Triage each finding by reading its evidence, subject and
counterpart anchors, and suggested action:

- keep the local acceptance condition and generalize or remove the redundant
  upper layer;
- split the item by independent change reason;
- consolidate the duplicated obligation; or
- record why the shape is intentional and keep it.

Do not bulk-rename or bulk-delete specification text just to silence a
warning. Do not weaken existing error or readiness gates because of quality
findings.

## Limits

Quality diagnostics report structural suspicion, not semantic error. They do
not judge whether a `governed_by` derivation is causally correct, whether a
Criterion sentence is truly falsifiable, or whether a named test proves its
Criterion. Deterministic rules (Q001/Q004/Q005) never fire on fuzzy
similarity, and short shared phrases stay silent. See [Spec quality
diagnostics](../understand/quality/spec-quality-diagnostics.md) for the
comparison rules and negative controls.
