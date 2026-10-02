# TASK-595: Test a conditional operation's condition where it is evaluated

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-595`

## Purpose

A tt value under `? :`, `&&`, or `||` lost the narrowing the operation's
condition gives in TypeScript.
`return cfg.name ? match (o) { A(n) => cfg.name.slice(n), B => cfg.name } : "anon";`
reported TS18048, and `s && match (o) { A(n) => s.charAt(n), B => s }`
reported TS18047 and TS2322, although the same TypeScript type-checks. The
lowering captured the condition (`const $tt_v1 = (cfg.name); if ($tt_v1)`),
which narrows only the capture, and the capture was annotated with the
whole operation's type (`const $tt_v4: string | null = (s)`), which also
defeats TypeScript's narrowing through `const` aliases. A reassigned `let`
under `u ? …`, `!u || …`, `u !== undefined && u.v > 0 && …`, and
`s && try g(s.length)` failed the same way.

## Scope

- Included: the emission of a conditional operation's condition
  (`src/codegen/core/emitter/host.rs`), detached storage written in an `if`
  test (`src/codegen/contextual.rs`), `docs/ai/tt.md`,
  `docs/design/program-lowering.md` §7.10,
  `docs/design/contextual-type-materialization.md`, updated output
  assertions, operand storage (`src/codegen/rope*`, `src/lib/`,
  `src/typescript/`), and regression tests.
- Excluded: an optional call (`f?.(v)`, `o.m?.(v)`) keeps its callee or
  receiver test; TASK-573 recorded that `obj?.cb?.(match (obj.name) …)`
  still loses the receiver's narrowing because the chain is not lowered
  link by link, and this task does not change that. The nullish narrowing
  of `l` under `l ?? v` is not carried (Decision 4).

## Decisions

### Decision 1: Test the condition where it is evaluated instead of testing a capture

- **Context**: ECMA-262 §13.14.1 (`ConditionalExpression`) and §13.13.1
  (`LogicalANDExpression`, `LogicalORExpression`, `CoalesceExpression`)
  evaluate the condition or left operand once, then branch on it. The
  TypeScript handbook ("Narrowing": truthiness narrowing, control flow
  analysis) narrows the references in that expression in the branch that
  runs. TypeScript narrows through a `const` alias of a condition only when
  the alias has no type annotation and the narrowed reference is itself a
  constant reference (TypeScript 4.4 release notes, "Control Flow Analysis
  of Aliased Conditions and Discriminants"), so a capture never narrows
  `cfg.name` or a reassigned `let`.
- **Alternatives considered**: (a) Drop the capture's annotation and rely
  on aliased-condition narrowing. It covers only constant references, not
  property accesses or reassigned variables. (b) Test the original
  expression and read it again for the result (`if (s) … else $r = s`).
  That evaluates the operand twice, which a getter or a Proxy observes.
  (c) Test the condition in place: a ternary tests its condition directly
  (`if (c)`), since its value is not used after the test; `&&`, `||`, and
  `??` store the left operand inside the test (`if ($l = l)`,
  `if (($l = l) == null)`). TypeScript narrows an assignment used as a
  condition by its right operand as well as its target
  (`narrowTypeByBinaryExpression` for `=`; checked with the pinned `tsc`
  for `s`, `cfg.name`, `!o`, and `o !== undefined && o.v > 0`).
- **Decision and rationale**: (c). It evaluates the condition exactly once
  at the same point and gives TypeScript the reference it narrows.
  TASK-160 Decision 17 (one region per operation, every path writes the
  result slot) still holds. A condition already captured by an earlier
  step, or one that is itself a tt value, is tested through its slot, and a
  provably inert condition is written in place as before, parenthesized.

### Decision 2: The left operand is kept in operand storage, typed as the operand

- **Context**: The left operand of `&&`, `||`, and `??` is the result when
  the right operand does not run, so it must be stored. The capture it
  used to be stored in was annotated with the contextual type found where
  it was read (`$r = $c`), the whole operation's type.
- **Alternatives considered**: (a) Store it in the operation's result slot
  (`if ($r = l)`). The result slot is annotated with the operation's
  contextual type, and the operation only requires the part of `l` it can
  produce (falsy for `&&`, truthy for `||`, non-nullish for `??`) to fit
  it: `const c: number = maybe ?? …` then reports TS2322 (the first
  implementation did, `conditional_operations_keep_their_types_without_undefined`).
  (b) An unannotated `let` typed only by TypeScript. Without
  `noImplicitAny` that is `any`, and the result's join would stop at it.
  (c) A new storage kind, operand storage (`MarkKind::OperandSlot`), which
  the backend annotates with the operand's own type: the widened type of
  the single value written to it, as a `const` initializer is typed, and
  never a contextual type found at a read.
- **Decision and rationale**: (c). `$l` is narrowed by the test where it
  is written to the result slot, as TypeScript narrows the operand in the
  operation's result type. `ContextualSlotQuery::operand` carries the kind
  to the host, which annotates such storage only after contextual
  propagation has settled, from its one assignment.

### Decision 3: Storage may be written in an `if` test

- **Context**: Detached storage (TASK-570) carries a value TypeScript types
  from its context through `const $tt_a0 = { value: … }` before the write,
  and required every write to be a statement of a block. The test write is
  an expression, and in a script's top level a generated `const` would be a
  global.
- **Alternatives considered**: (a) Move the carried value into a `const`
  before the `if`. That moves source bytes and their mappings. (b) Carry it
  in place as `({ value: l }).value`: the object literal is the operand of
  a property access, which has no contextual type, just as an unannotated
  `const` initializer has none.
- **Decision and rationale**: (b). `storage_writes` accepts an assignment
  that is an `if` test (under parentheses or on the left of `==`) and
  marks it `tested`; the refinement then inserts the wrapper around the
  value.

### Decision 4: `??` keeps no narrowing of its left operand

- **Context**: TypeScript narrows `l` to its nullish part in `v` for
  `l ?? v`.
- **Alternatives considered**: `($l = l) == null`,
  `($l = l) === null || $l === undefined`, `(($l = l) ?? null) === null`,
  and `!(($l = l) != null)` all narrow only `$l` (checked with the pinned
  `tsc`). `!($l = l) && $l == null` narrows `l` by falsiness, which is a
  different fact from nullishness.
- **Decision and rationale**: write `if (($l = l) == null)`: one
  evaluation, the right result, and no claim of a narrowing TypeScript
  would not make. The limitation is documented in `docs/ai/tt.md` and
  `docs/design/program-lowering.md` §7.10.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/repro/r4_narrowing.tt`
  (TS18048, TS18047, TS2322) and the reassigned-`let` and `try` forms.
  Checked which assignment-in-condition forms the pinned TypeScript
  (7.1.0-dev.20260826.1) narrows.
