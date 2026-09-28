# TASK-463: Rename a shorthand pattern binding from its declaration

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`const a = match (s) { Circle(radius) => radius * 2, ... }`: an LSP rename with the cursor on `radius` inside `Circle(radius)` answered `null`, and so did `let Rect(width, height) = s else { ... }` on `width`. The same rename from the use site worked and rewrote the pattern to `Circle(radius: zz)`, and `ttc --server rename` at the declaration returned the correct edits. LSP 3.17 `textDocument/rename` asks for a workspace edit for the symbol at the position, and a declaration position is as valid as a use.

## Scope

- Included: The engine's classification of a tt name at a position (`src/engine/names.rs`), its JSON-lines answer (`ttSymbol` in `src/server.rs`), and the VS Code adapter's rename gate (`editors/vscode/server/src/server.ts`, `engine.ts`).
- Excluded: Renaming a payload field itself (from the variant declaration or from an explicit `field: alias` key). That still needs tt-aware rewriting of the declaration, every pattern, and emitted keys, and stays refused. Or-pattern bindings are TASK-464.

## Decisions

### Decision 1: A shorthand pattern position is a field reference that also binds, and the engine says so

- **Context**: `tt_symbol_at` classified the identifier in `Circle(radius)` as `kind: "field"`, which is correct for hover and definition (it names `Shape.Circle`'s `radius`). `onRenameRequest` refused every position `ttSymbol` answered for, because renaming a field needs rewriting TypeScript cannot do. The shorthand position is also the declaration of the local `radius`, and that binding is what the engine's rename renames (TypeScript expands the emitted destructuring shorthand to `radius: <new>`, which the engine carries as `RENAME_PLACEHOLDER` text).
- **Alternatives considered**:
  - A fourth `TtSymbolKind` (`Binding`) for the shorthand. Hover and definition would lose the field answer, or every consumer would have to treat the new kind as a field again.
  - Dropping the adapter gate and letting the engine's atomic rename rule refuse field renames. Today the rule happens to refuse them because the positions map to no service text, but a pattern case tag or field key does map to emitted text, and the gate is what says a tt name is not TypeScript's to rename.
  - Deciding in the adapter from the text (`ttSymbol` range equals a word followed by `)` or `,`). A second opinion outside the engine, and wrong for nested patterns.
- **Decision and rationale**: `TtSymbol` gains `binds: bool`, set when the resolved field span coincides with a pattern binding's span in the analysis (`PatternAnalyses::binding_at`). That is exactly the shorthand form, nested leaves and let-else / if-let sites included, and no other position. The JSON answer carries `binds`; the adapter refuses a tt name only when it does not bind. The field itself stays distinct: the variant declaration's `radius: number` and an explicit key (`Circle(radius: r)`) report `binds: false` and remain refused.

## Work log

- 2026-09-28: Reproduced with the hunter's `ttc --server` driver: `rename` at `Circle(radius)` returns `radius: NN` plus the body use, while `ttSymbol` answers `kind: "field"`; through the LSP adapter the answer was `null`.
- 2026-09-28: Added `binds` in `src/engine/names.rs` (computed from the analysis' pattern bindings) and serialized it in `src/server.rs`; typed it in `editors/vscode/server/src/engine.ts` and changed the gate in `editors/vscode/server/src/server.ts`.
- 2026-09-28: Added `a_shorthand_payload_binding_is_a_field_that_also_binds` (`src/engine/names.rs`) and the LSP test "renaming a shorthand pattern binding at its declaration renames the binding" (`editors/vscode/server/src/test/server.test.ts`): match arm and let-else shorthands rename the binding; the declared field, an explicit field key, and a case tag answer `null`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`

## Result

Changed `src/engine/names.rs`, `src/server.rs`, `editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/test/server.test.ts`, `docs/tasks/TASK-463-shorthand-binding-rename.md`, and `docs/tasks/INDEX.md`. A rename at a shorthand pattern binding now renames the binding through the LSP, the same edits as from a use.
