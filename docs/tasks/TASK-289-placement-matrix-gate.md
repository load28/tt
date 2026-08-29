# TASK-289: Prerequisite placement matrix gate

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P-matrix** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P-matrix` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: before M0 or any L-slice, enumerate every §4.5 row at both a statement host and an expression-boundary host.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: the placement integration tests around `src/program_syntax.rs`, `src/evaluation_ir.rs`, `src/sema.rs`, and the public `ttc::analyze` path.

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

Test obligation from the plan: each row asserts a specific diagnostic code or parseable emitted TypeScript, with `catch_unwind` around `ttc::analyze`; include `using`/`await using`, constructor, generator/async generator, `for` declaration-init/test/update and `for-of` RHS, concise arrows, isolated crossings, and whole ResultRegion hosts.

Green condition: no row panics and no row ends as `verify-failed` “did not parse as a tt `try`”; all P5 claimer cases must therefore be closed before the gate passes.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
