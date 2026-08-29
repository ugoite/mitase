---
title: "Domain glossary"
description: "Stable terms for Mitase's executable specification graph."
sidebar_position: 3
---

# Domain glossary

This glossary is normative for the Mitase Re-Foundation. The terms describe a
repository specification; they do not describe an execution runtime.

## Philosophy

A durable principle that a project intends to preserve over time. A Philosophy
should change less frequently than implementation details.

## Policy

A design constraint, invariant, or decision rule derived from one or more
Philosophies.

## Requirement

An observable behavior or contract that the repository must satisfy. A
Requirement is not complete until its acceptance conditions are explicit.

## Criterion

An individual, verifiable acceptance condition for a Requirement. Criteria are
first-class objects so that coverage and verification can be reported without
collapsing several behaviors into one status.

## Feature

An implementation responsibility that realizes one or more Requirements. A
Feature is not limited to a user-facing capability and is not a work item,
task, or agent run.

## Binding

A typed connection from a Feature to a concrete Artifact responsibility. A
Binding owns the exact target selector. Criterion evidence is connected through
a Verification Claim.

## Artifact

An object in the repository or another inspected boundary, outside the
specification hierarchy. Examples include a Rust or TypeScript source file,
symbol, test, OpenAPI operation, Markdown section, configuration file, GitHub
workflow, SQL statement, JSON Schema, directory, or API definition.

## Artifact target

The repository-relative path and typed selector that identify the precise
Artifact a Binding refers to. Stable anchors and selectors are identity; line
ranges are resolved metadata and should not be used as persistent identity when
a more stable selector exists.

## Verification Claim

A declaration that a Criterion is proved by a particular Verifier, Test, or
Artifact. A claim is valid only when its Criterion exists and its target
resolves according to the repository validation rules.

## SpecIndex

The derived index of the specification graph. It provides reverse relations,
search, and impact queries without duplicating reverse references in source
documents.

## Inventory

The repository artifact catalog used to discover and resolve files, symbols,
tests, and structured targets. Inventory is evidence for resolution, not a
replacement for explicit ownership.

## Resolver / adapter

A component that maps a typed Artifact target to a repository object. New
artifact types should be added through this boundary rather than by adding
top-level specification kinds for every tool or file format.

## Coverage

The calculated relationship between Requirements, Criteria, Features, Bindings,
Artifacts, and Verification Claims. Coverage reports what is connected and
what remains unverified; it does not make an unowned artifact owned.

## Drift

A diagnostic produced when the declared specification and the current
repository no longer agree, such as a missing file, unresolved symbol, broken
selector, or conflicting owner.

## Derived status

A status calculated from current repository evidence. For example, a Feature
can be reported as `verified` only when its required Bindings resolve and its
Criteria have valid Verification Claims. A human declaration that disagrees
with derived evidence is a diagnostic.

## External execution tool

Any system that changes or runs the repository: a shell, test runner, patch
applier, implementation agent, task queue, or delivery service. Mitase may
describe the evidence these tools produce, but it does not own their lifecycle.
