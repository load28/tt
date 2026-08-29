# TASK-289: Prerequisite placement matrix gate

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-289: add placement matrix gate`

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

### Decision 1: Make one public-analysis matrix the prerequisite proof.

- **Context**: Placement coverage was distributed across parser, projection,
  Evaluation IR, and codegen tests, so no test proved the published §4.5
  classification as one contract.
- **Alternatives considered**: Rely on existing unit tests; assert only
  emitted text; invoke private lowering stages directly.
- **Decision and rationale**: Exercise representative rows through
  `catch_unwind(ttc::analyze)` and compile accepted rows with default
  verification. This proves public recovery without duplicating placement.

### Decision 2: Preserve the C-style Test diagnostic code in the matrix.

- **Context**: C-style Test is statement propagation and reports the existing
  repeated-propagation `LoweringPlanFailed`; Update is expression propagation
  and reports `try-placement`.
- **Alternatives considered**: Normalize both positions in this gate; omit
  the Test row because a focused test already exists.
- **Decision and rationale**: Assert the established distinct code for Test
  and `try-placement` for Update. Both are located rejections and neither
  reaches output verification.

## Work log

- 2026-08-30: Started enumerating the §4.5 placement rows through the public analysis API.
- 2026-08-30: Added accepted and rejected representatives for function,
  expression, loop, `using`, concise-arrow, owner, conditional, and Result
  host categories.
- 2026-08-30: Ran the required Rust checks and the full repository CI gate.

## Issues and resolutions

### Issue 1: A combined repeated-loop sample hid the Test diagnostic contract.

- **Symptom**: A source containing while, C-style Test, and Update reported
  the Test lowering diagnostic first.
- **Cause**: C-style Test remains statement propagation, unlike Update.
- **Resolution**: The matrix gives every repeated position its own row and
  asserts its specified diagnostic code.

## Verification

Test obligation from the plan: each row asserts a specific diagnostic code or parseable emitted TypeScript, with `catch_unwind` around `ttc::analyze`; include `using`/`await using`, constructor, generator/async generator, `for` declaration-init/test/update and `for-of` RHS, concise arrows, isolated crossings, and whole ResultRegion hosts.

Green condition: no row panics and no row ends as `verify-failed` “did not parse as a tt `try`”; all P5 claimer cases must therefore be closed before the gate passes.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Changed `tests/compile.rs` with the public placement matrix gate. The gate
proves accepted rows emit verified TypeScript, rejected rows preserve their
specified located code, and no row unwinds or reports `verify-failed`.
