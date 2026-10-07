# TASK-776: Lower an expression's tt values over one shared evaluation tree

- **Status**: In progress
- **Started**: 2026-10-07
- **Completed**: —
- **Commit**: —

## Purpose

TASK-774 left compiling many tt values in one expression quadratic in their
number: every value carries its own copy of the steps between it and its
owner, so `n` values in one array or one `+` chain cost `n²` steps and
inputs in every stage that reads schedules. The user chose to restructure
the lowering the way TypeScript handles the same problem: the generators
transform spills the operands evaluated before a `yield` in one top-down
pass, guided by a subtree flag (`transformers/generators.ts`,
`visitLeftAssociativeBinaryExpression` and `cacheExpression`, keyed on
`TransformFlags.ContainsYield`), so its work is linear in the expression.

## Scope

- Included: the evaluation protocol (`src/program_syntax/`), the schedules
  and their planning and validation (`src/evaluation_ir/`), and the target
  planning and emitter that read them (`src/codegen/core/`).
- Excluded: any change to what the compiler emits. Every case baseline,
  snapshot, and fixture must stay byte for byte as it is.

## Decisions

### Decision 1: A value's steps are a path in a tree shared by every value of the expression

- **Context**: Every tt value carried a vector of the steps between it and
  its owner, built and resolved for it alone; values under the same
  operations held equal copies.
- **Alternatives considered**: (a) Trimming each value's step to the inputs
  after the previous value (TASK-774 decision 1): it changes the steps two
  values of one conditional operation share and was reverted. (b) A
  persistent list per value, linked from the innermost step outward, with
  one link per (enclosing link, frame, the operand the value sits in): the
  values under one operand of one frame share that link and everything
  outside it, which is TypeScript's own shape (`Node.parent`).
- **Decision and rationale**: (b). `src/chain.rs` holds the list
  (`Chain`, and `ChainSlice` for a prefix or suffix of one) and
  `protocol_step` is split into `step_selection` (which operand of the frame
  holds the value) and the step that operand gives, so the link is built
  once per (outer link, frame, selection) — a loop test's step names the
  value itself and is keyed by it as well. The planned schedule reuses the
  resolution of a shared suffix: resolving it again would read the same
  slots and allocate nothing (a value whose call may complete reserves
  names for its inputs and is resolved alone, as before). Every step a
  value has is the step it had before, so every consumer reads the same
  sequence.

## Work log

- 2026-10-07: Implemented decision 1. Release timings with line tables:
  1,600 matches joined by `+` take 0.92 s (1.5 s before); the flat shapes
  (1,600 matches in one array) stay quadratic, because the step for an
  operand still lists every earlier operand.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Every case baseline unchanged

## Result

In progress.
