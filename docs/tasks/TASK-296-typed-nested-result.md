# TASK-296: Add typed nested-Result diagnostics

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

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

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

Test obligation from the plan: definite Result, union/non-Result/unknown/generic cases, `return try` edit, server and Engine typed paths.

Green condition: only checker-proven nested Results diagnose, and ordinary TypeScript errors remain TypeScript's responsibility.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
