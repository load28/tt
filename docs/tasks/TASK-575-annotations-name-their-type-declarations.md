# TASK-575: Annotate storage only with names that denote the type's own declarations

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-575: Annotate storage only with names that denote the type's own declarations`

## Purpose

`variant K { A, B } export function outer<T>(a: T) { function inner<T>(b: T, k: K) { const z = match (k) { A => a, B => a }; return [z, b] as const; } return inner("s", K.A); }`
emitted `let $tt_v0: T;` inside `inner`, where `T` is `inner`'s own type
parameter, not the `outer` one `a` has: TS2719 twice, and `--check-types`
reported errors the source does not have. A local `interface Item`
shadowing a module-level one gave TS2741 the same way. An annotation must
denote the type it was computed from.

## Scope

- Included: the annotation check of the backend host
  (`src/typescript/host.mjs`), the design note, `docs/ai/tt.md`, the note
  on TASK-546, and regression tests.
- Excluded: writing another name for a shadowed type (the node builder has
  no form for an outer type parameter; `GenerateNamesForShadowedTypeParams`
  writes a `T_1` nothing declares). A script already gets `globalThis.Item`
  from the node builder, which the new check accepts.

## Decisions

### Decision 1: Walk the type node and the type together and compare each name with the type's own symbol

- **Context**: TASK-546 Decision 1 compared a name's resolution at the
  declaration with its resolution where the type was observed (the use
  that supplied the contextual type, or the join's right-hand side). When
  the storage and that location are both inside the shadowing scope, both
  resolve to the shadowing declaration and the check passes.
- **Alternatives considered**: (a) Resolve at the declaration of the
  type's symbol instead of the observed location: a type has many names
  (one per type reference and type argument) and the node does not say
  which part each came from. (b) Collect every symbol the type reaches and
  accept a name resolving to any of them: a type holding both `T`s
  (`readonly [T, T]`) would accept either spelling for both. (c) Pass
  `GenerateNamesForShadowedTypeParams`: the generated `T_1` is declared
  nowhere. (d) Re-check the annotation in an updated snapshot: the checker
  API exposes assignability, not identity, and a shadowed interface with
  the same members is assignable.
- **Decision and rationale**: `denotes` in the host walks the printed node
  and the type in step: a union or intersection member pairs with one of
  the type's constituents, array and tuple elements and type arguments with
  the reference's type arguments (after the target's outer type
  parameters) or the alias's, members of a type literal with the property,
  method, call, construct and index signatures of the type, and a
  signature's type parameters, parameters, `this` parameter and return or
  predicate type with the signature's. A type reference's name, resolved at
  the declaration (a qualified name through its members' exports, an import
  through its alias), must be the type's alias symbol or its own symbol (a
  type parameter's own symbol, the class, interface or enum member a
  reference instantiates); a type parameter declared by a signature in the
  node itself is resolved to that signature's type parameter; a type query
  names the type's own symbol. A part that uses a name and cannot be paired
  (a mapped or conditional type, a computed member name) proves nothing, so
  the annotation is not written and the storage is typed from its
  assignments, as for every rejected annotation (TASK-546 Decision 2). The
  generated-storage exclusion (TASK-552) is kept inside the resolution.
  The observed location is no longer used; TASK-546's record says so.

## Work log

- 2026-09-29: Reproduced on `claude/ecstatic-dijkstra-qw5pf9` with the
  repro above and the local `interface Item` variant (`let $tt_v0: T;`,
  `let $tt_v0: Item;`; TS2719, TS2741 under `--check-types`).
- 2026-09-29: Probed the TypeScript 7 API with a standalone script
  (`target/repro/api/`): the node builder writes the outer `T`/`Item` by
  its bare name in a module and as `globalThis.Item` in a script;
  `Symbol.getExports()` of `globalThis`, a namespace and an enum resolves
  qualified names.
- 2026-09-29: Added `denotes` to `src/typescript/host.mjs` and used it in
  `annotation`; removed the observed-location resolution. Updated
  `docs/design/contextual-type-materialization.md`, `docs/ai/tt.md` and
  the TASK-546 record.
- 2026-09-29: With a temporary log of every rejected annotation across
  `--test integration --test snapshot --test compile --test cli --test
  native --test content_mapper --test practical_diagnostics`, the only
  rejections were the arm-local `Loc`/`Hidden`, `typeof Local` and
  `typeof $tt_v0` of TASK-546 and TASK-552; every other annotation and
  every fixture is unchanged.
- 2026-09-29: Added
  `an_annotation_names_the_declarations_its_type_refers_to`
  (`tests/integration/contextual.rs`), which type-checks and runs both
  shadowing cases and keeps the annotation that names the local `Item`
  when the arm values have it, and
  `types_reports_nothing_for_storage_inside_a_shadowing_scope`
  (`tests/cli.rs`). Both fail with the previous host (TS2719, TS2741,
  TS2339).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test integration --test snapshot --test compile --test cli --test native --test content_mapper --test practical_diagnostics`
- [x] The new tests fail without the change.

## Result

Changed `src/typescript/host.mjs`,
`docs/design/contextual-type-materialization.md`, `docs/ai/tt.md`,
`docs/tasks/TASK-546-accessible-storage-annotations.md`,
`tests/integration/contextual.rs`, `tests/cli.rs`, `docs/tasks/INDEX.md`,
and this record.
