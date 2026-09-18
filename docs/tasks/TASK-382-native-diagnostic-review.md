# TASK-382: Review and rebase native diagnostic focus scheduling

- **Status**: Complete
- **Started**: 2026-09-18
- **Completed**: 2026-09-18
- **Commit**: —

## Purpose

Review PR #123 against current main and verify its diagnostic scheduling fix after rebasing.

## Scope

- Included: Native diagnostic focus scheduling, rebase conflict resolution, and regression verification.
- Excluded: Unrelated compiler or editor changes.

## Decisions

### Decision 1: Preserve the native diagnostic scheduling owner

- **Context**: The PR enables focus pulls in the native TypeScript client.
- **Alternatives considered**: Reimplementing dependency refresh in tt would duplicate native scheduling and synchronization.
- **Decision and rationale**: Inspect the native client implementation and validate real editor transitions before deciding whether another production change is necessary.

### Decision 2: Make extension activation own readiness

- **Context**: The preserved patterns suite failed all four initial tt groups with only VS Code word suggestions, while the later ttx groups passed.
- **Alternatives considered**: Delaying or retrying each completion would conceal the missing activation boundary.
- **Decision and rationale**: Return the language-client startup promise from extension activation and await that public lifecycle in the editor test before issuing requests. Keep each completion assertion immediate.

## Work log

- 2026-09-18: Doctor passed. Fast-forwarded local main to 6f33edd and rebased PR #123 onto local main. Preserved both task histories and both the patterns and diagnostics editor suites when resolving conflicts.
- 2026-09-18: Confirmed the native language client removes the newly active document from its dependency background scheduler and supports an immediate onFocus pull at that transition.

## Issues and resolutions

### Issue 1: Concurrent editor suite and task index additions conflict

- **Symptom**: Rebase stopped in the task index and editor test runner.
- **Cause**: Main added patterns coverage and later tasks while the PR added diagnostics coverage and TASK-374.
- **Resolution**: Keep both suites with their expected counts, retain all tasks, and allocate TASK-382 for this review.

### Issue 2: Activation completes before language features are registered

- **Symptom**: Cold-start patterns testing returned word suggestions without variant cases for the first four groups.
- **Cause**: Extension activation discarded the client.start promise, and the test did not await extension activation. Later groups passed after startup completed.
- **Resolution**: Await client startup and mapper registration in activate, and await extension activation in the patterns suite.

## Verification

- Passed `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, all Rust tests, fuzz checks, and the agents/native gates via `./scripts/ci` (`/tmp/tt-pr123-ci.log`).
- Passed npm and website gates via `./scripts/ci npm website` outside the sandbox (`/tmp/tt-pr123-ci-network.log`). The initial sandbox run blocked dependency downloads and the prerender listening socket.
- Passed native extension build and all 16 unit tests.
- Passed patch reverse/check/reapplication and equality with all seven tested native source/test files, using the release builder's whitespace policy.
- Passed native-only diagnostics 4/4 (`run-VY6cmR`) and paired editor matrix 71/71 (`run-1F3xsu`) after rebase.
- Passed cold-start patterns 8/8 (`run-ZzFAnu`) after the activation fix; the original cold-start run failed 4/8 before the fix (`run-gDqZgh`).
- Passed ownership regression suite 6/6 after the activation fix (`/tmp/tt-pr123-ownership-final.log`).
- Passed final `./scripts/ci extension`: 191 tests passed with zero failures or skips (`/tmp/tt-pr123-extension-final.log`).
- Passed `node scripts/check-task-index` and `git diff --check`.

## Result

The native focus-pull change remains in the correct diagnostic owner. Rebase retains main's pattern suite and the PR's diagnostic suite. Fixed the additional activation lifecycle defect without per-request retries or delays.

Changed files in this follow-up: `editors/vscode/client/src/extension.ts`, `editors/vscode/test/patterns.cjs`, `docs/tasks/INDEX.md`, this record, and trailing whitespace in TASK-374. Rebase conflict resolutions also changed `editors/vscode/scripts/test-editor.mjs` and retained TASK-374 in the index.
