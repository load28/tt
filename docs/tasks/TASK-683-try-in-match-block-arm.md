# TASK-683: Keep the function target of a `try` in a match block arm

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-683`

## Purpose

`match (b) { true => { const n = try read(2); return n; }, false => 0 }`
inside a Result-returning function was rejected with `try-placement` ("in
an isolated value region"), while the expression arm `true => try read(1)`
was accepted. `docs/design/try-result-scopes.md` §4.6 says of an isolated
value region (a value-producing match arm) that the "function target remains
Legal if no outer ResultRegion is crossed". TASK-679 Issue 3 found it
through seven matrix cases (`tryStatement_*_matchBlockArm_*`).

## Scope

- Included: the statement `try` check in `src/sema/checker.rs` and the
  boundary index it reads (`src/sema.rs`, `HirFile::match_owned_tokens` in
  `src/hir/mod.rs`, moved from `src/core_ir/lower.rs`); the evaluation owner
  of values inside a match's arms (`src/program_syntax.rs`,
  `src/program_syntax/visit.rs`, `src/program_syntax/collector.rs`); the
  removal of the blanket block-arm rejection in
  `src/evaluation_ir/evaluation.rs`; `docs/ai/tt.md`; the case
  `tests/cases/compiler/tryInMatchBlockArm.tt` with `@run`; eight matrix
  baselines; `tests/oracle-failures.txt`.
- Excluded: a `try` in a conditional operation (TASK-684).

## Decisions

### Decision 1: Follow the design document; the block arm keeps the function target

- **Context**: Either the compiler moves to §4.6, or §4.6 and
  `docs/ai/tt.md` say that a block arm is excluded (TASK-679 Issue 3).
- **Alternatives considered**:
  - Exclude block arms in the documents: it would make the two arm forms of
    one match differ for no semantic reason. A block arm's `return` already
    delivers the arm value through a captured exit, not a JavaScript
    `return`, so a propagation's `return` in the same arm is unambiguous;
    Rust, which tt's `?`/`match` follow, lets `?` inside a block arm leave
    the enclosing function (The Rust Reference, "The question mark
    operator": it returns from the enclosing function or closure; a block
    expression is neither).
- **Decision and rationale**: The documents stand. The lowering already
  writes every accepted match as statements in its owner, so a block arm's
  propagation is emitted where the function can return it (checked at run
  time by the case below, Ok and Err paths, in a declaration, a template, a
  conditional branch, a concise arrow body, a `while` test, and a nested
  match).

### Decision 2: The statement form resolves its nearest Result scope instead of reading the region it was parsed in

- **Context**: `check_try` rejected every statement `try` in an isolated
  region unless a function was written inside that region; the region fact
  cannot see the function around the match. The file-wide boundary model
  could, but the lexer marks an arm block's `{` as a function body, so the
  arm itself would have been found as the target.
- **Decision and rationale**: The checker builds the boundary index
  (`FunctionTargets`) once per file with the match bodies and arm arrows
  skipped, the same set Core IR already used for its generator check (now
  `HirFile::match_owned_tokens`, since it is a fact about HIR's matches).
  The nearest scope is a `result` block when the `try` sits in its body with
  no function between them (and a crossing when an isolated region is
  between them, as before); otherwise it is the innermost boundary, judged
  as TASK-681 made it: an ordinary function is legal; a constructor, a
  generator, a static block, class code outside a method, and no function
  at all are `try-placement`. A function written inside a region inside a
  `result` block is now judged as that function (§4.2, "`try` in a function
  nested inside a `result` block leaves that nested function"), where the
  checker used to report a crossing.

### Decision 3: A match's stand-in function in the projection is not an evaluation owner

- **Context**: The value form in a block arm's statement
  (`const k = 1 + try read(2);`) is placed by Evaluation IR from the SWC
  projection, where a match is written as a call of a stand-in function
  (`(() => { ...arms... })()`). That function was taken as the statement's
  owner (`FunctionBody`) even at a module's top level, which is why a
  blanket rule (`isolated_arm_values`) rejected value forms written directly
  in arm statements; removing only that rule let a module-level arm emit a
  `return` (`verify-failed`).
- **Decision and rationale**: The collector records the stand-in function
  of each `DecisionCallExpression` placeholder and `evaluation_owner` looks
  through its body edge, and the function target inside it is inherited from
  around the match: the arm's statements are lowered in the match's owner,
  so that is their owner. Placement then follows the ordinary owner rules
  (a module top level, a constructor, a generator, or a static block
  rejects), and the blanket rule is removed.

## Work log

- 2026-09-30: Reproduced the statement and value forms in block arms under
  every host kind with `ttc --check`.
- 2026-09-30: Rewrote `check_try` over a file-wide boundary index; moved
  `match_owned_tokens` to HIR; made `function_target_at` and
  `user_function_target_at` test-only (they remain the oracle of the
  index's unit test).
- 2026-09-30: Removed `isolated_arm_values`; found the module-level
  `verify-failed`; made the stand-in function transparent to the owner.
- 2026-09-30: The full test run found Issue 2; fixed it.
- 2026-09-30: Added `tryInMatchBlockArm` (`@run`); removed the seven
  `TASK-679 Issue 3` lines from `tests/oracle-failures.txt`; ran
  `TT_MATRIX_CASES=all TT_CASES=matchBlockArm cargo test --test
  case_baselines`: the seven cases compile and print what their twins
  print, and `tryStatement_propagate_matchBlockArm_yield` now names the
  generator as the reason; accepted with `scripts/baseline-accept` after
  reading the diffs.

## Issues and resolutions

### Issue 1: Removing the block-arm rule alone accepted a module-level arm

- **Symptom**: `const b = match (c) { true => { const k = 1 + try read(2);
  return k; }, false => 0 };` at a module's top level reported
  `verify-failed: Return statement is not allowed here`.
- **Cause**: The owner of the statement was the projection's stand-in
  function (Decision 3).
- **Resolution**: Decision 3; it is now `try-placement`.

### Issue 2: Two compile tests failed

- **Symptom**: `match_arm_return_try_reports_placement_instead_of_panicking`
  expected `try-placement` for `true => { return try g(); }` in a concise
  arrow's match; `try_inside_a_function_inside_a_template_interpolation_is_allowed`
  (`${run(() => { try g(); return h(); })}`) reported `try-placement`.
- **Cause**: The first pinned the rejection this task removes (the arrow is
  the function the propagation returns from, and the emitted code does so).
  The second is a template literal: the file's token stream holds it as one
  token, so the boundary index cannot see the arrow in the interpolation.
- **Resolution**: The first test is now
  `match_arm_return_try_propagates_from_the_concise_arrow`, which checks the
  emitted propagation. For a `try` inside a token that spans it, the
  checker takes the function the `try`'s own region records (the region was
  parsed from the interpolation), and otherwise the boundary around the
  template.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tryInMatchBlockArm.tt`, and the matrix
  cases `tryStatement_declaration_matchBlockArm_finally` and
  `tryStatement_propagate_matchBlockArm_{await,exception,optionalChain,plain,spread,using}`.
- **Observed failure**: with the non-test changes reverted, the case
  failed its baselines, reporting `try-placement` for every block-arm
  `try` and printing no output; the matrix cases were listed as `does not
  compile cleanly` (TASK-679 Issue 3).

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=matchBlockArm cargo test --test case_baselines`: pass.
- [x] `cargo test --lib`, `--test compile`, `--test snapshot`,
  `--test passthrough`, and `TT_MATRIX_CASES=all cargo test --test
  case_baselines`: pass.
- [x] Baseline changes reviewed and committed with the change.
- The full gate is recorded in TASK-684.

## Result

Changed files: `src/sema.rs`, `src/sema/checker.rs`, `src/hir/mod.rs`,
`src/core_ir/lower.rs`, `src/flow/mod.rs`, `src/flow/syntax.rs`,
`src/program_syntax.rs`, `src/program_syntax/collector.rs`,
`src/program_syntax/visit.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `docs/ai/tt.md`,
`tests/compile/cases_01.rs`, `tests/cases/compiler/tryInMatchBlockArm.tt` and its baselines, eight
`tryStatement_*_matchBlockArm_*` baselines, `tests/oracle-failures.txt`,
`docs/tasks/INDEX.md`, and this record.
