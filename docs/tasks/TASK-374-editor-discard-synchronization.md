# TASK-374: Refresh native diagnostics across focus transitions

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Establish and resolve the cause of the remaining dependency-discard editor failures reported in PR #122, after its three reviewed regressions were fixed in TASK-373 and merged.

## Scope

- Included: Editor document lifecycle, dependency synchronization, and focused lifecycle regression coverage.
- Excluded: Reimplementing fixes already merged in TASK-373.

## Decisions

Investigate actual buffer and protocol transitions before changing production synchronization or test expectations. Preserve ordinary TypeScript ownership and the method-scoped content-mapper contract.

### Decision 1: Complete the native diagnostic scheduling contract

- **Context**: Document synchronization succeeds, but the active consumer leaves the inter-file background queue during dependency discard.
- **Alternatives considered**: Clearing errors would hide valid diagnostics. Timed refreshes and extra requests in the test would conceal the scheduling gap. Replacing TypeScript synchronization would duplicate the owning implementation.
- **Decision and rationale**: Enable the native language client's existing focus-pull option. Keep its dependency background scheduler and source document notifications intact; validate without edits or requests that would artificially refresh the consumer.

## Work log

- 2026-09-12: Ran doctor, fetched origin, fast-forward checked main at 1d3d659, and created the follow-up branch. Confirmed TASK-373 addressed the three review findings but explicitly left intermittent discard failures unattributed.

- 2026-09-12: Captured document changes, closes, buffer text, dirty state, and visible editors during the unmodified 71-case matrix. Confirmed clean restored providers despite three native consumer failures. Inspected vscode-languageclient 10.1.1 diagnostic scheduling.
- 2026-09-12: Extended the exact-pin native patch with focus pulls, added a native-only diagnostics suite, and strengthened existing discard assertions.

## Issues and resolutions

### Issue 1: Reverting a dependency leaves native consumer diagnostics stale

- **Symptom**: The unmodified paired editor matrix failed `ts -> ts`, `tsx -> ts`, and `tsx -> tsx` dependency-discard cases in `target/editor-tests/run-RYe82T`.
- **Cause**: Tracing confirmed that every provider buffer reverted to its clean disk contents. The language client's diagnostic scheduler removes the newly active consumer from its background queue. The native extension enabled change/save/tab pulls but left focus pulls disabled, so a background pass completing after this transition cannot refresh the active consumer.
- **Resolution**: Enable native `diagnosticPullOptions.onFocus`, pairing active-document pulls with the existing background scheduler. Keep server diagnostics authoritative and unchanged. Add native-only TS/TSX coverage and assert the full matrix's buffer/focus preconditions.

## Verification

- Passed `./scripts/ci`: agents, Rust formatting/clippy/tests/fuzz checks, npm, website, native, and extension. Log: `/tmp/tt374-ci.log`. Extension tests: 173 passed, zero skipped.
- Passed patched native extension build and 16 unit tests: `/tmp/tt374-native-build.log`, `/tmp/tt374-native-tests.log`.
- Passed native-only diagnostics suite: 4/4, `/tmp/tt374-diagnostics.log`.
- Passed ownership editor suite: 6/6 in `target/editor-tests/run-74eg87`.
- Passed the full paired editor matrix twice: 71/71 in `target/editor-tests/run-KjaLRA` and 71/71 in `target/editor-tests/run-BF1QpH`. These runs retained the original diagnostic timeout and included assertions proving actual buffer restoration and consumer activation.
- Verified clean patch application and equality of all patched native source/test files against the extension tested above.
- Passed `git diff --check` and the final task-index check.


## Result

Resolved the remaining attributed editor failure after PR #122 through native focus-pull scheduling. TASK-373's parser phase separation, wildcard recovery, and method-scoped feature ownership remain intact and covered by the passing gates. The new native-only suite and strengthened paired suite retain real unsaved overlays and do not force diagnostic refreshes on behalf of the implementation.

Changed files: the native extension patch and its README, `editors/vscode/scripts/test-editor.mjs`, `editors/vscode/test/{editor,diagnostics}.cjs`, the historical TASK-371 follow-up note, this record, and the task index. No installed global extension was modified.

