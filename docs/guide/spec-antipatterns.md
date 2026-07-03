# Spec anti-patterns

<!-- FEAT-DOCS-001 -->

Passing `syu validate .` is the floor, not the finish line. A workspace can
stay green while the four-layer design still drifts toward churn,
duplication, or vague ownership. This guide calls out common
**bad-but-valid** shapes and the refactors that usually help.

Use it when a spec technically validates but reviewers still ask questions
like:

- why is this philosophy changing every sprint?
- why does this policy just repeat the requirement text?
- why does this requirement read like an implementation note?
- why does one document keep absorbing unrelated work?

## 1. Philosophy that changes every sprint

Philosophy should protect values that survive multiple releases. If a
philosophy entry changes whenever the current delivery plan changes, it is
usually living in the wrong layer.

**Bad smell:** this conceptual sketch shows the problem title only; it is **not**
a full copy-pasteable `syu` document.

```text
- id: PHIL-CHECKOUT-001
  title: Move checkout onto GraphQL federation this quarter
```

That may be an important initiative, but it is too tied to one delivery
window and one implementation choice to anchor the whole spec.

**Refactor trigger:** change the item whenever a sprint plan, team structure,
tool choice, or current architecture changes.

**Usually better:** keep the philosophy at the durable value level, then push
the concrete rule or delivery choice down.

```text
- id: PHIL-CHECKOUT-001
  title: Checkout integrations should stay replaceable without rewriting the order flow
```

From there you can express the concrete expectations lower down:

- **policy**: contributors must keep checkout integrations behind stable
  boundaries
- **requirement**: checkout providers must be selected through a documented
  adapter
- **feature**: the current GraphQL or REST integration that satisfies that
  requirement

## 2. Policy that only repeats another layer

Policy should turn philosophy into repository-wide rules. It is the wrong
layer when it only restates a philosophy in stricter prose, or when it
duplicates one requirement or feature almost word for word.

**Bad smell:** this is a title-level sketch, not a full `syu` YAML document.

```text
- id: POL-AUTH-001
  title: Authentication should be secure
```

That does not tell contributors what they must do. It is still
philosophy-shaped.

Another failure mode goes the other direction:

```text
- id: POL-AUTH-002
  title: Login must hash passwords with bcrypt in src/auth/passwords.rs
```

Now the "policy" is really a requirement or an implementation note.

**Refactor trigger:** the policy has no obvious downstream requirements, or it
only exists to mirror one lower-layer item.

**Usually better:** write policy as a reusable rule that can govern several
requirements.

```text
- id: POL-AUTH-001
  title: Credentials must only be stored and compared using approved one-way password hashing
```

That policy can support several requirements and survive a later move away
from one file, crate, or library.

## 3. Requirements that should be split, promoted, or demoted

Requirements are where verification becomes concrete. A requirement is in
trouble when it mixes several obligations, carries repository-wide
governance, or names implementation details instead of a testable outcome.

### Split a requirement when one item hides multiple decisions

**Bad smell:** one requirement title contains several independent promises
joined by "and", or one requirement keeps accumulating exceptions and
sub-bullets.

```text
- id: REQ-CHECKOUT-001
  title: Checkout must calculate totals, reserve inventory, and send receipts
```

Those behaviors may ship separately, fail separately, and trace to different
tests. Split them when reviewers cannot answer "what proves this
requirement?" with one coherent test story.

### Promote a requirement when it is really a policy

**Bad smell:** the requirement tells every contributor how to work across the
repository instead of describing one concrete obligation.

```text
- id: REQ-CORE-001
  title: Every public API change must update user-facing documentation
```

That reads like a contributor rule. It likely belongs in **policy**, with
individual documentation requirements underneath it.

### Demote a requirement when it is really an implementation note

**Bad smell:** the requirement mainly names a file, symbol, library, endpoint
path, or framework mechanic.

```text
- id: REQ-AUTH-001
  title: Use `src/auth.rs` and Redis to track login attempts
```

If the real obligation is rate-limiting login attempts, keep that as the
requirement and let features plus implementation traces explain the current
Rust file or infrastructure choice.

## 4. Signals that a document or layer has become too broad

A spec can validate while still becoming hard to review. Watch for these
symptoms:

- one YAML document absorbs unrelated topics because it feels convenient
- an item links to nearly everything in adjacent layers
- titles keep falling back to catch-all names such as `core`, `misc`, or
  `other`
- unrelated teams keep touching the same file in parallel
- one requirement or feature owns several different delivery states and
  timelines

**Refactor trigger:** a reviewer needs extra explanation to understand why
those statements still belong together.

**Usually better:** split by stable concern or lifecycle. Good spec structure
makes it obvious why siblings live together, not just convenient for the
current week.

Some practical heuristics:

- split a **document** when its items stop sharing a clear topic or owner
- split an **item** when parts of it can be implemented, tested, or postponed
  independently
- keep a broad item only when the breadth is the real design choice, not
  accidental sprawl

## 5. When to merge, split, or rename spec items

Use these refactoring moves deliberately:

| Situation | Likely move | Why |
| --- | --- | --- |
| Two sibling items always change together, share the same links, and never tell different stories | **Merge** | Separate IDs add ceremony without adding clarity |
| One item has multiple behaviors, owners, or delivery states | **Split** | Independent work should not hide inside one status line |
| An item title still reflects an old tool, team name, or architecture choice | **Rename** | The title should explain current intent, not stale history |
| A requirement keeps growing repository-wide wording such as "every contributor" or "all changes" | **Promote** to policy | It is governing work, not one deliverable obligation |
| A philosophy or policy keeps naming files, symbols, frameworks, or sprint goals | **Demote** to requirement or feature | Higher layers should survive local implementation churn |

When you rename, preserve the layer meaning as well as the text. A rename
that only hides the smell without changing links or scope will come back
quickly.

## 6. Quick review checklist for a green-but-messy spec

Before accepting a spec item that validates, ask:

1. **Would this still read true after the next implementation rewrite?** If
   not, it is probably too high in the stack.
2. **Could more than one requirement reasonably satisfy this rule?** If yes,
   it may be policy rather than requirement.
3. **Can one coherent test or implementation story prove it?** If not, it may
   need splitting.
4. **Does the title describe intent or just the current mechanism?** Prefer
   intent in higher layers.
5. **Do adjacent links explain the design, or only satisfy validation?** Link
   because the relationship is real, not because the graph demanded a token
   edge.

A healthy four-layer spec does not only pass validation. It also makes future
change easier to reason about.

## Continue with these pages

- [syu concepts](./concepts.md) for the layer definitions and authoring basics
- [Getting started](./getting-started.md) to scaffold a workspace and practice
  the four-layer flow
- [Troubleshooting](./troubleshooting.md) when validation is failing, not just
  the structure
