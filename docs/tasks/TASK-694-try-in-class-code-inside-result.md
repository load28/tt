# TASK-694: Bound a `result` block's `try`s at class code, and name the module reason everywhere

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-694`

## Purpose

Two placement defects left open after TASK-681 to TASK-684: a `try` in a
class static block (or a class field initializer) written inside a
`result` block, with no function in between, was taken as leaving that
block, and a value-form `try` at a module's top level got the generic
"expression context" wording instead of the module reason the statement
form gets.

## Scope

- Included: the region-boundary facts the parser records for a `try`,
  let-else, or `if let` (`crate::flow::in_function_body`) and the depth a
  speculative `result` claim counts (`crate::flow::user_function_depth_at`),
  both in `src/flow/syntax.rs`; the `try-placement` wording for a `try` no
  function encloses (`src/diagnostics.rs`, `src/sema/checker.rs`,
  `src/lib/compile.rs`); two case files; `docs/ai/tt.md`;
  `docs/design/try-result-scopes.md` §4.2.
- Excluded: the lexical `yield` crossing check (`crate::flow::function_depth_at`),
  which a class body cannot hold anyway; the wording of the other
  placements, which already names their reason.

## Decisions

### Decision 1: Class code is a Result scope boundary for the `result` block as it is for a function

- **Context**: TASK-681 made a class body and a class static block the
  innermost target of a function-targeted `try` (`FunctionTarget::StaticBlock`,
  `FunctionTarget::ClassElement`, from the lexer's `class_body` and
  `static_block` facts), following ECMA-262 §15.7: class code is not
  evaluated by the function the class is written in. The `result` block
  was not given the same model. Its claim counted only function bodies
  (`user_function_depth_at` used `function_body_brace`), and a `try`
  statement's `in_function` fact, which tells sema that a boundary written
  in the region owns the `try`, counted only function bodies
  (`in_function_body`). So in
  `result { class K { static { const w = try read(2); } } return 1; }`
  the `try` claimed the block, sema accepted it as leaving the block
  (`Place::ResultRegion`), and codegen wrote `break $tt_v0;` inside the
  static block, which ECMA-262 §15.7.1 forbids (a
  `ClassStaticBlockStatementList` may contain no undefined break target
  and is parsed `[~Return]`); a `try` in a class field initializer there
  crashed codegen ("unscheduled expression try reached inline emission").
- **Alternatives considered**: (a) Reject a `try` in class code in sema
  whenever a `result` block encloses it: a second rule beside the boundary
  model, and it would reject the legal `result` block written inside a
  static block (its own `try` leaves it). (b) Lower such a `try` through
  the result block's expression boundary: no exit from class code can
  reach the block, which is the reason the static block is rejected for
  functions.
- **Decision and rationale**: The two facts count every function-like
  boundary `function_target_brace` names: function bodies, class bodies,
  and class static blocks. The `try` in class code then does not claim the
  enclosing `result` block, sema resolves its target with
  `FunctionTargets` (TASK-681), and it is rejected with the static-block
  or class-field wording. A `result` block written inside the static block
  is still the nearest scope of its own `try`. A let-else or `if let`
  written in class code inside a region is likewise placed by that class
  code, whose exits cannot leave it either.

### Decision 2: A `try` no function encloses gets one wording, from one constant

- **Context**: A statement `try` at a module's top level reports
  "`try` must be inside a function — ... at the top level of a module
  there is no function to return from" (sema). The value form is judged
  by the lowering plan (`try_placement_message`), whose owner-specific
  arms named parameters, class fields, class definitions, enums,
  constructors, generators, and static blocks, but not the module owner:
  its default reason `OwnerTakesNoStatements` fell to the generic "this
  expression context" arm. A statement `try` in a match block arm at the
  top level fell to sema's value-region wording, which since TASK-683 no
  longer describes any case (a match arm is not a boundary; its `try`
  keeps the function target, so the only `try` with no target is one no
  function encloses).
- **Alternatives considered**: Word the value form's module case
  separately: two sentences for one reason.
- **Decision and rationale**: `diagnostics::TRY_OUTSIDE_FUNCTION` holds
  the message and help; sema uses it for every `try` with no target and
  the plan uses it for the `Module` owner, before any reason (no reason
  makes a `try` legal there). A namespace body's owner is the module
  owner, so it gets the same answer.

## Work log

- 2026-09-30: Reproduced (a) with `ttc -p`: the static-block `try` emitted
  `break $tt_v0;` inside `static { }`; a class-field `try` in a claimed
  `result` block panicked. Reproduced (b) with `ttc --check`: the value
  forms in a match arm, an array, a conditional branch, and a `for...of`
  head at module or namespace level all reported the "expression context"
  wording.
- 2026-09-30: Changed `in_function_body` and `user_function_depth_at` to
  count `function_target_brace` boundaries; added `TRY_OUTSIDE_FUNCTION`.
- 2026-09-30: Added `tests/cases/compiler/tryInClassCodeInsideResult.tt`
  and `tests/cases/compiler/tryAtModuleTopLevel.tt`;
  `UPDATE_EXPECT=1 TT_CASES=... cargo test --test case_baselines`.
- 2026-09-30: `TT_MATRIX_CASES=all cargo test --test case_baselines`:
  three `try_*_topLevel_*` matrix cases now report the module reason
  (Issue 1); accepted with `UPDATE_EXPECT=1 TT_MATRIX_CASES=all
  TT_CASES=_topLevel_`.
- 2026-09-30: `cargo test --lib`: `flow::tests::module_and_non_function_braces_are_not`
  asserted that class braces are not boundaries (Issue 2). `cargo test
  --test compile`: four tests asserted the old wordings (Issue 3).

## Issues and resolutions

### Issue 1: Three matrix baselines held the generic wording

- **Symptom**: `try_binaryOperand_topLevel_plain`,
  `try_memberOfValue_topLevel_await`, and `try_siblings_topLevel_spread`:
  modified `.errors.txt`, 16 lines "`try` cannot be used in this expression
  context" became "`try` must be inside a function".
- **Cause**: Decision 2; these are module-level value-form `try`s.
- **Resolution**: Accepted after reading the diff.

### Issue 2: A unit test pinned class braces as transparent

- **Symptom**: `flow::tests::module_and_non_function_braces_are_not`
  failed on `class A { static { HERE; } }`.
- **Cause**: Decision 1 reverses that fact for `in_function_body`.
- **Resolution**: The three class lines moved to a new test,
  `class_code_is_a_boundary_of_its_own`, which asserts the opposite and
  adds a class expression's static block.

### Issue 3: Four compile tests pinned the value-region wording

- **Symptom**: `try_inside_match_arm_is_an_error`,
  `an_inline_chain_bottoming_out_in_an_iife_still_rejects_try`,
  `try_placement_reports_the_owning_reason` (`tests/compile/cases_03.rs`)
  and `a_misplaced_try_covers_the_propagation` (`cases_09.rs`) expected
  "`try` cannot be used here, in an isolated value region".
- **Cause**: Each is a module-level `try` in a match arm, whose reason is
  that no function encloses it (Decision 2); their comments described the
  arm as an IIFE, which it has not been since the match lowering stopped
  emitting one.
- **Resolution**: They now expect "`try` must be inside a function"; the
  stale comments are removed. Spans and the other assertions are
  unchanged.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tryInClassCodeInsideResult.tt` and
  `tests/cases/compiler/tryAtModuleTopLevel.tt`
  (`cargo test --test case_baselines`).
- **Observed failure**: without the changes under `src/`, the first case
  panics at `src/codegen/core/emitter/source.rs:1337:21` ("unscheduled
  expression try reached inline emission") and fails; the second's
  `.errors.txt` is out of date: five value-form `try`s report "`try`
  cannot be used in this expression context — propagating its `Err` would
  require moving an evaluation across its TypeScript control-flow
  boundary" and the match-block-arm `try` "`try` cannot be used here, in
  an isolated value region — ...", instead of "`try` must be inside a
  function".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (targeted: `case_baselines` with `TT_MATRIX_CASES=all`,
  `--lib`, `compile`, `snapshot`); the full gate is recorded in TASK-692,
  which ends this batch.
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/flow/syntax.rs`, `src/diagnostics.rs`, `src/sema/checker.rs`,
`src/lib/compile.rs`, `src/flow/tests.rs`, `tests/compile/cases_03.rs`,
`tests/compile/cases_09.rs`, three matrix `.errors.txt` baselines,
`docs/ai/tt.md`, `docs/design/try-result-scopes.md`, `docs/tasks/INDEX.md`;
added the two cases and their baselines.