- 2026-09-30: `src/codegen/core/emitter/host.rs`: `condition_test` and
  `condition_operand`, the logical and ternary operation emission; the
  optional call keeps `emit_condition_capture`. `src/codegen/contextual.rs`:
  `if`-test writes and their in-place carrier.
- 2026-09-30: Operand storage: `MarkKind::OperandSlot` and
  `push_operand_declaration` (`src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`), `MappedEmit::operand_slots`
  (`src/lib/mapped.rs`, `src/lib/compile.rs`),
  `ContextualSlotQuery::operand` (`src/typescript/backend.rs`,
  `src/typescript/contextual.rs`, `src/typescript/native.rs`), and its
  annotation in `src/typescript/host.mjs`.
- 2026-09-30: Updated the output assertions in `tests/compile/cases_09.rs`,
  `cases_11.rs`, and `cases_14.rs`. The `guard-evaluation-owner` fixture is
  unchanged: its condition is a tt value, tested through its slot.
- 2026-09-30: Tests
  `a_conditional_operation_tests_its_condition_where_it_evaluates_it`
  (`tests/compile/cases_14.rs`),
  `a_value_under_a_conditional_operation_keeps_the_conditions_narrowing`
  (`tests/native/cases_10.rs`, `--check-types` clean), and
  `runtime_a_conditional_operations_condition_is_evaluated_once_where_it_is_tested`
  (`tests/integration/cases_05.rs`, tsc + node, a getter condition read
  once per operation).

## Issues and resolutions

### Issue 1: Detached storage rejected a write in an `if` test

- **Symptom**: `internal compiler error: a write to value storage is not a
  statement of a block` for `const b = s && match …` and for a script's
  top-level `cfg.name ?? match …`.
- **Cause**: `storage_writes` accepted only expression statements of
  blocks, and a first `??` emission wrote the left operand as a statement
  at a script's top level.
- **Resolution**: Decision 3; every logical operation writes its left
  operand in the test.

### Issue 2: The result slot rejected the left operand's whole type

- **Symptom**: `conditional_operations_keep_their_types_without_undefined`
  reported TS2322 (`number | undefined` is not assignable to `number`) for
  `const c: number = maybe ?? match …` when the left operand was stored in
  the result slot.
- **Cause**: the operation's contextual type annotates the result slot,
  and only the narrowed left operand has to fit it.
- **Resolution**: Decision 2.

### Issue 3: A literal `false` condition made its branch unreachable

- **Symptom**: `pr115::result_expression_propagation_uses_its_lexical_failure_edge`
  reported TS2339 on the lowered `try` in `n = false ? try read(false) : 4`.
- **Cause**: the first version wrote an inert condition without its
  parentheses, `if (false)`, and TypeScript's binder takes a `false`
  keyword condition for an unreachable branch, where the lowering's
  narrowing guard narrows nothing.
- **Resolution**: an inert condition keeps the parentheses it was written
  with before this task, `if ((false))`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/codegen/core/emitter/host.rs`, `src/codegen/contextual.rs`,
`src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `src/lib/mapped.rs`,
`src/lib/compile.rs`, `src/typescript/backend.rs`,
`src/typescript/contextual.rs`, `src/typescript/native.rs`,
`src/typescript/host.mjs`, `docs/ai/tt.md`,
`docs/design/program-lowering.md`,
`docs/design/contextual-type-materialization.md`,
`tests/compile/cases_09.rs`, `tests/compile/cases_11.rs`,
`tests/compile/cases_14.rs`, `tests/native/cases_10.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record. A
value under `? :`, `&&`, or `||` now type-checks as the equivalent
TypeScript does.
