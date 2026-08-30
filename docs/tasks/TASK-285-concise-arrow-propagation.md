# TASK-285: Repair concise-arrow propagation

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-285: repair concise arrow propagation`

## Purpose

Item **P3** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P3` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: make ordinary, parenthesized, and pipeline-step concise arrows containing expression `try` lower to valid block-bodied arrows without moving the return target.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: arrow/pipeline ownership in `src/program_syntax.rs`, `src/evaluation_ir.rs`, and corresponding codegen printers.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Project pipeline-step arrows beside the opaque pipeline host

- **Context**: SWC cannot parse pipeline syntax, while a concise arrow inside
  a pipeline step must still expose its own `try` owner.
- **Alternatives considered**: assign the `try` to the enclosing pipeline;
  rewrite the pipeline as a statement region; or project the arrow separately.
- **Decision and rationale**: retain the pipeline as its existing placeholder
  and add a structurally projected step shadow. This lets the owner collector
  bind the propagation to the arrow without changing pipeline evaluation.

## Work log

- 2026-08-30: Started reproducing concise-arrow propagation across ordinary,
  parenthesized, and pipeline-step hosts.
- 2026-08-30: Added arrow-return emission for parenthesized concise bodies.
- 2026-08-30: Projected propagation-bearing pipeline steps and prevented their
  outer `Apply` from being promoted into the arrow's statement region.
- 2026-08-30: Added ordinary, parenthesized, and pipeline regression tests.

## Issues and resolutions

### Pipeline-step host was missing

- **Symptom**: `x => try next()` in a pipeline step reported `MissingHost`.
- **Cause**: the pipeline placeholder hid the nested arrow from the syntax
  owner collector.
- **Resolution**: the projection now carries a valid shadow of each affected
  step, and evaluation keeps the outer pipeline expression-shaped.

## Verification

Test obligation from the plan: default verification and `--no-verify` output parse, preserve evaluation order and lexical capture, and keep standalone concise `=> try x` behavior.

Green condition: every accepted output parses and the arrow, never its enclosing function, owns failure.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

`src/program_syntax.rs` now preserves arrow ownership for propagation-bearing
pipeline steps. `src/evaluation_ir.rs` avoids promoting an outer pipeline that
contains a separately hosted value. `src/codegen/core.rs` emits a valid lexical
arrow IIFE for a parenthesized concise body. `tests/compile.rs` covers all three
concise-arrow forms and their verified TypeScript output.
