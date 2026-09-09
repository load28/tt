# TASK-351: Tolerate registry propagation during latest verification

- **Status**: Complete
- **Started**: 2026-09-09
- **Completed**: 2026-09-09
- **Commit**: —

## Purpose

Prevent successful Nightly tag promotions from failing solely because immediate registry reads still return an old tag.

## Decisions

Recheck mismatched tags in bounded rounds after writes, without repeating tag mutations. Use online-preferring npm reads and retain a failing result when the retry budget is exhausted. Keep preflight and publication failures immediate. Inject waiting in tests so propagation cases do not require real delays.

## Work log

- 2026-09-09: Doctor passed; branched from current main. Run 34355686691 completed all tag writes but failed the linux-arm64 postflight; subsequent registry reads showed all eight expected tags. Propagation/cache delay is consistent with this evidence, but the exact registry cache layer is not established.

## Issues and resolutions

The original postflight used one read per package and could fail during propagation. Bounded rechecks preserve eventual success without concealing persistent mismatches.

## Verification

- All 47 npm release-tool tests passed, including delayed convergence, exhausted polling, and immediate preflight/write errors.
- Live read-only prepare passed with online-preferring npm queries for all eight packages; no tags were changed.
- `./scripts/ci agents rust npm` passed, including fmt, clippy, all Rust tests, fuzz compilation and generated-project checks. Log: `/tmp/tt-351-ci.log`.
- `git diff --check` and task-index validation passed.

## Result

Postflight verifies mismatches in six shared rounds with 60 seconds of scheduled waiting plus request time. Persistent mismatches remain failures and no tag writes are retried.

Changed files:

- `npm/scripts/promote-nightly-latest.mjs`
- `npm/scripts/promote-nightly-latest.test.mjs`
- `docs/manual-nightly-latest.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-351-nightly-tag-verification.md`

