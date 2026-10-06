# TASK-773: Fix defects found by the fifth audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

The fifth audit of the CLI, the compiler, and the editor engine found new
defects. This task fixes them in the layer that owns each one.

## Scope

- Included: the CLI findings C1–C10, the editor findings E1–E7 and the
  smaller completion gaps, and the compiler findings of the same round.
- Excluded: tsgo's own defects (the content-mapper diagnostic cost, a
  signature-help timeout that a plain `.ts` file shows too).

## Decisions

### Decision 1: Paths are compared by an identity measured once (C2)

- **Context**: `--check`, the build, and `--watch` were quadratic in the
  number of input files: 3,200 empty files took 29.4 s for `--check`, and
  `--watch` over 1,600 took 43.7 s to start and 34.5 s per saved file.
  `strace` counted about 1.3 million `readlink` and `getcwd` calls for 800
  files. `build_jobs` and `compile_jobs` compared every job with every
  earlier one through `same_file`, which canonicalizes both paths per call.
- **Decision and rationale**: `file_identity` computes what `same_file`
  compared — the canonical path of an existing file, the canonical
  directory and name of one that does not exist yet, and the normalized
  path otherwise — once per path, and the job deduplication, the
  compiled-output filter, and the overlapping-root check hash those
  identities. `same_file` is the equality of two identities, so every
  answer it gave is unchanged. After the change 3,200 files check in
  0.24 s, and `--watch` over 1,600 starts in 0.42 s and rebuilds an edit in
  0.43 s.

## Work log

- 2026-10-06: Ran the fifth audit as three read-only agents (CLI,
  compiler, editor) against a release build of the branch.
- 2026-10-06: Reproduced C2, traced it to the pairwise `same_file` loops,
  and replaced them with identity hashing (decision 1).

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
