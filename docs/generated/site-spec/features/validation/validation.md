---
title: "Validation"
description: "Generated reference for docs/syu/features/validation/validation.yaml"
---

> Generated from `docs/syu/features/validation/validation.yaml`.

## Parsed content

### Version

- 1

### Rules

- **code**: SYU-workspace-load-001
  - **genre**: workspace
  - **severity**: error
  - **title**: The specification workspace must load cleanly
  - **summary**: The validator cannot reason about a workspace it cannot parse or read.
  - **description**:
    - |
      Validation begins with loading the workspace root, configuration, and
      specification files. When that bootstrap step fails, every later result
      becomes untrustworthy because `syu` no longer knows which definitions,
      paths, or trace mappings should exist. This rule exists to fail early and
      honestly when the repository shape itself is broken.
- **code**: SYU-workspace-blank-001
  - **genre**: workspace
  - **severity**: error
  - **title**: Required specification fields must not be blank
  - **summary**: Every declared entity needs enough data to remain reviewable and machine-checkable.
  - **description**:
    - |
      A specification entry with an empty required field looks present while
      still withholding the information needed for review and validation. This
      rule protects against half-declared ideas by requiring IDs, titles, core
      prose, and status fields to be explicitly populated. The goal is to keep
      the spec usable both for humans trying to understand intent and for
      automation trying to verify it.
- **code**: SYU-workspace-duplicate-001
  - **genre**: workspace
  - **severity**: error
  - **title**: Definition IDs must be unique
  - **summary**: A stable ID has to name exactly one thing across the specification set.
  - **description**:
    - |
      IDs are the backbone of traceability. If one identifier points to multiple
      philosophies, policies, requirements, or features, links stop being
      meaningful and automated validation can no longer tell which item a trace
      or relationship intended to reference. This rule preserves the repository
      as a navigable graph instead of a collection of ambiguous labels.
- **code**: SYU-workspace-registry-001
  - **genre**: workspace
  - **severity**: error
  - **title**: Feature registry entries must match checked-in feature documents
  - **summary**: Feature YAML files should not exist on disk without an explicit registry entry.
  - **description**:
    - |
      Features are intentionally discovered through `docs/syu/features/features.yaml`
      so contributors can review which implementation-facing documents belong to
      the committed specification. When a feature file exists on disk but is not
      registered there, `syu list`, `syu browse`, and other discovery flows
      silently miss a document that looks authoritative in the repository. This
      rule keeps feature discovery explicit without allowing the checked-in file
      tree and the registry to drift apart.
- **code**: SYU-graph-reference-001
  - **genre**: graph
  - **severity**: error
  - **title**: Linked definitions must exist
  - **summary**: A declared relationship is only meaningful when the target actually exists.
  - **description**:
    - |
      The layered model only works when every forward link resolves to a real
      definition. Missing references silently break the chain from philosophy to
      implementation and make it impossible to explain why a change exists.
      This rule forces contributors to either declare the missing item or remove
      the stale relationship before the graph drifts further.
- **code**: SYU-graph-reciprocal-001
  - **genre**: graph
  - **severity**: error
  - **title**: Adjacent-layer links must be reciprocal
  - **summary**: Both sides of a relationship should agree that the relationship exists.
  - **description**:
    - |
      `syu` treats traceability as a graph that should be understandable in both
      directions. When only one side records a link, reviewers cannot reliably
      navigate from cause to effect or from implementation back to intent. This
      rule preserves explainability by requiring each adjacent layer to confirm
      the relationship from its own point of view.
- **code**: SYU-graph-links-001
  - **genre**: graph
  - **severity**: warning
  - **title**: Adjacent-layer expectations should be explicit
  - **summary**: Definitions should usually point to the layers they influence or satisfy.
  - **description**:
    - |
      Not every incomplete link is automatically fatal, but missing adjacent
      links are a strong smell that the spec is becoming harder to navigate.
      This warning nudges authors to keep the graph richly connected so the
      repository remains easy to browse, audit, and evolve over time.
- **code**: SYU-graph-orphaned-001
  - **genre**: graph
  - **severity**: error
  - **title**: Definitions must not be isolated from the layered graph
  - **summary**: A philosophy, policy, requirement, or feature should connect to at least one adjacent layer.
  - **description**:
    - |
      An isolated definition may contain prose, but it does not influence the
      rest of the repository. That means it cannot guide implementation,
      constrain behavior, or be justified by higher-level intent. This rule
      exists to keep every layer part of one connected explanation instead of
      allowing decorative or abandoned nodes to accumulate.
