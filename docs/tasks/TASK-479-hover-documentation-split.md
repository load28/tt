# TASK-479: Separate hover documentation from the signature

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`HoverInfo.documentation` was always empty: the engine asked the TypeScript language server for plaintext hover, and the pinned tsgo's plaintext answer concatenates the signature and the JSDoc with no separator (`function add(a: number): numberAdds two numbers.`). The documentation was glued onto the signature, and the TASK-468 native test had to check the whole answer instead (TASK-468, Issue 2).

## Scope

- Included: The hover content format the service declares (`src/typescript/service.rs`), the hover contents parser and its two callers (`src/engine/language/service.rs`, `src/engine/language/project.rs`), the TASK-468 native test, a new native test, an engine unit test, and a VS Code server test for the rendered hover.
- Excluded: The documentation format of completion resolve and signature help (they already arrive as a separate `documentation` field).

## Decisions

### Decision 1: Request Markdown hover and split at its code fence

- **Context**: The pinned tsgo (`typescript@7.1.0-dev.20260826.1`) exposes quick info only through the LSP `textDocument/hover` request of `tsgo --lsp`; the `typescript` package's JavaScript API has no quick-info call. Probing the server with a documented function showed three answer shapes, chosen by the client capabilities:
  - `contentFormat: ["plaintext", ...]`: `{ kind: "plaintext", value: "function add(a: number, b: number): numberAdds two numbers.\n\n@param `a` — the first ..." }` — no boundary between the parts.
  - `contentFormat: ["markdown", ...]`: `{ kind: "markdown", value: "```typescript\nfunction add(a: number, b: number): number\n```\nAdds two numbers.\n\n*@param* `a` — the first\n\n*@returns* — the sum" }` — the signature is the fenced block, and the documentation with its JSDoc tags follows it.
  - `_vs_supportsVisualStudioExtensions`: an additional `_vs_rawContent` classified-text tree; its documentation element omits the JSDoc tags, and it is a Visual Studio extension rather than LSP.
- **Alternatives considered**:
  - Keep plaintext and split at the first paragraph break or at a signature-shaped prefix. The server gives no boundary, so any split is a guess about text shape (contract 3).
  - Use `_vs_rawContent`. Non-standard, drops the tags, and changes the signature text (`...: number;`).
- **Decision and rationale**: The service declares `contentFormat: ["markdown", "plaintext"]` (LSP: the order is the client's preference), so the server's own MarkupContent structure carries the boundary. `split_hover` now reads the LSP `contents` value by its kind: Markdown splits at the leading code fence (signature) and keeps everything after the closing fence, JSDoc tags included, as the documentation; plaintext and a `{ language, value }` MarkedString are code with no documentation boundary. `HoverInfo.documentation` is therefore Markdown, which is what the VS Code adapter already renders: it builds LSP 3.17 `MarkupContent` of kind `markdown` with the signature in a `ts` fence and the documentation after it.

## Work log

- 2026-09-28: Reproduced: the TASK-468 native hover answer had `signature: "(property) Circle: (radius: number) => ShapeA circle around the origin."` and empty documentation. Probed `tsgo --lsp` directly with plaintext, Markdown, and Visual Studio capabilities (answer shapes above).
- 2026-09-28: Changed the hover `contentFormat` preference (`src/typescript/service.rs`); rewrote `split_hover` to take the LSP `contents` value (`src/engine/language/service.rs`); both hover paths pass `hover["contents"]` (`src/engine/language/project.rs`).
- 2026-09-28: Removed the stale doc comment on `split_hover` rather than rewording it (no new code comments).
- 2026-09-28: Tests: `hover_markdown_separates_signature_from_documentation_and_tags` (`src/engine/language/tests.rs`); `variant_case_and_field_docs_reach_hover_and_signature_help` now asserts the exact `signature` and `documentation`, and `hover_documentation_and_jsdoc_tags_are_separate_from_the_signature` covers JSDoc tags and an undocumented symbol (`tests/native/cases_03.rs`); the VS Code server test `a documented TypeScript hover renders its signature and documentation as separate parts` pins the rendered Markdown (`editors/vscode/server/src/test/server.test.ts`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension` (208 tests pass, none skipped)

## Result

Changed `src/typescript/service.rs`, `src/engine/language/service.rs`, `src/engine/language/project.rs`, `src/engine/language/tests.rs`, `tests/native/cases_03.rs`, `editors/vscode/server/src/test/server.test.ts`, `docs/tasks/TASK-468-variant-body-comments.md` (Issue 2 now points here), this record, and `docs/tasks/INDEX.md`. Hover answers carry the signature and the Markdown documentation, JSDoc tags included, as separate fields, and the editor shows them as a code block followed by the documentation.
