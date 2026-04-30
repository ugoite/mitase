# Adopting `doc_contains` without overwhelming a new team

<!-- FEAT-DOCS-001 -->

`doc_contains` is the lightest way to say "this symbol should keep carrying this
piece of intent in its own documentation comment." It sits between plain symbol
ownership and a heavier custom review checklist:

- **plain `file` + `symbols`** proves that the traced symbol exists
- **`doc_contains`** also proves that the symbol's nearby docs still mention the
  requirement or feature wording you care about

Use `doc_contains` when reviewers should be able to open one symbol and quickly
see the requirement-facing phrase, warning, or behavior promise that justified
the trace in the first place.

## When `doc_contains` is a better fit than symbol-only traces

Stay with plain symbol traces when you only need ownership:

- the symbol name alone is already enough to understand the mapping
- the file is internal plumbing and does not keep user-facing promises in
  comments or docstrings
- the language adapter cannot inspect docs for that language yet

Prefer `doc_contains` when the docs next to the symbol matter:

- an API or UI behavior should keep one exact phrase, warning, or invariant
- a requirement is easy to lose during refactors unless it stays in the symbol's
  doc comment
- reviewers want the trace to point at evidence they can read immediately,
  instead of only at the symbol name

If you are unsure, start with plain symbol traces and add `doc_contains` only to
the few symbols where comment-level evidence genuinely helps.

## Languages that support `doc_contains` today

`syu` currently validates `doc_contains` in:

- **Rust**
- **Python**
- **Go**
- **TypeScript / JavaScript**

For the full matrix, including strict ownership coverage and symbol-only
adapters, use the [trace adapter capability matrix](./trace-adapter-support.md).

Languages such as Java, C#, Ruby, Shell, YAML, JSON, Markdown, and Gitignore
can still use explicit `file` + `symbols` traces, but they do not validate
`doc_contains` yet.

## Starter-level examples to copy from

Use an existing checked-in example instead of inventing your first shape from
scratch:

| Example | Why start here |
| --- | --- |
| [`examples/python-only`](https://github.com/ugoite/syu/tree/main/examples/python-only) | Smallest newcomer-friendly `doc_contains` story with Python docstrings in both tests and implementation. |
| [`examples/rust-only`](https://github.com/ugoite/syu/tree/main/examples/rust-only) | Minimal Rust `///` comment example when your repository is Rust-first. |
| [`examples/browser-ui`](https://github.com/ugoite/syu/tree/main/examples/browser-ui) | Frontend-oriented TypeScript example showing `doc_contains` in a UI component flow. |

If you want the gentlest first adoption path, start with
`examples/python-only`. It keeps the repository shape tiny while still showing
the full loop:

1. one requirement trace
2. one feature trace
3. one concrete `doc_contains` string per symbol
4. one successful `syu validate .` run

## Add it one file at a time

Do not try to retrofit `doc_contains` across the whole repository in one pass.
The safest adoption loop is:

1. pick one requirement or feature that already has a stable owner
2. keep the existing `file` + `symbols` mapping working first
3. add a short doc comment or docstring to that one symbol
4. add one `doc_contains` string that matches the wording you want to keep
5. run `syu validate .`

Example:

```yaml
implementations:
  python:
    - file: python/app.py
      symbols:
        - explain_payment_status
      doc_contains:
        - returns a user-facing payment status summary
```

Then keep the source comment equally small:

```python
def explain_payment_status() -> str:
    """returns a user-facing payment status summary"""
    return "paid"
```

That pattern keeps the trace readable and makes review failures obvious: either
the symbol disappeared, or the wording drifted.

## How it interacts with wildcard ownership

`doc_contains` and wildcard ownership solve different problems:

- `symbols: ["*"]` says one item intentionally owns the whole file
- `doc_contains` says one specific symbol's docs must include certain wording

Because wildcard ownership does not point at one inspectable symbol, it cannot
use `doc_contains`. If you need comment-level evidence, switch that trace to an
explicit symbol list first.

Use wildcard ownership when the file is deliberately a single unit. Use
`doc_contains` when reviewers need evidence attached to one named symbol.

## How it fits with starter templates

Starter templates help you choose the repository shape. `doc_contains` helps you
raise the fidelity of a few important traces inside that shape.

- `syu init . --template python-only` gives you the easiest checked-in example to
  study before adding more traces
- `syu init . --template rust-only` is the smallest Rust-first reference
- `syu init . --template typescript-only` or `examples/browser-ui` give a
  TypeScript-oriented pattern when frontend or Node code is the first adoption
  target

You do **not** need to enable `doc_contains` everywhere just because a starter
template supports it. Start with the one or two symbols where the comment text
is part of the real promise you want to preserve.

## A good first rollout

For a newcomer team, a pragmatic first rollout looks like this:

1. scaffold with the closest starter template
2. keep `validate.require_symbol_trace_coverage` at its current comfortable
   level
3. add `doc_contains` to one requirement trace and one feature trace
4. let review feedback tell you whether the extra comment-level evidence is
   useful before expanding further

That keeps `syu` aligned with its "small steps, explicit links" philosophy
instead of turning the first adoption into a repository-wide documentation
rewrite.