- **code**: SYU-graph-duplicate-001
  - **genre**: graph
  - **severity**: error
  - **title**: Relationship lists must not repeat the same linked ID
  - **summary**: Repeating the same adjacent-layer reference does not create new traceability.
  - **description**:
    - |
      Link lists explain why one definition relates to another. When the same
      target ID appears more than once inside one list, the graph becomes
      noisier without becoming more informative. This rule keeps adjacent-layer
      relationships crisp and reviewable by requiring each linked ID to appear
      at most once per list.
- **code**: SYU-delivery-invalid-001
  - **genre**: delivery
  - **severity**: error
  - **title**: Delivery state values must be supported
  - **summary**: Requirements and features need a status vocabulary the validator understands.
  - **description**:
    - |
      Delivery-state automation only works when the state machine is explicit and
      finite. If contributors invent ad hoc status names, `syu` can no longer
      tell whether traces should exist, be absent, or be considered complete.
      This rule keeps workflow semantics predictable by limiting the state
      vocabulary to supported values.
- **code**: SYU-delivery-planned-001
  - **genre**: delivery
  - **severity**: error
  - **title**: Planned items can be forbidden by project policy
  - **summary**: Some repositories want the spec to describe only already-delivered work.
  - **description**:
    - |
      Teams reach moments when backlog-style entries should no longer remain in
      the committed specification for a branch, release line, or audited
      workspace. This rule lets a project tighten that policy without rewriting
      the entire validation engine. The goal is to support both exploratory
      workflows and locked-down release workflows through explicit config.
- **code**: SYU-delivery-planned-002
  - **genre**: delivery
  - **severity**: error
  - **title**: Planned items must not claim delivered traces
  - **summary**: Declaring tests or implementations for planned work blurs intent and completion.
  - **description**:
    - |
      `planned` means the work is accepted but not yet delivered. Once a planned
      item already points to tests or implementations, the status and the
      evidence disagree, which makes reports and roadmap conversations harder to
      trust. This rule preserves the meaning of planning by keeping future work
      distinct from delivered work.
- **code**: SYU-delivery-implemented-001
  - **genre**: delivery
  - **severity**: error
  - **title**: Implemented items must prove delivery
  - **summary**: A delivered requirement or feature needs explicit evidence in tests or code.
  - **description**:
    - |
      Marking something as implemented is a promise that the repository can
      already demonstrate it. Without traces, that promise becomes a belief
      rather than evidence. This rule exists to make implementation status
      reviewable and to keep status labels from drifting away from reality.
- **code**: SYU-delivery-missing-001
  - **genre**: delivery
  - **severity**: warning
  - **title**: Undefined delivery state leaves trace expectations unclear
  - **summary**: If status is omitted or invalid, absent traces become a warning rather than silent drift.
  - **description**:
    - |
      Sometimes a repository is partway through adoption and has not yet settled
      on a clean delivery state for every entry. In those cases `syu` still
      warns when no traces are declared so the gap remains visible. The warning
      preserves forward pressure toward explicit, reviewable delivery semantics.
- **code**: SYU-delivery-agreement-001
  - **genre**: delivery
  - **severity**: warning
  - **title**: Linked requirements and features should tell the same delivery story
  - **summary**: Adjacent delivery states should not imply contradictory implementation progress.
  - **description**:
    - |
      Requirements and features are two views of the same delivery chain. A
      planned requirement that already links to implemented features, or an
      implemented feature that links only to planned requirements, makes the
      specification harder to trust because the adjacent layers no longer tell a
      coherent story about what is delivered. This rule keeps delivery-state
      intent reviewable without escalating immediately to a hard failure.
- **code**: SYU-trace-language-001
  - **genre**: trace
  - **severity**: error
  - **title**: Trace mappings must use a supported language adapter
  - **summary**: Validation can only prove traces when it knows how to inspect that language.
  - **description**:
    - |
      A trace declaration is not self-validating. `syu` needs a language adapter
      to verify the file shape, symbol, and optional documentation snippets it
      refers to. This rule prevents repositories from gaining a false sense of
      security by declaring traces in languages the current validator cannot
      actually inspect.
