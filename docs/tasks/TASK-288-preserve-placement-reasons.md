# TASK-288: Preserve placement reasons

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P6** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P6` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: stop erasing `ExpressionBoundaryReason`, report static-block ownership accurately, and correct the stale `Place::ValueRegion` result-body comment at `src/sema.rs:252-253` to match the `Place::ResultRegion` behavior at `src/sema.rs:627-634`.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/lib.rs::try_target_errors`, `src/sema.rs` placement messages and comments, `src/program_syntax.rs::EvaluationOwner::StaticBlock`, diagnostic rendering.

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

Test obligation from the plan: distinct reason/message assertions for repeated loop positions, parameter owners, static blocks, constructors, and isolated crossings.

Green condition: all consumer surfaces retain the same typed reason and original try span, and sema documentation names the implemented place.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
