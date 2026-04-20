# typescript-only example

This example demonstrates a minimal TypeScript-first workspace using the built-in
TypeScript trace adapter.

It contains one philosophy, one policy, one requirement, and one feature, plus
one `package.json`, one checked-in `.nvmrc`, one TypeScript source file, and one
Node-backed TypeScript test file. It keeps the traced symbol names explicit in
JSDoc, but the current starter only relies on symbol existence checks so the
first TypeScript workflow stays honest with what the validator enforces today.

This workspace matches the built-in `syu init --template typescript-only`
starter, so you can either inspect the checked-in example first or generate the
same shape directly in your own repository.

## Files

| Path | What it defines |
|------|-----------------|
| `docs/syu/philosophy/foundation.yaml` | `PHIL-TS-001` — the guiding principle |
| `docs/syu/policies/policies.yaml` | `POL-TS-001` linked to `PHIL-TS-001` |
| `docs/syu/requirements/core/typescript.yaml` | `REQ-TS-001` with a TypeScript test trace |
| `docs/syu/features/languages/typescript.yaml` | `FEAT-TS-001` with a TypeScript implementation trace |
| `.nvmrc` | Pins the starter to the checked-in Node 20 runtime |
| `package.json` | Minimal npm metadata, a checked-in Node engine, and a `npm test` script |
| `tsconfig.json` | TypeScript compiler settings for the starter files |
| `src/app.ts` | TypeScript source file containing `typescriptFeature` |
| `src/app.test.ts` | TypeScript test file containing `typescriptRequirementTest` |
| `README.md` | Explains what the TypeScript adapter validates today |

## Try it

```bash
cd examples/typescript-only
nvm use "$(cat .nvmrc)"
npm install
npm test
syu validate .
syu list requirement
syu show REQ-TS-001
syu app .
```

A successful `syu validate .` produces output similar to:

```text
syu validate passed
workspace: examples/typescript-only
definitions: philosophies=1 policies=1 requirements=1 features=1
traceability: requirements=1/1 traces validated; features=1/1 traces validated
```

## What the TypeScript adapter validates today

- `typescriptRequirementTest` lives in `src/app.test.ts` and is the validated
  test symbol for `REQ-TS-001`.
- `typescriptFeature` lives in `src/app.ts` and is the validated implementation
  symbol for `FEAT-TS-001`.
- The starter keeps the IDs visible in JSDoc comments even though this first
  template only depends on symbol existence checks during validation.

## Key things to notice

- **TypeScript files are traced directly** — the requirement and feature point
  at real `.ts` files instead of a lighter file-only fallback.
- **Runtime expectations are checked in** — `.nvmrc` and `package.json#engines`
  both point at Node 20 so the first `npm install` does not depend on shell
  guesswork.
- **JSDoc keeps the IDs obvious** — both traced symbols still carry the stable
  spec ID even though validation currently keys off the traced symbol names.
- **The example stays small** — there is no bundler or frontend framework yet,
  only the minimum Node + TypeScript project shape needed to prove the tracing
  flow.
