# python-only example

This example demonstrates a minimal single-language Python workspace.
It contains one philosophy, one policy, one requirement, and one feature —
all mutually linked and traced to a Python test file.

It is the checked-in reference workspace that matches
`syu init . --template python-only` exactly.

It is also the best **starter-level `doc_contains` example** in this repository:
the workspace stays small enough for a newcomer to read end-to-end while still
showing both a requirement trace and a feature trace with real Python
docstrings. If you are adopting `doc_contains` for the first time, pair this
example with the [dedicated adoption guide](https://github.com/ugoite/syu/blob/main/docs/guide/doc-contains.md).

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-PY-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-PY-001` linked to `PHIL-PY-001` |
| `docs/syu/requirements/core/python.yaml` | `REQ-PY-001` with a test trace to `python/test_traceability.py` |
| `docs/syu/features/languages/python.yaml` | `FEAT-PY-001` with an implementation trace to `python/app.py` |
| `python/test_traceability.py` | Python test file containing the traced test function |
| `python/app.py` | Python source containing the traced implementation function |

## Try it

```bash
cd examples/python-only
syu validate .
syu list requirement
syu show REQ-PY-001
syu app .
```

A successful `syu validate .` produces output similar to:

```
syu validate passed
workspace: examples/python-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=2/2 features=2/2
```

## Key things to notice

- **Python tracing** — syu parses Python source files to verify that the
  named symbols (functions and classes) exist and contain the required
  `doc_contains` strings in their docstrings.
- **Docstring format** — `doc_contains: ["requirement doc line"]` means the
  string `"requirement doc line"` must appear somewhere in the function's
  docstring. Open `python/test_traceability.py` to see a working example.
- **Best first copy target** — if your repository already has Python tests or
  scripts, this is the smallest example to copy before you widen `doc_contains`
  usage across more files.
- **Reciprocal links** — `REQ-PY-001` and `FEAT-PY-001` must each reference
  the other; syu checks both directions.
