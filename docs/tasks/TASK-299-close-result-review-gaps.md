# TASK-299: Close Result scope review gaps

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history.

## Purpose

PR #88 exposed a broken append-only mapper code contract, stale post-cutover
Result wording, and missing runtime guards required by the ratified design.

## Scope

- Included: restore stable mapper diagnostic numbers, append every new code,
  update user-facing Result diagnostics and guidance, and add the missing
  disposal and rejected-owner runtime guards.
- Excluded: changing Result syntax, propagation ownership, or lowering rules.

## Decisions

### Decision 1: Preserve retired mapper slots

- **Context**: content-mapper diagnostic numbers are a public append-only wire
  contract.
- **Alternatives considered**: compact retired entries and renumber clients,
  or retain retired names as reserved slots.
- **Decision and rationale**: retain every existing slot and append new codes,
  preserving all published numbers while assigning nonzero codes to new rules.

## Work log

- 2026-08-30: Confirmed all three PR #88 review categories against the current
  branch and opened this task.
- 2026-08-30: Restored the three retired mapper slots, appended nine active
  diagnostics, and added an all-active-codes coverage assertion.
- 2026-08-30: Replaced pre-cutover Result explanations in diagnostics, sema,
  and the language guide.
- 2026-08-30: Added runtime disposal, constructor identity, generator
  completion, and `for...of` preservation guards.
- 2026-08-30: Ran targeted mapper and runtime tests, then the full local CI.

## Issues and resolutions

### Issue 1: Runtime disposal test lacked TypeScript library declarations

- **Symptom**: the targeted test reported missing `Disposable`,
  `AsyncDisposable`, `Symbol.dispose`, and `Symbol.asyncDispose` types.
- **Cause**: the shared integration runner targets ES2022, whose default library
  set does not include explicit resource management declarations.
- **Resolution**: added a runner option for feature-specific TypeScript flags
  and enabled `esnext.disposable` only for this runtime test.

### Issue 2: Restored wire number exposed a stale integration expectation

- **Symptom**: full CI emitted the correct `tt27` but
  `tests/content_mapper.rs` still expected the regressed `tt25`.
- **Cause**: the test had been changed alongside the accidental table
  compaction.
- **Resolution**: restored the published `tt27` expectation.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

The content mapper preserves all published diagnostic numbers and assigns
nonzero numbers to every active rule. Result guidance describes the shipped
nearest-scope language, and runtime tests now cover disposal and rejected-owner
protocol safety.
