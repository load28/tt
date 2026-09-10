# TASK-362: Isolate terminated coverage children

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-362: fix(ci): isolate terminated coverage children`

## Purpose

Keep deliberately terminated integration-test children from contributing
incomplete LLVM profile files to the repository coverage aggregate.

## Scope

- Included: Coverage-profile ownership for long-running `ttc` children that
  tests must terminate, shared test support, and CI regression verification
- Excluded: Product watch semantics, the coverage baseline, and ordinary child
  processes that exit normally and produce complete profiles

## Decisions

### Decision 1: The test that terminates a child owns its disposable profile

- **Context**: LLVM coverage is inherited by subprocesses. A watch process has
  no natural completion in its test, so a forced termination can leave a
  partially written profile in the aggregate directory.
- **Alternatives considered**: Retry the CI job; ignore all malformed profiles;
  remove subprocess coverage globally; or redirect only deliberately terminated
  children to a test-owned disposable directory.
- **Decision and rationale**: Redirect only deliberately terminated children.
  Normal subprocess coverage remains evidence, while a process that cannot
  finalize its profile cannot corrupt or weaken the aggregate.

## Work log

- 2026-09-10: Read PR #118's failed coverage log. All tests passed, then
  `llvm-profdata` rejected one corrupt `tt-*.profraw` file.
- 2026-09-10: Re-ran the identical coverage command locally; it completed at
  88.83% line coverage, confirming no threshold regression.
- 2026-09-10: Traced the profile to integration tests that deliberately kill
  long-running instrumented `ttc` watch processes.
- 2026-09-10: Added one shared workspace operation that redirects coverage only
  for deliberately terminated children and applied it to both watch tests.
- 2026-09-10: Re-ran both watch tests, the complete LLVM coverage merge, and
  the repository Rust gate successfully.

## Issues and resolutions

### Issue 1: A killed watch process can poison the complete coverage aggregate

- **Symptom**: The coverage job intermittently fails after every test passes
  with `invalid instrumentation profile data` and `no profile can be merged`.
- **Cause**: The child inherits the aggregate `LLVM_PROFILE_FILE` pattern but is
  forcibly terminated before the profile runtime can finalize its file.
- **Resolution**: Tests now redirect only the profiles of children they must
  forcibly terminate into their disposable workspace. Normally exiting child
  processes retain the aggregate profile path and remain coverage evidence.

## Verification

- [x] Terminated-child profile ownership: both watch tests passed
- [x] `cargo llvm-cov --workspace --summary-only --fail-under-lines 86.9`:
  88.82% line coverage
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci rust`

## Result

Forced termination can no longer introduce an invalid profile into the shared
coverage aggregate. The isolation is scoped to the two non-terminating watch
children; every normally completed subprocess remains measured.
