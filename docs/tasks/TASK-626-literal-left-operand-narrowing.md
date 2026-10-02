# TASK-626: Store a literal left operand of a logical operation so the test narrows it

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-626`

## Purpose

A literal left operand stayed in the type of a logical operation that
holds a tt value: `false || try read()` was typed `number | false`,
`true && match …` `string | true`, `null ?? try read()` `number | null`,
and `(false) || match …` `number | false`. TypeScript types each of these
as the right operand's type alone.

## Scope

- Included: the plan of a logical operation's condition
  (`src/evaluation_ir/planning.rs`), the round in which the TypeScript host
  settles operand storage (`src/typescript/host.mjs`),
  `docs/design/program-lowering.md` §7.10,
  `docs/design/contextual-type-materialization.md`, `docs/ai/tt.md`, and a
  case file.
- Excluded: the ternary's inert condition, which is not a value of the
  operation and keeps TASK-595's parenthesized in-place test; the left
  parts TypeScript's `&&` computes that no narrowing can express, and the
  right operand of an operation whose left operand is never falsy or never
  nullish (Issues 2 and 3); TypeScript 5.6's syntactic checks on a literal
  left operand (Issue 4).

## Decisions

### Decision 1: An inert left operand is stored in operand storage like any other

- **Context**: ECMA-262 §13.13.1 evaluates the left operand of `&&`, `||`,
  and `??` once, and its value is the result when the right operand does
  not run (§7.1.2 `ToBoolean` for `&&`/`||`, a `null`/`undefined` test for
  `??`). TypeScript types the operation from the left operand's type:
  `checkBinaryLikeExpressionWorker` in `checker.ts` gives `||` the union of
  `removeDefinitelyFalsyTypes(leftType)` and the right type, `&&` the
  union of `extractDefinitelyFalsyTypes(leftType)` and the right type, and
  `??` the union of `getNonNullableType(leftType)` and the right type, so
  `false || r()` has `r()`'s type (checked with the pinned
  TypeScript 7.1.0-dev.20260826.1: `false || read()`, `true && s()`,
  `null ?? read()`, `0 || read()`, `(false) || read()` are all the right
  type). TASK-595 stores the left operand in operand storage and tests the
  assignment (`if ($l = l)`), so TypeScript narrows `$l` where it becomes
  the result (the handbook's "Narrowing": truthiness narrowing and
  equality narrowing). A provably inert operand (§9 capture elision) was
  instead written twice in place, `if ((false)) { $r = (false); }`. A
  literal is not a reference, so nothing narrows it, and its type joined
  the result.
- **Alternatives considered**: (a) Decide the test at compile time from
  the literal's `ToBoolean` and drop the dead branch. That is constant
  folding by syntax in the lowering, which TypeScript does not do (it
  keeps the literal's type and narrows it), and it would handle literals
  only, not other inert operands. (b) Annotate the result slot from the
  operation's type computed by the host. The host sees only the lowered
  program, where the operation no longer exists. (c) Let the capture
  elision apply only where the elided input's value is not read again:
  a logical operation's left operand is its value, so it is stored.
  `$l` is then annotated with the literal's own type (`let $l: false`),
  and `if ($l = false)` narrows it to `never` in the branch that writes it
  to the result.
- **Decision and rationale**: (c). The inert operand takes the path every
  other left operand takes, so the result is typed by the same narrowing
  TypeScript applies, for `true`/`false`, numeric and string literals, and
  `null` alike. The test still evaluates the operand once, and an
  assignment is not a `false` keyword, so TypeScript's binder does not take
  the branch for unreachable (TASK-595 Issue 3).

### Decision 2: Operand storage settles before any other join is inferred

- **Context**: With Decision 1 alone, `false || try read()` stored
  `let $l: false` but its result slot `let $r: true | number`. The host
  answered operand storage and joins in the same round. There `$l` was
  still `let $l;`, whose evolving type widens the assigned `false` to
  `boolean`, which narrows to `true` in the truthy branch; the settled
  annotation `false` narrows to `never`. A join reading `$l`, or reading
  storage whose value reads it (the `result` block's slot through
  `const a = $r`), took the unsettled type.
- **Alternatives considered**: (a) Defer a join that reads pending
  operand storage (the work-in-progress patch left from the paused run:
  `readsPending(…, operandOnly)`). It follows declarations, not
  assignments, so the `result` block's join, which reads `$l` through `$r`,
  still settled early with `number | true`; following assignments would
  also make a join wait forever for operand storage that can never be
  annotated. (b) Annotate operand storage during contextual propagation.
  Its value may read storage that propagation has not settled yet, which
  is why TASK-595 waits for the fixed point. (c) Settle operand storage in
  a join round of its own: operand slots are answered first, and a round
  that answers one infers no other join. Operand storage that cannot be
  annotated gives no answer, so the joins proceed in that round.
- **Decision and rationale**: (c). Every join is then computed from
  settled operand storage, at the cost of one more checker round in a
  module whose operations store a left operand.

## Work log

- 2026-09-30: Reset onto `claude/ecstatic-dijkstra-qw5pf9` (2c918d2),
  `npm ci`. No repro for this task existed under
  `target/probe6-compiler`; wrote `tests/cases/compiler/literalLeftOperandNarrowing.tt`
  and generated its baselines on the unfixed code: `a: number | false`,
  `b: string | true`, `c: number | null`, `h: number | false`; `d`
  (`undefined`, an identifier, already stored) was `number`.
- 2026-09-30: Checked the equivalent TypeScript with the pinned `tsc`
  (`--declaration`): every form has the right operand's type.
- 2026-09-30: Reviewed the work-in-progress patch. Kept its planning
  change (the inert condition of a logical operation becomes a `Source`
  input, reusing its reserved slot name). Replaced its host change
  (Decision 2 (a)) with the ordering of Decision 2 (c).
- 2026-09-30: The `conditionalOperationNarrowing.ts` baseline was stale on
  the base (`number | boolean` for the `&&` storage); the base accepted
  `number | false` under TASK-623 (its Issue 2), merged here, and this
  task leaves the baseline as the base has it.
- 2026-09-30: `cargo test --test compile --test snapshot --test
  case_baselines`, and the `conditional`/`logical`/`nullish`/`operand`
  tests of `native` and `integration`.

## Issues and resolutions

### Issue 1: The result slot joined the operand's unsettled type

- **Symptom**: after the planning change, `orFalse` returned
  `TErr<string> | { kind: "Ok"; value: number | true }`.
- **Cause**: Decision 2's context.
- **Resolution**: Decision 2.

### Issue 2: `&&` maps a falsy non-literal primitive to its falsy literal

- **Symptom**: `n && match …` with `n: number | null` is typed
  `number | null | string`; TypeScript gives `0 | null | string`
  (TASK-623 Issue 1).
- **Cause**: `extractDefinitelyFalsyTypes` maps `number` to `0`, `string`
  to `""`, and `bigint` to `0n`, while narrowing `$l` by falsiness keeps
  `number` (`NaN` is falsy too). No test TypeScript narrows produces `0`
  from `number`, and an assertion in the emitted code is ruled out by
  AGENTS.md contract 2.
- **Resolution**: Open. The result is a supertype of TypeScript's, so a
  read of it type-checks; an assignment to a target that accepts only
  `0` does not. TASK-692 Decision 2 records why no lowering can carry it.

### Issue 3: A right operand that cannot run still joins the result

- **Symptom**: `true || try read()` is typed `true | number`; TypeScript
  gives `true`, because it types the operation as the left type when the
  left type is never falsy (never nullish for `??`).
- **Cause**: the branch that runs the right operand writes the result slot
  whatever `$l` narrowed to there; TypeScript does not take a branch whose
  test narrows a reference to `never` for unreachable.
- **Resolution**: Open, as Issue 2; it holds for non-literal operands
  (`obj || try read()`) as well, and did before this task. Fixed by
  TASK-692.

### Issue 4: TypeScript 5.6's checks on a literal left operand are not reproduced

- **Symptom**: TypeScript reports `null ?? read()` (TS2871) and
  `"" || read()` (TS2873). ttc reported neither for `null ?? try read()`
  and, before this task, reported TS2873 for `"" || try read()` only
  because the generated `if ((""))` repeated the literal.
- **Cause**: those checks are syntactic (TypeScript 5.6 release notes,
  "Disallowed Nullish and Truthy Checks") and read the source operation,
  which the lowered program no longer contains.
- **Resolution**: Open. With the operand stored, `"" || try read()` no
  longer reports TS2873. TASK-692 Decision 3 records why it stays open.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/literalLeftOperandNarrowing.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: without the changes to
  `src/evaluation_ir/planning.rs` and `src/typescript/host.mjs`, the case
  failed with `modified baseline: …/literalLeftOperandNarrowing.map.txt is
  out of date`; its `.types` baseline generated on the unfixed code had
  `a: number | false`, `b: string | true`, `c: number | null`, and
  `h: number | false`, and with the planning change alone `orFalse`
  returned `value: number | true`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/evaluation_ir/planning.rs`, `src/typescript/host.mjs`,
`docs/design/program-lowering.md`,
`docs/design/contextual-type-materialization.md`, and `docs/ai/tt.md`;
added
`tests/cases/compiler/literalLeftOperandNarrowing.tt` and its baselines.
A logical operation with a literal left operand now has TypeScript's type.
Issues 2 to 4 remain open.
