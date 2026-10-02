# TASK-642: Give a match that mixes tag and literal patterns no single-discriminant dispatch

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-642`

## Purpose

`consume(match (flag) { e => 1, false => 2 });` stopped every untyped
pipeline with an internal compiler error, "switch variant alternative is not
constructor" (`src/codegen/core/emitter/pattern.rs`), while the same match as
a declaration's value reported `match-mixed-patterns`. TASK-637's fuzz replay
listed the input in `fuzz/regressions/expected-failures.txt`.

## Scope

- Included: the dispatch the core IR gives a match decision
  (`src/core_ir/lower.rs`, `match_kind`), a case file, and the expected
  failure line.
- Excluded: the typed pass's exhaustiveness answer for a mixed match (Issue 2).

## Decisions

### Decision 1: Fix the dispatch model, not the emitter or the diagnostic's reach

- **Context**: `match-mixed-patterns` is a sema diagnostic
  (`src/sema/checker.rs`, `check_match`) and is recoverable: like the other
  match diagnostics it leaves the file emittable, so the typed pass can
  still run. `ttc::analyze` reported it for the call argument too; the crash
  came after it, when emission ran. `match_kind` gave every switchable match
  whose arms were not all literal the `VariantSwitch` dispatch, so a match of
  tag and literal arms reached the value-selector emitter, which labels each
  alternative with `.kind` and has no literal to label.
- **Alternatives considered**: (a) Make `match-mixed-patterns` block the
  projection (`blocks_projection`): the construct would never be emitted, but
  the file would lose its emission and its typed diagnostics, which no other
  match diagnostic costs, and the IR would still describe a mixed match as a
  variant switch. (b) Make the selector emitter label each alternative by its
  own test, as the statement-position `emit_switch` already does: patches one
  emitter path and leaves a plan that claims one discriminant where there
  are two. (c) Give a mixed match `Conditional` dispatch in the core IR.
- **Decision and rationale**: (c). A switch compares one discriminant; a match
  whose tested arms mix literal and variant tests has two (`$tt_m` and
  `$tt_m.kind`, the fact `check_match` states), so the only dispatch that
  represents it is the ordered conditional chain, which tests each arm by
  its own pattern. Every position then plans the match the same way, the
  sema diagnostic is reported in every position before any output, and the
  emitted TypeScript (withheld while the error stands) is well formed. A tag
  mixed with an `is` pattern already had `Conditional` dispatch through
  `pattern_has_instance_test`.

## Work log

- 2026-09-30: Reproduced with `ttc --check` on the fuzz input; the declaration
  form reports `match-mixed-patterns`. Traced the panic to `match_kind` and
  the selector emitter.
- 2026-09-30: Changed `match_kind`; added
  `tests/cases/compiler/mixedPatternsInEveryPosition.tt` (call argument,
  array element, operand inside an arrow body); removed the input's line from
  `fuzz/regressions/expected-failures.txt` and kept the input as a
  regression.

## Issues and resolutions

### Issue 1: None beyond the crash

- **Symptom**: The internal compiler error above.
- **Cause**: Decision 1's context.
- **Resolution**: Decision 1.

### Issue 2: The typed pass still answers exhaustiveness for a mixed match

- **Symptom**: `ttc --check-types` reports `match-not-exhaustive` ("missing
  true") next to `match-mixed-patterns`, in declaration position as before
  this task and in every other position now that they no longer crash.
  `check_match` suppresses the untyped coverage answer for a mixed match
  ("report the cause, not its effects"); the typed coverage does not read
  that suppression.
- **Cause**: Not traced further; it is independent of this crash.
- **Resolution**: Left open and reported; the new case's `.errors.txt` pins
  the current output so a fix shows as a baseline change. Fixed by TASK-693.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/mixedPatternsInEveryPosition.tt`
  (`cargo test --test case_baselines`), and `tests/fuzz_regressions.rs`
  with the line removed from `fuzz/regressions/expected-failures.txt`.
- **Observed failure**: Without the change the case run panics at
  `src/codegen/core/emitter/pattern.rs:979:13` and fails, and
  `every_committed_crash_input_replays_as_the_list_says` reports
  "compile_any_bytes/mixed-patterns-in-a-call-argument.tt crashes:
  src/codegen/core/emitter/pattern.rs: switch variant alternative is not
  constructor".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` (final gate, TASK-643)
- [x] `cargo test` (targeted: `--lib`, `compile`, `case_baselines`,
  `snapshot`, `fuzz_regressions`; full gate in TASK-643)
- [x] Baseline changes reviewed: three `match-mixed-patterns` errors from
  `ttc --out-dir`, nothing emitted.

## Result

Changed files: `src/core_ir/lower.rs`, `fuzz/regressions/expected-failures.txt`,
`tests/cases/compiler/mixedPatternsInEveryPosition.tt` and its baselines,
`docs/tasks/INDEX.md`, and this record.
