# TASK-266: Diagnose fields of imported variants in every compiler path

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

A field misspelling in a pattern over an imported variant currently compiles to
an `undefined` binding without a tt diagnostic. Make cross-module pattern
resolution enforce the same field contract as a local variant.

## Scope

- Included: imported variant declaration data, pattern field resolution,
  `--check`, `--check-types`, content-mapper, and server regression coverage
- Excluded: unknown case behavior, nested-pattern rendering, and general
  checker-cascade policy, which are tracked by TASK-267, TASK-268, TASK-270,
  and TASK-273

## Decisions

### Decision 1: Preserve one semantic declaration model across module boundaries

- **Context**: The batch path currently imports case tags without payload
  fields, while the typed engine has a richer declaration model.
- **Alternatives considered**: infer fields from generated TypeScript; add a
  spelling heuristic in codegen; or carry the source declaration through the
  resolver.
- **Decision and rationale**: Extend the compiler-owned declaration model and
  resolve imported fields in the resolver. Generated-code inference and
  codegen heuristics violate the compiler-layer contract and cannot provide
  source-level suggestions reliably.

### Decision 2: Test behavior at every public diagnostic boundary

- **Context**: The defect differs across batch, typed, content-mapper, and
  server paths.
- **Alternatives considered**: unit-test the resolver only, or snapshot only
  the CLI.
- **Decision and rationale**: Keep focused resolver tests and add public-path
  regressions so all consumers share the same source diagnostic contract.

## Work log

- 2026-08-28: Reproduced the missing imported-field diagnostic on current
  `main` and traced the batch path to tag-only `ExternVariant` declarations.
- 2026-08-28: Registered TASK-266 through TASK-274 from the nightly diagnostic
  audit before implementation.
- 2026-08-28: Replaced tag-only external declarations with case and payload
  declarations, including generic and field type text, and taught both CLI and
  content-mapper collectors to preserve them through import aliases.
- 2026-08-28: Extracted one resolver-diagnostic author in `sema` and reused it
  from the typed engine, merging identical file-local results while retaining
  imported-name diagnostics.
- 2026-08-28: Added resolver, batch CLI, typed CLI/server, and content-mapper
  regressions for the same imported `Card(brnad)` source error.
- 2026-08-28: Ran `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test`; all passed.

## Issues and resolutions

### Issue 1: Imported declarations lose payload fields

- **Symptom**: `Card(brnad)` is accepted when `Card` comes from a direct `.tt`
  import, although the equivalent local declaration reports `unknown-field`.
- **Cause**: `ExternVariant` carries only case tags into coverage semantics.
- **Resolution**: `ExternVariant` now carries cases and fields into the shared
  resolver, which emits `unknown-field` from the original token.

### Issue 2: Initial cross-path assertions used presentation coordinates

- **Symptom**: The first focused tests expected column 31/40 and read a server
  edit directly from the suggestion object.
- **Cause**: The source indentation places `brnad` at column 32, and server
  suggestions wrap replacements in an `edit` object by protocol contract.
- **Resolution**: Corrected the assertions to the measured source coordinate
  and structured wire format; all four focused paths then passed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Imported declarations now retain generic, case, and payload-field semantics.
The resolver reports the same `unknown-field` cause, exact token range, and
replacement through `--check`, `--check-types`, `--server typedCheck`, and the
TypeScript content mapper, while the generated TS2339 consequence stays
suppressed. Changed files: `src/lib.rs`, `src/resolve/mod.rs`, `src/analysis/mod.rs`,
`src/sema.rs`, `src/main.rs`, `src/content_mapper.rs`,
`src/engine/semantics.rs`, `src/engine/declarations.rs`, `tests/compile.rs`,
`tests/resolve.rs`, `tests/integration.rs`, `tests/native.rs`, and
`tests/content_mapper.rs`.
