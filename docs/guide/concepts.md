# syu concepts

<!-- FEAT-DOCS-001 -->

`syu` uses four specification layers on purpose. They answer different
questions, and mixing them together makes both design and validation weaker.

The model is guided by three product ideas:

- stay involved through implementation and maintenance, not only early design
- fit repositories regardless of their primary programming language
- stay simple enough to adopt without resenting the tool

## Philosophy

Philosophy describes the ideal state a project is trying to protect.

It is intentionally high-level. A philosophy entry should explain:

- what kind of system you want to build
- what values must survive implementation details
- what trade-offs the project prefers

Good philosophy is stable. It should change less often than code, features, or
individual requirements, and it should remain relevant after the project moves
from initial design into maintenance.

## Policy

Policy turns philosophy into rules.

If philosophy says *what kind of project this should be*, policy says *what
contributors must do to make that true*. Policies are more concrete than
philosophy, but they still speak in repository-wide rules rather than single
features.

Examples:

- every requirement must link to at least one feature
- every traced symbol must carry a stable ID in its documentation
- every isolated philosophy / policy / requirement / feature should be treated as drift
- generated reports must be readable by both humans and automation

## Requirements

Requirements are the concrete obligations that satisfy policy.

Each requirement should be specific enough that you can:

- tell whether it is implemented
- trace it to tests
- explain which policy it satisfies

Requirements also carry delivery intent:

- `status: planned` means the requirement is accepted but its test traces should
  not exist yet
- `status: implemented` means the requirement must already declare valid tests

Requirements are where verification begins to become operational.
They are also where you decide how much discipline the repository wants: some
checks can stay optional until the project is ready for them.

## Features

Features are the implemented capabilities that satisfy requirements.

Features should describe *what the system now does*, not just *what file was
edited*. A feature should link directly back to the requirements it satisfies
and forward to the concrete implementation symbols that prove it exists.

Like requirements, features also carry delivery intent:

- `status: planned` means implementation traces are intentionally absent
- `status: implemented` means implementation traces must be present and valid

## Why define all four?

Without philosophy, the project loses its values.

Without policy, philosophy becomes decorative.

Without requirements, policy cannot be verified concretely.

Without features, requirements never connect to running software.

`syu` keeps all four layers explicit because traceability is strongest when the
repository can explain itself from ideals down to code and tests without being
tied to a single implementation language.

## Authoring guidelines

### Write philosophy for stability

Philosophy should survive multiple releases. Avoid implementation details and
tooling trivia there.

### Write policy as enforceable rules

A policy should imply at least one checkable requirement. If nobody could tell
whether the policy is being followed, it is still too vague.

### Write requirements for verification

A requirement should be concrete enough that you can point to tests and say,
\"these prove it.\"

### Write features for implementation traceability

A feature should map to symbols, commands, or files that clearly implement the
behavior. If the link is too fuzzy to validate, tighten it.

## Continue with these pages

- [Getting started](./getting-started.md) to scaffold a workspace and run the CLI
- [Configuration](./configuration.md) to tune validation and runtime behavior
- [Specification Reference](../generated/site-spec/index.md) to inspect the self-hosted repository contract
- [Latest validation report](../generated/syu-report.md) to see the current repository state
