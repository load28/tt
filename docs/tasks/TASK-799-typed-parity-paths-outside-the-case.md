# TASK-799: Show typed-parity paths outside the case as the CLI does

- **Status**: Complete
- **Started**: 2026-10-13
- **Completed**: 2026-10-14
- **Commit**: `TASK-799: Show typed-parity paths outside the case as the CLI does`

## Purpose

The scheduled CI run has failed since 2026-10-04, so no Nightly has been
published (`release-publish.yml` promotes only a successful run). One of the
two failing jobs is the typed-parity sweep (`tests/corpus.rs`,
`typescript_cases_type_check_as_typescript_does`): every case with a
diagnostic in a TypeScript library file fails as `transport`.

## Scope

- Included: the typed-parity harness's rendering of a diagnostic path that
  lies outside the case directory.
- Excluded: the CLI's path display (TASK-769 Decision 7 stands), the server
  protocol, and the fuzz crashes of the same scheduled run (TASK-800).

## Decisions

### Decision 1: The harness shows a path outside the case relative to it

- **Context**: TASK-769 Decision 7 made `ttc --check-types` show a file
  outside the working directory relative to it with `..`, as `tsc` does.
  The harness compares the CLI's places with the server's `typedCheck`
  paths, which are absolute; it stripped the case directory and kept any
  other path absolute. A diagnostic in `node_modules/@typescript/.../lib.*.d.ts`
  therefore read `../../../../../../node_modules/...` from the CLI and
  `/home/runner/work/tt/tt/node_modules/...` from the server. The oracle
  audit (`tsc -p` against the oracle's diagnostics) has the same mismatch
  for the same files.
- **Alternatives considered**: Listing the cases in
  `tests/typed-parity-differences.txt` would record a harness artefact as a
  ttc difference. Reverting the CLI to absolute paths would undo a decision
  that matches `tsc`.
- **Decision and rationale**: `shown_from` renders a path relative to the
  case directory, `..` included, the way the CLI and `tsc` do. It replaces
  the prefix strip in both the transport comparison and `file_of` (oracle
  audit and expected/actual lists), so every view of one file names it the
  same way.

## Work log

- 2026-10-13: Read run 38233947903 (job 114759903774): nine `transport`
  differences in the last lines of the log, each only in a library path.
  Confirmed the CLI's format change in `src/main/typed.rs::shown`
  (0776bbb8, TASK-769 Decision 7).
- 2026-10-13: Reproduced locally with `TTC_TYPED_FILTER` on the nine cases;
  locally the oracle audit failed first with the same path mismatch.
- 2026-10-13: Added `shown_from` to `tests/corpus.rs`; the nine cases pass.

## Issues and resolutions

### Issue 1: Every case with a library diagnostic failed as `transport`

- **Symptom**: `ttc --check-types and the server's typedCheck disagree`,
  with identical code, message, and line, and a relative versus absolute
  library path.
- **Cause**: TASK-769 changed the CLI's display of a path outside the
  working directory; the harness still kept such a path absolute.
- **Resolution**: Decision 1.

## Regression test (fails before the fix)

- **Path**: `tests/corpus.rs::typescript_cases_type_check_as_typescript_does`
  with `TTC_TYPED_FILTER=duplicateNumericIndexers`
- **Observed failure**: `the oracle's diagnostics are not the ones \`tsc -p\`
  prints` with `../../../../../../node_modules/@typescript/typescript-linux-x64/lib/lib.es5.d.ts(1508,5)`
  against `/home/user/tt/node_modules/@typescript/typescript-linux-x64/lib/lib.es5.d.ts(1508,5)`;
  in CI, `transport: ttc --check-types and the server's typedCheck disagree`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `TTC_TYPED_CASES=all cargo test --release --test corpus typescript_cases_type_check`:
  all 12831 cases, 8721 compared, 8 differ (8 listed), passed. That run's
  `ttc` was built before TASK-800's compiler changes.
- [x] Baseline changes reviewed and committed with the change (none)

## Result

`tests/corpus.rs` (`shown_from`); the typed-parity sweep passes.
