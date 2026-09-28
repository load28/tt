# TASK-464: Or-pattern bindings navigate and rename as one binding

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A binding written in every alternative of an or-pattern was invisible to navigation and rename:

```
variant Shape { Circle(size: number), Square(size: number), Point }
if let Circle(size: q) | Square(size: q) = s { return q; }
let Circle(size: z) | Square(size: z) = s else { return 0; };
const b = match (s) { Circle(size) | Square(size) => size, Point => 0 };
```

Definition on the use of `q` or `z` answered `[]`, references listed only the use, and rename answered `null` for all three. A single alternative worked. LSP 3.17 `textDocument/definition` may answer several `Location`s, `textDocument/references` lists every reference including declarations when `includeDeclaration` is set, and `textDocument/rename` answers one `WorkspaceEdit` that must rename every occurrence.

## Scope

- Included: The or-pattern binding lowering's source record (`src/codegen`), the emit metadata that carries it (`src/lib/mapped.rs`, `src/lib/compile.rs`, `src/codegen/contextual.rs`), and the engine's position and location mapping for definition, references, and rename (`src/engine/language.rs`, `src/engine/language/{service,project}.rs`).
- Excluded: Hover on an alternative's binding. It keeps isolating the alternative (`Project::hover`), because the per-alternative payload type is more precise than the merged binding's type. The emitted TypeScript is unchanged byte for byte.

## Decisions

### Decision 1: Record a 1:N shared binding in codegen instead of a byte mapping

- **Context**: The lowering writes one destructuring (`const { size: q } = $tt_t0;`) for all alternatives, as compiler text with no mapping. `tests/emit_map.rs` (`or_pattern_bindings_are_left_unmapped`) fixes why: an `EmitMapping` is 1:1, and claiming one alternative's bytes would let a rename rewrite that alternative alone. With no mapping, the source position could not reach the service, and the service's answer (the generated binding) could not come back.
- **Alternatives considered**:
  - Map the generated binding to the first alternative. It breaks the 1:1 invariant's purpose: rename would rewrite one alternative and leave the program inconsistent.
  - Extend the engine's analysis fallback (`body_definitions`) to let-else and if-let sites and synthesize references and rename from pattern analysis. That would reimplement scope resolution outside the checker and could not rename uses TypeScript resolves (shadowing).
  - Emit one destructuring per alternative. It changes emitted code and does not type-check for a narrowed union.
- **Decision and rationale**: Codegen records a `SharedBinding { out, out_end, occurrences }` for each binding it writes in an or-pattern destructuring. Each `BindingOccurrence` is one source span in one alternative, with `shorthand` set when the occurrence is also the field name (`Circle(size)`). The occurrences are every binding with the same name across all alternatives of the outermost or-pattern (nested or-patterns included), in source order. The rope carries it as marks (`SharedBindingStart`, `SharedBindingOccurrence`, `SharedBindingEnd`), the same mechanism declared names use, so contextual annotation shifts it like every other output offset. `EmitMapping` stays 1:1 and the existing contract test stands.

### Decision 2: The engine maps a shared binding in both directions, and a rename edit to every occurrence

- **Context**: The service answers in generated coordinates, and a rename edit's text depends on the generated shape (`size: <new>` for a shorthand, `<new>` for an alias).
- **Alternatives considered**: Reuse TypeScript's `newText` for every occurrence. With mixed shapes (`Circle(size) | Square(size: size)`), the shorthand expansion written into an alias occurrence would produce `size: size: zz`.
- **Decision and rationale**: `to_service_name` maps a source position to the generated binding when no byte mapping covers it (used by `locations` and `rename` only; hover still uses `to_service`). `map_shared_target` turns a service range that is exactly a shared binding into one location per occurrence. Rename accepts only the two shapes TypeScript writes for the generated binding (`<new>` or `<generated>: <new>`) and writes each occurrence in its own source shape: `field: <new>` for a shorthand occurrence, `<new>` for an alias. Any other shape still refuses the whole rename, so the atomic rename rule holds.

## Work log

- 2026-09-28: Reproduced with the `ttc --server` driver: definition `[]`, references only the use, rename `null` for `q`, `z`, and `size`.
- 2026-09-28: Added `SharedBinding`/`BindingOccurrence` (`src/lib/mapped.rs`), rope marks and printing (`src/codegen/rope.rs`, `src/codegen/rope/builder.rs`), carried it through `src/lib/compile.rs` and `src/codegen/contextual.rs`, and recorded occurrences in `emit_binding` (`src/codegen/core/emitter/pattern.rs`, with `every_binding` and the or-pattern in `BindingGroup` in `helpers.rs`).
- 2026-09-28: Added `to_service_name` and `map_shared_target` (`src/engine/language/service.rs`) and used them in `Project::locations` and `Project::rename` (`src/engine/language/project.rs`); updated the hover, definition, and emit-map comments that described or-pattern bindings as unreachable.
- 2026-09-28: Listed TASK-463's `binds` field in the `ttSymbol` protocol summary at the top of `src/server.rs`, which TASK-463 left out.
- 2026-09-28: Tests: `an_or_pattern_binding_stands_for_every_alternative_it_is_written_in` (`src/engine/language/tests.rs`: occurrences, shorthand flags, tuple element, single alternative unaffected), `or_pattern_bindings_navigate_and_rename_as_one_binding` (`tests/native/cases_03.rs`: definition, references, and rename from each declaration and from the use), and the LSP test "or-pattern bindings navigate and rename as one binding across the LSP adapter" (`editors/vscode/server/src/test/server.test.ts`).

## Issues and resolutions

### Issue 1: Definition at a shorthand or-pattern occurrence lands on the variant field

- **Symptom**: In the native test, definition at `Circle(size)` answered the `size` fields of the variant declaration.
- **Cause**: The generated binding is a destructuring shorthand (`const { size } = ...`), and TypeScript answers definition on a shorthand with the property declaration, the same as for `const { size } = o` in plain TypeScript. The LSP adapter answers this position from `ttSymbol` (a field that binds, TASK-463) before asking the engine.
- **Resolution**: Kept TypeScript's answer. The test checks navigation for shorthand bindings from the use, and rename from every position.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`
- [x] Emitted TypeScript for or-pattern samples is byte-identical before and after.

## Result

Changed `src/lib/mapped.rs`, `src/lib/compile.rs`, `src/server.rs` (protocol comment), `src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `src/codegen/contextual.rs`, `src/codegen/core/emitter/helpers.rs`, `src/codegen/core/emitter/pattern.rs`, `src/engine/language.rs`, `src/engine/language/service.rs`, `src/engine/language/project.rs`, `src/engine/language/tests.rs`, `tests/emit_map.rs`, `tests/native/cases_03.rs`, `editors/vscode/server/src/test/server.test.ts`, this record, and `docs/tasks/INDEX.md`. Definition, references, and rename now treat an or-pattern binding as one binding written in every alternative.
