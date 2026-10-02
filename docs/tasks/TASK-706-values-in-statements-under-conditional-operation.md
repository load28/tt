# TASK-706: Write a value in a statement of a block a conditional operation holds

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-706`

## Purpose

TASK-700 (Issue 8) found that
`return flip() && result { const n = try read(x) * 2; return n; };`
reports `verify-failed` ("generated TypeScript failed to parse: Expression
expected"), and the same in `||`, `??`, a conditional branch, and the JSX
conditional positions, while `try read(x)` as a whole initializer there
compiles. A `result` block is a value TASK-684's conditional-operation
lowering accepts in those operands, so its body must lower as it does
anywhere else, with the operation's short-circuiting kept.

## Scope

- Included: which values a conditional operation consumes
  (`consumed_exprs` in `src/codegen/core/planning.rs`), a compiler case
  with `@run` and a twin, the six listed defects in
  `tests/oracle-failures.txt` and their baselines.
- Excluded: the planning of conditional operations themselves
  (`src/evaluation_ir/planning.rs`), which was right.

## Sources

- ECMA-262 §13.13 (Binary Logical Operators), §13.14 (Conditional
  Operator), and §13.3.9 (Optional Chains): the right operand, the selected
  branch, and an optional call's arguments are evaluated only when the
  operation reaches them; the case's `@run` output and its twin check it.
- `docs/ai/tt.md` (match, "A value under `&&`/`||`/`??`/`? :`/`f?.()`
  lowers the WHOLE operation as one region") and TASK-684 (Decision 1, and
  Issue 2: an operation's values are emitted by the operation, and their
  authored occurrences inside the replaced operation span print nothing).

## Decisions

### Decision 1: An operation consumes only the values of its own owner

- **Context**: The emitted body read `const n =  * 2;`: the `try`'s slot
  was not written where it stands. `consumed_exprs` marks the values whose
  authored occurrence prints nothing because the operation's region emits
  them. It took every compose value of every owner whose source lies inside
  any compose operation's span. The statement `const n = try read(x) * 2`
  in the block's body is its own owner with its own compose rewrite; its
  `try` lies inside the span of the `&&` around the block, so it was taken
  as consumed by that operation, and its occurrence printed nothing.
- **Alternatives considered**: (a) Exclude values inside a `result` body
  specifically: a match block arm (`flag && match (n) { _ => { const d = 2
  * try read(n); return d; } }`) and an arrow body in an arm (`() => 2 *
  Number(match ...)`) fail the same way, the last one silently (Issue 2),
  so a rule about one construct leaves the others. (b) Test containment
  against the value's owner span instead of the value's own span: owners
  nest in the same way, so the question would still mix owners. (c) Pair
  each operation only with the values of the compose rewrite that holds it.
- **Decision and rationale**: (c). A compose rewrite is one host owner's
  plan, and the values an operation re-emits (its condition's values and
  the values before its conditional step, TASK-684 Issue 2) are values of
  that same plan; a value of another owner, however nested, is emitted by
  that owner's own walk. The rule now says that, for every construct.

## Work log

- 2026-10-01: Reproduced with `--no-verify`: `const n =  * 2;`, also with
  `2 * try read(x)`, `String(try read(x))` (emitted `$tt_v10()`), a
  ternary branch, an optional call's argument, a match block arm, and an
  arrow in an arm whose call argument is a match.
- 2026-10-01: Found the span test in `consumed_exprs`; limited it to the
  compose rewrite that holds the operation.
- 2026-10-01: Added
  `tests/cases/compiler/valueRegionStatementsInConditionalOperand.tt`
  (`@run`, `@twin`): `&&`, `||`, `??`, `? :`, an optional call, a match
  block arm, and an arrow in an arm, each with inputs that take and skip
  the operand and with a failing `try`. Removed the six lines from
  `tests/oracle-failures.txt`; `UPDATE_EXPECT=1
  TT_CASES=source-not-typescript cargo test --test case_baselines`
  removed their `.errors.txt` (the fixed cases now compile cleanly and
  print what their twins print).

## Issues and resolutions

### Issue 1: The verify failure

- **Symptom**: as in Purpose.
- **Cause**: Decision 1.
- **Resolution**: Decision 1.

### Issue 2: The same cause also miscompiled silently

- **Symptom**: `flag && match (n) { 0 => () => 0, _ => () => 2 *
  Number(match (n) { 1 => 3, _ => 4 }) }` emitted `2 * $tt_v8()`: `Number`
  was called without its argument, so the arrow returned `0` instead of
  `6`, with no diagnostic, because the dropped value was a whole argument
  and the remaining text still parsed.
- **Cause**: Decision 1: the inner match is a value of the arrow body's
  owner.
- **Resolution**: Decision 1; the case's `later` rows pin it.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/valueRegionStatementsInConditionalOperand.tt`
  (`cargo test --test case_baselines`), and the matrix cases
  `source-not-typescript_resultBody_{conditionalBranch,jsxConditional,jsxTernary,logicalAnd,logicalOr,nullish}_fixed`.
- **Observed failure**: Without the change in `src/codegen/core/planning.rs`,
  the case reported `error[verify-failed]: generated TypeScript failed to
  parse: Expression expected` at `main.tt:16:60` and later lines, so it was
  not run (`.stderr` says so) and five of its baselines differed; the
  matrix cases were listed as not compiling cleanly.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 TT_CASES=source-not-typescript cargo test --test
  case_baselines`: passed. The full gate, with every matrix case, runs once
  at the end of the batch (see TASK-712).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/codegen/core/planning.rs`, `tests/oracle-failures.txt`, the
compiler case and its baselines, and the six removed matrix baselines. A
tt value inside a statement of a `result` block, a match block arm, or a
function that a conditional operation's operand holds is written where it
stands.
