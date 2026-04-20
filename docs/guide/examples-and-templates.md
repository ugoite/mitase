# Examples and templates: which one should you start from?

<!-- FEAT-DOCS-001 -->

`syu` ships two different newcomer aids:

- **starter templates** for `syu init --template ...`, which generate a fresh
  scaffold directly in your repository
- **checked-in examples** under `examples/`, which are working reference
  workspaces you can study, validate, or copy from selectively

Use this page when you know you want help getting started, but you are not sure
whether you should scaffold a template or open one of the repository examples.

## Quick rule of thumb

| If you want to... | Start with... | Why |
| --- | --- | --- |
| create a new spec tree inside your own repository right now | `syu init` + a starter template | the CLI writes `syu.yaml`, the starter spec files, and the feature registry for you |
| inspect a complete, already-linked workspace before touching your own repo | a checked-in example under `examples/` | you can browse a full working shape without committing to a scaffold yet |
| copy ideas from a similar language mix, but keep your repository layout and naming | an example first, then `syu init` if it still fits | examples show the whole repository story, not just the first generated files |
| stay as close as possible to the default first-run flow | `syu init` without a template | this keeps the scaffold generic and minimal |

## Matrix: what exists today

| Path | Type | Best for | How to start |
| --- | --- | --- | --- |
| `csharp-fallback` | Example only | C#-heavy repositories that want a staged adoption path with spec-first traces before they trace every C# file directly | `examples/csharp-fallback` |
| `docs-first` | Template + example | documentation-heavy repositories that want starter markdown acceptance anchors, one shell trace, and one wildcard-owned YAML file | `syu init . --template docs-first` or `examples/docs-first` |
| `generic` | Template only | the shortest neutral scaffold when you do not want language-specific starter copy yet | `syu init .` |
| `go-only` | Template + example | Go-first repositories that want starter IDs, file names, a minimal `go.mod`, and small Go source/test files from the first scaffold | `syu init . --template go-only` or `examples/go-only` |
| `java-only` | Template + example | Java-first repositories that want starter IDs, file names, a minimal `pom.xml`, and small Java source/test files from the first scaffold | `syu init . --template java-only` or `examples/java-only` |
| `typescript-only` | Template + example | TypeScript-first repositories that want starter IDs, file names, a minimal `package.json`, checked-in Node runtime metadata, `tsconfig.json`, and small TypeScript source/test files from the first scaffold | `syu init . --template typescript-only` or `examples/typescript-only` |
| `rust-only` | Template + example | Rust-first repositories that want starter IDs, file names, and copy tuned for Rust work | `syu init . --template rust-only` or `examples/rust-only` |
| `python-only` | Template + example | Python-first repositories that want the same tuned starter shape for Python workflows | `syu init . --template python-only` or `examples/python-only` |
| `polyglot` | Template + example | repositories that already expect multiple languages and want the starter text to say that from the first commit | `syu init . --template polyglot` or `examples/polyglot` |
| `team-scale` | Example only | repositories that already outgrew the single-feature starter shape and want to inspect a larger split-by-area workspace | `examples/team-scale` |

## What templates give you

Templates are for **scaffolding**:

- they create `syu.yaml`
- they create starter philosophy, policy, requirement, and feature documents
- they create `features/features.yaml`
- they keep the default newcomer flow inside the CLI

Choose a template when you want to start writing in your own repository
immediately and the built-in starter shape is already close enough.

## What examples give you

Examples are for **reference and comparison**:

- they show a complete checked-in workspace instead of only the generated first
  files
- they are validated in this repository's automated test suite
- they let you inspect naming, links, traces, and layout before you scaffold
  anything locally

Choose an example when you want to study a working repository story first, or
when you expect to copy only parts of the shape rather than accept a starter
scaffold wholesale.

## Common starting paths

1. **Fastest path into your own repository**: run `syu init .`, then switch to a
   language-specific template only if the generic starter feels too abstract.
2. **I already know the repo is docs-first / Rust-first / Python-first / Go-first / Java-first / TypeScript-first / polyglot**: start
   with the matching template, then compare against the matching example when
   you want a fuller repository story.
3. **I want a staged C# adoption path instead of tracing every C# file on day one**:
   open `examples/csharp-fallback` first to study the lighter pattern before
   you turn strict C# ownership on everywhere.
4. **I am Go-first and want to inspect native Go tracing before I scaffold
   anything**: open `examples/go-only` first, then copy the shape you need into
   your own repository.
5. **I am Java-first and want to inspect native Java tracing before I scaffold
   anything**: open `examples/java-only` first, then copy the shape you need into
   your own repository.
6. **I am TypeScript-first and want to inspect native TypeScript tracing before I scaffold
   anything**: open `examples/typescript-only` first, then copy the shape you need into
   your own repository.
7. **I am still deciding whether `syu` fits my repo**: read the example first so
   you can inspect a working shape without creating files locally yet.

## Continue with these pages

- [Getting started](./getting-started.md) for the first-run CLI flow
- [Tutorial](./tutorial.md) for a longer, narrated repository story
- [Configuration](./configuration.md) when you want to tune `syu.yaml` after the
  first scaffold
