# TASK-720: Lower a returned tt value in an `if let` or let-else block inside a `result` block or match block arm

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-720`

## Purpose

A `return` whose operand is a tt value (`return try rd(b);`,
`return match (b) {...};`, or a template holding a `match`) completes the
enclosing `result` block or match block arm wherever it stands
(`docs/ai/tt.md`, "result block"). Written in the body of an `if let`, a
let-else's `else` block, or an `if` nested in either, the operand was
dropped (`const $tt_t1 = ;`, `const $tt_m = ;`, reported as
`verify-failed`) or the compiler stopped ("returned structured value was
not emitted"; `validate_source_preservation ... (SourceOmitted)`). The same
returns directly in the block, or in a plain function, compiled. The
round-8 probe found it; the matrix's statement hosts always exited with a
plain value.

## Scope

- Included: where the Evaluation IR places a value that a region's exit
  delivers (`src/evaluation_ir/builder.rs`, `placement`), the matrix exit
  axis (`tests/matrix/positions.mjs`, `scripts/generate-cases`,
  `CONTRIBUTING.md`), and three cases.
- Excluded: a `try` inside a template interpolation, which crosses an
  isolated value region and stays `try-crosses-value-region` (Decision 2).

## Decisions

### Decision 1: A value an exit delivers is nested under the exit's region, however deep the exit stands

- **Context**: The emitter (`emit_result_body_with_exits`, `emit_body_with_exits`)
  already completes the region from every exit the projection collected
  for it, in the region's own body and in the bodies of the decisions
  inside it (`program_syntax/visit.rs`, `visit_return_stmt`, collects a
  `return` at the region's function depth wherever it stands). The
  Evaluation IR decided whether a value is delivered by an exit by looking
  only at its immediate parent region's exits. For a value in an `if let`
  body the parent is the decision's region, which has no exits, so the
  value was planned as an ordinary value of the inner `return` statement:
  a slot replacement over the `try` (`$tt_v1`) consumed the operand when
  the exit emitted it, and a returned `match` lost its source the same way.
- **Alternatives considered**: (a) Make the emitter ignore the slot
  replacement while it emits an exit's operand: two layers would disagree
  about who owns the value, and the source-preservation check (which reads
  the plan) still sees the operand as claimed twice. (b) Copy the region's
  exits onto each decision region nested in it: duplicates the projection's
  facts and every consumer of exits would see them twice. (c) Walk the
  regions a value is built inside (recorded as `enclosing`, whatever each
  region's placement) and nest the value when any of them has an exit
  whose argument delivers it, with the existing containment rule
  (`delivered_by_exit`, and the owner holding the whole argument, so a
  callback inside a returned value is not crossed).
- **Decision and rationale**: (c). It is the one rule the projection
  already states (an exit belongs to the region at its function depth),
  applied in the layer that plans ownership, and it fixes the `try`, the
  `match`, the template, the let-else `else` block, and the nested `if`
  together, in a `result` block and in a match block arm.

### Decision 2: A `try` in a returned template stays a value-region crossing

- **Context**: The probe expected `` return `<${try rd(b)}>`; `` in an
  `if let` body to run. Directly in a `result` block the same `return` is
  `try-crosses-value-region` (`docs/ai/tt.md`: "``result { return `x${try r}`; }``
  is claimed and then rejected as `try-crosses-value-region`"), because an
  interpolation is an isolated value region (`docs/design/try-result-scopes.md`
  §4.6). Sema already reported it in the nested body; the compiler stopped
  in emission before the report reached the user.
- **Decision and rationale**: The documented rule holds in the nested body
  too; with Decision 1 the emission no longer stops, and the diagnostic is
  reported. The runtime case returns a template holding a `match` instead,
  and `resultBlockTemplateTryInIfLetBodyCrossesTheRegion.tt` pins the
  diagnostic in an `if let` body and a let-else `else` block.

### Decision 3: The matrix varies a statement host's exit as its own hosts, for the forms that write it

- **Context**: A statement form receives its host's exit (`c.s(exit)`) and
  writes it where it leaves early (`ifLet_exitInBody`, every let-else).
  The `result` body host exited with `return base;` and the match block arm
  with `throw`, so no case returned a tt value from a nested body.
- **Alternatives considered**: (a) Insert the new hosts beside their base
  hosts: the all-pairs generator then re-picks the companions of every
  statement form, renaming about 460 cases whose forms never write the
  exit. (b) Add `matchBlockArmValueExit`, `matchBlockArmTemplateExit`,
  `resultBodyValueExit`, and `resultBodyTemplateExit` with an `exitAxis`
  flag, which the generator runs only for forms (and diagnostic examples)
  whose template takes the exit argument.
- **Decision and rationale**: (b). The exits are `return match (...)`, a
  template holding a `match`, and (in the `result` body) `return try read(...)`
  on a failing read, whose `Err` completes the block; each twin returns the
  same value written in TypeScript. 156 runtime cases, 8 diagnostic cases,
  and their editor cases were generated; the companions of the existing
  `ifLet_exitInBody` and let-else forms moved (92 removed, their baselines
  deleted).

## Work log

- 2026-10-01: Reproduced the probe's two cases and smaller ones
  (`return try`, `return match`, templates, at the top of a `result` block
  and in an `if let` body); found the slot replacement covering the
  operand by tracing `source_range_rope`, and the owner slot planned for
  the `try` by printing the plan's owner slots.
- 2026-10-01: Implemented Decision 1 (`enclosing`, the ancestor walk in
  `placement`); added the cases
  `tests/cases/compiler/resultBlockReturnsTtValueFromIfLetBody.tt` and
  `matchBlockArmReturnsTtValueFromIfLetBody.tt` (from the probe; the
  `result` case's template returns a `match`) and
  `resultBlockTemplateTryInIfLetBodyCrossesTheRegion.tt`.
- 2026-10-01: Implemented Decision 3; `node scripts/generate-cases`;
  `UPDATE_EXPECT=1 TT_MATRIX_CASES=all` for `ifLet_`, `letElse_`, `Exit`,
  and `let-else` in `case_baselines` and for `ifLet_`, `letElse_`, `Exit`
  in `editor_cases`; deleted the baselines of the removed cases; added the
  exit axis to "The case matrix" in `CONTRIBUTING.md`.

## Issues and resolutions

### Issue 1: A form named `exitInBody` already existed

- **Symptom**: `node scripts/generate-cases` stopped with "two rows
  generate ifLet_exitInBody_componentBody_plain" after a new `if let` form
  was added.
- **Cause**: `tests/matrix/ifLet.mjs` already has `exitInBody`, a body that
  writes the host's exit.
- **Resolution**: The new form was dropped; the exit-axis hosts run the
  existing one.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/resultBlockReturnsTtValueFromIfLetBody.tt`,
  `matchBlockArmReturnsTtValueFromIfLetBody.tt`,
  `resultBlockTemplateTryInIfLetBodyCrossesTheRegion.tt`, and the
  exit-axis matrix cases (`TT_CASES=Exit TT_MATRIX_CASES=all`)
- **Observed failure**: with `src/evaluation_ir` reverted, each of the
  three cases panicked at `src/codegen/core/emitter/source.rs:680:36`
  ("returned structured value was not emitted"), and 122 exit-axis cases
  failed, among them
  `let-else-placement_lexical_matchBlockArmValueExit` ("validate_source_preservation
  broke the contract ... (SourceOmitted)").

## Verification

- [x] `TT_MATRIX_CASES=all` `case_baselines` for `ifLet_`, `letElse_`,
  `Exit`, `let-else`, and `editor_cases` for `ifLet_`, `letElse_`, `Exit`:
  pass
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

A `return` of a tt value completes its `result` block or match block arm
from an `if let` body, a let-else `else` block, or an `if` nested in
either, as it does from the block itself; the matrix's statement hosts
now exit with tt values too.
