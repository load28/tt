# TASK-692: Write a logical operation's right-operand branch as the operation over its stored operand

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-692`

## Purpose

TASK-626 left three differences from TypeScript in the type of a logical
operation that holds a tt value: (a) `n && …` with `n: number` is typed
`number` where TypeScript gives `0`; (b) `true || try r()` still includes
the right operand's type, which TypeScript leaves out because the right
operand never runs; (c) TypeScript's syntactic checks of the left operand
(TS2871 "This expression is always nullish", TS2873 "This kind of
expression is always falsy", and their siblings TS2869 and TS2872) are not
reproduced. Each is examined against the pinned checker, fixed where the
emitted program can carry it without type tricks, and otherwise recorded.

## Scope

- Included: the branch of a lowered `&&`, `||`, or `??` that runs the
  right operand (`src/codegen/core/emitter/host.rs`); tests that pinned
  its text; a case file; `docs/design/program-lowering.md` §7.10;
  `docs/ai/tt.md`; pointers in TASK-626, TASK-691, TASK-693.
- Excluded: the ternary, whose condition is not a value of the operation;
  the host's join and widening rules (`src/typescript/host.mjs`).

## Decisions

### Decision 1: The right-operand branch writes `$l <op> v'`, not `v'`

- **Context**: The pinned checker (TypeScript 7.1.0-dev.20260826.1,
  commit 5739027c, `tsc/internal/checker/checker.go`,
  `checkBinaryLikeExpressionWorker`) types `l && r` as `l`'s type when
  `!hasTypeFacts(l, TypeFactsTruthy)`, else
  `extractDefinitelyFalsyTypes(l) | r`; `l || r` as `l`'s type when
  `!hasTypeFacts(l, TypeFactsFalsy)`, else
  `getNonNullableType(removeDefinitelyFalsyTypes(l)) | r`; `l ?? r` as
  `l`'s type when `!hasTypeFacts(l, TypeFactsEQUndefinedOrNull)`, else
  `getNonNullableType(l) | r`. The lowering (TASK-595, TASK-626) writes
  `$r = $l` in one branch and `$r = v'` in the other, and the host types
  `$r` as the union of what is written. In `if ($l = true) {…} else { $r = v' }`
  the else branch narrows `$l` to `never` but TypeScript does not take it
  for unreachable, so `v'`'s type joined the result (`true | number`).
- **Alternatives considered**: (a) Decide in the lowering that the branch
  cannot run and drop it: ttc has no types, and a syntactic decision would
  cover literals only (TASK-626 Decision 1 (a)). (b) Have the host leave
  out an incoming value whose branch narrows the condition to `never`: the
  host would model TypeScript's operator typing in JavaScript, a second
  type system (contract 2, "ttc infers no types"). (c) Write the branch's
  value as the operation itself over the stored operand: `$r = $l || v'`.
  `$l` is a local the test already evaluated; `ToBoolean` and the nullish
  test have no observable effects (ECMA-262 §7.1.2, §13.13.1), so the
  value is `v'` exactly as before, and TypeScript types the expression by
  its own rule over `$l` as the test narrowed it.
- **Decision and rationale**: (c). For a left operand that is never falsy
  (`||`), never truthy (`&&`), or never nullish (`??`), the narrowed `$l`
  is `never`, `getTypeFacts(never)` is `TypeFactsNone`, so the expression
  is `never` and the result has the left operand's type alone; checked
  with the pinned `tsc` on hand-lowered TypeScript (`orTrue(): true`,
  `orObject(): { a: number }`, `andNull(): null`). For every other left
  operand the union is the same set of types as before (the falsy or
  nullish part the expression adds is already written by the other
  branch). The right operand's contextual type is the operation's, as in
  the source. The operation's region still evaluates each operand once.

### Decision 2: `0` for `number && …` is not carried (open)

- **Context**: `extractDefinitelyFalsyTypes` maps `number` to `0`,
  `string` to `""`, and `bigint` to `0n` (`getDefinitelyFalsyPartOfType`);
  it maps the type, it does not narrow a reference. The only value
  written for the falsy case is `$l` narrowed by falsiness, which stays
  `number` (`NaN` and `-0` are falsy, and TypeScript's truthiness
  narrowing filters constituents, it does not map them).
- **Alternatives considered**: (a) `if ($l === 0)` narrows to `0` but
  sends `NaN` down another path, which then still writes `number`.
  (b) Write `$l && <never-typed helper>()` in the falsy branch: TypeScript
  would type it `0`, but the helper exists only to steer the checker, a
  type trick contract 2 rules out. (c) `$r = $l && $v` after the branches,
  with `$v` written only in the truthy one: TypeScript does not correlate
  the two tests, so `$v` is used before being assigned (TS2454) unless its
  type admits `undefined`, which then joins the result.
- **Decision and rationale**: Left open and documented
  (`docs/ai/tt.md`, `docs/design/program-lowering.md` §7.10). The result
  is a supertype of TypeScript's, so reading it type-checks; assigning it
  where only `0 | …` is accepted does not.

### Decision 3: TypeScript's syntactic checks of a literal left operand are not reproduced (open)

- **Context**: TS2869, TS2871, TS2872, and TS2873 come from
  `checkNullishCoalesceOperandLeft` and `checkTruthinessOfType`, which
  call `getSyntacticNullishnessSemantics` and `getSyntacticTruthySemantics`
  on the operator's left operand: they judge its syntax kind (a literal,
  `null`, `void`, the `undefined` symbol), not its type. In the emitted
  code that operand is the identifier `$l` ("sometimes"), and the test is
  an assignment, which neither function looks through for truthiness.
