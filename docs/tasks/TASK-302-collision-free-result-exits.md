# TASK-302: Use a collision-free label for Result exits

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

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

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
