# TASK-282: Preserve the planner's existing failure return

- **Status**: Complete
- **Started**: 2026-08-29
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

### Decision 1: Preserve a dedicated planner-failure diagnostic at the public boundary

- **Context**: Evaluation IR and host projection can already return typed
  failures for source-dependent shapes, but codegen converted them into
  `ice::bug!` before any public client could report them.
- **Alternatives considered**: retain the panic for invariant failures; reuse
  `verify-failed`; or expose the internal error through a new stable code.
- **Decision and rationale**: introduce `lowering-plan-failed` and convert
  every fallible host-lowering stage into a located diagnostic. The code
  distinguishes this input-visible failure from the emitted-TypeScript
  self-check, and all callers share the same `compile_report`/`analyze` path.

## Work log

- 2026-08-29: Confirmed that `codegen::lowering_plan` converts
  `EvaluationFile::lowering_plan` failures into `ice::bug!`, while public
  compilation clients either omit or cannot represent that failure.
- 2026-08-29: Added `LoweringFailure`, propagated host projection and
  Evaluation IR failures through `analyze`, `compile`, and `compile_report`,
  and registered the public diagnostic code and guide entry.

## Issues and resolutions

### Issue 1: C-style `for` declaration initialization reached a generated projection parse failure

- **Symptom**: `for (let i = try next();;)` panicked while constructing the
  TypeScript host projection.
- **Cause**: The generated statement overlay is not legal in a C-style loop
  initializer, and the former wrapper classified every non-source projection
  failure as an ICE.
- **Resolution**: Preserve the host projection failure as a located
  `lowering-plan-failed` diagnostic. TASK-283 owns making this Legal.

## Verification

Test obligation from the plan: run the for-header, expression-boundary Result, and discarded-Result failures through `catch_unwind` at library level and through CLI/server/mapper/Engine; assert non-empty located diagnostics, not exit 101, a bug payload, or an empty vector.

Green condition: no source input unwinds and every consumer reports the same tt code.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Added located `lowering-plan-failed` reporting for fallible host-lowering
stages, a regression test for the C-style `for` failure, and guide coverage.
