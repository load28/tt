# TASK-529: Answer a document opened through a symlink under its own URI

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

A `.tt` file opened through a symlinked directory received rename edits,
definitions and references under its target's path. VS Code applied the
rename to a second, unopened document and opened a duplicate tab for a
definition in the same file.

## Scope

- Included: How the extension turns an engine path into a URI
  (`editors/vscode/server/src/server.ts`).
- Excluded: The engine's own identity of a file, which stays its real path.

## Decisions

### Decision 1: The engine keeps real paths; the adapter names files the way the editor opened them

- **Context**: `engine::normalize_document_path` canonicalizes every path, so
  one file has one project identity however it is reached. The adapter built
  `URI.file(path)` from those answers, and `runTypedCheck` in `ttc.ts` already
  compared real paths to accept its own file's diagnostics.
- **Alternatives considered**: Making the engine keep the spelling it was
  given would split one file into two project members and let an import
  reach it under a third name.
- **Decision and rationale**: `editorUri` in `server.ts` returns the URI of
  the open document that is the same file — the same path, or the same real
  path — and `URI.file` otherwise. Every engine path that becomes a URI
  passes through it: definitions (both the tt-symbol and the service
  answer), references, rename edits, and related information on service and
  typed diagnostics.

## Work log

- 2026-09-29: Reproduced with `link7 -> p7`: `rename` on a local answered
  `…/p7/use2.tt` for the open `…/link7/use2.tt`. Added `editorUri` and
  `realPath`, and the LSP case "a document opened through a symlink receives
  its own locations and edits"; it fails on the previous `server.ts` and
  passes with the change.

## Issues and resolutions

None.

## Verification

- [x] `editors/vscode`: `npm run compile`, then all server and client tests:
  222 passed.
- [x] The Rust side is unchanged; `TTC_REQUIRE_TSGO=1 cargo test` passed on
  the same tree.

## Result

Changed `editors/vscode/server/src/server.ts` and
`editors/vscode/server/src/test/server.test.ts`. Rename, definitions,
references and related information for a document opened through a symlink
now address that document.
