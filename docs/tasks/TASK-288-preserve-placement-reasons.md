# TASK-288: Preserve placement reasons

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-288: preserve placement reasons`

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

### Decision 1: Carry the typed boundary reason and owner to diagnostic rendering.

- **Context**: Evaluation IR retained `ExpressionBoundaryReason`, but the
  public error builder discarded it and emitted one generic message.
- **Alternatives considered**: Infer reasons from source in diagnostics;
  introduce separate public diagnostic codes for every boundary.
- **Decision and rationale**: Preserve the existing `try-placement` code and
  carry the typed reason with `EvaluationOwner` to its renderer. This keeps
  all public clients on one diagnostic contract while naming the actual host
  boundary.

### Decision 2: Treat class static blocks as non-function owners.

- **Context**: Expression projection already had `StaticBlock` ownership,
  but its capability and direct statement diagnostics could describe it as a
  generic expression or module position.
- **Alternatives considered**: Permit a generated return from the static
  block; treat every nested token as statically owned.
- **Decision and rationale**: Static blocks receive a dedicated placement
  message, while a nested user-written ordinary function remains its own
  return target.

## Work log

- 2026-08-30: Started tracing expression-boundary reasons from Evaluation IR to public diagnostics.
- 2026-08-30: Carried the owner and reason from Evaluation IR through the
  lowering plan to the shared `try-placement` renderer.
- 2026-08-30: Added reason and source-span coverage for repeated loops,
  parameter initializers, static blocks, constructors, and isolated regions.
- 2026-08-30: Ran the required Rust checks and the full repository CI gate.

## Issues and resolutions

### Issue 1: Host placement diagnostics collapsed distinct boundaries.

- **Symptom**: Repeated, parameter, static-block, and conditional positions
  all reported the same generic expression message.
- **Cause**: The diagnostic conversion ignored the typed reason computed by
  Evaluation IR.
- **Resolution**: The lowering plan now retains both reason and owner for the
  renderer, which selects a stable message without changing the diagnostic
  code.

## Verification

Test obligation from the plan: distinct reason/message assertions for repeated loop positions, parameter owners, static blocks, constructors, and isolated crossings.

Green condition: all consumer surfaces retain the same typed reason and original try span, and sema documentation names the implemented place.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Changed Evaluation IR, its codegen consumer, shared diagnostic rendering,
semantic placement checks, and flow ownership detection. Changed compile tests
to prove each public message retains the original `try` span and that a nested
function in a static block remains valid. Result-body comments now name
`Place::ResultRegion`, matching the implementation.