- **code**: SYU-trace-file-001
  - **genre**: trace
  - **severity**: error
  - **title**: Trace mappings must declare a file path
  - **summary**: A trace without an explicit file path cannot identify repository evidence.
  - **description**:
    - |
      Traceability depends on concrete, inspectable files. If a mapping omits the
      file path entirely, the validator has nowhere to look for evidence and
      reviewers cannot tell which artifact is supposed to satisfy the spec. This
      rule keeps every declared trace grounded in an explicit repository path.
- **code**: SYU-trace-file-002
  - **genre**: trace
  - **severity**: error
  - **title**: Declared trace files must exist
  - **summary**: A trace that points to a missing file cannot prove anything about the repository.
  - **description**:
    - |
      Traceability depends on concrete, inspectable files. If a path is stale or
      points outside the current repository contents, the spec stops pointing to
      real evidence and starts pointing to an aspiration. This rule ensures that
      every declared trace remains grounded in a repository artifact that both
      reviewers and automation can actually inspect.
- **code**: SYU-trace-file-003
  - **genre**: trace
  - **severity**: warning
  - **title**: Trace file paths should use canonical repository-relative form
  - **summary**: Portable path spelling keeps checked-in traces easier to review and reuse.
  - **description**:
    - |
      Trace paths become harder to review when they rely on redundant `./` or
      `..` segments, backslash separators, or other spellings that are not the
      repository's canonical relative form. This rule warns on path notation
      drift and suggests the normalized repository-relative path so the spec can
      stay portable without silently rewriting checked-in YAML.
- **code**: SYU-trace-unreadable-001
  - **genre**: trace
  - **severity**: error
  - **title**: Trace files must be readable to the validator
  - **summary**: Evidence that cannot be read cannot be validated.
  - **description**:
    - |
      Repositories sometimes contain files with broken permissions, encoding
      issues, or transient access problems. A trace into such a file is still a
      broken trace because the validator cannot confirm the promised behavior.
      This rule exists to surface those operational failures instead of silently
      assuming the trace is probably fine.
- **code**: SYU-trace-extension-001
  - **genre**: trace
  - **severity**: error
  - **title**: The declared trace language must match the traced file
  - **summary**: File extensions and language declarations should describe the same artifact.
  - **description**:
    - |
      Language adapters carry language-specific inspection logic. If a trace says
      "Rust" while pointing to a non-Rust file, the validator is forced into an
      invalid interpretation and reviewers lose confidence in what the trace
      means. This rule keeps the declared language and the referenced artifact in
      sync.
- **code**: SYU-trace-id-001
  - **genre**: trace
  - **severity**: error
  - **title**: Traced files must mention the owning specification ID
  - **summary**: A trace should be explicit inside the implementation or test artifact itself.
  - **description**:
    - |
      `syu` is built around repository-native explainability. It is not enough
      for YAML to say a file belongs to a requirement or feature; the file should
      also mention that ownership so someone reading the code or test can see the
      link without consulting another system. This rule keeps traceability visible
      at the point where work actually happens.
- **code**: SYU-trace-symbol-001
  - **genre**: trace
  - **severity**: error
  - **title**: Trace mappings must identify at least one symbol
  - **summary**: Validation needs a concrete symbol target instead of an empty mapping.
  - **description**:
    - |
      Files often contain many behaviors. If a trace mapping does not name any
      symbols at all, the validator cannot tell which implementation or test case
      is supposed to satisfy the requirement or feature. This rule keeps traces
      precise enough to support audits, refactors, and future maintenance.
- **code**: SYU-trace-symbol-002
  - **genre**: trace
  - **severity**: error
  - **title**: Trace symbol entries must not be blank
  - **summary**: Whitespace-only symbol entries hide which repository behavior the trace intends to verify.
  - **description**:
    - |
      A symbol list that contains blank entries looks more complete than it is.
      Reviewers and tooling still lack a usable target, but the mapping no longer
      reads as obviously empty. This rule keeps symbol declarations explicit by
      requiring every listed entry to name a real symbol.
- **code**: SYU-trace-symbol-003
  - **genre**: trace
  - **severity**: error
  - **title**: Declared trace symbols must exist in the traced file
  - **summary**: A trace can only prove ownership when the named symbol is present in the referenced file.
  - **description**:
    - |
      Symbol-level traces are useful because they point to the exact behavior a
      requirement or feature relies on. If the named symbol is missing, the spec
      no longer identifies real evidence and may be drifting behind refactors or
      stale documentation. This rule keeps symbol traces anchored to actual code
      and tests.
