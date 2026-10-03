# TASK-736: Make path verification portable on macOS

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Correct two test assumptions blocking the macOS gate without changing compiler behavior.

## Scope

- Included: canonical scan expectations and invalid-byte path tests.
- Excluded: changing source discovery or diagnostic behavior.

## Decisions

### Decision 1: Assert path contracts independently of fixture capabilities

- **Context**: Project scans return canonical paths; macOS rejects invalid UTF-8 filenames at fixture creation.
- **Alternatives considered**: Normalize the entire temporary root, ignore fixture errors, or separate the asserted contracts.
- **Decision and rationale**: Keep logical collection checks, compare canonical scan identities, and test Unix invalid-path rejection without filesystem creation. Retain Linux filesystem traversal coverage. User approved the implementation plan.

## Work log

- 2026-10-03: Reproduced both failures before editing the tests. The sandbox retry reported permission denied at invalid-byte fixture creation; the earlier unsandboxed run recorded OS error 92. Both stop before compiler behavior is exercised.
- 2026-10-03: All targeted suites passed under the default temporary root. Logs: `/private/tmp/tt-736-cache-after.log`, `/private/tmp/tt-736-workflow-after.log`. Clippy passed with the current branch after replacing an `err().expect()` assertion.

## Issues and resolutions

- Scan expectation compared `/var` to `/private/var`; use canonical expected identities.
- Invalid filename fixture fails before ttc runs; cover validation independently and retain the directory-entry integration on Linux.

## Regression test (fails before the fix)

Not applicable: test-only portability corrections; no production behavior is changed.
Existing failing tests: `tests/engine_cache.rs::source_walk_skips_excluded_names_before_following_links`
and `tests/workflow_repairs.rs::an_input_path_that_is_not_unicode_is_refused_before_anything_is_written`.
Logs: `/private/tmp/tt-736-scan-before.log`, `/private/tmp/tt-736-name-before.log`.

## Verification

- [x] Targeted tests: engine cache 10/10, ownership 1/1, workflow repairs 19/19
- [x] Formatting and clippy
- [x] Final local gate

## Final verification (2026-10-03)

- `./scripts/ci` passed all six stages: agents, rust, npm, website, native, and extension.
- Default macOS temporary root; `RUST_TEST_THREADS=2 GOMAXPROCS=2`. Disposable test commits used process-local `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false`.
- 419 library tests, 172 native tests, and 238 extension tests passed. Baseline audit: 5,468 compared, none unused; 5,793 unsampled matrix baselines remain outside this standard gate.
- Log: `/private/tmp/tt-737-final-ci-2.log`. Full nightly matrices and complete upstream sweeps remain separate TASK-732 work.

## Result

Both portability corrections pass under the default macOS temporary root. Production behavior is unchanged; all required local gates passed.

## Approved execution steps

1. Reproduce the logical/canonical scan mismatch and compare canonical expected identities without changing source collection.
2. Reproduce the invalid-byte fixture failure; keep Linux directory-entry coverage and add direct Unix validation tests without creating an invalid filename.
3. Run focused tests and include the corrections in the final complete gate under the default temporary root.

## Changed files

- `tests/engine_cache.rs`
- `tests/workflow_repairs.rs`
- `src/main/ownership.rs` (tests only)
- This record, `docs/tasks/INDEX.md`, and the TASK-732 follow-up record.
