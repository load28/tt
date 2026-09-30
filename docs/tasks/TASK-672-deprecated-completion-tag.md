# TASK-672: Keep TypeScript's deprecated tag on completion entries

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-672`

## Purpose

A completion entry for a declaration marked `@deprecated` (`oldAdd`, the
member `api.prev`) is struck through in a `.ts` file: tsgo answers it
with `tags: [1]`. In a `.tt` file the entry was plain. The engine's
`CompletionItem` had no tags, and its TypeScript client did not say it
renders them.

## Scope

- Included: `ttc::engine::CompletionItemTag` and `CompletionItem::tags`,
  the service reader, the engine's TypeScript client capabilities
  (`src/typescript/service.rs`), the server protocol's `tags`, the
  adapter (`engine.ts`, `server.ts`), the editor case
  `deprecatedCompletionTag` with its twin, tags in the editor-case
  completion lines and parity, the runner's LSP clients' capabilities,
  and the API and protocol baselines.
- Excluded: diagnostics' tags (TASK-515 carries those already).

## Decisions

### Decision 1: Ask for tags, carry them typed, and send them only to a client that renders them

- **Context**: LSP 3.17 defines `CompletionItemTag` (`Deprecated = 1`) and
  `CompletionClientCapabilities.completionItem.tagSupport.valueSet`; a
  server sends only the tags a client lists there. The engine is the
  client of tsgo, and the adapter is the server of the editor.
- **Alternatives considered**: Mark deprecation through the adapter's own
  reading of the entry's documentation (`@deprecated` in the resolved
  detail): only after resolve, and a second opinion on what TypeScript
  already decided.
- **Decision and rationale**: The engine declares
  `tagSupport: { valueSet: [1] }` when it initializes tsgo, reads each
  item's `tags` into `CompletionItem::tags: Vec<CompletionItemTag>`
  (unknown values dropped, as the specification tells a client to handle
  them), and the server sends `tags` as LSP numbers. The adapter records
  the editor's `tagSupport.valueSet` at `initialize` and forwards only
  the tags in it, as it already forwards `labelDetails` only when
  `labelDetailsSupport` is declared.

### Decision 2: The runner's clients declare the capability too

- **Decision and rationale**: The editor cases' tsgo and adapter clients
  declare `tagSupport`, so the twin's answer is TypeScript's full answer;
  completion lines and the parity view print `[deprecated]`.

## Work log

- 2026-09-30: Added `tests/cases/editor/deprecatedCompletionTag.{tt,ts}`
  from `target/probe7-editor/cases` and generated its baseline with the
  unfixed code: `oldAdd (Function, 16)` and `prev (Field, z11)` with no
  tag in both the engine and the adapter blocks. The probe's twin
  narrowed `r.kind` without the variant's value `R` or the binding `v`,
  so parity listed `R` and `v` as tt-only; the twin now declares both.
- 2026-09-30: Implemented Decision 1 and 2; regenerated the editor, API,
  and protocol baselines; only this case and the two API files changed.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/deprecatedCompletionTag.tt`
  (`tests/baselines/reference/editor/deprecatedCompletionTag.baseline`).
- **Observed failure**: Without the fix, `oldAdd (Function, 16) from
  "./util"`, `oldAdd (Function, 216)`, `prev (Field, z11)`, and
  `prev (Field, 2z11)` had no `[deprecated]`; the committed baseline has
  it on all four, so the unfixed run reports a modified baseline.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases --test public_api`,
  diffs read; parity `same` for both markers.
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/language.rs`, `src/engine/mod.rs`,
`src/engine/language/service.rs`, `src/typescript/service.rs`,
`src/server.rs`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`, `tests/editor_cases.rs`,
`tests/cases/editor/deprecatedCompletionTag.{tt,ts}` and its baseline,
`tests/baselines/reference/api/{ttc.api.txt,server-protocol.txt}`,
`docs/tasks/INDEX.md`, and this record.
