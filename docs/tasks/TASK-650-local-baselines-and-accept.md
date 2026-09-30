# TASK-650: Write new baselines beside the reference, and accept them with one command

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-650`

## Purpose

A failing baseline comparison here prints a diff and asks for a rerun with
`UPDATE_EXPECT=1`, which overwrites the committed files. There is no place
where a run's new outputs can be read side by side with the committed ones,
no way to accept exactly what one run produced, and a suite's loop stops at
its first mismatch, so one run shows one difference. TypeScript and
typescript-go write every new output to `baselines/local`, mark removed
outputs with `.delete`, and accept with one command. This task adopts that
model and reverses TASK-634's Decision 4.

## Scope

- Included: `tests/common/baseline.rs` (`compare`, `compare_absent`,
  `finish`, `local_path`; `expect` and `expect_absent` on top of them),
  the loops of `tests/case_baselines.rs`, `tests/snapshot.rs`, and
  `tests/practical_diagnostics.rs` (every comparison of a run is made
  before the test fails), `scripts/baseline-accept`, `scripts/baseline-diff`,
  `scripts/check-baselines`, `scripts/ci`, `.gitignore`, `CONTRIBUTING.md`
  ("Managing the baselines"), and reversal notes at the top of TASK-634 and
  TASK-635.
- Excluded: the editor case runner, whose single baseline per case already
  fails with everything in it, and the extension suite's own fixture.

## Sources modelled

- microsoft/TypeScript `release-6.0` at
  `050880ce59e30b356b686bd3144efe24f875ebc8`: `src/harness/harnessIO.ts`,
  `Baseline.writeComparison` (lines 1449 to 1497: delete the local file
  first; on a difference write the actual output, or `<file>.delete` when
  the actual output is "no content"; the error names `hereby
  baseline-accept`); `Herebyfile.mjs`, `baselineAcceptTask` (lines 854 to
  876: copy every local file except `*.delete` over the reference, then
  delete each reference a `.delete` names) and the `diff` task (line 847,
  `$DIFF reference local`); `scripts/build/tests.mjs`, `localBaseline =
  "tests/baselines/local/"` and `cleanTestDirs` (lines 22 and 190 to 193:
  the local directory is emptied before a run).
- microsoft/typescript-go (`Herebyfile.mjs` of the snapshot the earlier
  tasks read, `16c25522`): `checkUnusedBaselines` writes a `.delete` marker
  into `testdata/baselines/local` for each unused reference (lines 758 to
  776), and `baselineAcceptTask` also removes each applied `.delete`
  (lines 1015 to 1020). `internal/testutil/baseline/baseline.go`,
  `writeComparison`, is the same shape in Go.
- microsoft/TypeScript `.github/workflows/ci.yml` (`baselines` job): run
  the tests, `hereby baseline-accept`, then diff the tree to produce
  `fix_baselines.patch` (TASK-635 adopted the diff; the accept step is now
  the same).

## Decisions

### Decision 1: One local tree for every baseline root

- **Context**: TypeScript has one reference root. Here the case runners
  write under `tests/baselines/reference/`, and the fixture suites write
  `expected.*` files beside their inputs under `tests/fixtures/`.
- **Alternatives considered**: One local directory per root (several
  places to look and several accept paths); a local copy beside each
  fixture (untracked files scattered through the source tree).
- **Decision and rationale**: `tests/baselines/local/` mirrors
  `tests/baselines/reference/` at the same relative path, as TypeScript's
  does, and holds every other baseline at its repository path
  (`tests/baselines/local/tests/fixtures/emit/<case>/expected.ts`).
  `local_path` refuses a reference directory named `tests`, the one layout
  that would make the two ambiguous. The directory is ignored by git.

### Decision 2: Compare everything, then fail

- **Context**: `expect` panics at the first difference. A suite that loops
  over fixtures, or a case with four artifacts, stops there, so the local
  tree would hold one file and accept would fix one thing per run.
- **Decision and rationale**: `compare` and `compare_absent` return the
  failure instead of panicking; the loops collect them and call `finish`,
  which fails with all of them. `expect` and `expect_absent` remain for
  single comparisons (`tests/public_api.rs`, the editor runner). A matching
  comparison deletes its local file and `.delete`, as `writeComparison` does,
  so a later accept cannot resurrect an old output.

### Decision 3: `UPDATE_EXPECT=1` stays as a shortcut

- **Context**: Every guide and task record since TASK-220 says
  `UPDATE_EXPECT=1 cargo test --test <suite>`.
- **Decision and rationale**: It still writes the committed baselines
  directly (and clears their local copies); it is the one-suite form of the
  run-and-accept cycle. The failure message names both:
  "Run `scripts/baseline-diff` to review this run's new baselines and
  `scripts/baseline-accept` to accept them (or `UPDATE_EXPECT=1 cargo test
  --test <suite>`)."

### Decision 4: The unused check and CI accept through the same command

- **Decision and rationale**: `check-baselines --tracking` writes a
  `.delete` marker for each unused baseline (typescript-go's
  `checkUnusedBaselines`) instead of telling the reader to delete it.
  `--run` empties `tests/baselines/local/` first (`cleanTestDirs`), runs
  the owning suites without `UPDATE_EXPECT` and with `--no-fail-fast`, so
  every suite writes its outputs, and `--accept` then runs
  `scripts/baseline-accept`. `--ci` is `--run --accept` followed by
  TASK-635's staged diff and `fix_baselines.patch`, so CI's patch is
  exactly what `baseline-accept` produces. `./scripts/ci rust` empties the
  local directory before `cargo test`. `scripts/baseline-accept` removes
  the local directory once it has applied it; `scripts/baseline-diff`
  shows each file as a `git diff --no-index` against its reference, or
  hands the two paths to `$DIFF` when it is set.

## Work log

- 2026-09-30: Read `writeComparison`, `baselineAcceptTask`, and
  `cleanTestDirs` at `050880c`, and typescript-go's unused-baseline markers.
- 2026-09-30: Split `compare` from `expect` in `tests/common/baseline.rs`,
  converted the loops, added the two scripts, and changed
  `check-baselines`, `scripts/ci`, `.gitignore`, and `CONTRIBUTING.md`.
- 2026-09-30: Negative checks (below), then restored the tree with
  `scripts/baseline-accept`, which left `git status` clean.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this task changes baseline tooling and fixes no compiler
bug. Its behaviour is shown under Verification.

## Verification

- [x] With `armWithoutBody.map.txt` edited, `armWithoutBody.types` removed,
  and a stale `declaratorListLaterValue.errors.txt` added, filtered
  `case_baselines` runs failed with "2 baseline(s) differ" (modified and
  missing, each naming its local file) and "stale baseline"; the local tree
  held `armWithoutBody.map.txt`, `armWithoutBody.types`, and
  `declaratorListLaterValue.errors.txt.delete`; `scripts/baseline-diff`
  showed the three diffs; `scripts/baseline-accept` reported "2 accepted, 1
  deleted" and `git status` was clean.
- [x] An edited `tests/fixtures/diagnostic/let-else-not-diverging/expected.stderr`
  landed at `tests/baselines/local/tests/fixtures/diagnostic/let-else-not-diverging/expected.stderr`
  and was accepted back.
- [x] `node scripts/check-baselines --ci` with an untracked
  `tests/baselines/reference/orphan.types` and an edited
  `tryAlwaysFailing.map.txt`: the suites ran to the end, the orphan got a
  `.delete` marker, `baseline-accept` deleted it and restored the map, and
  the staged tree equalled the commit (exit 0). The failing-diff path
  (groups and patch) is TASK-635's code, unchanged.
- [x] `bash -n scripts/ci`; `node --check scripts/check-baselines`;
  `cargo clippy --tests` for the four touched suites.
- [x] The full gate ran over the final tree of TASK-647 to TASK-651; its
  results are in TASK-651.

## Result

Changed files: `tests/common/baseline.rs`, `tests/case_baselines.rs`,
`tests/snapshot.rs`, `tests/practical_diagnostics.rs`,
`scripts/baseline-accept`, `scripts/baseline-diff`,
`scripts/check-baselines`, `scripts/ci`, `.gitignore`, `CONTRIBUTING.md`,
`docs/tasks/TASK-634-case-runner-baselines.md`,
`docs/tasks/TASK-635-baseline-tracking-and-ci.md`, `docs/tasks/INDEX.md`,
and this record.
