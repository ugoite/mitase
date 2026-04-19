# java-only example

This example demonstrates a minimal Java-first workspace using the built-in
Java trace adapter.

It contains one philosophy, one policy, one requirement, and one feature, plus
one `pom.xml`, one Java source file, and one Java test file. The example uses
pattern-based symbol matching for the real `.java` files, so `syu validate .`
proves the Java-backed links directly.

This workspace now matches the built-in `syu init --template java-only`
starter, so you can either inspect the checked-in example first or generate the
same shape directly in your own repository.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-JAVA-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-JAVA-001` linked to `PHIL-JAVA-001` |
| `docs/syu/requirements/core/java.yaml` | `REQ-JAVA-001` with a Java test trace |
| `docs/syu/features/languages/java.yaml` | `FEAT-JAVA-001` with a Java implementation trace |
| `pom.xml` | Minimal Maven metadata for the checked-in Java files |
| `src/main/java/example/app/OrderSummary.java` | Java source file containing `JavaFeatureImpl` |
| `src/test/java/example/app/OrderSummaryTest.java` | Java test file containing `JavaRequirementTest` |
| `README.md` | Explains what the Java adapter validates today |

## Try it

```bash
cd examples/java-only
syu validate .
syu list requirement
syu show REQ-JAVA-001
syu app .
```

A successful `syu validate .` produces output similar to:

```text
syu validate passed
workspace: examples/java-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 traces validated; features=1/1 traces validated
```

## What the Java adapter validates today

- `JavaRequirementTest` lives in
  `src/test/java/example/app/OrderSummaryTest.java` and is the validated test
  symbol for `REQ-JAVA-001`.
- `JavaFeatureImpl` lives in `src/main/java/example/app/OrderSummary.java` and
  is the validated implementation symbol for `FEAT-JAVA-001`.
- The Java adapter currently supports pattern-based symbol validation and
  strict ownership coverage, but not `doc_contains` checks.

## Key things to notice

- **Java files are traced directly** — the requirement and feature point at the
  real `.java` files instead of a markdown workaround.
- **The example stays small** — one source file and one test file are enough to
  demonstrate Java-backed requirement and feature ownership.
- **`doc_contains` is still out of scope** — keep Java traces to `file` plus
  `symbols` today, then add richer evidence only after adapter support grows.
