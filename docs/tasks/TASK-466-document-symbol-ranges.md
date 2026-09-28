# TASK-466: DocumentSymbol ranges enclose the whole variant and each case

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

For `/** doc */ export declare variant P { R(v: number) }`, the `declarations` answer gave the variant's range as `P { R(v` (from the name to the last field name), and each case's range as its tag alone. LSP 3.17 `DocumentSymbol` says `range` encloses the whole symbol, "everything including leading/trailing whitespace but not comments", which clients use to find the symbol under the cursor and to fold, and that `selectionRange` is the name and must be contained in `range`.

## Scope

- Included: The case span in the AST and parser (`src/ast.rs`, `src/parser/variants.rs`), its HIR extent (`src/hir/lower.rs`), the declaration surface (`src/engine/declarations.rs`), the JSON-lines `declarations` answer (`src/server.rs`), and the VS Code adapter's `onDocumentSymbol` and engine types (`editors/vscode/server/src/server.ts`, `engine.ts`).
- Excluded: Leading documentation comments. The LSP text lets `range` omit comments, and the parser's declaration extent does not own them.

## Decisions

### Decision 1: Take the ranges from the parser's own extents

- **Context**: `tt_declarations` rebuilt the variant range from the name and the last name-bearing HIR node (a tag or a field name), so it could never reach a modifier, a type, or a delimiter. Cases had only a tag node. The parser already records the whole declaration (`VariantDecl::span`, from the first modifier through `}`), and HIR lowering records it as the name node's extent (`HirSourceMap::node_extent`). A case had no extent at all.
- **Alternatives considered**:
  - Scan the source text from the name for `export`/`declare` and forward for the matching `}` and `)`. A second parser outside the parser, and wrong around comments and nested generics.
  - Add span fields to the HIR `VariantData`. `node_extent` already models "the complete authored extent of a node" for statements; a case is the same kind of fact.
- **Decision and rationale**: The parser gives each `VariantCase` a `span` from the tag through the payload's closing `)` (the tag alone for a unit case). HIR lowering records it as the case node's extent. `tt_declarations` reads the variant's `span` from the name node's extent and gives each case `name_span` (the tag) and `span` (its extent). No existing extent consumer is affected: codegen asks extents only of statement nodes.

### Decision 2: Cases carry `nameSpan` and `span`, as variants do

- **Context**: The case's `span` had meant the tag. An outline needs both the enclosing range and the selection range.
- **Alternatives considered**: Keep `span` as the tag and add a differently named extent (`range`). The variant and its cases would then use the same key for different things.
- **Decision and rationale**: `TtCaseDecl` and the JSON answer now mirror the variant: `name_span`/`nameSpan` is the tag and `span` is the whole case. `onDocumentSymbol` was the only consumer of the case span, and it now uses `span` for `range` and `nameSpan` for `selectionRange`.

## Work log

- 2026-09-28: Reproduced with `declarations` on the example: variant `span` covered `P { R(v`, the case covered `R`.
- 2026-09-28: Added `VariantCase::span` (`src/ast.rs`, `src/parser/variants.rs`), recorded it as the case node's extent (`src/hir/lower.rs`), and rewrote the variant and case spans in `src/engine/declarations.rs`; added `nameSpan` for cases in `src/server.rs` and `editors/vscode/server/src/engine.ts`; changed `onDocumentSymbol` in `editors/vscode/server/src/server.ts`.
- 2026-09-28: Searched for other consumers of these spans (adapter, Rust callers, docs): only `onDocumentSymbol` and the `declarations` tests read them, and no document describes the outline range.
- 2026-09-28: Tests: `outline_ranges_enclose_the_whole_declaration_and_each_case` (`src/engine/declarations.rs`: `export declare` with a leading doc comment, multi-line `declare`, generics and optional fields; unit and payload cases), a tightened assertion in `locals_come_with_spans_imports_and_builtins_without`, and the LSP test "document symbol ranges enclose the whole variant and each case" (`editors/vscode/server/src/test/server.test.ts`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`

## Result

Changed `src/ast.rs`, `src/parser/variants.rs`, `src/hir/lower.rs`, `src/engine/declarations.rs`, `src/server.rs`, `editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/test/server.test.ts`, this record, and `docs/tasks/INDEX.md`. Outline symbols now enclose the whole declaration and each whole case, with the name as the selection range.
