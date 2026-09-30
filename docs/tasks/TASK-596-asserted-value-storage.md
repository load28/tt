# TASK-596: Keep an asserted value's own type in its storage

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-596`

## Purpose

`as T`, `<T>`, and `satisfies T` after a tt value became the storage's
annotation. `const n = match (o) { A => x, B => 0 } as number;` with
`x: unknown` reported TS2322, and
`const m = match (o) { A => ({ a: 1 }), B => ({ a: 2 }) } satisfies { a?: number };`
made `m.a` possibly `undefined` (TS18048), although the same TypeScript
type-checks. When no checker refined the output (the source outside the
TypeScript program), `match … satisfies Ev` with object-literal arms got no
contextual type, so `kind` widened to `string` (TS1360), while
`const q: string = match … as unknown as string` was annotated `: string`
and rejected valid arms.

## Scope

- Included: the syntactic contextual type in `src/program_syntax`
  (assertion operands), the storage declaration codegen writes for it
  (`src/codegen`), asserted storage through the contextual rounds
  (`src/typescript/contextual.rs`, `backend.rs`, `native.rs`, `host.mjs`),
  the refinement edits that replace a written annotation
  (`src/codegen/contextual.rs`), `docs/ai/tt.md`,
  `docs/design/contextual-type-materialization.md`, and regression tests.
- Excluded: a value lowered in the selector form (the arms stay in the
  expression, `(sel === 0 ? a : b) satisfies T`) already keeps the
  operand's type and is unchanged.

## Decisions

### Decision 1: An assertion's contextual type types the arms, not the storage

- **Context**: TypeScript's `getContextualType` gives the operand of `as T`,
  `<T>e`, and `satisfies T` the contextual type `T`, and passes a context
  through parentheses, `!`, and `as const`. A type assertion checks only
  that the operand and `T` are comparable (TypeScript handbook, "Everyday
  Types: Type Assertions"); `satisfies` checks that the operand is
  assignable to `T` and leaves its type unchanged (TypeScript 4.9 release
  notes, "The `satisfies` Operator"). The backend took the contextual type
  at the storage's read (`$tt_v0 as number`) as its annotation, which makes
  the arms an assignment to `T`.
- **Alternatives considered**: (a) Never annotate such storage. The arms
  lose `T` as their context: object-literal tags widen (TS1360 under
  `satisfies`), callbacks lose parameter types. (b) Move the assertion onto
  each arm. `as` compares a union operand by *some* constituent, so a
  per-arm assertion rejects valid code (`(c ? "a" : 1) as number`), and it
  would copy the authored type into arm scopes that can shadow its names.
  (c) Annotate with `T` for one round so the arms are typed under `T`,
  then replace it with the join of the arms' types.
- **Decision and rationale**: (c). The host marks such an answer
  provisional (`ContextualSlotType::provisional`); the slot stays a site
  (`ContextualSlotQuery::asserted`), and in the join phase the host answers
  the join of its assignments' types under `T`, or `null` to clear the
  annotation when the join is `any`, `unknown`, or not writable. Each type
  is widened as TypeScript widens at a mutable location: a literal is kept
  only where `T` has a literal type of its kind (the checker's
  `isLiteralOfContextualType`, written over the API's type flags), so
  `satisfies 1 | 2` keeps `1 | 2` and `satisfies number` gives `number`, as
  `let w = … satisfies number; w = 5;` requires. The provisional round is
  never detached, since the source position has a contextual type.

### Decision 2: The syntactic annotation stops at an assertion

- **Context**: Without the checker, codegen annotates the storage of a
  whole initializer or returned expression with the declared type. The
  path to the value passes through `as`/`satisfies` as transparent
  wrappers, so the declared type reached an assertion's operand.
- **Alternatives considered**: (a) Stop only for `as`. (b) Stop at every
  assertion and use `satisfies T`'s own `T`. (c) Stop at every assertion
  and annotate nothing.
- **Decision and rationale**: (b). The collector records the innermost
  enclosing assertion (`ParentCollector::assertions`), and the context
  uses it when the value's transparent path reaches an assertion edge
  (`is_assertion_edge`). `as T` and `<T>` annotate nothing: `T` would make
  the arms an assignment. `satisfies T` annotates the storage with `T`,
  which accepts exactly the values the operator accepts, so the TS1360
  case type-checks without the checker; the declaration is marked
  (`MarkKind::AssertedAnnotationEnd`, `MappedEmit::asserted_slots`) so the
  checker, when present, treats it as asserted storage and the refinement
  replaces or removes the written annotation. Without the checker the
  storage then reads as `T`, not the operand's narrower type; that is the
  one precision a checker-less build cannot have.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/repro/r3_as_satisfies.tt`
  (TS2322, TS18048) and, with a tsconfig that does not include the
  source, TS1360 for `satisfies Ev` and a `: string` annotation under
  `as unknown as string`.
- 2026-09-30: `src/program_syntax.rs`, `src/program_syntax/visit.rs`,
  `src/program_syntax/collector.rs`: assertion facts and
  `EvaluationContext::contextual_type_asserted`.
  `src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`
  (`push_slot_declaration`), `src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`, `src/lib/mapped.rs`, `src/lib/compile.rs`:
  the marked declaration. `src/codegen/contextual.rs`: replacing and
  clearing edits. `src/typescript/contextual.rs`, `backend.rs`,
  `native.rs`, `host.mjs` (`assertionOperand`, `widenedIn`): provisional
  and asserted storage.
- 2026-09-30: Tests
  `an_assertion_operand_takes_no_storage_type_from_outside_the_assertion`
  (`tests/compile/cases_14.rs`), `an_asserted_value_keeps_its_own_type`
  (`tests/native/cases_10.rs`, `--check-types` clean), and
  `an_asserted_value_type_checks_without_a_checker_at_compile_time`
  (`tests/integration/cases_05.rs`, tsc over the checker-less output).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/{collector,visit}.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`,
`src/codegen/rope.rs`, `src/codegen/rope/builder.rs`,
`src/codegen/contextual.rs`, `src/lib/{compile,mapped}.rs`,
`src/typescript/{contextual,backend,native}.rs`, `src/typescript/host.mjs`,
`docs/ai/tt.md`, `docs/design/contextual-type-materialization.md`,
`tests/compile/cases_14.rs`, `tests/native/cases_10.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record.
