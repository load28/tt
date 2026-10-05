# TASK-762: Audit structural editor recovery for regressions

- **Status**: In progress
- **Started**: 2026-10-05
- **Completed**: —
- **Commit**: —

## Purpose

Review PR #140 (TASK-759 to TASK-761) from several independent perspectives,
fix every confirmed regression at its responsible layer, and repeat the review
until no further defect is found.

## Scope

- Included: parser recovery, editor projection and emission separation,
  diagnostic publication across engine, server and LSP layers, strict-path
  differential testing against the pre-PR compiler.
- Excluded: editor validation performance (TASK-763), language features and
  toolchain changes.

## Decisions

### Decision 1: The service answer states the causes its projection hides

- **Context**: An editor repair replaces malformed host text, so TypeScript
  never reports that cause. `retains` only kept a text-layer diagnostic at the
  same position, and the strict text layer reports one cause per file.
- **Alternatives considered**: Make the server `check` method report editor
  recovery diagnostics (breaks its equality with the one-shot `ttc --check`);
  rely on the typed layer (absent when `tt.typedChecks` is off).
- **Decision and rationale**: The layer whose projection hid the cause states
  it, as the content mapper already does. `retains` entries carry the range
  and message, and the adapter publishes a retained cause that no text
  diagnostic states; the typed layer still replaces it by rule and position.

## Work log

- 2026-10-05: Ran `./scripts/doctor` (TypeScript missing; ran `npm ci`).
  Built the pre-PR compiler (`e114ef3b^1`) in a scratch worktree.
- 2026-10-05: Differential test of `check`, `emitMap` and `semanticTokens`
  over all 10,214 `.tt`/`.ttx` files under `tests/cases`: zero differences
  between the pre-PR and current compilers. The strict path is unchanged.
- 2026-10-05: Diagnostic-publication review found Issue 1.

## Issues and resolutions

### Issue 1: A repaired syntax cause disappears when typed tt checks are off

- **Symptom**: With `tt.typedChecks: false`, two unclosed calls published one
  diagnostic; a missing operand after an earlier raw error published none.
- **Cause**: The editor projection repairs the text, so TypeScript reports
  nothing there. The repaired cause existed only in the projection report,
  which reaches the editor through the typed layer, whose tt diagnostics the
  adapter filters out when typed checks are off.
- **Resolution**: `Project::service_retained_syntax` returns `RetainedSyntax`
  (code, range, message); the server and adapter carry it, and
  `publishedDiagnostics` states a retained cause no text diagnostic states.

## Regression test (fails before the fix)

- **Path**: `editors/vscode/server/src/test/server.test.ts`, "a repaired
  syntax cause is published when typed tt checks are off"; unit test in
  `diagnostics.test.ts`, "a repaired cause no other layer states is published
  from the service answer".
- **Observed failure**: Against the unfixed compiler the LSP test published
  only `3:11`; `assert.ok(starts.has("3:0") && starts.size >= 2)` failed with
  `actual: false`.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change
- [ ] `./scripts/ci`

## Result

Pending.
