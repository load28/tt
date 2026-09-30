# TASK-612: Name the module an auto-import entry imports from

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-612: Name the module an auto-import entry imports from`

## Purpose

An auto-import completion entry in a `.tt` file showed only its name. In a
`.ts` file TypeScript shows the module beside it (`kkValueLib  ./lib`), so
two exports of the same name from different modules can be told apart
before one is accepted.

## Scope

- Included: `CompletionItem::label_detail` and `CompletionItem::description`
  (`src/engine/language.rs`), taken from the service's entry
  (`ts_completions`), the `labelDetails` of the `completion` answer
  (`src/server.rs`), and the adapter's `labelDetails`, sent when the client
  declares `labelDetailsSupport`.
- Excluded: The specifier auto-import writes (unchanged).

## Decisions

### Decision 1: Carry the service's label details as they are

- **Context**: The service already answers `labelDetails` (LSP 3.17
  `CompletionItemLabelDetails`), because the engine's client declares
  `completionItem.labelDetailsSupport`: the raw entry for `kkValue` was
  `"labelDetails": { "description": "./shapes.tt" }`, its `data.source`
  `"./shapes.tt"`, and `data.autoImport.moduleSpecifier` `"./shapes.tt"`.
  `ts_completions` dropped the field.
- **Alternatives considered**: Rebuild the description from
  `data.autoImport.moduleSpecifier` or `data.source`: the same text, taken
  from a field whose shape is the server's private resolve data rather than
  the protocol's display field. Rewrite the specifier to a tt form: the
  service's specifier for a served tt module is already the tt one
  (`./shapes.tt`, the served `.tt.ts` document less its `.ts`), and it is
  the specifier the entry's import edit writes, so the description names
  exactly the import that accepting the entry adds.
- **Decision and rationale**: The engine keeps `labelDetails.detail` and
  `labelDetails.description` of every service entry; the adapter forwards
  them when the editor declared `textDocument.completion.completionItem.
  labelDetailsSupport` (LSP 3.17: a server sends label details to clients
  that support them). The regression test pins that the description and the
  resolved import edit name the same module.

## Work log

- 2026-09-30: Reproduced with a native case: the raw service entries carried
  `labelDetails.description` (`./lib`, `./shapes.tt`); the engine's items had
  none.
- 2026-09-30: Added the fields, the JSON, and the adapter forwarding with
  the capability check. Checked through the adapter with the probe harness
  (`ld.cjs`): `kkValueLib` shows `./lib`, `kkValue` shows `./shapes.tt`.
- 2026-09-30: Test: `an_auto_import_entry_names_the_module_it_imports_from`
  (`tests/native/editor_service.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native an_auto_import_entry`
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/engine/language.rs`, `src/engine/language/service.rs`,
`src/server.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/server.ts`, `editors/vscode/server/src/engine.ts`,
and the task index. Auto-import entries name the module they import from.