- **code**: SYU-trace-duplicate-001
  - **genre**: trace
  - **severity**: error
  - **title**: Trace lists must not repeat the same mapping
  - **summary**: Duplicated trace records inflate evidence without adding new repository facts.
  - **description**:
    - |
      A trace list should enumerate distinct pieces of repository evidence.
      Repeating the exact same file, symbol, and documentation mapping inside
      one language list makes the specification look more complete than it is
      and makes review harder. This rule keeps trace evidence explicit by
      requiring each mapping to appear only once per list.
- **code**: SYU-trace-inspection-001
  - **genre**: trace
  - **severity**: error
  - **title**: Rich symbol inspection must succeed when required
  - **summary**: Language-aware validation should fail loudly when the inspection pipeline breaks.
  - **description**:
    - |
      `syu` prefers real parsing and runtime-backed inspection over brittle text
      guesses. When those richer inspection paths fail, the safest behavior is to
      surface the failure immediately instead of falling back to a weaker,
      success-shaped assumption. This rule protects trust in the validator's
      conclusions.
- **code**: SYU-trace-doc-001
  - **genre**: trace
  - **severity**: error
  - **title**: Required trace documentation snippets must be present
  - **summary**: Documentation assertions in trace mappings should be enforced, not advisory.
  - **description**:
    - |
      Some trace relationships need more than a symbol name - they need specific
      explanatory text to remain maintainable. This rule enforces those declared
      documentation snippets so that code, tests, and generated reports all
      preserve the rationale the spec asked for.
- **code**: SYU-trace-docscope-001
  - **genre**: trace
  - **severity**: error
  - **title**: Wildcard traces cannot carry symbol-level doc assertions
  - **summary**: `doc_contains` checks need a single symbol target instead of a wildcard file claim.
  - **description**:
    - |
      Wildcard ownership is useful when one specification item intentionally owns
      every relevant symbol in a file. Documentation assertions are different:
      they need one concrete symbol so the validator knows exactly which docs to
      inspect. This rule keeps doc-level checks precise instead of letting them
      ride on an ambiguous wildcard mapping.
- **code**: SYU-trace-docsupport-001
  - **genre**: trace
  - **severity**: error
  - **title**: Documentation assertions require rich symbol inspection
  - **summary**: `doc_contains` promises should only be made where the validator can inspect symbol docs.
  - **description**:
    - |
      A repository should not declare documentation constraints that the current
      adapter cannot verify. Otherwise the spec would appear stricter than the
      tool can honestly enforce. This rule preserves integrity by requiring
      doc-level checks to stay within languages with meaningful documentation
      inspection support.
- **code**: SYU-coverage-walk-001
  - **genre**: coverage
  - **severity**: error
  - **title**: Coverage inventory paths must be walkable
  - **summary**: Strict trace coverage starts by discovering Rust sources under `src/` and `tests/`.
  - **description**:
    - |
      The strict trace coverage rule only means something when `syu` can walk the
      repository paths that are supposed to contain owned Rust source and test
      files. If directory discovery fails, the inventory itself is incomplete and
      any 100-percent coverage conclusion would be misleading. This rule surfaces
      repository layout problems before coverage claims become untrustworthy.
- **code**: SYU-coverage-read-001
  - **genre**: coverage
  - **severity**: error
  - **title**: Coverage inventory files must be readable
  - **summary**: Unreadable Rust sources prevent honest trace coverage verification.
  - **description**:
    - |
      Trace coverage can only be enforced when `syu` can read the Rust source and
      test files it is supposed to inventory. If a file is unreadable, the safest
      behavior is to fail loudly instead of pretending the workspace was fully
      scanned. This rule protects the credibility of strict coverage mode.
- **code**: SYU-coverage-parse-001
  - **genre**: coverage
  - **severity**: error
  - **title**: Coverage inventory sources must parse successfully
  - **summary**: Broken Rust syntax prevents reliable coverage inventory.
  - **description**:
    - |
      Strict trace coverage relies on understanding the Rust items that appear in
      the workspace. When a source file does not parse, `syu` cannot build a
      trustworthy inventory of public APIs or tests. This rule keeps coverage
      results explicit by surfacing syntax failures instead of hiding them behind
      incomplete scans.
