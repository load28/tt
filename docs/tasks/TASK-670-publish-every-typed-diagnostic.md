# TASK-670: Publish every typed diagnostic that states a rule no other layer states

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-670`

## Purpose

`return total();`, where `total` needs an argument and returns `number`
in a function returning `string`, has two TypeScript errors at one call:
TS2554 (missing argument) and TS2322 (type mismatch). The editor published
only TS2554. `mergeTyped` in the VS Code adapter dropped every typed
diagnostic whose start already held a problem, even one the same typed
pass had just added.

## Scope

- Included: `editors/vscode/server/src/diagnostics.ts` (the published-list
  rule, moved out of `server.ts`), `server.ts` (`validate` calls it),
  `editors/vscode/server/src/test/diagnostics.test.ts`, and the editor case
  `tests/cases/editor/twoErrorsAtOneCall.tt` with its twin.
- Excluded: where the typed pass places and how it words a TS2322 (the
  parity difference TASK-675 Issue 1 records; TASK-673 is about its
  wording).

## Decisions

### Decision 1: Two layers state the same diagnostic only when rule and start agree

- **Context**: The skip existed for the overlap the typed pass has with the
  other layers (TASK-072, TASK-117): `--check` and the typed pass both
  decide variant exhaustiveness from their own facts, and the language
  service and the typed pass both report TypeScript's checker diagnostics.
  In both overlaps the two statements carry the same code: tt's rules have
  one stable wire code in both passes (`DiagnosticCode::as_str`,
  `src/diagnostics.rs`), and a TypeScript rule is `2322` from the service
  and `ts2322` from the typed pass. The "any problem at this start" rule
  went further and made the start of a diagnostic its identity.
- **Alternatives considered**:
  - Keep the position rule but only against the other layers, not the
    typed pass's own entries: still drops TS2322 when the text layer or the
    service has a different problem at the start (`missing-arm-body` and a
    TS2304 inside the arm).
  - Compare ranges instead of starts: TS2554 covers `total`, the typed
    TS2322 covers `total()`, but the identity question is still which rule
    is stated, not how wide.
- **Decision and rationale**: `sameDiagnostic`: the same rule (code, with
  the typed pass's `ts` prefix removed before digits) at the same start.
  A typed diagnostic replaces another layer's statement of the same
  diagnostic, and is otherwise added; the typed pass's own diagnostics are
  never compared with each other. TypeScript itself reports several rules
  at one node (its `Diagnostic` identity is file, start, length, code, and
  message: `compareDiagnostics` and `sortAndDeduplicateDiagnostics` in
  `src/compiler/utilities.ts` of microsoft/TypeScript), so a code is part
  of the identity there too.
  LSP 3.17 gives the publishing server the whole list
  (`textDocument/publishDiagnostics` replaces the client's list), so the
  merge is the adapter's to get right.

### Decision 2: The rule lives in one function

- **Decision and rationale**: `publishedDiagnostics(layers)` in
  `diagnostics.ts` holds the restated-code filter, the typed merge, and the
  source-order sort that `validate` used to spread over forty lines;
  `validate` gathers the layers and calls it. It is pure, so its unit tests
  run without a compiler, and the editor cases (TASK-675) reach it through
  the running adapter.

## Work log

- 2026-09-30: Added `twoErrorsAtOneCall` from `target/probe7-editor/cases`
  and generated its baseline with the unfixed adapter: `published: 2
  diagnostic(s)`, TS2554 only at each call.
- 2026-09-30: Moved the rule to `diagnostics.ts` with the identity rule;
  wrote `diagnostics.test.ts`; regenerated the editor baselines.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/twoErrorsAtOneCall.tt`
  (`tests/baselines/reference/editor/twoErrorsAtOneCall.baseline`), and
  `editors/vscode/server/src/test/diagnostics.test.ts` ("two rules the
  typed pass reports at one call are both published", "a different rule
  at the same start as another layer's is its own diagnostic").
- **Observed failure**: Before the fix the editor case published
  `published: 2 diagnostic(s)`, the two `ts2554` entries without the
  `ts2322` at `total()`, so the baseline with four entries is a modified
  baseline. With the old position rule restored in `diagnostics.ts`, the
  two unit tests failed (`- 'ttc ts2322'` and `- 'ttc ts2304'` missing
  from the actual list).

## Verification

- [x] `node --test server/out/test/diagnostics.test.js`: 5 passed.
- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases`: only the new case
  and `failingParity.txt` changed.
- [x] Extension suite `node --test server/out/test/*.test.js` with this
  build's `ttc` on `PATH`: 234 passed, 0 failed, 0 skipped.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `editors/vscode/server/src/diagnostics.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/diagnostics.test.ts`,
`tests/cases/editor/twoErrorsAtOneCall.{tt,ts}`,
`tests/baselines/reference/editor/twoErrorsAtOneCall.baseline`,
`tests/baselines/reference/editor/failingParity.txt`,
`docs/tasks/INDEX.md`, and this record.
