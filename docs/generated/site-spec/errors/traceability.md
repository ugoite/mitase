---
title: "Traceability"
description: "Generated reference for docs/syu/errors/traceability.yaml"
---

> Generated from `docs/syu/errors/traceability.yaml`.

## Parsed content

### Genre

- Traceability

### Version

- 1

### Rules

- **code**: unsupported-language
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
- **code**: trace-file-missing
  - **severity**: error
  - **title**: Trace files must exist and be named explicitly
  - **summary**: A trace without a real file cannot prove anything about the repository.
  - **description**:
    - |
      Traceability depends on concrete, inspectable files. If a path is absent,
      blank, or stale, the spec stops pointing to real evidence and starts
      pointing to an aspiration. This rule ensures that every declared trace is
      grounded in a repository artifact that reviewers and automation can both
      inspect.
- **code**: trace-file-unreadable
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
- **code**: extension-mismatch
  - **severity**: error
  - **title**: The declared trace language must match the traced file
  - **summary**: File extensions and language declarations should describe the same artifact.
  - **description**:
    - |
      Language adapters carry language-specific inspection logic. If a trace says
      “Rust” while pointing to a non-Rust file, the validator is forced into an
      invalid interpretation and reviewers lose confidence in what the trace
      means. This rule keeps the declared language and the referenced artifact in
      sync.
- **code**: trace-id-missing
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
- **code**: trace-symbol-missing
  - **severity**: error
  - **title**: Trace mappings must identify concrete symbols
  - **summary**: Validation needs a real symbol to inspect instead of a vague file-level hint.
  - **description**:
    - |
      Files often contain many behaviors. Without a symbol target, a trace may
      point at a broad area of the repository while still leaving uncertainty
      about what actually satisfies the requirement or feature. This rule keeps
      traces precise enough to support audits, refactors, and future maintenance.
- **code**: trace-inspection-failed
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
- **code**: trace-doc-missing
  - **severity**: error
  - **title**: Required trace documentation snippets must be present
  - **summary**: Documentation assertions in trace mappings should be enforced, not advisory.
  - **description**:
    - |
      Some trace relationships need more than a symbol name—they need specific
      explanatory text to remain maintainable. This rule enforces those declared
      documentation snippets so that code, tests, and generated reports all
      preserve the rationale the spec asked for.
- **code**: trace-doc-unsupported
  - **severity**: error
  - **title**: Documentation assertions need a language that can verify them
  - **summary**: `doc_contains` promises should only be made where the validator can inspect docs.
  - **description**:
    - |
      A repository should not declare documentation constraints that the current
      adapter cannot verify. Otherwise the spec would appear stricter than the
      tool can honestly enforce. This rule preserves integrity by requiring
      doc-level checks to stay within languages with meaningful documentation
      inspection support.

## Source YAML

```yaml
genre: Traceability
version: 1

rules:
  - code: unsupported-language
    severity: error
    title: Trace mappings must use a supported language adapter
    summary: Validation can only prove traces when it knows how to inspect that language.
    description: |
      A trace declaration is not self-validating. `syu` needs a language adapter
      to verify the file shape, symbol, and optional documentation snippets it
      refers to. This rule prevents repositories from gaining a false sense of
      security by declaring traces in languages the current validator cannot
      actually inspect.

  - code: trace-file-missing
    severity: error
    title: Trace files must exist and be named explicitly
    summary: A trace without a real file cannot prove anything about the repository.
    description: |
      Traceability depends on concrete, inspectable files. If a path is absent,
      blank, or stale, the spec stops pointing to real evidence and starts
      pointing to an aspiration. This rule ensures that every declared trace is
      grounded in a repository artifact that reviewers and automation can both
      inspect.

  - code: trace-file-unreadable
    severity: error
    title: Trace files must be readable to the validator
    summary: Evidence that cannot be read cannot be validated.
    description: |
      Repositories sometimes contain files with broken permissions, encoding
      issues, or transient access problems. A trace into such a file is still a
      broken trace because the validator cannot confirm the promised behavior.
      This rule exists to surface those operational failures instead of silently
      assuming the trace is probably fine.

  - code: extension-mismatch
    severity: error
    title: The declared trace language must match the traced file
    summary: File extensions and language declarations should describe the same artifact.
    description: |
      Language adapters carry language-specific inspection logic. If a trace says
      “Rust” while pointing to a non-Rust file, the validator is forced into an
      invalid interpretation and reviewers lose confidence in what the trace
      means. This rule keeps the declared language and the referenced artifact in
      sync.

  - code: trace-id-missing
    severity: error
    title: Traced files must mention the owning specification ID
    summary: A trace should be explicit inside the implementation or test artifact itself.
    description: |
      `syu` is built around repository-native explainability. It is not enough
      for YAML to say a file belongs to a requirement or feature; the file should
      also mention that ownership so someone reading the code or test can see the
      link without consulting another system. This rule keeps traceability visible
      at the point where work actually happens.

  - code: trace-symbol-missing
    severity: error
    title: Trace mappings must identify concrete symbols
    summary: Validation needs a real symbol to inspect instead of a vague file-level hint.
    description: |
      Files often contain many behaviors. Without a symbol target, a trace may
      point at a broad area of the repository while still leaving uncertainty
      about what actually satisfies the requirement or feature. This rule keeps
      traces precise enough to support audits, refactors, and future maintenance.

  - code: trace-inspection-failed
    severity: error
    title: Rich symbol inspection must succeed when required
    summary: Language-aware validation should fail loudly when the inspection pipeline breaks.
    description: |
      `syu` prefers real parsing and runtime-backed inspection over brittle text
      guesses. When those richer inspection paths fail, the safest behavior is to
      surface the failure immediately instead of falling back to a weaker,
      success-shaped assumption. This rule protects trust in the validator's
      conclusions.

  - code: trace-doc-missing
    severity: error
    title: Required trace documentation snippets must be present
    summary: Documentation assertions in trace mappings should be enforced, not advisory.
    description: |
      Some trace relationships need more than a symbol name—they need specific
      explanatory text to remain maintainable. This rule enforces those declared
      documentation snippets so that code, tests, and generated reports all
      preserve the rationale the spec asked for.

  - code: trace-doc-unsupported
    severity: error
    title: Documentation assertions need a language that can verify them
    summary: "`doc_contains` promises should only be made where the validator can inspect docs."
    description: |
      A repository should not declare documentation constraints that the current
      adapter cannot verify. Otherwise the spec would appear stricter than the
      tool can honestly enforce. This rule preserves integrity by requiring
      doc-level checks to stay within languages with meaningful documentation
      inspection support.
```
