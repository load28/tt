# TASK-359: Preserve type-service failures across compiler and editor boundaries

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-359: fix(types): preserve type-service failures`

## Purpose

Audit type inference and type-information delivery through the CLI, compiler,
and editor under production project conditions, then repair reproduced failures
at the architectural boundary that owns them.

## Scope

- Included: TypeScript language-service outcomes, typed engine protocol results,
  editor semantic requests, and regression coverage for representative project
  and type-inference workflows
- Excluded: New language syntax and unrelated developer-workflow changes already
  tracked by TASK-358

## Decisions

### Decision 1: Keep an empty semantic answer distinct from a failed request

- **Context**: The TypeScript service currently turns protocol errors and request
  timeouts into JSON null, so callers cannot distinguish a valid empty hover or
  completion from a checker failure.
- **Alternatives considered**: Preserve the existing best-effort null behavior;
  add editor-only diagnostics; or preserve the error at the TypeScript adapter
  seam and let the existing engine protocol carry it.
- **Decision and rationale**: Preserve failures at the adapter seam. The engine
  already models semantic calls as `Result`, and its JSON-lines protocol already
  has a separate error variant, so this keeps one failure contract through every
  consumer without duplicating TypeScript-specific logic in the editor.

### Decision 2: Resolve implicit configuration from every input

- **Context**: A multi-package CLI invocation must choose one project identity.
  The documented rule selects the nearest configuration above the inputs'
  common directory, but the implementation searched upward from only the first
  collected file.
- **Alternatives considered**: Require `--project` for every monorepo command;
  open the first package and silently exclude the rest; or compute the common
  ancestor before applying the existing nearest-config rule.
- **Decision and rationale**: Compute the common ancestor first. A one-file
  editor project still selects its nearest package configuration, while a CLI
  invocation spanning packages selects the shared root configuration and keeps
  every requested package in the TypeScript program.

## Work log

- 2026-09-10: Ran `./scripts/doctor`; the repository and pinned toolchains were
  ready.
- 2026-09-10: Audited contextual type materialization, the project engine, the
  TypeScript LSP adapter, the JSON-lines server, and the VS Code engine client.
- 2026-09-10: Confirmed that `src/typescript/service.rs` erases TypeScript LSP
  errors and timeouts even though every layer above it can represent an error.
- 2026-09-10: Separated successful null, protocol error, timeout, and disconnect
  outcomes. A timeout now retires the stalled service so the next request can
  create a new conversation.
- 2026-09-10: Reproduced a multi-package check that selected the first package's
  `tsconfig.json` and omitted a type error in the second package; changed
  implicit configuration discovery to begin at the inputs' common ancestor.
- 2026-09-10: Ran the 66-case native TypeScript backend suite, the 34-case
  contextual typing suite, and all 161 VS Code server/client tests.
- 2026-09-10: Ran the complete local CI gate successfully outside the restricted
  sandbox after the sandbox denied registry and loopback access.

## Issues and resolutions

### Issue 1: Type-service failures appear as valid empty results

- **Symptom**: Hover, completion, definition, signature help, and related typed
  features silently return no result when tsgo answers with an LSP error or does
  not answer before the request deadline.
- **Cause**: `Service::request` intentionally maps both outcomes to JSON null.
- **Resolution**: Preserved protocol errors as engine errors, preserved genuine
  null as an empty result, and classified timeouts separately so they both
  surface and retire the stalled service.

### Issue 2: A multi-package CLI check can omit later packages

- **Symptom**: Given a root configuration and a nested configuration in the
  first package, `ttc --check-types packages` selected the nested configuration
  and did not report a TypeScript error in a second package.
- **Cause**: `find_tsconfig` claimed to search from the inputs' common directory
  but initialized the search from the first file's parent.
- **Resolution**: Walk all canonical input paths to their common ancestor before
  searching upward. The regression asserts that the second package's `ts2322`
  is reported through the real CLI and TypeScript backend.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] VS Code server tests: 161 passed
- [x] Native TypeScript backend: 66 passed, including the multi-package project
  regression
- [x] Contextual inference matrix: 34 passed
- [x] `./scripts/ci`: agents, Rust, npm, website, native, and extension passed

## Result

Changed `src/typescript/service.rs`, `src/engine/project.rs`, `tests/native.rs`,
and this task record. Type information now distinguishes a valid empty answer
from a failed or timed-out checker request, timed-out services recover on the
next request, and multi-package CLI checks use the shared project configuration
instead of silently narrowing to the first package.
