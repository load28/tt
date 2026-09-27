# TASK-400: Version quick-fix edits and convert fallback diagnostic columns

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Two coordinate defects in the language server: (a) a quick fix applied the line/column edit stored in a diagnostic's `data` to whatever the document had become since, with no version check, so a fix requested after an edit replaced the wrong text; (b) the one-shot fallback (`ttc --check`, `ttc --check-types --overlay`) passed the compiler's character (code point) columns through as protocol columns, so every diagnostic after an astral character such as an emoji was misplaced by one UTF-16 unit per such character, unlike the engine path.

## Scope

- Included: `toDiagnostic`, `suggestedFixes` and the initialize capability read in `editors/vscode/server/src/server.ts`; the one-shot paths in `ttc.ts`; regression tests.
- Excluded: the engine's own conversion (`src/server.rs` `protocol_position`), which is already correct and is the reference.

## Decisions

### Decision 1: A fix belongs to the document version its diagnostic was computed for

- **Context**: LSP 3.17 (*Code Action Request*) returns `CodeAction.edit: WorkspaceEdit`. The edit here is the compiler's, computed with the diagnostic for one version of the text and carried in `Diagnostic.data` ("A data entry field that is preserved between a `textDocument/publishDiagnostics` notification and `textDocument/codeAction` request"). The client may send a diagnostic from an earlier publish, because the replacement publish for the new version has not arrived yet.
- **Alternatives considered**:
  - Re-validate on every code action request. Rejected: it recomputes the whole check on a request that must be fast, and the published diagnostic the user is looking at would still describe the old text.
  - Shift the stored ranges through the edits since. Rejected: the server keeps no edit history, and a compiler fix is only valid for the text the compiler saw.
- **Decision and rationale**: `toDiagnostic` stores the document version beside the suggestions in `data`. `suggestedFixes` offers a compiler fix only when that version is the document's current version, so a stale diagnostic produces no edit. When the client declares `workspace.workspaceEdit.documentChanges` (LSP 3.17, *WorkspaceEditClientCapabilities*), the edit is sent as `documentChanges: [TextDocumentEdit]` whose `textDocument` is an `OptionalVersionedTextDocumentIdentifier` carrying that version (LSP 3.17, *TextDocumentEdit*: "The text document to change"; *WorkspaceEdit*: "If the client can handle versioned document edits and if `documentChanges` are present, the latter are preferred over `changes`"), so a client that enforces document versions can refuse the edit if the document changes between the response and the user choosing the fix. A client without the capability receives `changes` as before. The server-side version gate is the guard that matters in VS Code: `vscode-languageclient` 9.0.1 declares `documentChanges` (`lib/common/client.js`), but its code-action conversion (`protocolConverter.js` `asWorkspaceEdit`) rebuilds a `vscode.WorkspaceEdit` without the version, and it compares versions only for server-initiated `workspace/applyEdit` requests.

### Decision 2: The one-shot fallback converts columns at its own boundary

- **Context**: LSP 3.17 (*Position*): "Character offset on a line in a document (zero-based). The meaning of this offset is determined by the negotiated `PositionEncodingKind`", and the server negotiates none, so UTF-16 applies. The compiler renders code point columns (`src/error.rs` `utf16_column` documents the difference), and the engine path converts at the protocol boundary (`src/server.rs` `protocol_position`). Verified with the built `ttc`: after `"😀😀"`, `--check` reported column 39 where the UTF-16 column is 41.
- **Alternatives considered**: converting in `toDiagnostic` for every diagnostic. Rejected: engine diagnostics are already UTF-16 and would be converted twice.
- **Decision and rationale**: `TtcDiagnostic` columns are UTF-16 on every path. The one-shot functions, which own the text they checked, convert each parsed column with `utf16Column`, a port of `src/error.rs` `utf16_column` (same line splitting on `\n`, same treatment of the end-of-line position and of positions past it). `parseStderr` still returns exactly what the compiler printed.

## Work log

- 2026-09-27: Added `a quick fix edits only the document version its diagnostic was computed for` to `server.test.ts` (with and without `documentChanges`), and two tests to `compiler.test.ts` that force the one-shot path with a wrapper compiler that refuses `--server` and compare its column with the engine's after an emoji for `--check` and for the typed check. All failed against the previous sources (the stale fix was offered; columns 39 and 46 where the engine reports 41 and 48).
- 2026-09-27: Implemented both decisions in `server.ts` and `ttc.ts`, and added a unit test for `utf16Column` in `parse.test.ts`.

## Issues and resolutions

### Issue 1: The first typed assertion assumed the wrong anchor

- **Symptom**: the engine reported `ts2322` at the initializer `"wrong"`, not at the declared name.
- **Cause**: the typed layer's type-mismatch renderer anchors at the value; the test had assumed TypeScript's default anchor.
- **Resolution**: the test asserts the engine column against the initializer and then requires the one-shot column to equal the engine's.

### Issue 2: VS Code does not enforce the version of a code action's edit

- **Symptom**: reading `vscode-languageclient` 9.0.1 showed that `TextDocumentEdit.textDocument.version` is dropped when a code action's edit is converted.
- **Cause**: the client's version check exists only in its `workspace/applyEdit` handler.
- **Resolution**: the fix does not rely on the client: the server offers no edit for a diagnostic whose version is not the current one, which covers every code action request made after an edit reached the server. The remaining window, an edit typed after the code action response and before the user picks the fix, is the client's to close; VS Code re-requests code actions when the document changes. Recorded as a residual limitation rather than worked around.

## Verification

- [x] `npm run compile` in `editors/vscode`
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js` with `target/debug` on `PATH`
- [x] `node scripts/check-task-index`

## Result

Changed files: `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/ttc.ts`, `editors/vscode/server/src/test/server.test.ts`, `editors/vscode/server/src/test/compiler.test.ts`, `editors/vscode/server/src/test/parse.test.ts`, `docs/tasks/INDEX.md`, this record. Compiler fixes apply only to the text they were computed for, and the fallback reports the same positions as the engine.
