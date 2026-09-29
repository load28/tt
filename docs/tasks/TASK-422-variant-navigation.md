# TASK-422: Navigate from variant use sites to the variant declaration

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Go to definition on a variant or case from any use site answered nothing, and
find references left out the declaration. For
`variant V { A(x: number), B }` followed by `const v: V = V.A(1);`,
`const w = V.B;` and `function f(q: V) { return q; }`, the server answered
`{"locations":[]}` for `definition` at `V` in a type position, at `V` and `A`
in `V.A(1)`, at `V` and `B` in `V.B`, and at `q: V`. Across files, the
definition of `Shape` in `(s: Shape)` was empty and its references did not
include `shapes.tt`. References also marked whichever location came first as
the definition, so across files the import specifier was the definition and
the real declaration was not.

## Scope

- Included: The glue names a variant lowering declares
  (`src/codegen/core/emitter/helpers.rs`, `src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`, `src/codegen/contextual.rs`,
  `src/core_ir/`, `src/lib/mapped.rs`, `src/lib/compile.rs`), mapping
  navigation targets through them (`src/engine/language/service.rs`,
  `src/engine/language/project.rs`), and the definition flag on references.
- Excluded: Rename of a variant or case, which stays refused (Decision 2).
  Pattern tags (`A(x) =>`) are not references in the checker's program
  because they lower to `kind` comparisons, so references from a case still
  list only the declaration and the constructor uses. `ttSymbol` keeps
  answering only inside declarations and patterns (Decision 4).

## Decisions

### Decision 1: The lowering records which source name each glue declaration name stands for

- **Context**: TASK-105 Decision 1 hands use sites to the checker. The
  checker resolves `V`, `V.A` and `V.B` correctly, but its targets are the
  lowered `type V`, `const V` and the constructor properties `A:` and `B:`.
  Those names are compiler-written glue with no `EmitMapping`, so
  `map_target` dropped every one of them. The information needed to map them
  back exists only in the lowering: the emitter writes each of those names
  for one source name (HIR `VariantItem.node`, `VariantData.node`,
  `FieldData.node` already carry the name spans).
- **Alternatives considered**:
  - Map the glue names as ordinary `EmitMapping` chunks. A mapping claims
    that the output bytes are copied source bytes; the source name `V`
    appears twice in the output (`type V`, `const V`), and the target
    validator requires every source byte to be emitted once and in order
    (`docs/design/program-lowering.md` §11). It would also make rename edit
    the glue.
  - Recognise a target inside an `AnchorKind::Variant` anchor by comparing
    its text with the variant's names. That is a text heuristic on emitted
    output, the kind of fix AGENTS.md contract 3 rules out.
  - Let `ttSymbol` answer at use sites. TASK-105 Decision 1 rejected that
    for the false positives it causes, and the checker already resolves the
    name exactly (shadowing, imports, aliases).
- **Decision and rationale**: A new rope mark pair records a
  `DeclaredName { src, src_end, out, out_end }` for every glue name that
  declares a source name: the union type name, the constructor object name,
  each constructor property, and each payload property of the union arms,
  in both the runtime and the ambient (`declare`) forms. The text of the
  output is unchanged (byte-identical emission, TASK-050). The contextual
  annotation pass shifts the records like every other output offset.

### Decision 2: Only navigation resolves a declared name; edits still need copied source

- **Context**: `map_target` serves navigation (skip a result it cannot map)
  and rename (refuse the operation). A rename of `V.B` would edit the
  constructor property but not the `kind: "B"` literals or the match patterns
  that name the case, so the program would break.
- **Decision and rationale**: `map_target` takes a `TargetUse`. `Navigation`
  falls back to the declared name whose output range is exactly the target;
  `Edit` keeps requiring an exact mapping, so rename is refused as before.
  Because several glue names stand for one source name (`type V` and
  `const V`), the locations are de-duplicated after mapping. LSP 3.17
  `textDocument/definition` and `textDocument/references` return a list of
  `Location`s; identical entries carry no information.

### Decision 3: The definition flag is the checker's definition, not list position

- **Context**: `Project::references` set `is_definition: index == 0`. The
  extension filters on this flag when the LSP 3.17 `ReferenceContext`
  asks for `includeDeclaration: false`, so a wrong flag removes a use and
  keeps the declaration. TASK-025 Decision 3 had already decided to compute
  the flag by comparing against the definition result, because the
  reference entries TypeScript returns do not carry a usable definition
  flag; the engine port lost that.
- **Decision and rationale**: `references` also asks
  `textDocument/definition` at the same position and marks each mapped
  reference that equals a mapped definition. Across files the import
  specifier is a reference and the declaration in `shapes.tt` is the
  definition, which is what TypeScript's go-to-definition resolves through
  the alias.

### Decision 4: `ttSymbol` stays null at use sites

- **Context**: The report listed `ttSymbol` returning null at each use
  site.
- **Decision and rationale**: Not a defect. TASK-105 Decision 1 assigns use
  sites to the checker and pins it with
  `ordinary_identifiers_are_left_to_the_checker`; the extension already
  falls back to the engine `definition` when `ttSymbol` is null. With this
  task that fallback answers, so there is no longer a gap to fill.

## Work log

- 2026-09-27: Reproduced through `ttc --server` with the repository
  TypeScript: all six local definitions were empty, `references` at `V`
  omitted the declaration, and the cross-file `references` marked the import
  specifier as the definition and omitted `shapes.tt`.
- 2026-09-27: Added `AdtVariant::node` and `AdtField::node`, the
  `DeclaredNameStart`/`DeclaredNameEnd` marks, `Rope::push_declared_name`,
  `Flat::declared_names` and `MappedEmit::declared_names`, and rewrote
  `emit_adt` to push its names through them.
- 2026-09-27: Added `ServiceDoc::declared_names`, `TargetUse`,
  `declared_name_span`, de-duplication in `Project::locations`, and the
  definition comparison in `Project::references`.
- 2026-09-27: Added
  `variant_glue_names_stand_for_their_source_names_in_navigation_only`
  (engine unit test) and
  `variant_navigation_lands_on_the_variant_declaration`
  (`tests/native/cases_03.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] With the source changes reverted,
  `variant_navigation_lands_on_the_variant_declaration` fails
  (`"v: V"+3`: left `[]`); with them it passes. The unit test does not
  compile without `declared_name_span`.
- [x] `ttc --server` after the change: every local definition lands on
  `V` (0:8), `A` (0:12) or `B` (0:26); `references` at `V` includes 0:8
  marked as the definition; the cross-file `definition` of `Shape` is
  `shapes.tt` 0:15, and its `references` mark only that location as the
  definition. `rename` at `V.B` and at `V` is still refused, and the
  payload access `v.x` goes to the field `x` (0:14).

## Result

Changed `src/codegen/contextual.rs`, `src/codegen/core/emitter/helpers.rs`,
`src/codegen/core/emitter/result.rs`, `src/codegen/core/emitter/source.rs`,
`src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `src/core_ir/lower.rs`,
`src/core_ir/mod.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/engine/language/tests.rs`, `src/engine/names.rs`, `src/lib/compile.rs`,
`src/lib/mapped.rs` and `tests/native/cases_03.rs`. Go to definition and
find references now reach the variant declaration from every use site the
checker resolves, in the same file and across files.
