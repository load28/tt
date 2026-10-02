# TASK-546: Annotate generated storage only with names visible at its declaration

> TASK-575 replaces Decision 1: a name is compared with the symbol the
> type refers to, not with its resolution where the type was observed,
> which is inside the same shadowing scope as the storage when the
> storage is (`function inner<T>` inside `function outer<T>`).

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

The type annotation the TypeScript backend adds to generated storage could
name a class declared inside an arm block.
`const v = match (s) { A => { class Loc { tag = "a"; } return new Loc(); }, B => ({ tag: "b" }) };`
hoisted `let $tt_v0: Loc;` before the match, where `Loc` is out of scope
(TS2552; TS2304 for a name no outer scope declares). With an outer class of
the same name the annotation silently bound the outer class (TS2741). The
same happened in `result` blocks. `docs/ai/tt.md` promises that scoped values
keep their contextual types through ordinary annotations on generated
storage, and emitted TypeScript must pass `tsc`.

## Scope

- Included: the backend host's contextual and join annotations
  (`src/typescript/host.mjs`), the design note, `docs/ai/tt.md`, and a
  runtime and type-check regression test.
- Excluded: choosing a different printable form for a type whose name is
  not visible (an anonymous object type, an import type). The checker offers
  no printer option for it, and inference from the assignments already
  gives the storage the observed type.

## Decisions

### Decision 1: Verify every referenced name at the declaration with the checker's own resolution

- **Context**: `checker.typeToTypeNode(type, declaration)` prints a symbol's
  name even when the symbol is not accessible from `declaration`. TypeScript's
  declaration emitter checks accessibility through `isSymbolAccessible`,
  which the TypeScript 7 API this host speaks does not expose, and the
  node builder flags have no mode that fails on an inaccessible symbol.
- **Alternatives considered**: (a) Walk the type's structure and compare
  symbols; the printed node does not say which symbol each name came from.
  (b) Keep annotations and drop only the inaccessible part; an annotation is
  one type, and a partial one would be a different type.
- **Decision and rationale**: The host keeps where each type was observed:
  the reference that supplied the contextual type, or the right-hand side of
  the join's assignment. For the printed annotation, the head of every type
  reference and type query must resolve through `checker.resolveName`, with
  the reference's meaning, to the same symbol at the declaration as at that
  location. The observed location is where the type is in scope under that
  name, so a name that is out of scope at the declaration, or shadowed
  there, fails the comparison.

### Decision 2: A storage whose annotation is rejected stays unannotated

- **Context**: The storage still needs a type.
- **Decision and rationale**: Without an annotation, TypeScript types the
  `let` from its assignments, which is the observed type itself. That is the
  existing behavior for storage the backend has no definite type for
  (`docs/design/contextual-type-materialization.md`), so no new form is
  introduced.

## Work log

- 2026-09-29: Reproduced on `1d568d9` through `compile` (TS2552, TS2741 with
  an outer `Loc`, TS2304 for an unnamed outer scope) and in a `result`
  block.
- 2026-09-29: Changed `src/typescript/host.mjs` (the `annotation` helper,
  used by both the contextual and the join path); documented the rule in
  `docs/design/contextual-type-materialization.md` and `docs/ai/tt.md`.
- 2026-09-29: Added `generated_storage_names_no_type_declared_inside_an_arm`
  (`tests/integration/contextual.rs`), which type-checks and runs an arm
  class shadowed by an outer class, the same in a `result` block, and an arm
  class no outer scope declares. Checked with a temporary log that an
  accessible join (`Loc` declared outside the match) keeps its annotation.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] The new test fails without the change (TS2741, TS2353, TS2339,
  TS2304).

## Result

Changed `src/typescript/host.mjs`,
`docs/design/contextual-type-materialization.md`, `docs/ai/tt.md`,
`tests/integration/contextual.rs`, `docs/tasks/INDEX.md`, and this record.
