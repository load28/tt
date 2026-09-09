# TASK-302: Use a collision-free label for Result exits

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-302: repair Result completion defects`

## Purpose

Repairs defect **D1** in the shipped Result completion model. All of these
defects first ship in `a308c64` (#88) and are independent of any future language
proposal — they are wrong behaviour in released code, not design questions.

Severity: Critical.

## Symptom

A Result exit is emitted as an unlabeled `break`, so an inner `for`, `while`, `do`, or `switch` inside a claimed `result` block captures it. The propagated `Err` or the owned success is then overwritten by whatever the region assigns next, and the emitted TypeScript stays valid, so neither ttc nor tsc reports anything. This is a silent wrong runtime value in shipped code.

## Scope

- Included: Emit a collision-free named Result label and target it from every Result exit. Add emitted-output and runtime tests for `for`, `while`, `do`, and `switch`, each with a failure path, a success path, and trailing side effects, in `tests/compile.rs`, `tests/integration.rs`, and the emit fixtures.
- Excluded: any change to the Result language model. This task does not revisit
  what `return` means, the success channel, or placement rules. A change to those
  is a change to `docs/design/try-result-scopes.md` first.

Files and symbols: `src/codegen/core.rs::{emit_result_region_continued, emit_result_body_with_exits, emit_result_statements_with_exits, emit_region_propagate, emit_value_delivery_with_exit}` and the `HostExit` plumbing that decides when a label is required.

## Green condition

No Result exit can be captured by a construct the user wrote inside the block; the runtime value observed by the caller matches the block's owned completion on every path.

## Decisions

Record every decision this task makes, including any new public diagnostic code
and any wire-compatibility choice, with its alternatives.

### Decision 1: Label the compiler-owned Result boundary

- **Context**: Result completion currently uses an unlabeled `break`, which can
  be captured by a user-authored breakable construct nested inside the region.
- **Alternatives considered**: Reject breakable constructs, return from an
  expression-boundary closure, or assign a generated label to the Result boundary.
- **Decision and rationale**: Generate a collision-free label through the normal
  emitter name allocator and target every Result completion at that label. This
  preserves the language model and makes the intended control-flow edge explicit.

## Work log

- 2026-08-30: Began tracing Result completion exits and their continuation plumbing.
- 2026-08-30: Routed success and failure completion through the generated value-slot label and pinned every breakable host at compile and runtime.

## Issues and resolutions

- The first full gate exposed a clippy argument-count failure; grouped Result continuations and the exit label into one emission context.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Result completion now targets a collision-free generated label. Compile and
runtime coverage proves that `for`, `while`, `do`, and `switch` cannot capture it.
