# TASK-570: Type a value with no contextual type as TypeScript does at its source position

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-570: Type a value with no contextual type as TypeScript does at its source position`

## Purpose

TASK-553 Issue 1: `const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) }; b.m().zzz;`
lowers each arm to `$tt_v0 = {…}`, an assignment to storage declared
`let $tt_v0;`. The assignment contextually types the object literal by the
storage's implicit `any`, so `this` in its methods is `any` and
`--check-types` misses the TS2339 that the equivalent
`const b = n === 1 ? {…} : {…}` reports. Annotating the storage does not
help: the annotation repeats `m(): any`.

## Scope

- Included: the refinement of generated storage after the backend's
  contextual rounds (`src/codegen/contextual.rs`,
  `src/typescript/contextual.rs`), the contextual query's settled storage and
  the kind of an answer (`src/typescript/backend.rs`, `native.rs`,
  `host.mjs`), the arm-selector declaration (`src/codegen/rope*`,
  `src/codegen/core/emitter/host.rs`, `src/lib/`), the design note,
  `docs/ai/tt.md`, tests and fixtures.
- Excluded: storage whose source position has a contextual type; it keeps
  the annotation and the assignment form TASK-546..553 settled. The lowering
  itself (codegen before refinement) is unchanged, and so is the unrefined
  output.

## Decisions

### Decision 1: Decide from the backend's contextual rounds, per slot

- **Context**: Whether the source position of a value has a contextual type
  is a checker fact. The rounds already ask it at every use of every
  generated slot, and annotate a slot whose uses have one.
- **Alternatives considered**: Deciding syntactically from the source
  position (a declaration without annotation, an argument of an untyped
  callee …): a guess at the checker's answer.
- **Decision and rationale**: A slot still unannotated when contextual
  propagation first reaches its fixed point has no contextual type at its
  source position, and is detached. This happens before joins are inferred,
  since a join annotation is itself computed from the values written to the
  slot (a join of the `any`-typed literal is `m(): any`) and would otherwise
  give a nested slot a context its source position does not have. The host
  now says whether an answer is a contextual type or an inferred join
  (`ContextualSlotType::inferred`): a contextual answer for a detached slot
  (a context a later join exposed) attaches it again, so each slot changes
  state at most twice and the rounds still terminate.

### Decision 2: Carry the value in a property of an arm-local `const`

- **Context**: The approved design writes the value to an arm-local `const`
  first. TypeScript declares an unannotated variable by rules of its own on
  top of the initializer's type.
- **Alternatives considered**: (a) `const $tt_a0 = value; $tt_v0 = $tt_a0;`:
  an empty array literal initializer declares an evolving array, which
  failed `pr115::join_storage_preserves_array_inference_under_strict_checking`
  with TS7034/TS7005, and a `Symbol()` initializer of a `const` declares a
  `unique symbol`; neither happens at the source position. (b) An array
  element (`[value]`, read `[0]`): `noUncheckedIndexedAccess` adds
  `undefined`. (c) A comma operand (`(0, value)`): TS2695.
  (d) `satisfies unknown`: gives the value the contextual type `unknown`.
- **Decision and rationale**:
  `const $tt_a0 = { value: … }; $tt_v0 = $tt_a0.value;`. A property of an
  object literal with no contextual type has none, and no declaration rule
  applies to it; it only widens a fresh literal type, which storage with no
  contextual type (evolving or joined) widens anyway. A standalone
  `tsc --strict` check confirmed `never[]` for `[]`, `symbol` for
  `Symbol()`, and the type parameter itself for a generic reference.

### Decision 3: Every value written to a detached slot, except an arm index

- **Context**: Which writes the rule covers.
- **Alternatives considered**: Only values whose type depends on a
  contextual type. TypeScript consults the contextual type for object and
  array literals, functions and their return expressions, generic calls,
  references to a type parameter with a union constraint, and the operators
  that pass it on; `isContextSensitive` does not even cover
  `m() { return this; }`. Choosing a subset would re-implement the checker
  syntactically.
- **Decision and rationale**: Every write, so the fixture churn is every
  write to such a slot. The one exception is structural: the slot a deferred
  arm selection writes (`$tt_v1 = 0; … ($tt_v1 === 0 ? a : b)`) holds the
  compiler's arm index, not a value of the source; codegen declares it
  through `push_selector_declaration` (`MarkKind::SelectorSlot`,
  `MappedEmit::selector_slots`) and it is never detached.

### Decision 4: Find the writes in the emission's syntax tree

- **Context**: The refinement edits an emission after codegen, as the
  annotations do, and must reach every write of a slot; the standalone path
  transfers the per-slot result from the analysis emission to the requested
  one by slot index.
- **Alternatives considered**: A mark at each write site in the emitters:
  around fifteen sites, several of them text edits of authored `return`s,
  with nothing to catch a site that is missed.
- **Decision and rationale**: `storage_writes` parses the emission with SWC
  and takes every assignment statement whose target is the slot's generated
  name, which hygiene makes unique in its file. Writes are statements of a
  block, function body or `switch` case, where a `const` can be declared;
  any other is an internal compiler error. The layout follows the write: on
  a line of its own the assignment moves to the next line at the same
  indentation, otherwise it stays on the line. An emission that does not
  parse (an editor buffer mid-edit) is not detached. The arm-local `const`s
  are hygienic (`$tt_a0`, …) and listed to the host as settled storage, so
  no annotation names them (TASK-552).

### Decision 5: The unrefined output is unchanged

- **Context**: Without a TypeScript toolchain nothing is known about
  contextual types (TASK-354: the refinement is removed, not the
  compilation).
- **Decision and rationale**: No slot is detached; the output assigns values
  directly, byte for byte as before.

## Work log

- 2026-09-29: Reproduced on `claude/ecstatic-dijkstra-qw5pf9`
  (`let $tt_v0: { k: number; m(): any; }`, `--check-types` exits 0).
- 2026-09-29: Replaced `insert_annotations` with `refine`
  (`src/codegen/contextual.rs`), which applies a slot's annotation and
  detachment to an unrefined emission; `materialize` keeps the unrefined
  emissions and re-applies the per-slot state each round
  (`src/typescript/contextual.rs`). Renamed `ContextualSlotQuery::annotated`
  to `settled` and added `ContextualSlotType::inferred`
  (`backend.rs`, `native.rs`, `host.mjs`).
- 2026-09-29: First form `const $tt_a0 = value;` failed the `pr115` evolving
  array test; moved to the object-literal property (Decision 2).
- 2026-09-29: Added the selector declaration after the deferred arm index
  was detached (Decision 3).
- 2026-09-29: Regenerated the snapshot fixtures
  (`UPDATE_EXPECT=1 cargo test --test snapshot`) and read the diff; updated
  the output assertions of 46 `tests/compile` tests and
  `emit_map::anchors_do_not_change_the_emitted_bytes`, which compares the
  unrefined emission and now compiles with `defer_to_checker`, and of
  `content_mapper::tests::incomplete_match_arms_preserve_mapped_siblings_in_both_source_kinds`.
- 2026-09-29: Added `a_value_with_no_contextual_type_is_typed_as_at_its_source_position`,
  `a_contextual_this_type_still_reaches_the_arm_values`
  (`tests/integration/contextual.rs`) and
  `types_type_a_value_with_no_contextual_type_as_at_its_source_position`
  (`tests/cli.rs`). With the sources of the base branch the first and third
  fail (no TS2339); the second holds on both.
- 2026-09-29: Rebased onto the branch's new head (TASK-561) and re-ran the
  suites.

## Issues and resolutions

### Issue 1: A write in a function body was reported as outside a block

- **Symptom**: Internal compiler error "a write to value storage is not a
  statement of a block" for `const q = (try Ok(10)) * 2;`.
- **Cause**: SWC models a function body as `FunctionBody`, not
  `BlockStmt`.
- **Resolution**: `storage_writes` also collects the statements of
  `FunctionBody`.

### Issue 2: A join annotation elides a recursive anonymous type

- **Symptom**: The detached example now joins as
  `let $tt_v0: { k: number; m(): { k: number; m(): any; }; };`, so
  `b.m().m().zzz` is missed; `function mk() { return { m() { return this; } }; }`
  with `match (n) { 1 => mk(), _ => mk() }` joins as `{ m(): any; }` on the
  base branch as well.
- **Cause**: With `NoTruncation`, TypeScript's node builder writes a
  recursive anonymous type's cycle as `any` (the declaration emitter prints
  it as `/*elided*/ any`; the API client does not carry the comment).
  TASK-553 assumed the node builder writes no node for such a type.
- **Resolution**: Not resolved here: it concerns the join annotation, not
  the contextual type of the value. Left as a follow-up; the regression test
  asserts the first level, which this task fixes.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `scripts/check-task-index`

## Result

Changed `src/codegen/contextual.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/codegen/core/emitter/host.rs`,
`src/lib/compile.rs`, `src/lib/mapped.rs`, `src/typescript/contextual.rs`,
`src/typescript/backend.rs`, `src/typescript/native.rs`,
`src/typescript/host.mjs`, `docs/design/contextual-type-materialization.md`,
`docs/ai/tt.md`, `docs/tasks/TASK-553-untruncated-annotations.md`,
`src/content_mapper/tests.rs`, `tests/cli.rs`, `tests/emit_map.rs`, `tests/integration/contextual.rs`,
`tests/compile/cases_01.rs`, `cases_02.rs`, `cases_05.rs`, `cases_06.rs`,
`cases_07.rs`, `cases_09.rs`, `cases_10.rs`, `cases_11.rs`, `cases_13.rs`,
21 fixtures under `tests/fixtures/emit/`, `docs/tasks/INDEX.md`, and this
record. Every write to storage whose source position has no contextual type
now goes through an arm-local `const`; the join annotation's elided cycles
remain (Issue 2).
