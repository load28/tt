# TASK-296: Add typed nested-Result diagnostics

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history

## Purpose

Item **L5** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L5` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: implement the definite Result-shape checker query and `result-return-nested` without adding untyped guesses.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/engine/projection.rs`, `src/typescript/backend.rs`, `src/engine/semantics.rs`, diagnostic suggestions.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Query checker shape rather than parsing type display text

- **Context**: aliases, generics, and unions make a checker display string an
  unsound source of Result identity.
- **Alternatives considered**: recognize `TResult` text in Rust, or query the
  TypeScript checker for a definite structural Result shape.
- **Decision and rationale**: Ask the TypeScript backend whether the value is
  structurally a two-member `Ok`/`Err` union with the corresponding payload
  fields. The emitted return value is first named by a collision-free compiler
  temporary, because a position on `Result.Ok(...)` identifies the constructor
  rather than the call result. Rust owns the diagnostic and edit after the
  checker returns that fact.

## Work log

- 2026-08-30: Started and traced the existing projection-to-checker query
  pipeline.
- 2026-08-30: Added the batched Result-shape protocol, source/output marker,
  and `result-return-nested` diagnostic with a `try ` insertion edit.
- 2026-08-30: Verified the native server path for definite Result, union,
  ordinary value, unknown, and generic value returns.

## Issues and resolutions

None.

## Verification

Test obligation from the plan: definite Result, union/non-Result/unknown/generic cases, `return try` edit, server and Engine typed paths.

Green condition: only checker-proven nested Results diagnose, and ordinary TypeScript errors remain TypeScript's responsibility.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Only checker-proven Result values receive the nested-return diagnostic; union,
unknown, generic, and ordinary values remain TypeScript-owned.
