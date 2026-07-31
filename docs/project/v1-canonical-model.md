# ADR 0001: Syu v1 canonical model

Status: accepted

Syu v1 uses strict, single-version documents for specifications, project configuration, work requests, and work plans. Specifications retain four layers. Relations are persisted only from rules to principles, criteria to rules, and bindings to criteria. `SpecIndex` derives every reverse view.

Stable local anchors identify principles, rules, criteria, bindings, and contracts. Binding-owned targets require a local target ID, adapter, repository-relative path, and typed selector. Line ranges are resolved metadata and are never persistent identity.

Contracts remain Feature-local entities and connect provider/consumer-style participants across facets. Executable slices may only originate from explicit bindings with resolved targets. Heuristics may produce review candidates but never executable scope.

The cutover intentionally provides no compatibility parser, migration command, field aliases, or deprecated commands.
