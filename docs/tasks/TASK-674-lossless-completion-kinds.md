# TASK-674: Carry each completion entry's LSP kind from TypeScript to the editor unchanged

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-674`

## Purpose

`Color.` offered `Red` and `Green` as properties, and `return { | }` for
an object type offered `verbose` as a property, where TypeScript's own
server says `EnumMember` (20) and `Field` (5). The engine reduced the
service's LSP `CompletionItemKind` to one of twelve element-kind strings
(anything else became `"property"`), and the adapter mapped the strings
back to LSP kinds.

## Scope

- Included: `ttc::engine::CompletionItemKind` and `CompletionItem::kind`
  (`src/engine/language.rs`, `mod.rs`), the service reader
  (`src/engine/language/service.rs`), the server protocol's `kind`, the
  adapter (`engine.ts`, `server.ts`), the editor case
  `completionEntryKinds` with its twin, kinds in the editor-case completion
  lines and completion parity, and the API and protocol baselines.
- Excluded: tt's own items (pattern completions, keyword snippets,
  constructors), whose kinds the adapter chooses.

## Decisions

### Decision 1: The engine carries LSP 3.17's `CompletionItemKind` as a typed enum

- **Context**: tsgo answers `textDocument/completion` with LSP items whose
  `kind` is LSP 3.17 `CompletionItemKind` (1 `Text` to 25
  `TypeParameter`; the pinned tsgo answers `20` for `Color.Red` and `5`
  for a property of an object literal's contextual type, as the twin
  shows), and the editor speaks the same enumeration. Any intermediate
  vocabulary can only lose values.
- **Alternatives considered**:
  - Extend the string table: still two lossy translations, and every kind
    TypeScript adds needs both sides changed.
  - A bare number in the API: correct, but an embedder reads a magic
    number; the other engine enums (`ServiceSeverity`, `ServiceTag`) are
    typed.
- **Decision and rationale**: `CompletionItemKind`, one variant per LSP
  3.17 value, with `from_lsp` and `lsp`; `CompletionItem::kind` is
  `Option<CompletionItemKind>` because LSP makes `kind` optional. The
  server sends the number (`null` when absent) and the adapter passes it
  through. tt's module-path entries (TASK-609) are `File`, as tsgo's are.

### Decision 2: Completion parity compares kinds

- **Context**: TASK-639 compared completion labels only "since kinds are
  numbers in LSP and strings in the engine".
- **Decision and rationale**: Both are LSP kinds now, so the parity view
  is `label (Kind)`; every current twin is still the same, and a lost kind
  is now a parity difference.

## Work log

- 2026-09-30: Added `tests/cases/editor/completionEntryKinds.{tt,ts}` from
  `target/probe7-editor/cases` and generated its baseline with the unfixed
  code (`Green (property, 11)`, `verbose (property, 11)`; the adapter
  `Property` for all three).
- 2026-09-30: Implemented Decision 1 and 2; regenerated the editor, API,
  and protocol baselines and read each diff: every engine completion line
  now names the LSP kind (`var` → `Variable`, `script` → `File`,
  `directory` → `Folder`, the switch-case snippet `property` →
  `Snippet`), the adapter's `Color` members are `EnumMember`, and the
  protocol's `kind` is a number.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/completionEntryKinds.tt`
  (`tests/baselines/reference/editor/completionEntryKinds.baseline`).
- **Observed failure**: Without the fix the baseline had `Green
  (property, 11)`, `Red (property, 11)`, and `verbose (property, 11)` in
  the engine answer and `Property` for all three in `editor completion`;
  the committed baseline has `EnumMember` and `Field`, so the unfixed run
  reports a modified baseline.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases` and `--test
  public_api`, diffs read; `failingParity.txt` unchanged.
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native
  a_module_specifier_completes`.
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/language.rs`, `src/engine/mod.rs`,
`src/engine/language/service.rs`, `src/engine/language/project.rs`,
`src/server.rs`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`, `tests/editor_cases.rs`,
`tests/native/editor_service.rs`, `tests/cases/editor/completionEntryKinds.{tt,ts}`,
`tests/baselines/reference/editor/*.baseline`,
`tests/baselines/reference/api/{ttc.api.txt,server-protocol.txt}`,
`docs/tasks/INDEX.md`, and this record.
