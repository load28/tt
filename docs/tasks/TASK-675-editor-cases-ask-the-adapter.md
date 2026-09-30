# TASK-675: Baseline what the editor adapter publishes and offers in the editor cases

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-675`

## Purpose

The editor case runner (TASK-639) answered `@diagnostics` with the engine's
`tsDiagnostics`, the language service's first-pass list. The editor never
shows that list: the VS Code adapter merges it with the text-level check,
the typed compiler pass, and the hints, and publishes the result. Completion
baselines printed the engine's element-kind string, not the LSP
`CompletionItemKind` an editor receives, and no tags. A defect in the
merge (TASK-670) or in the kinds and tags the adapter delivers (TASK-672,
TASK-674) was therefore invisible to the baselines.

## Scope

- Included: `tests/editor_cases.rs` (an LSP client for the adapter, the
  `published:` and `editor completion:` sections, diagnostics parity over
  the published list), the regenerated `tests/baselines/reference/editor/`
  baselines and `failingParity.txt`, and `CONTRIBUTING.md` ("Adding an
  editor case").
- Excluded: changing any answer. The merge defect, the lost kinds, and the
  lost tags are fixed by TASK-670, TASK-674, and TASK-672, which this task
  makes observable.

## Decisions

### Decision 1: Ask the adapter itself, over LSP, for what it publishes

- **Context**: The published list is computed in
  `editors/vscode/server/src/server.ts` (`validate`: the restated-code
  filter, `mergeTyped`, the source-order sort) from four engine answers.
  The runner had to reach that rule.
- **Alternatives considered**:
  - Re-implement the merge in the runner: a second copy of the rule, which
    is exactly the drift the baseline should catch.
  - Move the merge into the Rust engine and expose it as a server method:
    the adapter's one-shot fallbacks (`ttc --check`,
    `ttc --check-types --overlay`, used when the engine server cannot be
    reached) would then need a second merge in TypeScript, or would lose
    the typed layer, which changes behaviour.
  - Spawn the adapter (`node server/out/server.js --stdio`) as the editor
    does, open the case's units, and read its
    `textDocument/publishDiagnostics` notifications.
- **Decision and rationale**: The third. The rule stays in one place, the
  adapter, and the runner reaches it through the protocol the editor uses,
  so the baseline is what the editor shows (LSP 3.17,
  `textDocument/publishDiagnostics`: each notification replaces the file's
  list). TASK-670 then gathers the rule into one function of the adapter.
  The runner answers `workspace/configuration` with
  `{ compilerPath: <this build's ttc>, sidecar: "off" }`, so the adapter
  drives the compiler under test and writes nothing into the case.

### Decision 2: The published answer is the settled one

- **Context**: The adapter validates each open buffer after a 300 ms
  debounce and may validate a buffer again when another opens.
- **Decision and rationale**: The runner opens every `.tt`/`.ttx` unit,
  waits until each has a publish, and then until no publish has arrived for
  1.5 seconds, and takes each unit's last list. Two consecutive plain runs
  after regeneration matched the baselines.

### Decision 3: Completions show the adapter's items with LSP kind names and tags

- **Context**: LSP 3.17 defines `CompletionItemKind` (1 `Text` to 25
  `TypeParameter`) and `CompletionItemTag` (1 `Deprecated`).
- **Decision and rationale**: Each `completions` question adds an
  `editor completion:` block: the adapter's items sorted by `sortText` and
  label, each as `label (Kind, sortText) [tags]`. Items TypeScript ranks as
  globals or keywords (the adapter's `2` layer prefix before `15` or
  `z15`, with no `source`) are counted, as the engine block counts them.

### Decision 4: Diagnostics parity compares the published list

- **Decision and rationale**: The TypeScript twin's pull answer
  (`textDocument/diagnostic`) is what a `.ts` file shows, so it is compared
  with what the `.tt` file shows. A code the typed pass sends as `ts2322`
  is compared as `2322`.

### Decision 5: The suite needs the built adapter

- **Decision and rationale**: Without `server/out/server.js` the suite
  skips with the build command, as it skips without TypeScript;
  `TT_REQUIRE_EXTENSION=1` (set by `scripts/ci rust`, which compiles the
  extension before `cargo test`) and `UPDATE_EXPECT=1` fail instead.

## Work log

- 2026-09-30: Read `validate`, `mergeTyped`, `typeDiagnostics`, and
  `typedDiagnosticsFor` in the adapter and the runner's `Lsp` client.
- 2026-09-30: Generalized `Lsp` to spawn either `tsgo --lsp` or the adapter
  with a settings answer, recorded `publishDiagnostics` notifications, and
  added `render_published` and `render_editor_completion`.
- 2026-09-30: `UPDATE_EXPECT=1 cargo test --test editor_cases`, read every
  baseline diff, then two plain runs.

## Issues and resolutions

### Issue 1: The published list is not TypeScript's for plain TypeScript

- **Symptom**: `plainTypeScript` and `plainTsx`, whose units are plain
  TypeScript, now differ from their twins: TypeScript reports TS2322 at
  the declared name (`unused`) with its own message, and the adapter
  publishes the typed pass's `type mismatch: expected ..., found ...` at
  the initializer.
- **Cause**: The typed compiler pass replaces the service's checker
  diagnostics with its structured rendering (`mergeTyped` with
  `replacesTypes`), including for code with no tt syntax.
- **Resolution**: Recorded in `failingParity.txt` as a reviewed difference;
  not changed here.

### Issue 2: Findings the new sections show

- **Symptom**: `unfinishedIfLet` publishes `stray-if-let` though the
  service list is empty; `signatureHelpInTry` publishes
  `result-no-success-value` beside TS1109; `armGuardWithoutBody` publishes
  `match-not-exhaustive` and `missing-arm-body`.
- **Cause**: These are the text-level check's diagnostics, which the old
  section never showed.
- **Resolution**: Recorded; each is the intended text-level answer.

## Regression test (fails before the fix)

Not applicable: this task extends the test runner and fixes no product
defect; TASK-670, TASK-672, and TASK-674 use the new sections for theirs.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases`, then two plain runs
  (57 s and 52 s): no change.
- [x] `cargo fmt --check`; `cargo clippy --test editor_cases -- -D warnings`.
- [x] Baseline changes reviewed and committed with the change. The full
  gate is recorded in TASK-667.

## Result

Changed files: `tests/editor_cases.rs`,
`tests/baselines/reference/editor/*.baseline`,
`tests/baselines/reference/editor/failingParity.txt`, `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and this record.
