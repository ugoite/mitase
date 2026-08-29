---
title: "The specification model"
description: "Move from stable intent to exact implementation and verification through mitase's four layers."
sidebar_position: 1
---

# The specification model

`mitase` keeps intent, responsibility, and evidence distinct so a repository
can change its implementation without losing its specification:

1. Philosophy records the durable ideal.
2. Policy turns that ideal into rules.
3. Requirement states what must be true.
4. Criterion turns a Requirement into an independently verifiable condition.
5. Feature owns the implementation responsibility.
6. Binding connects that responsibility to an exact Artifact target.

The [concepts](./concepts.md) page is the compact vocabulary guide. The
[domain glossary](./domain-glossary.md) provides the complete terms. The
[architecture](./v1-architecture.md) explains the canonical graph and
traceability rules behind it.
