# TASK-532: Show TypeScript's declarations in a tt file's outline

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

The outline and breadcrumbs of a `.tt` file listed only its variants.
Functions, classes, interfaces, methods and variables — everything a `.ts`
file's outline shows — were missing.

## Scope

- Included: A `documentSymbols` engine answer (`Project::document_symbols`,
  `source_symbols`, the service's `documentSymbol` capability, the server
  method) and the extension's `onDocumentSymbol`.
- Excluded: Workspace symbols, and the variant entries themselves, which keep
  coming from tt's declarations.

## Decisions

### Decision 1: The outline is TypeScript's, mapped onto the source

- **Context**: The service answers `textDocument/documentSymbol` over the
  projection. Its entries include names ttc wrote: generated bindings
  (`$tt_v0`), and the type alias and constructor object a `variant` becomes.
- **Alternatives considered**: Building an outline from tt's parser would
  repeat TypeScript's declaration model for every host construct.
- **Decision and rationale**: The session declares
  `hierarchicalDocumentSymbolSupport` (LSP 3.17) and `source_symbols` keeps an
  entry when its selection range maps verbatim to the source. An entry
  whose name is glue is dropped and its mapped children take its place. A
  kept entry's range takes each end from the source where that end was
  copied, and otherwise stays at the name, so a function whose body holds a
  `match` covers its whole text. Each level is sorted by source position,
  because lowering can hoist a pattern binding above its `match`.

### Decision 2: Variants join the tree where they are declared

- **Context**: A variant's outline entry (cases as members) comes from tt's
  declarations and works without a toolchain.
- **Decision and rationale**: `onDocumentSymbol` asks for both and places
  each variant in source order inside the innermost TypeScript entry whose
  range contains it (`insertSymbol`), so a variant in a namespace nests
  under it. Without an engine answer the outline is the variants alone, as
  before.

## Work log

- 2026-09-29: Probed a file with an interface, a variant, a class whose
  method holds a `match` and a pipeline, and functions: the engine answer
  lists every user declaration with source ranges, no generated name and no
  variant alias. Added the extension merge.
- 2026-09-29: Added `the_outline_keeps_the_users_declarations_and_leaves_out_generated_ones`
  (`tests/native/cases_06.rs`) and the LSP case "the outline lists
  TypeScript's declarations with the variants in source order".

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests, 224 passed

## Result

Changed `src/typescript/service.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/engine/mod.rs`, `src/server.rs`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`, `tests/native/cases_06.rs` and
`editors/vscode/server/src/test/server.test.ts`. A `.tt` file's outline and
breadcrumbs show its functions, classes, interfaces, members and variables
beside its variants.
