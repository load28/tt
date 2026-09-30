# TASK-684: Lower a `try` nested in an operand of a conditional operation

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-684`

## Purpose

`flag && (try a) + (try b)`, `c ? (try a) + (try b) : d`, `h?.(try r * 2)`,
`h?.((try r).toFixed(2))`, and the same shapes in JSX `&&` and `? :` were
rejected with `try-placement` ("cannot be rebuilt without changing
evaluation order"), while a single `try` that is a whole branch or argument
was accepted. `docs/ai/tt.md` ("try") says the value form "preserves
left-to-right, conditional, optional-call, and concise-arrow evaluation",
and `docs/design/try-result-scopes.md` §4.5 lists logical, nullish, ternary,
and optional-call argument positions as legal. TASK-679 Issue 4 found it
through nine matrix cases. The task also takes TASK-679 Issue 5 (a
TypeScript error in a tuple match's generated test placed under the whole
`match (...)` head), which is small and a clear defect.

## Scope

- Included: conditional-operation planning
  (`src/evaluation_ir/planning.rs`, `PlannedBranch`, `PlannedOperand`, and
  the `active` documentation in `src/evaluation_ir.rs`), its emission
  (`src/codegen/core/emitter/host.rs`, `mod.rs`, `src/codegen/core/mod.rs`,
  `src/codegen/core/planning.rs`), the variant test's source context
  (`src/codegen/core/emitter/pattern.rs`), a unit test in
  `src/evaluation_ir/tests.rs`, the cases
  `tests/cases/compiler/tryInConditionalOperand.tt` (`@run`) and
  `tests/cases/compiler/tupleMatchImpossiblePattern.tt`, nine matrix
  baselines, and `tests/oracle-failures.txt`.
- Excluded: optional-chain member and index tails and `switch` case tests,
  which remain `UnmodeledConditional` as §4.5 documents.

## Decisions

### Decision 1: A conditional operation owns every value in its branch or argument, not only one that is the whole of it

- **Context**: Evaluation IR groups the tt values whose schedule has one
  conditional step into a whole conditional operation (the branch runs only
  when the condition selects it). The plan refused a group of more than one
  value in a logical operation, more than one value in a ternary side, and
  any optional-call argument value with evaluation steps before the
  conditional step (a value inside `r * 2` or `(r).toFixed(2)`); refused
  groups fell back to the expression-boundary reason, which is reported as
  `try-placement`.
- **Alternatives considered**:
  - Leave the rejection and change the documents: the single-value forms
    already lower these positions, and the compose machinery that lowers
    `(try a) + (try b)` outside a condition is the same work inside a branch.
  - Lower each value as its own conditional operation: two operations over
    one condition would evaluate the condition twice or need a shared
    capture, and the branch text could not be rebuilt once.
- **Decision and rationale**: The plan keeps a branch's values in source
  order (`PlannedBranch::Values`) and records, for every value that is not
  the whole branch or argument by itself, its active plan: the branch or
  argument span and the steps between the value and it. An optional call's
  argument holding values inside a larger expression is
  `PlannedOperand::Composed`. The emitter evaluates those values in order
  inside the branch (each into its slot, followed by its steps, which
  capture the source between values), then writes the branch or argument
  rebuilt from their slots. JavaScript evaluates the condition first and an
  operand only when it is selected (ECMA-262 §13.13, Binary Logical Operators, and §13.14, Conditional Operator:
  `LogicalANDExpression`, `CoalesceExpression`, `ConditionalExpression`),
  and an optional call's arguments only past its nullish test (§13.3.9, Optional Chains,
  `OptionalChain`); the plan keeps all of it, which the case's `@run`
  output and the matrix twins check.

### Decision 2: A capture in the branch reads a value already written in that branch from its slot

- **Context**: The first run of `flag && (try a) + note() + (try b)` emitted
  `const $tt_v3 = (() + note("middle", 10));`: the capture before the second
  value covers the first value's source, and the capture writer lowered
  that value again as an expression, which a value that is a statement
  (a propagation) cannot be.
- **Decision and rationale**: The emitter records the values a conditional
  branch has written, and the capture writer reads such a value from its
  slot, as it already did for the values of a composed statement
  (`slot_exprs`).

### Decision 3: A tuple match's variant test carries its pattern as source context (TASK-679 Issue 5)

- **Context**: A single match's `case "Slow":` label is anchored with the
  pattern's tag as context, so TypeScript's error on it is reported at the
  pattern. A tuple match tests with `$tt_m1.kind === "Slow"`, which had no
  context, so TS2367 on that comparison was reported under `match (a, b)`.
- **Decision and rationale**: The variant test is anchored the same way as
  the label (`anchored_with_context` with the constructor's tag). The error
  is legitimate (the case cannot occur), and now points at the pattern
  that wrote it.

## Work log

- 2026-09-30: Reproduced the four shapes; read the planner's refusals
  (`members.len() != 1`, one value per ternary side, `conditional_index !=
  0` for optional calls).
- 2026-09-30: Implemented Decision 1; found the capture defect (Decision 2)
  with the new case; fixed it.
- 2026-09-30: Removed the nine `TASK-679 Issue 4` lines from
  `tests/oracle-failures.txt`; ran `TT_MATRIX_CASES=all TT_CASES=try_ cargo
  test --test case_baselines`: the nine cases compile and print what their
  twins print; accepted their baselines with `scripts/baseline-accept`
  after reading them.
- 2026-09-30: Reproduced Issue 5 with `ttc --check-types` and fixed it
  (Decision 3); added `tupleMatchImpossiblePattern`.

## Issues and resolutions

### Issue 1: A capture rebuilt a branch value that had already run

- **Symptom**: `verify-failed: Expression expected` at the first `+` of
  `flag && (try read(n)) + note("middle", 10) + (try read(n + 1))`.
- **Cause**: Decision 2.
- **Resolution**: Decision 2.

### Issue 2: A later operation's capture read an earlier operation's value twice

- **Symptom**: The full gate's `runtime_sibling_conditional_trys_each_evaluate_their_operand_in_order`
  (`tests/integration/cases_05.rs`) failed: `(n && try p(a)) + (n && try
  p(b))` emitted `const $tt_v12 = ($tt_v13$tt_v7);`, which TypeScript
  rejects (TS2304).
- **Cause**: The set of delivered values (Decision 2) outlived the
  operation that wrote them, so the capture of the first operation, whose
  source is already replaced by the operation's result slot, also wrote the
  inner value's slot.
- **Resolution**: An operation's values leave the set when the operation's
  emission ends: outside it, only its result slot stands for them.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tryInConditionalOperand.tt`,
  `tests/cases/compiler/tupleMatchImpossiblePattern.tt`,
  `src/evaluation_ir/tests.rs` `several_values_in_one_branch_join_one_operation`,
  and the matrix cases `try_binaryOperand_optionalCall_finally`,
  `try_memberOfValue_optionalCall_plain`, and
  `try_siblings_{conditionalBranch_optionalChain,jsxConditional_plain,jsxTernary_await,logicalAnd_using,logicalOr_exception,nullish_plain,optionalCall_exception}`.
