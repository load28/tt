# TASK-459: Collect variants exported through export specifiers for cross-file exhaustiveness

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`shape.tt` declared `variant Color { Red, Green }` and exported it with `export { Color as Hue };`. A `match` over `Hue` in an importer that covered only `Red` passed `ttc --check` with exit 0; only `--check-types` reported the missing `Green`. `docs/ai/tt.md` says exported variants of a direct relative import are checked, and this variant is exported.

## Scope

- Included: A module's local export specifiers at module level (`export { X }`, `export { X as Y }`, `export type { ... }`, including `export { X as default }`), for every consumer of a module's exported variants: the CLI extern cache, the content mapper, and the engine's snapshot, project cache, go-to-definition, and language-service collection.
- Excluded: Re-exports (`export { X } from "./a.tt"`) and default import bindings (`import D from "./a.tt"`). Both stay outside the documented one-hop, named-binding scope. A re-export is a chain, and a default import binding is not recorded by the import clause collector.

## Decisions

### Decision 1: Compute a module's exported variants once, under their exported names, in the library API

- **Context**: Each consumer filtered the parsed declarations on `VariantSymbol::exported` (the `export` modifier), and the CLI and content mapper used `exported_variants_with_kind`, which applied the same filter. Nothing read `export { ... }` clauses, so a variant exported that way had no exported name anywhere.
- **Alternatives considered**: Rewriting the specifier name on the importer side would need the target module's export clauses at every collection site. That means six places with the same logic. Marking the declaration `exported` alone would keep the local name (`Color`), and an importer binding `Hue` would still find nothing.
- **Decision and rationale**: The parser gains `local_export_specifiers`, which reads `export [type] { ... }` clauses at bracket depth 0 that are not followed by `from`, as (local, exported) pairs. The public `exported_variant_symbols_with_kind` returns the `export variant` declarations, then one entry per specifier that names a declared variant, renamed to the exported name, with the declaration's offsets. `exported_variants_with_kind` derives from it, and the engine caches it per content version in place of the unfiltered symbol list, which it only ever read through the `exported` filter. The import side and its alias handling are unchanged, and the semantic cache key (the resolved extern symbols) now reflects export-clause edits too.

### Decision 2: `export { X as default }` is collected under the name `default`

- **Context**: The export surface records whatever name the specifier exports. `import { default as D } from "./a.tt"` already produces the named entry `("default", Some("D"))`.
- **Alternatives considered**: Dropping `default` entries would reject a named import that works. Collecting the default import binding would change the import clause model that TASK-458 left in place.
- **Decision and rationale**: Record it. The named form works, and the default binding stays out of scope. `docs/ai/tt.md` states both halves.

## Work log

- 2026-09-28: Reproduced: `ttc --check m2.tt shape.tt` exited 0; `--check-types` reported `missing "Green"`.
- 2026-09-28: Found that every extern collection path filtered on the `export` modifier (`src/lib/api.rs`, `src/main/loading.rs` through it, `src/content_mapper.rs` through it, and `src/engine/{project,names}.rs`, `src/engine/semantics/declarations.rs`, `src/engine/language/service.rs` directly).
- 2026-09-28: Added `local_export_specifiers` (`src/parser/imports.rs`, re-exported from `src/parser/mod.rs`), `exported_variant_symbols[_with_kind]` (`src/lib/api.rs`), and switched the engine caches (`src/engine/projection.rs`, `src/engine/snapshot.rs`) and collectors to it.
- 2026-09-28: Checked `export { X }`, `export type { X }`, `export { X as default }` with `import { default as C }`, a rename on import, and a namespace import (all report `missing "Green"`); a re-export chain still compiles unchecked, as documented.
- 2026-09-28: Added tests in `tests/compile/cases_05.rs` (API surface), `tests/cli.rs` (default path), and `tests/native/cases_03.rs` (typed engine resolves a nested payload through an aliased export). Updated `docs/ai/tt.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `ttc --check m2.tt shape.tt` exits 1 with `match on variant Hue (imported from "./shape.tt") is not exhaustive: missing "Green"`.

## Result

Changed `src/parser/imports.rs`, `src/parser/mod.rs`, `src/lib/api.rs`, `src/engine/projection.rs`, `src/engine/snapshot.rs`, `src/engine/project.rs`, `src/engine/names.rs`, `src/engine/semantics/declarations.rs`, `src/engine/language/service.rs`, `tests/compile/cases_05.rs`, `tests/cli.rs`, `tests/native/cases_03.rs`, `docs/ai/tt.md`, `docs/tasks/TASK-459-variant-export-specifiers.md`, and `docs/tasks/INDEX.md`.
