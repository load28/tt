# TASK-282: Preserve the planner's existing failure return

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P0** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P0` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: stop the codegen wrapper from converting Evaluation IR's already-fallible lowering result into `ice::bug!`, and carry it as a located diagnostic across every public client.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/codegen/core.rs::lowering_plan` (the wrapper at `:49-76` that converts `evaluation_ir::EvaluationFile::lowering_plan`'s existing `Result<LoweringPlan, EvaluationError>` at `src/evaluation_ir.rs:484` into `ice::bug!`; `LoweringPlan` itself is `src/evaluation_ir.rs:109`), `src/lib.rs::{analyze,compile_report}`, CLI, `src/server.rs`, `src/content_mapper.rs`, and Engine/Snapshot entry points.

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

Test obligation from the plan: run the for-header, expression-boundary Result, and discarded-Result failures through `catch_unwind` at library level and through CLI/server/mapper/Engine; assert non-empty located diagnostics, not exit 101, a bug payload, or an empty vector.

Green condition: no source input unwinds and every consumer reports the same tt code.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
