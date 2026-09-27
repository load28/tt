# TASK-389: Run the CLI typed-check suite against the repository TypeScript

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Every `--check-types` case in `tests/cli.rs` was silently skipped, locally and in CI. The cases ran in the system temporary directory, where ttc cannot resolve TypeScript, and their private guard asked ttc to run in such a directory, so it always answered "no TypeScript". `TTC_REQUIRE_TSGO=1`, which CI sets to turn a missing toolchain into a failure, was never consulted.

## Scope

- Included: The typed helpers and guard in `tests/cli.rs` and `tests/cli/cases_01.rs`.
- Excluded: Test assertions, and the compiler itself.

## Decisions

### Decision 1: Use the shared toolchain answer and in-repository workspaces

- **Context**: ttc resolves TypeScript only from `node_modules` at or above the project (`src/typescript/toolchain.rs`, AGENTS.md). `tests/common/mod.rs` already provides `Workspace::in_repo` for projects that need that resolution and `common::toolchain()`, which honours `TTC_REQUIRE_TSGO`.
- **Alternatives considered**: Symlinking `node_modules` into each temporary project would duplicate the resolution rule inside the tests. Keeping the private probe but moving it into the repository would still ignore `TTC_REQUIRE_TSGO`.
- **Decision and rationale**: The typed helpers create their projects with `Workspace::in_repo`, and the guard uses `common::toolchain()`, the same answer the native and fixture suites use.

## Work log

- 2026-09-27: While adding TASK-388's regression cases, noticed the typed cases finished in 0.01 s. `--nocapture` printed `skipping: no node, or no TypeScript for ttc to drive` with the pinned TypeScript 7.1.0-dev.20260826.1 installed. Replaced the probe, moved the helper workspaces, and re-ran the suite with `TTC_REQUIRE_TSGO=1`.

## Issues and resolutions

### Issue 1: Eighteen typed CLI cases never ran

- **Symptom**: `cargo test --test cli types_` passed in 0.01 s and printed the skip message for every case.
- **Cause**: `have_typescript()` ran ttc in `std::env::temp_dir()`, outside every `node_modules`, and the helpers wrote their projects there as well.
- **Resolution**: The helpers use `Workspace::in_repo`; the macro uses `common::toolchain()`. All eighteen cases now run (about 6 s) and pass.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test cli -- --nocapture`: 85 passed, no skip message.

## Result

Changed `tests/cli.rs` and `tests/cli/cases_01.rs`. The typed CLI cases now run against the repository-pinned TypeScript and fail rather than skip under `TTC_REQUIRE_TSGO=1`.
