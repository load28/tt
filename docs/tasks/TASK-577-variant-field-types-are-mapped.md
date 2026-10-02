# TASK-577: Map a variant's field types and type parameters to the source

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-577: Map a variant's field types and type parameters to the source`

## Purpose

With `export interface Money { cents: number }` and
`export variant Price { Fixed(amount: Money), Free }`, hover and definition
on `Money` inside `Fixed(amount: Money)` returned null, completion there
offered no TypeScript names, references to `Money` left out the variant,
rename of `Money` returned null everywhere, and a typo `Fixed(amount: Mony)`
was reported at `Price` as "(in code ttc generated for this construct)".
A hand-written union gets every one of these.

## Scope

- Included: how the emitter writes a variant's field types and its `<...>`
  type parameter list (`emit_adt`), the mapping invariant that allows it,
  and the two service answers that meet the repeated text (rename edits,
  exact diagnostics).
- Excluded: which tt items completion adds at a type position (TASK-578).

## Decisions

### Decision 1: Copy the type text from the source at every place it is written

- **Context**: `emit_adt` wrote each field's `ty_text` and the variant's
  `generics` as literal glue in the union (`| { kind: "Fixed"; amount: Money }`)
  and again in the constructor (`Fixed: (amount: Money): Price => …`).
  Glue has no `EmitMapping`, so a source position there had no service
  position, and every answer about the output text had no source place
  (`docs/design/lsp-architecture.md`: only verbatim bytes map).
- **Alternatives considered**:
  - Write the type once and refer to it from the other place, e.g. the
    constructor's parameter typed as `Extract<Price, { kind: "Fixed" }>["amount"]`.
    That is a type trick in the emitted TypeScript (contract 2) and changes
    what hover and signature help show.
  - A second record kind beside `EmitMapping` for repeated tt text, like
    `DeclaredName`. Every consumer (service mapping, diagnostics, source
    maps, the content mapper's span map) would need to learn it, while an
    `EmitMapping` is already what each of them reads.
- **Decision and rationale**: The field's type and the type parameter list
  are the user's TypeScript, so the emitter copies them from the source
  (`Rope::push_src`) wherever it writes them: the HIR and Core IR carry
  their source spans (`FieldData::ty_span`, `VariantItem::generics_span`,
  `AdtField::ty_span`, `Adt::generics`). The emitted text is unchanged
  byte for byte; only its mappings are new. In a `.ttx` file the
  constructor's list is the copied list without its `>` followed by `,>`,
  as before.

### Decision 2: A source chunk may be mapped from more than one output place

- **Context**: `EmitMapping` documented chunks as non-overlapping in both
  coordinate spaces, and `tests/emit_map.rs` asserted it. A type written in
  the union and the constructor is one source chunk at two output places.
- **Decision and rationale**: Output chunks still never overlap, and
  pass-through text is still copied exactly once
  (`validate_source_preservation`); a tt construct's own text, which its
  lowering already may repeat (`SourcePreservation::owned`), may be copied,
  and mapped, at each place. The invariant test now requires that chunks
  overlapping in the source are copies of one text (they start at the same
  byte). Every lookup from a source position already takes the first
  covering chunk; tsgo's content mapper accepts the span map (TS2552 is
  reported at the field type, `a_type_error_in_a_variant_field_reports_at_the_field_type`).

### Decision 3: One source place is one rename edit and one diagnostic

- **Context**: TypeScript answers rename with an edit at each copy, and the
  checker reports an unknown name at each copy. Both map to the same
  source range. References already merged equal locations, and the typed
  report already merged equal diagnostics.
- **Decision and rationale**: `Project::rename` keeps one of two equal
  edits, and `Project::service_diagnostics` one of two equal exact
  diagnostics, as it already did for diagnostics on glue. Two edits at one
  place with different text are still kept apart for the workspace's
  conflict check.

## Work log

- 2026-09-29: Reproduced with `target/probe4-editor/r1.cjs` and a probe
  of hover, definition, references, rename, completion and the typo
  through the language server.
- 2026-09-29: Added the spans (`src/ast.rs`, `src/parser/variants.rs`,
  `src/hir/{mod,lower}.rs`, `src/core_ir/{mod,lower}.rs`) and the copies in
  `emit_adt` (`src/codegen/core/emitter/helpers.rs`, callers in
  `source.rs` and `result.rs`). `emit_map`'s invariant test failed on the
  repeated chunks; revised the documented invariant (`src/lib/mapped.rs`)
  and the test (Decision 2).
- 2026-09-29: The probe then answered hover `interface Money`, the
  definition, references `0:17 1:37 5:36`, completion with `Money`, `Date`
  and `string`, and TS2552 at `1:37`, but rename listed `1:37` twice and
  `tsDiagnostics` reported TS2552 twice. Added Decision 3.
- 2026-09-29: Tests: `a_variant_field_type_is_typescript_to_every_service_feature`
  (`tests/native/cases_10.rs`),
  `a_type_error_in_a_variant_field_reports_at_the_field_type`
  (`tests/content_mapper.rs`) and
  `variant_field_types_and_type_parameters_are_mapped_where_they_are_written`
  (`tests/emit_map.rs`). The first two fail with the field type written as
  a literal again (hover null; the error at `price.tt(2,16)`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --lib --bins --test snapshot --test compile --test emit_map --test content_mapper --test sidecar --test cli_outputs --test practical_diagnostics`
- [x] Full suites with TASK-579 (see that record).

## Result

Changed `src/ast.rs`, `src/parser/variants.rs`, `src/hir/mod.rs`,
`src/hir/lower.rs`, `src/core_ir/mod.rs`, `src/core_ir/lower.rs`,
`src/codegen/core/emitter/{helpers,source,result}.rs`, `src/lib/mapped.rs`,
`src/engine/language/project.rs`, `tests/emit_map.rs`,
`tests/content_mapper.rs` and `tests/native/cases_10.rs`. A variant's field
types and type parameters answer hover, definition, references, rename,
completion and the checker's errors as the same text in a hand-written
union does.
