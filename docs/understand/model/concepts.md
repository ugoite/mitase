# Concepts

The active v1 model is built from four persistent spec layers and two temporary work artifacts.

Persistent layers:

- philosophies
- policies
- requirements
- features

Temporary work artifacts:

- work requests (`mitase/work-request/v1`)
- work plans (`mitase/work-plan/v1`)
- exported context packs (`mitase/context-pack/v1`)

Core ideas:

- bindings connect spec anchors to exact artifact targets
- requirements express criteria
- features satisfy criteria through implementation bindings
- verification bindings provide execution evidence
- work planning derives bounded execution slices from the graph

## Philosophy

The durable ideal that should survive implementation changes.

## Policy

The repository-wide rule that makes the ideal actionable and reviewable.

## Requirements

The concrete criteria that must be satisfied and can be checked against
implementation or verification evidence.

## Features

The implemented capability that connects requirements to exact code, tests, and
other evidence.
