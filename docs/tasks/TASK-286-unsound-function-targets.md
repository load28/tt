# TASK-286: Reject unsound function targets

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P4** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P4` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: reject statement and expression `try` in constructors, generators, and async generators, including constructor positions before/inside/after `super`, while retaining nested ResultRegion and `using` legality.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::EvaluationOwner`, `src/evaluation_ir.rs::TargetCapability`, `src/sema.rs` placement reporting, diagnostics.

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

Test obligation from the plan: located `try-placement` for both forms; runtime guard that `new C() instanceof C`; generator guard proving no emitted program can produce `{value: Err, done: true}` or silently truncate `for...of`; nested Result and disposal acceptance tests.

Green condition: unsafe inputs emit nothing and never rely on `ts2409`, TypeScript types, or consumer behavior as the signal.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
