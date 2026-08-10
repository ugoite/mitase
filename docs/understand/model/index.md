---
title: "The specification model"
description: "Move from stable intent to exact implementation and verification through mitase's four layers."
sidebar_position: 1
---

# The specification model

`mitase` keeps four persistent layers distinct so a repository can change its
implementation without losing its intent:

1. Philosophy records the durable ideal.
2. Policy turns that ideal into rules.
3. Requirement states what must be true and how it can be verified.
4. Feature connects the requirement to implementation and evidence.

The [concepts](./concepts.md) page is the compact vocabulary guide. The
[v1 architecture](./v1-architecture.md) explains the canonical document and
traceability rules behind it.
