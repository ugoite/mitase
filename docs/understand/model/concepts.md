# Concepts

The active Mitase model is a persistent specification graph. It describes the
conditions a repository must satisfy and the artifacts that carry the
responsibility or proof. It does not own a work queue or an execution session.

Core ideas:

- Requirements state observable behavior or contracts.
- Criteria make each Requirement independently verifiable.
- Features are implementation responsibilities, not execution jobs.
- Bindings connect Features to exact Artifact targets.
- Verification Claims connect Criteria to the evidence that proves them.
- Reverse relationships are derived by `SpecIndex` rather than persisted twice.

See the [domain glossary](./domain-glossary.md) for the complete vocabulary and
the [architecture](./v1-architecture.md) for validation and dependency rules.

## Philosophy

The durable ideal that should survive implementation changes.

## Policy

The repository-wide rule that makes the ideal actionable and reviewable.

## Requirement

The observable behavior or contract that must be satisfied.

## Criterion

One concrete acceptance condition used to determine whether a Requirement is
satisfied.

## Feature

The implementation responsibility that realizes one or more Requirements.

## Binding

The connection between a Feature and an exact repository Artifact target.

## Artifact

An object outside the specification hierarchy, such as source, test,
configuration, API definition, workflow, directory, or symbol.
