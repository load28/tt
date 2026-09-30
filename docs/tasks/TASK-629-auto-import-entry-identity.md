# TASK-629: Resolve each auto-import entry against its own module

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-629: Resolve each auto-import entry against its own module`

## Purpose

When two modules export the same name, completion in a `.tt` file listed
two `kkValueLib` entries (label details `./lib` and `./shapes.tt`, TASK-612),
but accepting either one added `import { kkValueLib } from "./shapes.tt"`.
In a `.ts` file each entry imports from the module it names.

## Scope

- Included: The identity the engine caches a service entry under
  (`ServiceSession::last_completion`, `ts_completions`), the entry's
  `CompletionItem::source`, `Project::completion_resolve`'s `source`
  argument, the `source` of the `completion` answer and the
  `completionResolve` request (`src/server.rs`), and the adapter's
  `TsCompletionData` (`editors/vscode/server/src/server.ts`, `engine.ts`).
- Excluded: Which entries are listed, their labels and label details, and
  the edits an entry makes (unchanged).

## Decisions

### Decision 1: An entry is identified by its label and the service's `source`

- **Context**: The engine cached the raw service items under (file, asked
  offset, label), so the second `kkValueLib` overwrote the first, and the
  adapter's resolve data named only the label. Both entries resolved to
  whichever was listed last.
- **Alternatives considered**: (a) Pass the service's whole `data` object
  through the adapter and resolve by it: `data` is the TypeScript server's
  private resolve payload in served coordinates (`fileName` is the served
  `.tt.ts` document, `position` an offset in the emitted text), which would
  carry the service's projection past the engine. (b) Key the cache by the
  entry's index in the answer: an index is an artifact of one list, and
  resolve asks for the list again when the cache has moved on. (c) Tell
  entries apart by their label details: display text, and absent for most
  entries.
- **Decision and rationale**: TypeScript identifies a completion entry by
  its name and source (`CompletionEntryIdentifier` in
  `services/completions.ts`; `getCompletionEntryDetails(fileName, position,
  entryName, formatOptions, source, preferences, data)`), and tsgo's LSP
  item carries that source in `data.source` (`"./lib"`, `"./shapes.tt"`,
  or a snippet kind such as `SwitchCases/`). The engine keeps it as
  `CompletionItem::source`, caches each raw item under (file, offset,
  label, source), and resolves the entry named by label and source. The
  adapter stores the source in the item's `data`, which LSP 3.17 defines as
  the field preserved between `textDocument/completion` and
  `completionItem/resolve`, and sends it back in `completionResolve`.
  A module specifier is already what the entry displays, so nothing
  service-private leaves the engine.

## Work log

- 2026-09-30: Reproduced with the probe harness (`comp.cjs w/ai/a.tt
  kkValue lib.ts=… shapes.tt=…` with `RES=1`): both entries resolved to
  `Add import from "./shapes.tt"`. tsgo on the `.ts` twin resolved `./lib`
  and `./shapes`; its raw items carry `data.source`.
- 2026-09-30: Added the source to the item, the cache key, the resolve
  argument, the server protocol, and the adapter's resolve data; updated
  the existing resolve call sites (`tests/native/cases_06.rs`,
  `tests/native/cases_08.rs`, `completion.test.ts`) to name their entry.
- 2026-09-30: Re-ran the harness: `kkValueLib ./lib` resolves to
  `import { kkValueLib } from "./lib"`, `kkValueLib ./shapes.tt` to
  `./shapes.tt`.
- 2026-09-30: Tests: `entries_of_one_name_from_two_modules_each_import_their_own`
  (`tests/native/editor_service.rs`) resolves both entries in both orders;
  with the source left out of the cache key it fails (`left:
  "import { kkValue } from \"./shapes.tt\"…"`, `right: "…\"./lib\"…"`).
  The extension test "each auto-import entry of a name exported by two
  modules imports from its own module" (`server.test.ts`) fails when the
  adapter does not carry the source (both entries import `./shapes.tt`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native entries_of_one_name`
- [x] `node --test --test-name-pattern=auto-import server/out/test/server.test.js`
- [x] Full gate (recorded in TASK-633, run once for TASK-629 to TASK-633)

## Result

Changed `src/engine/language.rs`, `src/engine/language/service.rs`,
`src/engine/language/project.rs`, `src/server.rs`,
`tests/native/editor_service.rs`, `tests/native/cases_06.rs`,
`tests/native/cases_08.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/test/server.test.ts`,
`editors/vscode/server/src/test/completion.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. Each auto-import
entry imports from the module it names.
