# Trace adapter capability matrix

When you choose a `tests:` or `implementations:` language key, three validator
capabilities matter most:

- **symbol existence validation** — whether `syu validate` can confirm that the
  declared symbol is really present in the traced file
- **`doc_contains` validation** — whether `syu` can inspect symbol
  documentation comments and enforce required snippets
- **strict `validate.require_symbol_trace_coverage` inventory** — whether `syu`
  can scan the repository for unowned public APIs and tests in that language

This page summarizes the built-in trace adapters that ship with this checked-in
version of `syu`. Use it before you turn on `doc_contains` checks widely or
enable strict coverage in a mixed-language repository.

## Built-in adapter matrix

| Built-in adapter | Accepted `lang:` aliases / files | Symbol existence validation | `doc_contains` validation | Strict `require_symbol_trace_coverage` inventory |
| --- | --- | --- | --- | --- |
| Rust | `rust`, `rs` / `.rs` | ✅ Rich symbol inspection plus declaration matching | ✅ | ✅ |
| Python | `python`, `py`, `pytest`, `unittest` / `.py` | ✅ Rich symbol inspection plus pattern fallback | ✅ | ✅ |
| Go | `go`, `golang`, `gotest` / `.go` | ✅ Rich doc-comment inspection plus pattern fallback | ✅ | ✅ |
| Java | `java`, `junit` / `.java` | ✅ Pattern-based symbol matching | ❌ | ✅ |
| TypeScript / JavaScript | `typescript`, `ts`, `tsx`, `javascript`, `js`, `jsx`, `vitest`, `bun`, `bun-test` / `.ts`, `.tsx`, `.js`, `.jsx` | ✅ Rich symbol inspection plus pattern fallback | ✅ | ✅ |
| Shell | `shell`, `sh`, `bash`, `zsh` / `.sh`, `.bash`, `.zsh` | ✅ Pattern-based symbol matching | ❌ | ❌ |
| YAML | `yaml`, `yml` / `.yaml`, `.yml` | ✅ Pattern-based symbol matching | ❌ | ❌ |
| JSON | `json` / `.json` | ✅ Pattern-based symbol matching | ❌ | ❌ |
| Markdown | `markdown`, `md` / `.md` | ✅ Pattern-based symbol matching | ❌ | ❌ |
| Gitignore | `gitignore`, `ignore` / `.gitignore` | ✅ Filename-aware, pattern-based matching | ❌ | ❌ |

## What the strict inventory scans

When `validate.require_symbol_trace_coverage: true` is enabled, `syu` currently
builds ownership inventories for these languages only:

- **Rust** — public items in `src/`, plus `#[test]` functions in `src/` and
  `tests/`
- **Python** — public names in `src/` that do not start with `_`, plus
  `test_...` and `Test...` symbols in `tests/`
- **Go** — exported identifiers in `src/`, plus `Test...`, `Benchmark...`,
  `Fuzz...`, and `Example...` symbols in `_test.go` files
- **Java** — public classes/interfaces/enums/records plus public or implicit
  interface members in `src/`, plus JUnit `@Test` methods and legacy `test...`
  methods in `tests/`
- **TypeScript / JavaScript** — exported symbols in `src/`, plus `test...` and
  `Test...` symbols in `tests/`

Other built-in adapters still validate the traces you declare explicitly, but
they do **not** participate in the repository-wide strict ownership scan.

## How to choose the right promises

- Need `doc_contains`? Rust, Python, Go, and TypeScript / JavaScript traces all support it today.
- Need strict ownership coverage? Rust, Python, Go, Java, and
  TypeScript / JavaScript all participate today.
- Using Shell, YAML, JSON, Markdown, or Gitignore traces? Keep the mapping to
  `file` + `symbols` (or `symbols: ["*"]` when one file intentionally belongs
  to one item), but do not expect doc-comment inspection or strict ownership
  inventory.
- Want a runnable reference for that lighter mapping? Start with the
  [`examples/docs-first` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/docs-first),
  which demonstrates a shell symbol trace, markdown-backed requirement
  evidence, and cautious wildcard YAML ownership in one small workspace.

Even in rich-inspection languages, wildcard traces cannot use `doc_contains`
because `symbols: ["*"]` does not point to one inspectable symbol.

## What about unsupported languages?

Unsupported adapters such as `csharp` still raise `SYU-trace-language-001`.
Keep those repositories connected through the spec layers first, and only add
language-specific code traces once adapter support lands.
If you need a Go-first starting point today, study the
[`examples/go-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/go-only)
or scaffold `syu init . --template go-only`. Both keep real Go files in the
repository while validating explicit symbol mappings, and Go traces can now add
`doc_contains` when reviewers want comment-level evidence too.
If you need a Java-first starting point today, study the
[`examples/java-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/java-only)
or scaffold `syu init . --template java-only`. Both keep real Java files in the
repository while validating explicit symbol mappings, but Java traces should
still stay with `file` plus `symbols` because `doc_contains` is not supported yet.
If you need a concrete fallback shape today, study the
[`examples/csharp-fallback` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/csharp-fallback).
It keeps real C# files in the repository while validating supported shell and
markdown evidence around them instead of inventing unsupported `csharp:` trace
keys.
