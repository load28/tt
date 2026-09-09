# TASK-350: Manually promote a published Nightly to latest

- **Status**: Complete
- **Started**: 2026-09-09
- **Completed**: 2026-09-09
- **Commit**: —

## Purpose

Allow temporary, explicitly dispatched promotion of the published main Nightly to npm latest without changing release versions or the automatic Nightly policy.

## Scope

A manual workflow, registry promotion helper, regression tests, and English operating instructions. No actual npm tags are changed during development.

## Decisions

Resolve the candidate automatically from successful scheduled/manual main CI and require its immutable packages to be published under next. Freeze the verified candidate before production approval, then revalidate before mutation. Move all compiler platform, launcher, scaffold and independent unplugin tags together. Serialize with the existing publisher; npm provides no multi-package transaction, so retain previous tags in the summary and support rerunning partial promotions.

## Work log

- 2026-09-09: Doctor passed; created a branch from current origin/main. Inspected the CI metadata, publisher, package matrix and local gates.

## Issues and resolutions

The read-only live preflight exposed that unplugin Nightlies carry their own `-next.<compiler-build>` suffix, rather than a stable independent version. Validation now checks that suffix against the compiler Nightly and a regression rejects mismatched builds.

## Verification

- New promotion regressions passed (7 tests), including all-package preflight, stale next, source mismatch, dependency mismatch, partial failure recovery, no-op repeats and postflight verification.
- Existing npm release tests passed (44 tests).
- Both changed workflows parsed successfully with Ruby Psych.
- A read-only prepare against CI 34325384433 and the live npm registry passed for all eight packages; no npm tag mutation was executed.
- Full `./scripts/ci` passed: agents, Rust (fmt/clippy/tests/fuzz), npm/create-tt, website, native, extension (160 passed, zero skipped). Log: `/tmp/tt-350-ci.log`.
- `git diff --check` and task-index validation passed.

## Result

Implemented the manual promotion workflow and documented the temporary policy. Actual dispatch and registry mutation remain operator actions after merge.

Changed files:

- `.github/workflows/promote-nightly-latest.yml`
- `.github/workflows/release-publish.yml`
- `npm/scripts/promote-nightly-latest.mjs`
- `npm/scripts/promote-nightly-latest.test.mjs`
- `docs/manual-nightly-latest.md`
- `CONTRIBUTING.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-350-manual-nightly-latest.md`

