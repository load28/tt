# TASK-635: Fail on missing, modified, and unused baselines locally and in CI

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-635`

## Purpose

A baseline nobody compares reads as coverage that does not exist, and a
baseline left out of a commit is only noticed by whoever runs the suite
next. TypeScript's CI regenerates every baseline and fails on any
difference, and typescript-go fails a run that leaves a reference baseline
untouched. This task adds both to tt.

## Scope

- Included: path recording in `tests/common/baseline.rs`,
  `scripts/check-baselines`, the `rust` stage of `scripts/ci`, the `check`
  job of `.github/workflows/ci.yml`, `.gitignore`, and `CONTRIBUTING.md`
  ("Managing the baselines").
- Excluded: baselines owned by the VS Code extension suite
  (`tests/fixtures/practical-diagnostics/immutable-cache/expected/cache.fixed.tt`
  is read by `editors/vscode/server/src/test/server.test.ts`, which does not
  run under `cargo test`), and TypeScript's local/reference directory split
  (TASK-634, Decision 4).

## Sources modelled

- typescript-go `internal/testutil/baseline/baseline.go` (`recordBaseline`
  on every `Run`) and `testmain.go` (`Track`, one tracking file per test
  package under `TSGO_BASELINE_TRACKING_DIR`).
- typescript-go `Herebyfile.mjs`: `baselineTrackingEnabled` is false when
  `--tests` filters the run; `collectUsedBaselines` and
  `checkUnusedBaselines` compare the recorded names with
  `testdata/baselines/reference/**` and fail with "Found N unused baseline
  file(s)", writing `.delete` markers for `baseline-accept`.
- microsoft/TypeScript `release-6.0` `.github/workflows/ci.yml`, job
  `baselines`: remove the references, run the tests, accept, `git add`,
  then `git diff --staged` grouped by `--diff-filter=ACR` ("Missing
  baselines"), `MTUXB` ("Modified baselines"), and `D` ("Unused
  baselines"), writing `fix_baselines.patch` and uploading it as an
  artifact when the step fails.
- `src/harness/harnessIO.ts`, `Baseline.writeComparison`: "The baseline
  file X has changed. (Run "hereby baseline-accept" if the new baseline is
  correct.)".

## Decisions

### Decision 1: Record each comparison, judge after the run

- **Context**: libtest has no hook that runs after every test of a binary,
  and the three baseline suites are separate binaries.
- **Alternatives considered**: (a) A test inside each binary that lists the
  expected files from the inputs, as `no_fixture_file_is_stale_or_missing`
  does in `tests/snapshot.rs`: it cannot know what a runner produced, and a
  case's `.errors.txt` exists only when something fails. (b) TypeScript's
  "remove all references, regenerate, diff" alone: correct in CI, but a
  local check that rewrites the tree. (c) typescript-go's tracking: every
  comparison appends its path to a file in `TT_BASELINE_TRACKING_DIR`, and a
  script compares the union with the files on disk after the run.
- **Decision and rationale**: (c) for the unused check, and (b) in CI on top
  of it for the patch. `expect` records `present <path>`, `expect_absent`
  records `absent <path>` (a baseline that must not exist is still one the
  run decided about), and each binary writes one
  `<binary>-<pid>.txt` whose first line is `binary <name> filtered <bool>`.
  The existing structural snapshot test stays.

### Decision 2: A filtered run is incomplete, not clean

- **Context**: A run of `cargo test --test snapshot foo` compares only some
  baselines; judging it would report every other one as unused.
- **Alternatives considered**: Ignore tracking from filtered runs silently
  (typescript-go disables tracking when `--tests` is given): a check that
  passes without having looked.
- **Decision and rationale**: The binary marks itself filtered when libtest
  received a test name, `--skip`, `--ignored`, or `--list`, or when
  `TT_CASES` narrowed the case runner. `scripts/check-baselines` then fails
  with "ran with a test filter", and it fails with "recorded no baseline"
  when an owning binary did not run or skipped for want of a toolchain.
  Each reference root names its owning binary (`OWNERS`), and a recorded
  baseline outside every root fails, so a new root cannot go unchecked.

### Decision 3: Where CI runs it and what it costs

- **Context**: TypeScript runs `baselines` as its own job; a separate job
  here would rebuild the crate and reinstall TypeScript.
- **Alternatives considered**: A separate job (several minutes of build);
  steps in the `check` job, reusing its build.
- **Decision and rationale**: Steps in `check`. The `Test` step sets
  `TT_BASELINE_TRACKING_DIR`, "Check that every baseline was used" runs
  `--tracking` (under a second), and "Regenerate baselines and compare with
  the commit" runs `--ci` whenever the test step ran, including after a
  failing test, because that is when the patch is most useful. `--ci`
  reruns the three owning binaries with `UPDATE_EXPECT=1` (measured locally:
  about 22 seconds, 18 of them the case runner on four workers), deletes
  unused references, stages the roots, classifies the staged diff with
  TypeScript's three filters, and writes `fix_baselines.patch`, which the
  next step uploads on failure. `scripts/ci rust` runs `cargo test` with
  tracking and the `--tracking` check; it does not regenerate, because a
  local gate must not rewrite the working tree, and the comparison failures
  already name the command that does.

## Work log

- 2026-09-30: Added recording and filter detection to
  `tests/common/baseline.rs`.
- 2026-09-30: Added `scripts/check-baselines` (`--tracking`, `--run`,
  `--run --accept`, `--ci`).
- 2026-09-30: Wired `scripts/ci` (`stage_rust`) and
  `.github/workflows/ci.yml`; ignored `fix_baselines.patch`; documented
  "Managing the baselines" in `CONTRIBUTING.md`.
- 2026-09-30: Negative checks (below), then removed their files.

## Issues and resolutions

None.

## Verification

- [x] `node scripts/check-baselines --run`: "baselines: 95 compared, none
  unused".
- [x] An untracked `tests/baselines/reference/orphan.types` and
  `tests/fixtures/practical-diagnostics/dashboard/expected.old`: `--run`
  fails with "2 unused baseline(s): no test compared them" and lists both.
- [x] `cargo test --test snapshot --test practical_diagnostics --test
  case_baselines every` with tracking: `--tracking` fails with "baseline
  tracking is incomplete", naming the two filtered binaries and the one
  that recorded nothing.
- [x] `--ci` with a new case that has no baselines, a hand-edited
  `.map.txt`, and the two orphans: the orphans are deleted, the edit is
  regenerated away, the new case's three files are listed under "Missing
  baselines", `fix_baselines.patch` holds exactly them, and the exit status
  is 1.
- [x] `bash -n scripts/ci`; the workflow parses as YAML with the steps in
  order.
- [x] `cargo fmt --check`; `cargo clippy --test case_baselines --test
  snapshot --test practical_diagnostics -- -D warnings`.
- [x] The full gate ran once over the final tree of TASK-634 to TASK-636;
  its results are in TASK-636.

## Result

Changed files: `tests/common/baseline.rs`, `scripts/check-baselines`,
`scripts/ci`, `.github/workflows/ci.yml`, `.gitignore`, `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and this record.