- **Alternatives considered**: (a) Write a primitive-literal left operand
  again as the operation's left operand (`$r = "" || v'`), which is
  unobservable to re-evaluate. Implemented and measured: the pinned `tsc`
  reports TS2873 at `""` and TS2871 at `null`, mapped to the source
  literal, but the literal's fresh type then reaches the result slot,
  whose join widens a fresh literal at mutable storage (as TypeScript
  widens `let`), so `const b = true || try r()` became `boolean` where
  TypeScript gives `true` (and before this task `true | number`); the
  slot cannot know whether the source declared `const` or `let`.
  (b) Report the checks as tt diagnostics: a reimplementation of
  TypeScript's checks in ttc, which contract 2 leaves to TypeScript.
- **Decision and rationale**: Neither; left open and documented. The
  regression (a) introduces in the common `true || …` case outweighs
  reproducing an error that TypeScript reports for code whose right
  operand is dead.

## Work log

- 2026-09-30: Fetched `tsc/internal/checker/checker.go` at 5739027c and
  read `checkBinaryLikeExpressionWorker`, `getTypeFactsWorker`,
  `extractDefinitelyFalsyTypes`, `removeDefinitelyFalsyTypes`,
  `checkTruthinessOfType`, `getSyntacticTruthySemantics`,
  `checkNullishCoalesceOperandLeft`, `getSyntacticNullishnessSemantics`.
- 2026-09-30: Checked hand-lowered TypeScript with the pinned `tsc
  --strict --declaration`: `$l || $v` with `$l: true` gives `true`;
  `$l && $v` with `$l: number` gives `string | number`; `"" || $v` and
  `null ?? $v` report TS2873 and TS2871.
- 2026-09-30: Implemented Decision 1 and, for Decision 3 (a), a
  `literal_left` syntax fact threaded from `program_syntax` through the
  plan; measured the widening and removed the latter.
- 2026-10-01: Updated five `tests/compile` assertions and the
  `guard-evaluation-owner` fixture (Issue 1); `UPDATE_EXPECT=1 cargo test
  --test case_baselines` (eight baselines, Issue 1); added
  `tests/cases/compiler/logicalOperationRightNeverRuns.tt`.
- 2026-10-01: Merged `claude/ecstatic-dijkstra-qw5pf9` (dde5144) and ran
  the full gate for the batch (TASK-691 to TASK-694) on the merged tree,
  below; a container restart interrupted the first matrix runs, which
  were run again.

## Issues and resolutions

### Issue 1: Tests pinned the right branch as `$r = v'`

- **Symptom**: `a_capture_never_escapes_a_generated_conditional_region`,
  `a_conditional_operation_lowers_as_one_region` (`cases_09.rs`),
  `a_conditional_operand_owns_its_branch_in_a_pipeline_head`
  (`cases_14.rs`), and the two `a_match_under_a_conditional_operation_*`
  tests (`cases_11.rs`) failed; the `guard-evaluation-owner` fixture and
  eight `.ts`/`.map.txt` baselines differed.
- **Cause**: Decision 1 changes that line. Two tests asserted that the
  output holds no `&&` at all, which stood for "the operation is not
  evaluated again".
- **Resolution**: The assertions name the new line, and the two
  "no `&&`" assertions now check that the original operand is not
  re-evaluated (`flag &&`, `g() &&`). The baseline diffs change only that
  line (and mapping columns after it) except
  `conditionalOperationNarrowing.ts`, whose result slot annotation reads
  `(number | "") | (string | null)`: the `&&` branch's value is now
  `"" | number` (`extractDefinitelyFalsyTypes` of the narrowed `string`),
  the same union of types; its `.types` baseline is unchanged.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/logicalOperationRightNeverRuns.tt`
  (`cargo test --test case_baselines`).
- **Observed failure**: without the change to
  `src/codegen/core/emitter/host.rs`, its `.types` baseline is out of
  date: `orTrue: number | true`, `orObject: number | { a: number }`,
  `nullishObject: number | { a: number }`, `andNull: number | null`,
  `andFalse: number | false`, where the fix gives `true`, `{ a: number }`,
  `{ a: number }`, `null`, and `false`, TypeScript's types.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the batch's full gate, run on the tree merged with
  `claude/ecstatic-dijkstra-qw5pf9` (dde5144, TASK-687 to TASK-690):
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast` passed (30 test binaries and the doc tests), and
  `node scripts/check-baselines --tracking <dir>` reported "731 compared,
  none unused".
- [x] `TT_MATRIX_CASES=all cargo test --test case_baselines` (1116 s) and
  `TT_MATRIX_CASES=all TT_REQUIRE_EXTENSION=1 cargo test --test
  editor_cases` (1052 s) on the merged tree: pass.
- [x] `./scripts/ci agents`: passes (doctor reports the release `ttc` and
  VSIX that `./scripts/setup` builds as missing, which this batch did not
  run, as AGENTS.md asks).
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/codegen/core/emitter/host.rs`, `tests/compile/cases_09.rs`,
`tests/compile/cases_11.rs`, `tests/compile/cases_14.rs`,
`tests/fixtures/emit/guard-evaluation-owner/expected.ts`, eight
baselines, `docs/design/program-lowering.md`, `docs/ai/tt.md`,
`docs/tasks/TASK-626-literal-left-operand-narrowing.md`,
`docs/tasks/TASK-691-postfix-step-ends-optional-chain.md`,
`docs/tasks/TASK-693-one-coverage-question-per-match.md`,
`docs/tasks/INDEX.md`; added the case and its baselines. Issue (b) of
TASK-626 is fixed; (a) and (c) remain open for the reasons above.