- **Observed failure**: with the non-test changes reverted,
  `tryInConditionalOperand` failed its baselines with `try-placement`
  ("cannot be used in this conditional operation") for each multi-value
  operand, and `tupleMatchImpossiblePattern` placed TS2367 at 7:10 (the
  `match (a, b)` head) instead of 9:9 (`Slow`); the matrix cases were
  listed as `does not compile cleanly` (TASK-679 Issue 4).

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=try_ cargo test --test case_baselines`: pass.
- [x] `cargo test --lib`, `--test compile`, `--test snapshot`,
  `--test passthrough`, `--test case_baselines`: pass.
- [x] Full gate (all four tasks), serialized:
  `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_MATRIX_CASES=all TT_BASELINE_TRACKING_DIR=<dir>
  cargo test --no-fail-fast`; `node scripts/check-baselines --tracking
  <dir>`; `./scripts/ci agents`. `fmt`, `clippy`, and every test target
  passed except the integration test of Issue 2 (all 3,286 matrix cases
  included, 2,076 s); `check-baselines`: "9265 compared, none unused";
  `ci agents`: passed (its warnings are the missing `rolldown` and the
  release `ttc`/VSIX artifacts `./scripts/setup` builds, which were not
  rebuilt). After the Issue 2 fix: `clippy`, `--test integration` (215),
  `--test compile` (579), `--test snapshot`, `--test case_baselines`, and
  `TT_MATRIX_CASES=all TT_CASES=try_ --test case_baselines` pass.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`,
`src/evaluation_ir/tests.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/mod.rs`,
`src/codegen/core/emitter/host.rs`, `src/codegen/core/emitter/pattern.rs`,
the two cases and their baselines, nine matrix baselines,
`tests/oracle-failures.txt` (now empty of entries: every TASK-679 defect is
fixed), `docs/tasks/INDEX.md`, and this record.
