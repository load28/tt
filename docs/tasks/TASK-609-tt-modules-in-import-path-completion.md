# TASK-609: Offer `.tt` and `.ttx` modules in import path completion

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-609: Offer .tt and .ttx modules in import path completion`

## Purpose

`import { kk } from "./|"` in `a.tt`, next to `shapes.tt` and `lib.ts`,
offered `jsx` and `lib` and no tt module, although auto-import already
writes `./shapes.tt`. A TypeScript developer gets every sibling module
there.

## Scope

- Included: The module entries the engine adds to a completion answer in a
  relative module specifier (`module_specifier_at`, `tt_module_entries` in
  `src/engine/language/service.rs`, `Project::completion`), the replacement
  range a completion entry carries (`CompletionItem::range`, taken from the
  service's `textEdit` through the emit mapping), the service's file and
  folder kinds, and the adapter's `textEdit`.
- Excluded: Bare package specifiers (a tt module is imported relatively),
  and the specifier auto-import writes (already `./shapes.tt`).

## Decisions

### Decision 1: The engine lists the tt modules of the directory a relative specifier names

- **Context**: TypeScript lists a module specifier's directory from the file
  system and keeps the files with a TypeScript extension
  (`services/stringCompletions.ts`,
  `getCompletionEntriesForDirectoryFragment`). A `.tt` file is a module only
  to the engine: the service sees it as the `.tt.ts` document the engine
  serves, which exists in no directory listing, so TypeScript cannot offer
  it. The raw answer at `"./"` was `lib` (kind 17, `detail: "lib.ts"`).
- **Alternatives considered**: (a) Write the lowered modules to disk so the
  listing finds them: the editor would write generated files into the
  user's tree. (b) Register `.tt` with the service as a content-mapper
  extension in every arrangement: the service reads a mapped file through
  the installed `@openload28/tt-lang` process, which a project without the
  package does not have (`Arrangement`).
- **Decision and rationale**: The engine owns which files are tt modules and
  the specifier that imports them, so it adds them. A position is in a
  module specifier when the lexer's string token there follows `from`,
  `import` (not `import.`), or the `(` of `import(` or `require(`, the
  ECMAScript `FromClause`, side-effect `ImportDeclaration`, and `ImportCall`
  positions. For a relative specifier (`./`, `../`), the directory is the
  importing file's directory joined with the typed text up to its last `/`,
  and every `.tt`/`.ttx` file in it (on disk, or an open buffer not yet
  saved) except the importing file is offered under its file name, the form
  tt's imports and auto-import write (`./shapes.tt`), with TypeScript's
  kind for a file (`script`) and rank (`11`, `SortText.LocationPriority`).
  An entry TypeScript also offered is not repeated.

### Decision 2: A completion entry replaces the range the service names, mapped to the source

- **Context**: TypeScript's path entries replace the fragment after the last
  `/` when it is not an identifier (`getDirectoryFragmentTextSpan`): at
  `"./li."` the entry `lib` came with `textEdit.range` over `li.`. The engine
  dropped `textEdit`, so the editor replaced only its own word and wrote
  `./li.lib`; a tt module's name contains a `.`, so the same happened for
  every one of them.
- **Alternatives considered**: Leave replacement to the editor's word range:
  the word of `./shapes.t` is `t`.
- **Decision and rationale**: An entry's `textEdit` range (`replace` of an
  `InsertReplaceEdit`, else `range`) is mapped to the source with the rule
  auto-import edits already follow (`source_edit`: both ends copied from the
  source, a probe's placeholder removed); an entry whose range does not map
  keeps no range and the editor's word range, as before. A tt module entry
  replaces the fragment after the last `/` through the end of the
  specifier, as TypeScript computes it. The adapter sends the range as the
  item's `textEdit`.

## Work log

- 2026-09-30: Reproduced with a native case: the raw answer at `"./"` was
  only `lib`, and at `"./li."` it carried a `textEdit` the engine dropped.
- 2026-09-30: Added the entries, the range, the file and folder kinds
  (`script`, `directory`, as TypeScript names them), and the adapter's
  `textEdit`.
- 2026-09-30: Checked through the adapter with the probe harness: `"./"`
  offers `jsx`, `lib`, and `shapes.tt`.
- 2026-09-30: Test:
  `a_module_specifier_completes_the_sibling_tt_modules_as_tt_imports_them`
  (`tests/native/editor_service.rs`: `import ... from`, a typed fragment and
  its range, `export * from`, `import(...)`, and no tt module in a bare
  specifier).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native a_module_specifier`
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/engine/language.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`, `src/server.rs`,
`tests/native/editor_service.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/engine.ts`, `docs/design/lsp-architecture.md`,
and the task index. A module specifier completes the sibling `.tt` and
`.ttx` modules as tt imports them.
