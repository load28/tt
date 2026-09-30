# TASK-611: Answer `textDocument/prepareRename` by the rules rename refuses by

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-611: Answer textDocument/prepareRename by the rules rename refuses by`

## Purpose

The server did not offer `textDocument/prepareRename`, so the editor opened
the rename box on anything and a refused rename (a tt tag, a standard
library symbol) showed "No result" only after the user had typed a new
name. TypeScript's server answers `prepareRename` with the name's range, or
refuses with a reason ("You cannot rename elements that are defined in the
standard TypeScript library.") before the box opens.

## Scope

- Included: The `prepareProvider` capability and the `onPrepareRename`
  handler (`editors/vscode/server/src/server.ts`), `Workspace::prepare_rename`
  and `PrepareRename`, `Project::rename_answer` (TypeScript's refusal
  reason), `Service::answer` (a server's error answer apart from a failed
  conversation), and the `prepareRename` server method.
- Excluded: Which renames are allowed: the rules are the existing ones.

## Decisions

### Decision 1: Prepare is the rename, asked ahead of the new name

- **Context**: LSP 3.17 `textDocument/prepareRename` (client capability
  `rename.prepareSupport`, server capability
  `renameProvider.prepareProvider`) answers `Range | { range, placeholder }
  | { defaultBehavior } | null`: null when a rename is not valid at the
  position, and an error whose message the client shows when it cannot be
  performed for a reason. VS Code calls `RenameProvider.prepareRename`
  before it opens the input box and reports the rejection there.
- **Alternatives considered**: (a) Forward TypeScript's `prepareRename`
  answer mapped to the source: TypeScript accepts renames the engine then
  refuses whole (an edit landing in glue, a project that refuses), so
  prepare would promise what rename does not do. (b) A separate set of
  prepare rules: two rule sets drift.
- **Decision and rationale**: `Workspace::prepare_rename` runs the rename
  itself, with the placeholder name, across every project exactly as
  `Workspace::rename` does, and answers the range of its edit in the
  requesting file that covers the position (the placeholder is that text).
  It refuses when the rename refuses or has no edit at the position. A
  rename is asked for only on the user's request, so computing it once more
  there costs nothing per keystroke.

### Decision 2: A refusal carries its reason

- **Context**: TypeScript refuses some renames with an error answer, which
  the service client turned into a failed request, the same as a dead or
  timed-out conversation: the engine reported an engine error and the
  editor showed nothing.
- **Alternatives considered**: Keep the error a failure: the user sees no
  reason, and a refusal is logged as if the engine had failed.
- **Decision and rationale**: `Service::answer` returns a server's error
  answer as its value (`Ok(Err(message))`), apart from a failed
  conversation (`Err`); `request` keeps its contract on top of it. Rename
  reads TypeScript's `prepareRename` through it, so a refusal is a refusal
  (`Project::rename_answer`) with TypeScript's words, and `rename` answers
  `None` for it instead of an error. The adapter answers a refusal with a
  reason as a `RequestFailed` error carrying the reason, a tt name the
  adapter refuses (a variant, a case tag, or a payload field that binds
  nothing) with an error naming it, and any other refusal with null.

## Work log

- 2026-09-30: Confirmed the cause in `server.ts`: the server advertised
  `renameProvider: true` without `prepareProvider` and registered no
  `prepareRename` handler, so the editor never asked before opening the
  rename box.
- 2026-09-30: Added the capability, the handler, `prepare_rename`,
  `rename_answer`, and `Service::answer`.
- 2026-09-30: Re-ran the harness: bindings in `flow`, `result`, a guard, and
  let-else prepare with their range; `console.log` refuses with
  TypeScript's message; a case tag refuses with "A tt case tag cannot be
  renamed."; a shorthand payload binding prepares its name.
- 2026-09-30: Tests:
  `prepare_rename_answers_the_range_a_rename_would_replace_or_refuses_as_it_would`
  (`tests/native/editor_service.rs`) and the extension test "prepare rename
  answers the name, refuses a tt name with a reason" (`server.test.ts`).

## Issues and resolutions

### Issue 1: A standard library symbol failed the request instead of refusing

- **Symptom**: The first native test failed with `TypeScript language
  service request textDocument/prepareRename failed: You cannot rename
  elements that are defined in the standard TypeScript library.`
- **Cause**: The service client folded TypeScript's error answer into the
  failure channel.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native rename`
- [x] `node --test server/out/test/server.test.js` (rename cases)
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/typescript/service.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/workspace.rs`,
`src/engine/mod.rs`, `src/server.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/server.ts`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/test/server.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. The editor refuses a
rename before the user types a name, with the reason, and prepares an
allowed one with the name's range.
