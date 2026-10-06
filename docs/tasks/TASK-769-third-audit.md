# TASK-769: Fix defects found by the third audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

A third audit of the compiler, the command line and `ttc --server` found
defects in output layout, sidecar preservation, configuration lookup and
request validation. Fix each in the layer that owns it.

## Scope

- Included: the output layout of several inputs (R3L1).
- Excluded: to be recorded as the task proceeds.

## Decisions

### Decision 1: Every input mirrors under the directory all inputs share

- **Context**: `ttc -o out src lib` wrote `out/a.ts` and `out/x.ts`, while
  `a.ts` still imported `../lib/x.js`; `--types -o` laid sidecars out the
  same way. TASK-352 Decision 5 kept each directory input mirroring under
  itself so the collision contracts of TASK-321 and TASK-338 answered as
  before.
- **Alternatives considered**: Keep per-directory roots and rewrite the
  specifiers (the output would no longer mirror the inputs); one root for
  all inputs.
- **Decision and rationale**: `tsc` writes every output relative to one
  common source directory, so an import between two inputs keeps its
  meaning in the output tree. ttc now does the same for builds and
  sidecars (`input_root`). Neither collision contract is weakened: two
  separate roots no longer share an output, and overlapping roots give a
  source one output. Their tests now pin that behaviour; TASK-352 records
  the reversal.

## Work log

- 2026-10-06: Started from the third audit's reports.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
