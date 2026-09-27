# TASK-417: Keep generated bindings out of completion, hover, and references

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

In a function containing `const a = match (o) {…};` and `const n = try parse(a);`:

- `completion` listed the compiler's `$tt_v0` and `$tt_t0`;
- `hover` on the `match` keyword answered `let $tt_v0: string`, over the whole
  match;
- `references` on `match` listed the whole match as a reference.

None of these names exist in the user's program.

## Scope

- Included: the record of generated names (`src/generated_names.rs`,
  `src/evaluation_ir/evaluation.rs`), which is carried on the emission
  (`src/codegen/rope/builder.rs`, `src/codegen/rope.rs`, `src/codegen/core/mod.rs`,
  `src/lib/mapped.rs`, `src/lib/compile.rs`); service documents and completion
  probes (`src/engine/language.rs`, `src/engine/language/service.rs`,
  `src/engine/language/project.rs`); and the service-span mapping
  (`src/typescript/mapper.rs`).
- Excluded: diagnostics, which are reported at the construct that owns the
  glue on purpose (`diagnostic_source_span`), and tt-only surfaces.

## Decisions

### Decision 1: Filter completion by the names the allocator handed out

- **Context**: A completion item carries only a label and no source span, so
  span ownership cannot classify it. TASK-404 made `generated_names` the
  single allocator for every generated binding. It avoids every `$tt_`-prefixed
  name the file already contains, so a generated name never equals a user
  name.
- **Alternatives considered**: Prefix-match `$tt_`. That would hide a user's
  own `$tt_x` and duplicates the allocator's knowledge (rejected as the task
  asked).
- **Decision and rationale**: `GeneratedNames` records each name it allocates.
  The plan passes the names it reserved (its final occupied set minus the
  file's occupied set). The emitter adds the names it allocates itself.
  `MappedEmit::generated_names` carries the set to `ServiceDoc` and
  `ProbeDoc`, and `ts_completions` drops items whose label is in the set of
  the document it asked. Because the set comes from the allocator, it is
  exact.

### Decision 2: A service span maps back only when every byte of it is copied source

- **Context**: Navigation already dropped glue targets, but
  `from_service_span` used the inclusive boundary lookup on both ends. A glue
  identifier that sits exactly between two mapped chunks (`const a = $tt_v0;`)
  therefore mapped to the source span between them (the whole `match`). Hover
  and references accepted it.
- **Alternatives considered**: Special-case hover to check the hovered text
  against generated names. That fixes one request and leaves references and
  definitions open.
- **Decision and rationale**: `mapper::to_source_span` maps a non-empty span
  only when its first byte lies inside a mapping and the span continues through
  mappings that are contiguous in both coordinate spaces. This is the
  `EmitMapping` contract that these output bytes are these exact source bytes.
  An empty span keeps the inclusive cursor semantics. Hover, definition,
  references, and rename all read spans through `from_service_span`, so they
  share the fact.

## Work log

- 2026-09-27: Reproduced through `ttc --server` (`completion` at an empty
  statement, `hover` and `references` on `match`).
- 2026-09-27: Implemented both decisions. Added
  `tests/native/cases_03.rs::generated_bindings_never_surface_as_user_symbols`.
  It checks completion, hover on `match`, hover on `a`, and references on
  `match`. It fails before the change (the labels include `$tt_v0` and `$tt_t0`)
  and passes after it. Added the unit test
  `typescript::mapper::tests::a_span_maps_only_over_copied_source_bytes`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --lib --test native --test emit_map --test integration --test snapshot`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` and `./scripts/ci extension` (the final run at the end of TASK-421)

## Result

Completion no longer lists compiler bindings. Hover and references on glue
answer nothing, and hover on a user binding still answers
(`const a: string`).
