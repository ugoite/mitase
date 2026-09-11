# CLI machine contract

This is the 0.1.x machine-readable contract for the normal specification
commands. It fixes the current behavior for integrations and CI; it does not
add a compatibility parser or a second semantic implementation.

## JSON success shapes

Pass `--format json` to `check`, `validate workspace`, `validate change`,
`query`, `show`, or `list`. A successful command writes exactly one JSON value
to stdout and does not write diagnostics to stderr.

| Command | Top-level JSON shape |
| --- | --- |
| `check`, `validate ...` | `ValidationResult`: `diagnostics` plus optional `readiness` |
| `query` | `{ source, relations }` |
| `show` | `{ id, kind, title, summary, description, status, source, anchors, criteria, authored_relations, derived_relations, bindings, verification_claims }` |
| `list` | `{ items, unverified_criteria }` |

The read models use the canonical specification identifiers and exact relation
references. Their arrays are deterministic for the same workspace and command
arguments.

## Diagnostic JSON

Every item in a `diagnostics` array is the shared `mitase-diagnostics` value:

```json
{
  "code": "MITASE-TARGET-002",
  "phase": "targets",
  "severity": "error",
  "reason": "target resolution is ambiguous",
  "primary": {
    "path": "spec/feature.yaml",
    "line": 12,
    "column": 5,
    "end_line": 12,
    "end_column": 19,
    "label": null
  },
  "related_spans": [],
  "subject": null,
  "reference": null,
  "candidates": [],
  "evidence": [],
  "suggested_action": null,
  "fix": null
}
```

`primary` and the other optional structured fields remain present in the
serialized contract with `null` or empty-array values where the model defines
defaults. `fix` is descriptive only; Mitase never applies it.

## Exit codes

| Code | Meaning | Machine-readable output |
| --- | --- | --- |
| `0` | The command completed and its result is valid or successfully inspected. | Result on stdout. |
| `1` | The workspace loaded, but validation/readiness failed, or loading produced a structured frontend diagnostic. | `ValidationResult` on stdout for JSON; text diagnostics follow the command's text rendering. |
| `2` | A top-level error occurred: invalid arguments, an unreadable workspace, an unknown query/show source, or another command failure. | No JSON error envelope; `error: ...` is written to stderr. |

Integrations must use the exit code rather than infer failure from diagnostic
severity alone. Informational and warning diagnostics can be present in a
successful `ValidationResult`.

## stdout and stderr

- JSON results and structured diagnostics belong to stdout.
- Human-readable validation output belongs to stdout when validation runs and
  produces a result.
- Top-level errors belong to stderr and leave stdout empty.
- The CLI does not execute runners, modify files, or emit a machine-readable
  error envelope for exit code `2`.

The contract applies to the current 0.1.x dual-source period. The 0.2.x
authoring cutover changes accepted source policy, not these output and exit
code responsibilities.
