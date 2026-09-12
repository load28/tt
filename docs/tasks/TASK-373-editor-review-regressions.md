# TASK-373: Resolve editor audit review regressions

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Address the three reproducible review findings on [PR #122](https://github.com/load28/tt/pull/122): wildcard arm recovery, recursive candidate parsing, and loss of native-only editor actions. This task supersedes TASK-372's blanket ownership decision and corrects its incomplete parser validation.

## Scope

- Included: Parser recognition/recovery contracts, capability ownership, regression tests and direct editor verification.
- Excluded: Unattributed native-only dependency-discard failures already reported during review.

## Decisions

### Decision 1: Materialize arm bodies only after committing to a grammar

- **Context**: Trying tuple and single arm grammars recursively constructed the same nested bodies; recovering inside strict recognition amplified this cost.
- **Alternatives considered**: Failing early alone fixes the reported ordering but still revisits wildcard-first and wildcard-only bodies. Time limits would mask the parser's repeated work.
- **Decision and rationale**: Strict lists fail immediately. Arm recognition retains borrowed token slices and spans; only the selected complete match constructs guard and body ASTs. Recovery explicitly uses a separate list parser. A deterministic body count covers four grammar/order variants at three depths.

### Decision 2: Shared wildcard patterns are neutral recovery evidence

- **Context**: A bare wildcard belongs to both arm grammars, so a nonempty candidate list does not identify a form.
- **Alternatives considered**: Prefer the first grammar or special-case the reported input; either would make recovery depend on incidental ordering.
- **Decision and rationale**: Select recovery using discriminating non-wildcard patterns. Keep genuinely conflicting or unclassified forms uncommitted. Both single and tuple regressions verify recovery spans and source-map round trips.

### Decision 3: Transfer complete editor features individually

- **Context**: tt provides compiler quick fixes but cannot replace the native code-action provider, which also implements Organize Imports and refactorings.
- **Alternatives considered**: Reimplement native actions in tt, or suppress duplicate results after both providers execute. Neither defines the actual capability boundary.
- **Decision and rationale**: Version 2 of the pinned native extension API accepts explicit LSP methods per extension. Claims union across contributor leases; unclaimed methods stay native, and document synchronization never transfers. tt claims its complete overlapping providers and leaves code actions complementary. The patch remains tied to the exact upstream source commit.

## Work log

- 2026-09-12: Read all three review comments on commit `3a585d8` and ran `./scripts/doctor` successfully. Continued on the existing PR branch, preserving `.task-agent-disabled`.
- 2026-09-12: Reproduced wildcard projection failure and 12,285 arm body constructions at nesting depth 12 before changing the parser. Split syntax recognition from recursive AST construction and added wildcard-neutral recovery evidence.
- 2026-09-12: Replaced blanket ownership in the native patch and tt client with method-scoped claims. Added native policy tests and an actual Organize Imports workspace-edit regression for both `.tt` and `.ttx`.
- 2026-09-12: Built both development extensions and the debug compiler. Ran Rust, extension and ownership suites. Applied the tracked patch to a fresh copy of the exact pinned upstream source; all seven resulting files match the tested native build.
- 2026-09-12: Operated a VS Code development host directly. Organize Imports removed an unused import from `.ttx`. With an incomplete `Gue` arm and a wildcard fallback, typing `name.` in the valid sibling displayed String method completions.

## Issues and resolutions

### Issue 1: A wildcard fallback invalidates partial-match recovery

- **Symptom**: A valid sibling loses its projected code and type support while another arm is incomplete.
- **Cause**: Wildcards populate both candidate lists, invalidating the former exclusive-nonempty test.
- **Resolution**: Infer the recovery form from non-wildcard patterns while preserving wildcard arms in that form.

### Issue 2: Nested matches repeatedly construct speculative ASTs

- **Symptom**: The reviewer reported approximately 8.36 seconds for a 606-byte, depth-21 match in a debug build.
- **Cause**: Strict candidate recognition recovered into nested bodies, then another candidate recursively parsed the same bodies again.
- **Resolution**: Borrow syntax ranges while recognizing candidates and materialize each committed arm once. The depth-21 input now has a local five-sample median of 3.538 ms; this is a current-machine measurement, not a controlled reproduction of the reviewer's timing. Exact body counts are the regression contract, not wall-clock thresholds.

### Issue 3: Delegation removes native-only code actions

- **Symptom**: Organize Imports disappears while tt owns editor features.
- **Cause**: The native selector filter transferred every UI feature despite tt's narrower implementation.
- **Resolution**: Scope ownership by extension and LSP method. Keep code actions, formatting, folding and highlights native unless explicitly claimed; retain native synchronization throughout claim/release.

## Verification

- Passed `./scripts/ci rust`: formatting, clippy with warnings denied, Rust tests, doctests and fuzz target checks.
- Passed compiler unit tests: 282 tests, including wildcard recovery and exact body counts for four forms at depths 4, 12 and 24.
- Passed tt extension build and unit tests: 173 tests.
- Passed patched native extension build and unit tests: 16 tests.
- Passed actual paired VS Code ownership suite: 6/6, including `.tt` and `.ttx` Organize Imports edits, single-owner completion/hover/diagnostics and native consumer synchronization through late claim/release. Artifacts: `target/editor-tests/run-7F4qk0`.
- Passed clean-source patch application and comparison against all seven tested native source/test files at TypeScript commit `5739027c9a7df24e27123f453a50c011b37717b6`.
- Passed direct VS Code UI checks described above; UI automation overhead is not a keystroke latency measurement.
- Passed paired editor matrix: 71/71 in `target/editor-tests/run-mVas23`, including all dependency-discard cases in this run. This does not establish the cause or resolution of the earlier intermittent review failures.
- Passed `./scripts/ci npm` with network access after sandboxed scaffold dependency downloads failed. Passed the agents/task-index gate.

## Result

All three review findings are addressed through parser phase separation and per-feature ownership. Changes cover `src/parser/matches.rs`, `src/engine/language/tests.rs`, the tt mapper client, ownership suite and runner, the pinned native patch, editor/patch documentation, TASK-372's supersession note, this record and the task index.
