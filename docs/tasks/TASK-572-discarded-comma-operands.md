# TASK-572: Evaluate a comma operand before a value as a statement

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A comma expression's operand before a tt value was captured into a
`const` and read again where it was written.
`export function f(k: K) { return (tick(), match (k) { A => 1, B => 2 }); }`
emitted `const $tt_v1 = (tick());` and `return ($tt_v1, (…));`, which
`tsc` and `ttc --check-types` reject with TS2695 ("Left side of comma
operator is unused and has no side effects"). The comma operator evaluates
its left operand only for its effects (ECMA-262 §13.16.1: `GetValue` of the
left operand, whose result is discarded), so the operand's value must not be
captured and re-read.

## Scope

- Included: the evaluation mode of a comma operand in the protocol step of
  a value after it (`src/program_syntax`), capture elision for it
  (Evaluation IR planning), its statement emission, its removal from every
  delivered reading of the source (the plan's source replacements, a
  scheduled operand, a nested capture), the preservation claim of its
  comma, `docs/design/program-lowering.md` §7.7, and regression tests.
- Excluded: a tt value that is itself a non-final comma operand
  (`(match (k) { A => { tick(); return 1; }, B => 2 }, tock())`). Its value
  is still delivered where it was written, as a slot read or an arm
  selection whose arms may run there, and a slot read there is also TS2695.
  Removing it needs a discarding value continuation (arms lowered as
  statements, no deferred arm values), which is a change to value
  continuations rather than to input capture; it is left as a follow-up.

## Decisions

### Decision 1: A comma operand before a value is a discarded input

- **Context**: `protocol_step` gave every earlier position of an ordered
  frame (array, object, comma, unary) the mode `Value`, so the capture
  machinery treated a comma operand like an array element whose value the
  expression uses.
- **Alternatives considered**: (a) Keep the capture and replace the operand
  with an expression TypeScript does not consider side-effect free (for
  example `void 0`). That targets one diagnostic's rule and leaves a
  meaningless operand in the output. (b) Record that the value of the
  operand is discarded and lower it as the comma operator defines it.
- **Decision and rationale**: (b). `EvaluationInputMode::Discarded` marks
  the earlier positions of a comma frame. Planning elides an inert one as
  it elides any inert value input. The prelude writes the operand as an
  expression statement in its order (`(tick());`, grouped so that no
  operand can start a declaration or a block), and every reading of the
  source removes the operand and the comma after it
  (`discarded_operand_comma`), keeping the trivia between them. The comma
  is claimed as rewritten and relocated source, as a compound assignment's
  operator is (TASK-522).

## Work log

- 2026-09-29: Reproduced `target/probe4-compiler/min/b3.tt` (TS2695 under
  `ttc --check-types`) and variants with several operands, a comment
  before the comma, a logical branch, and a `try` operand.
- 2026-09-29: Added `EvaluationInputMode::Discarded`
  (`src/program_syntax.rs`) and set it for comma frames
  (`src/program_syntax/protocol.rs`); elided inert ones
  (`src/evaluation_ir/planning.rs`); added `discarded_operand_comma`,
  `discarded_operand_commas`, and the replacements
  (`src/codegen/core/planning.rs`); emitted the statement and removed the
  operand in scheduled and nested readings
  (`src/codegen/core/emitter/host.rs`).
- 2026-09-29: Added
  `runtime_a_comma_operand_before_a_value_runs_once_as_a_statement`
  (`tests/integration/cases_05.rs`) and
  `a_comma_operand_before_a_value_checks_clean`
  (`tests/native/cases_10.rs`); documented the lowering in
  `docs/design/program-lowering.md` §7.7.

## Issues and resolutions

### Issue 1: A comment before the comma was dropped

- **Symptom**: `(tick() /* note */, match …)` delivered `( ($tt_v…))`
  without the comment.
- **Cause**: The first version removed one span from the operand through
  its comma.
- **Resolution**: The operand and the comma are removed as two spans, so
  the trivia between them is written.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change (TS2695 from `tsc` and from
  `ttc --check-types`).

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/protocol.rs`,
`src/evaluation_ir/planning.rs`, `src/codegen/core/planning.rs`,
`src/codegen/core/emitter/host.rs`, `docs/design/program-lowering.md`,
`tests/integration/cases_05.rs`, `tests/native/cases_10.rs`,
`docs/tasks/INDEX.md`, and this record.
