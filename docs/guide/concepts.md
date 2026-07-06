# Concepts

The active v1 model is built from four persistent spec layers and two temporary work artifacts.

Persistent layers:

- philosophies
- policies
- requirements
- features

Temporary work artifacts:

- work requests (`syu/work-request/v1`)
- work plans (`syu/work-plan/v1`)
- exported context packs (`syu/context-pack/v1`)

Core ideas:

- bindings connect spec anchors to exact artifact targets
- requirements express criteria
- features satisfy criteria through implementation bindings
- verification bindings provide execution evidence
- work planning derives bounded execution slices from the graph
