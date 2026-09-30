# TASK-682: Report a `try` in a C-style `for` test as try-placement at the `try`

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-682`

## Purpose

A `try` in a C-style `for` test is documented as rejected with
`try-placement` ("`while`/`do` test; C-style `for` test/update: Reject
`RepeatedInOwner`", `docs/design/try-result-scopes.md` §4.5, and §5.7's
`for (; try ready(); ) work();`), as it is in a `while` test. Instead
`for (; flag && try read(1); )` reported `lowering-plan-failed: the
evaluation position ... maps to no source` at 1:1, `for (; try read(1); )`
reported `lowering-plan-failed: a propagation would repeat in its loop
header`, and `for (; flag ? try read(1) : 0; )` reported
`source-not-typescript`. TASK-679 Issue 2 found it through six matrix
cases (`try_*_forTest_*`).

## Scope

- Included: the `try` claim in `src/parser/parse.rs`, the two compile tests
  that pinned the old diagnostic (`tests/compile/cases_03.rs`,
  `tests/compile/cases_04.rs`), the case `tests/cases/compiler/tryInForTest.tt`,
  six matrix baselines, and `tests/oracle-failures.txt`.
- Excluded: the placement rule itself, which Evaluation IR already derives
  for a `while` test (`OwnerReach::Repeated` → `RepeatedInOwner`).

## Decisions

### Decision 1: A `try` in a `for` test is the value form, as in every other expression position

- **Context**: The parser claimed a `try` that is not at a statement start
  as the value form (`TryExpr`), except anywhere in a `for` head's test
  clause, where it kept the statement grammar (`TryStmt`) "so Evaluation IR
  can report the repeated evaluation". A statement `try` is a `Propagate`
  root, which the lowering plan refuses with an internal
  `RepeatedPropagation` error rather than a placement fact; in the middle of
  an operand, the statement claim also swallowed the clause's `;`, so the
  projection could not map the host span back to the source (the 1:1
  report), and in a `? :` branch the statement parse failed and the `try`
  was left as TypeScript that does not parse.
- **Alternatives considered**:
  - Turn `RepeatedPropagation` into a `try-placement` diagnostic and fix the
    span mapping: two repairs to keep a claim whose grammar is wrong for the
    position (a `for` test is an `Expression`, ECMA-262 §14.7.4
    `ForStatement : for ( [lookahead ≠ let [] Expression ; Expression ;
    Expression ) Statement`), and it still leaves the ternary unparsed.
- **Decision and rationale**: Remove the exception (and the now-unused
  `for_head_clause`): a `for` test `try` is the value form like the one in a
  `while` test, and Evaluation IR's existing reach model (`ForStmt::Test` is
  `Repeated` in `frequency_within_owner`/`owner_reach`) reports
  `try-placement` "a repeated loop position" at the `try` for every shape
  of the test. The initializer clause is unaffected (a declaration's
  `= try` is claimed by `parse_try_decl`, and an expression initializer was
  already the value form); the accepted `for (let i = try n();;)`,
  `for (i = try n();;)`, `for (try n();;)`, and `for (const x of [try n()])`
  were rechecked.

### Decision 2: The compile tests that pinned `lowering-plan-failed` now pin `try-placement`

- **Context**: `placement_matrix_prerequisite_gate` expected
  `LoweringPlan` for `for (; try ready(); ) {}`, and
  `try_in_repeated_for_test_reports_a_located_lowering_diagnostic` expected
  `lowering-plan-failed` at the `try`. Both recorded the stopgap, not the
  documented rule.
- **Decision and rationale**: The row expects `Placement` (the `LoweringPlan`
  expectation is removed), and the second test is replaced by
  `try_in_a_for_test_is_a_repeated_loop_placement_at_the_try`, which checks
  the whole test, an `&&` operand, and a `? :` branch each report exactly one
  `try-placement` at the `try`.

## Work log

- 2026-09-30: Reproduced with `ttc --check` for the whole test, an `&&`
  operand, a parenthesized operand, an update, and a `? :` branch; only the
  parenthesized operand and the update were already `try-placement`.
- 2026-09-30: Removed the test-clause exception; all shapes now report
  `try-placement` at the `try`.
- 2026-09-30: Updated the two compile tests; added the case `tryInForTest`
  (errors only); removed the six `TASK-679 Issue 2` lines from
  `tests/oracle-failures.txt`; ran `TT_MATRIX_CASES=all TT_CASES=forTest
  cargo test --test case_baselines` and accepted the six changed baselines
  with `scripts/baseline-accept` after reading them (each now reports
  `try-placement` at the `try` on the line of the `for`, replacing the 1:1
  `lowering-plan-failed`).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tryInForTest.tt`;
  `tests/compile/cases_04.rs` `try_in_a_for_test_is_a_repeated_loop_placement_at_the_try`;
  the matrix cases `try_{binaryOperand_forTest_exception,matchOperand_forTest_optionalChain,operand_forTest_plain,operand_forTest_spread,operand_forTest_using,parenthesized_forTest_await}`.
- **Observed failure**: with the non-test change reverted, the case failed
  with `tryInForTest: does not report try-placement`, and the compile test
  panicked finding no `try-placement` diagnostic; the matrix cases were
  listed as `does not report try-placement` (TASK-679 Issue 2).

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=forTest cargo test --test case_baselines`: pass.
- [x] `cargo test --lib`, `--test compile`, `--test case_baselines`,
  `--test snapshot`, `--test passthrough`, `--test corpus`: pass.
- [x] Baseline changes reviewed and committed with the change.
- The full gate is recorded in TASK-684.

## Result

Changed files: `src/parser/parse.rs`, `tests/compile/cases_03.rs`,
`tests/compile/cases_04.rs`, `tests/cases/compiler/tryInForTest.tt` and its
`.errors.txt` baseline, six `try_*_forTest_*` baselines,
`tests/oracle-failures.txt`, `docs/tasks/INDEX.md`, and this record.