- **code**: SYU-coverage-public-001
  - **genre**: coverage
  - **severity**: error
  - **title**: Public API symbols must belong to at least one feature
  - **summary**: Public surface area should never appear in the repository without an owning feature.
  - **description**:
    - |
      Public functions, classes, modules, and similar API-facing symbols create
      long-term maintenance obligations. If they exist without a feature link,
      the repository has no durable explanation for why that surface area exists
      or which behavior owns it. This rule encourages deliberate API growth by
      forcing public surface area to stay attached to an explicit feature.
- **code**: SYU-coverage-test-001
  - **genre**: coverage
  - **severity**: error
  - **title**: Tests must belong to at least one requirement
  - **summary**: Every test should prove some requirement instead of becoming orphaned automation.
  - **description**:
    - |
      Tests are executable claims about intended behavior. When a test has no
      requirement owner, it becomes much harder to tell whether it is obsolete,
      redundant, or still essential to the product. This rule keeps the test
      suite aligned with the specification by ensuring that each test remains
      justified by at least one requirement.

## Source YAML

```yaml
# FEAT-CHECK-001
version: 1

rules:
  - code: SYU-workspace-load-001
    genre: workspace
    severity: error
    title: The specification workspace must load cleanly
    summary: The validator cannot reason about a workspace it cannot parse or read.
    description: |
      Validation begins with loading the workspace root, configuration, and
      specification files. When that bootstrap step fails, every later result
      becomes untrustworthy because `syu` no longer knows which definitions,
      paths, or trace mappings should exist. This rule exists to fail early and
      honestly when the repository shape itself is broken.

  - code: SYU-workspace-blank-001
    genre: workspace
    severity: error
    title: Required specification fields must not be blank
    summary: Every declared entity needs enough data to remain reviewable and machine-checkable.
    description: |
      A specification entry with an empty required field looks present while
      still withholding the information needed for review and validation. This
      rule protects against half-declared ideas by requiring IDs, titles, core
      prose, and status fields to be explicitly populated. The goal is to keep
      the spec usable both for humans trying to understand intent and for
      automation trying to verify it.

  - code: SYU-workspace-duplicate-001
    genre: workspace
    severity: error
    title: Definition IDs must be unique
    summary: A stable ID has to name exactly one thing across the specification set.
    description: |
      IDs are the backbone of traceability. If one identifier points to multiple
      philosophies, policies, requirements, or features, links stop being
      meaningful and automated validation can no longer tell which item a trace
      or relationship intended to reference. This rule preserves the repository
      as a navigable graph instead of a collection of ambiguous labels.

  - code: SYU-workspace-registry-001
    genre: workspace
    severity: error
    title: Feature registry entries must match checked-in feature documents
    summary: Feature YAML files should not exist on disk without an explicit registry entry.
    description: |
      Features are intentionally discovered through `docs/syu/features/features.yaml`
      so contributors can review which implementation-facing documents belong to
      the committed specification. When a feature file exists on disk but is not
      registered there, `syu list`, `syu browse`, and other discovery flows
      silently miss a document that looks authoritative in the repository. This
      rule keeps feature discovery explicit without allowing the checked-in file
      tree and the registry to drift apart.

  - code: SYU-graph-reference-001
    genre: graph
    severity: error
    title: Linked definitions must exist
    summary: A declared relationship is only meaningful when the target actually exists.
    description: |
      The layered model only works when every forward link resolves to a real
      definition. Missing references silently break the chain from philosophy to
      implementation and make it impossible to explain why a change exists.
      This rule forces contributors to either declare the missing item or remove
      the stale relationship before the graph drifts further.

  - code: SYU-graph-reciprocal-001
    genre: graph
    severity: error
    title: Adjacent-layer links must be reciprocal
    summary: Both sides of a relationship should agree that the relationship exists.
    description: |
      `syu` treats traceability as a graph that should be understandable in both
      directions. When only one side records a link, reviewers cannot reliably
      navigate from cause to effect or from implementation back to intent. This
      rule preserves explainability by requiring each adjacent layer to confirm
      the relationship from its own point of view.

  - code: SYU-graph-links-001
    genre: graph
    severity: warning
    title: Adjacent-layer expectations should be explicit
    summary: Definitions should usually point to the layers they influence or satisfy.
    description: |
      Not every incomplete link is automatically fatal, but missing adjacent
      links are a strong smell that the spec is becoming harder to navigate.
      This warning nudges authors to keep the graph richly connected so the
      repository remains easy to browse, audit, and evolve over time.

  - code: SYU-graph-orphaned-001
    genre: graph
    severity: error
    title: Definitions must not be isolated from the layered graph
    summary: A philosophy, policy, requirement, or feature should connect to at least one adjacent layer.
    description: |
      An isolated definition may contain prose, but it does not influence the
      rest of the repository. That means it cannot guide implementation,
      constrain behavior, or be justified by higher-level intent. This rule
      exists to keep every layer part of one connected explanation instead of
      allowing decorative or abandoned nodes to accumulate.

  - code: SYU-graph-duplicate-001
    genre: graph
    severity: error
    title: Relationship lists must not repeat the same linked ID
    summary: Repeating the same adjacent-layer reference does not create new traceability.
    description: |
      Link lists explain why one definition relates to another. When the same
      target ID appears more than once inside one list, the graph becomes
      noisier without becoming more informative. This rule keeps adjacent-layer
      relationships crisp and reviewable by requiring each linked ID to appear
      at most once per list.

  - code: SYU-delivery-invalid-001
    genre: delivery
    severity: error
    title: Delivery state values must be supported
    summary: Requirements and features need a status vocabulary the validator understands.
    description: |
      Delivery-state automation only works when the state machine is explicit and
      finite. If contributors invent ad hoc status names, `syu` can no longer
      tell whether traces should exist, be absent, or be considered complete.
      This rule keeps workflow semantics predictable by limiting the state
      vocabulary to supported values.

  - code: SYU-delivery-planned-001
    genre: delivery
    severity: error
    title: Planned items can be forbidden by project policy
    summary: Some repositories want the spec to describe only already-delivered work.
    description: |
      Teams reach moments when backlog-style entries should no longer remain in
      the committed specification for a branch, release line, or audited
      workspace. This rule lets a project tighten that policy without rewriting
      the entire validation engine. The goal is to support both exploratory
      workflows and locked-down release workflows through explicit config.

  - code: SYU-delivery-planned-002
    genre: delivery
    severity: error
    title: Planned items must not claim delivered traces
    summary: Declaring tests or implementations for planned work blurs intent and completion.
    description: |
      `planned` means the work is accepted but not yet delivered. Once a planned
      item already points to tests or implementations, the status and the
      evidence disagree, which makes reports and roadmap conversations harder to
      trust. This rule preserves the meaning of planning by keeping future work
      distinct from delivered work.

  - code: SYU-delivery-implemented-001
    genre: delivery
    severity: error
    title: Implemented items must prove delivery
    summary: A delivered requirement or feature needs explicit evidence in tests or code.
    description: |
      Marking something as implemented is a promise that the repository can
      already demonstrate it. Without traces, that promise becomes a belief
      rather than evidence. This rule exists to make implementation status
      reviewable and to keep status labels from drifting away from reality.

  - code: SYU-delivery-missing-001
    genre: delivery
    severity: warning
    title: Undefined delivery state leaves trace expectations unclear
    summary: If status is omitted or invalid, absent traces become a warning rather than silent drift.
    description: |
      Sometimes a repository is partway through adoption and has not yet settled
      on a clean delivery state for every entry. In those cases `syu` still
      warns when no traces are declared so the gap remains visible. The warning
      preserves forward pressure toward explicit, reviewable delivery semantics.

  - code: SYU-delivery-agreement-001
    genre: delivery
    severity: warning
    title: Linked requirements and features should tell the same delivery story
    summary: Adjacent delivery states should not imply contradictory implementation progress.
    description: |
      Requirements and features are two views of the same delivery chain. A
      planned requirement that already links to implemented features, or an
      implemented feature that links only to planned requirements, makes the
      specification harder to trust because the adjacent layers no longer tell a
      coherent story about what is delivered. This rule keeps delivery-state
      intent reviewable without escalating immediately to a hard failure.

  - code: SYU-trace-language-001
    genre: trace
    severity: error
    title: Trace mappings must use a supported language adapter
    summary: Validation can only prove traces when it knows how to inspect that language.
    description: |
      A trace declaration is not self-validating. `syu` needs a language adapter
      to verify the file shape, symbol, and optional documentation snippets it
      refers to. This rule prevents repositories from gaining a false sense of
      security by declaring traces in languages the current validator cannot
      actually inspect.

  - code: SYU-trace-file-001
    genre: trace
    severity: error
    title: Trace mappings must declare a file path
    summary: A trace without an explicit file path cannot identify repository evidence.
    description: |
      Traceability depends on concrete, inspectable files. If a mapping omits the
      file path entirely, the validator has nowhere to look for evidence and
      reviewers cannot tell which artifact is supposed to satisfy the spec. This
      rule keeps every declared trace grounded in an explicit repository path.

  - code: SYU-trace-file-002
    genre: trace
    severity: error
    title: Declared trace files must exist
    summary: A trace that points to a missing file cannot prove anything about the repository.
    description: |
      Traceability depends on concrete, inspectable files. If a path is stale or
      points outside the current repository contents, the spec stops pointing to
      real evidence and starts pointing to an aspiration. This rule ensures that
      every declared trace remains grounded in a repository artifact that both
      reviewers and automation can actually inspect.

  - code: SYU-trace-file-003
    genre: trace
    severity: warning
    title: Trace file paths should use canonical repository-relative form
    summary: Portable path spelling keeps checked-in traces easier to review and reuse.
    description: |
      Trace paths become harder to review when they rely on redundant `./` or
      `..` segments, backslash separators, or other spellings that are not the
      repository's canonical relative form. This rule warns on path notation
      drift and suggests the normalized repository-relative path so the spec can
      stay portable without silently rewriting checked-in YAML.

  - code: SYU-trace-unreadable-001
    genre: trace
    severity: error
    title: Trace files must be readable to the validator
    summary: Evidence that cannot be read cannot be validated.
    description: |
      Repositories sometimes contain files with broken permissions, encoding
      issues, or transient access problems. A trace into such a file is still a
      broken trace because the validator cannot confirm the promised behavior.
      This rule exists to surface those operational failures instead of silently
      assuming the trace is probably fine.

  - code: SYU-trace-extension-001
    genre: trace
    severity: error
    title: The declared trace language must match the traced file
    summary: File extensions and language declarations should describe the same artifact.
    description: |
      Language adapters carry language-specific inspection logic. If a trace says
      "Rust" while pointing to a non-Rust file, the validator is forced into an
      invalid interpretation and reviewers lose confidence in what the trace
      means. This rule keeps the declared language and the referenced artifact in
      sync.

  - code: SYU-trace-id-001
    genre: trace
    severity: error
    title: Traced files must mention the owning specification ID
    summary: A trace should be explicit inside the implementation or test artifact itself.
    description: |
      `syu` is built around repository-native explainability. It is not enough
      for YAML to say a file belongs to a requirement or feature; the file should
      also mention that ownership so someone reading the code or test can see the
      link without consulting another system. This rule keeps traceability visible
      at the point where work actually happens.

  - code: SYU-trace-symbol-001
    genre: trace
    severity: error
    title: Trace mappings must identify at least one symbol
    summary: Validation needs a concrete symbol target instead of an empty mapping.
    description: |
      Files often contain many behaviors. If a trace mapping does not name any
      symbols at all, the validator cannot tell which implementation or test case
      is supposed to satisfy the requirement or feature. This rule keeps traces
      precise enough to support audits, refactors, and future maintenance.

  - code: SYU-trace-symbol-002
    genre: trace
    severity: error
    title: Trace symbol entries must not be blank
    summary: Whitespace-only symbol entries hide which repository behavior the trace intends to verify.
    description: |
      A symbol list that contains blank entries looks more complete than it is.
      Reviewers and tooling still lack a usable target, but the mapping no longer
      reads as obviously empty. This rule keeps symbol declarations explicit by
      requiring every listed entry to name a real symbol.

  - code: SYU-trace-symbol-003
    genre: trace
    severity: error
    title: Declared trace symbols must exist in the traced file
    summary: A trace can only prove ownership when the named symbol is present in the referenced file.
    description: |
      Symbol-level traces are useful because they point to the exact behavior a
      requirement or feature relies on. If the named symbol is missing, the spec
      no longer identifies real evidence and may be drifting behind refactors or
      stale documentation. This rule keeps symbol traces anchored to actual code
      and tests.

  - code: SYU-trace-duplicate-001
    genre: trace
    severity: error
    title: Trace lists must not repeat the same mapping
    summary: Duplicated trace records inflate evidence without adding new repository facts.
    description: |
      A trace list should enumerate distinct pieces of repository evidence.
      Repeating the exact same file, symbol, and documentation mapping inside
      one language list makes the specification look more complete than it is
      and makes review harder. This rule keeps trace evidence explicit by
      requiring each mapping to appear only once per list.

  - code: SYU-trace-inspection-001
    genre: trace
    severity: error
    title: Rich symbol inspection must succeed when required
    summary: Language-aware validation should fail loudly when the inspection pipeline breaks.
    description: |
      `syu` prefers real parsing and runtime-backed inspection over brittle text
      guesses. When those richer inspection paths fail, the safest behavior is to
      surface the failure immediately instead of falling back to a weaker,
      success-shaped assumption. This rule protects trust in the validator's
      conclusions.

  - code: SYU-trace-doc-001
    genre: trace
    severity: error
    title: Required trace documentation snippets must be present
    summary: Documentation assertions in trace mappings should be enforced, not advisory.
    description: |
      Some trace relationships need more than a symbol name - they need specific
      explanatory text to remain maintainable. This rule enforces those declared
      documentation snippets so that code, tests, and generated reports all
      preserve the rationale the spec asked for.

  - code: SYU-trace-docscope-001
    genre: trace
    severity: error
    title: Wildcard traces cannot carry symbol-level doc assertions
    summary: "`doc_contains` checks need a single symbol target instead of a wildcard file claim."
    description: |
      Wildcard ownership is useful when one specification item intentionally owns
      every relevant symbol in a file. Documentation assertions are different:
      they need one concrete symbol so the validator knows exactly which docs to
      inspect. This rule keeps doc-level checks precise instead of letting them
      ride on an ambiguous wildcard mapping.

  - code: SYU-trace-docsupport-001
    genre: trace
    severity: error
    title: Documentation assertions require rich symbol inspection
    summary: "`doc_contains` promises should only be made where the validator can inspect symbol docs."
    description: |
      A repository should not declare documentation constraints that the current
      adapter cannot verify. Otherwise the spec would appear stricter than the
      tool can honestly enforce. This rule preserves integrity by requiring
      doc-level checks to stay within languages with meaningful documentation
      inspection support.

  - code: SYU-coverage-walk-001
    genre: coverage
    severity: error
    title: Coverage inventory paths must be walkable
    summary: Strict trace coverage starts by discovering Rust sources under `src/` and `tests/`.
    description: |
      The strict trace coverage rule only means something when `syu` can walk the
      repository paths that are supposed to contain owned Rust source and test
      files. If directory discovery fails, the inventory itself is incomplete and
      any 100-percent coverage conclusion would be misleading. This rule surfaces
      repository layout problems before coverage claims become untrustworthy.

  - code: SYU-coverage-read-001
    genre: coverage
    severity: error
    title: Coverage inventory files must be readable
    summary: Unreadable Rust sources prevent honest trace coverage verification.
    description: |
      Trace coverage can only be enforced when `syu` can read the Rust source and
      test files it is supposed to inventory. If a file is unreadable, the safest
      behavior is to fail loudly instead of pretending the workspace was fully
      scanned. This rule protects the credibility of strict coverage mode.

  - code: SYU-coverage-parse-001
    genre: coverage
    severity: error
    title: Coverage inventory sources must parse successfully
    summary: Broken Rust syntax prevents reliable coverage inventory.
    description: |
      Strict trace coverage relies on understanding the Rust items that appear in
      the workspace. When a source file does not parse, `syu` cannot build a
      trustworthy inventory of public APIs or tests. This rule keeps coverage
      results explicit by surfacing syntax failures instead of hiding them behind
      incomplete scans.

  - code: SYU-coverage-public-001
    genre: coverage
    severity: error
    title: Public API symbols must belong to at least one feature
    summary: Public surface area should never appear in the repository without an owning feature.
    description: |
      Public functions, classes, modules, and similar API-facing symbols create
      long-term maintenance obligations. If they exist without a feature link,
      the repository has no durable explanation for why that surface area exists
      or which behavior owns it. This rule encourages deliberate API growth by
      forcing public surface area to stay attached to an explicit feature.

  - code: SYU-coverage-test-001
    genre: coverage
    severity: error
    title: Tests must belong to at least one requirement
    summary: Every test should prove some requirement instead of becoming orphaned automation.
    description: |
      Tests are executable claims about intended behavior. When a test has no
      requirement owner, it becomes much harder to tell whether it is obsolete,
      redundant, or still essential to the product. This rule keeps the test
      suite aligned with the specification by ensuring that each test remains
      justified by at least one requirement.
```
