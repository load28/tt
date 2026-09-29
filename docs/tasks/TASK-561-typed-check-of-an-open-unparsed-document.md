# TASK-561: Check an open document through its faithful projection while it does not parse

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-561: Check an open document through its faithful projection while it does not parse`

## Purpose

An unchanged type error changed its range and wording whenever another line
of the buffer stopped parsing: `export const bad: number = "x";` read
`0:27-0:30 ts2322 type mismatch: expected number, found "x"` (the typed
layer) and, while line 2 was `1 + ;`, `0:13-0:16 2322 Type 'string' is not
assignable to type 'number'.` (the service layer). Assignability is rendered
from checker facts by tt's shared renderer (docs/ai/tt.md); a transient
syntax error must not switch it to another rendering.

## Scope

- Included: Which projection a snapshot holds for a document whose
  TypeScript does not parse (`ProjectedDocument::project_for_snapshot`),
  the projection cache, and the declarations written from such a document.
- Excluded: The batch path over files read from disk, which stays blocked
  as `tsc` reports only syntax errors while there are any (TASK-433,
  TASK-527); the service layer's own wording, which the typed layer
  replaces whenever it checked the buffer.

## Decisions

### Decision 1: The typed layer keeps checking an open document mid-edit

- **Context**: The editor merges the typed pass over the service layer and
  lets it replace the service's problems only when the pass checked the
  buffer (TASK-527, Decision 4). A buffer whose TypeScript does not parse
  was a blocked file of its snapshot (`export {};` in its place), so the
  service layer's TypeScript prose was shown until the syntax error went
  away, and the typed rendering came back after it: the switch was the
  flicker.
- **Alternatives considered**: (a) Render the service layer's 2322 through
  the shared renderer: the language service reports no checker facts
  (expected/found types, the mismatched expression's range), and deriving
  them from its message is the prose rewriting the rendering contract
  forbids. (b) Keep the previous typed answer while the pass is blocked: a
  stale answer about text that changed. (c) Check every file whose
  TypeScript does not parse, batch included: `ttc --check-types` would then
  report semantic errors and TypeScript's syntax errors beside the tt
  restatement, unlike `tsc`.
- **Decision and rationale**: TASK-527 already builds the faithful
  projection (`ProjectionReport::withheld`) whenever no tt text is left as
  written, and the service reads the buffer through it. A document held
  open (`Project::opened`: an editor buffer, or a `--overlay` of the
  editor's one-shot fallback, which must answer as the server does) is now
  projected through it, marked `unparsed`; the typed pass checks it and
  answers with the checker's facts — the same `type mismatch` at the same
  range — and TypeScript's own syntax diagnostics, and is not `blocked`, so
  it replaces the service layer as it does for a parsing buffer. A file
  read from disk is still blocked. A cached projection built for an open
  document is not reused once the file is read from disk. No declarations
  are written from an unparsed projection. This narrows TASK-527's
  Decision 4; that record says so.

## Work log

- 2026-09-29: Reproduced through `ttc --server` (`typedCheck` with
  `includeTypes`, `tsDiagnostics`) with the reported buffer and its edit:
  the typed pass answered `blocked: true` and no 2322 for the edited
  buffer.
- 2026-09-29: Added `ProjectedDocument::unparsed` and the `open` argument
  of `project_for_snapshot` (`src/engine/projection.rs`), the cache
  condition in `Project::update` (`src/engine/project.rs`), and the
  declarations filter (`src/engine/semantics/declarations.rs`). The edited
  buffer answers `blocked: false`, `1:28 ts2322 type mismatch: expected
  number, found "x"`, and `ts1109` at the `;`.
- 2026-09-29: Tests: `a_type_error_keeps_its_rendering_while_an_open_document_does_not_parse`
  (`tests/native/cases_10.rs`, fails with the open rule disabled; also
  pins that a file read from disk stays blocked), "an untouched type error
  reads the same while another line does not parse" (`server.test.ts`).
  Updated TASK-527's tests to the new contract: `typedcheck.test.ts` (a
  buffer mid-edit is checked; rolled-back tt text still blocks) and
  "a syntax error keeps the file's type errors and is stated once"
  (`server.test.ts`).

## Issues and resolutions

### Issue 1: The typed layer's statement of the syntax error was dropped

- **Symptom**: With the typed pass answering, "a syntax error keeps the
  file's type errors" published no 1005 for `o.` above `export {};`.
- **Cause**: The compiler layer's `verify-failed` sat at the position of
  TypeScript's `ts1005`, so `mergeTyped` skipped the typed `ts1005` as a
  second problem there; the restatement was removed only after the merge,
  leaving neither.
- **Resolution**: `validate` removes the restatements (from the compiler
  layers and the typed answer) before the merge
  (`editors/vscode/server/src/server.ts`).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed.
- [x] `editors/vscode`: `npm run compile`, then all server and client tests.

## Result

A type error reads the same while another line of the buffer is being
typed. Changed `src/engine/projection.rs`, `src/engine/project.rs`,
`src/engine/semantics/declarations.rs`, `src/lib/scaling_tests.rs`,
`tests/native/cases_10.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/server.test.ts`,
`editors/vscode/server/src/test/typedcheck.test.ts`,
`docs/design/lsp-architecture.md`, and the TASK-527 record.
