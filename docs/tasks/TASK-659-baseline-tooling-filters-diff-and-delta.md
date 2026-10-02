# TASK-659: Read every libtest filter form, run `DIFF` from the root, and always clean up `diagnostic-delta`

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-659`

## Purpose

Three defects in the baseline tooling of TASK-635, TASK-650, and TASK-651.
`cargo test --test case_baselines -- --skip=NAME` wrote a tracking header
that called the run whole, so `scripts/check-baselines` reported every
skipped baseline as unused and `scripts/baseline-accept` would delete them.
`scripts/baseline-diff` ran `DIFF` from the caller's directory with paths
relative to the repository root, read a `DIFF` with arguments as one
program name, and printed nothing when the tool could not be started.
`scripts/diagnostic-delta` left its base worktree and binaries behind when
a build failed, and judged "the change touches no baseline" from committed
history while it built the working tree.

## Scope

- Included: `tests/common/baseline.rs` (the filter reading),
  `tests/baseline_tracking.rs`, `scripts/baseline-diff`,
  `scripts/diagnostic-delta`, `npm/scripts/baseline-tools.test.mjs`.
- Excluded: the tracking format and `scripts/check-baselines`, which read
  the header correctly once it is right.

## Sources

- libtest's command line, `library/test/src/cli.rs` (`optgroups`,
  `parse_opts`): it is parsed with getopts; a free argument is a name
  filter; `--skip FILTER` is an `optmulti`; `--ignored`,
  `--exclude-should-panic`, `--bench`, and `--list` select or list tests;
  `--logfile`, `--test-threads`, `--color`, `--format`, `--shuffle-seed`,
  and `-Z` take a value. getopts accepts a long option's value as the next
  argument or after `=` (`--skip=NAME`), a short option's attached
  (`-Zunstable-options`), and every argument after `--` as free.
  The Rust book, "Controlling How Tests Are Run", and `cargo test --help`
  describe the same filter and `--skip` forms.
- git, `git help diff`, "GIT_EXTERNAL_DIFF": git runs the external diff
  through the shell with the paths as positional parameters, so a value
  may carry its own arguments. POSIX `sh`, "Exit Status for Commands":
  127 when a command is not found, 126 when it is found but cannot run.
- Node.js `child_process.spawnSync`: a spawn failure is reported in
  `result.error`, not by an exit status.

## Decisions

### Decision 1: Parse the libtest command line the way getopts does

- **Context**: `filtered()` knew `--skip NAME` but not `--skip=NAME`, and
  skipped the value of `--test-threads=2`-style options by accident.
- **Alternatives considered**: (a) Add `--skip=` as one more string
  match; the next `=` form (`--format=json` followed by a filter) would
  misread again. (b) Model getopts: split `--name=value`, treat the
  value-taking options as consuming the next argument only when no `=`
  value was given, treat everything after `--` as free, and call any free
  argument or selecting flag a filter.
- **Decision and rationale**: (b), as `filtered_by(args, TT_CASES)`, a pure
  function so every form is tested without spawning a test binary.
  `--include-ignored` runs every test and stays unfiltered.

### Decision 2: `DIFF` runs like `GIT_EXTERNAL_DIFF`, from the root

- **Context**: The paths printed and passed are relative to the root, so
  the tool has to run there; a `DIFF` such as `code --diff --wait` needs
  arguments.
- **Alternatives considered**: (a) Pass absolute paths and keep the
  caller's directory; a tool that prints the paths would print long ones,
  and `DIFF` with arguments would still fail. (b) Split `DIFF` on
  whitespace; quoting would differ from every other tool's. (c) Run it
  through `sh -c '<DIFF> "$@"'` with the two paths as parameters, the way
  git runs `GIT_EXTERNAL_DIFF`, from the repository root, and on Windows
  through the shell with both paths quoted.
- **Decision and rationale**: (c). A spawn error, or a shell's 126/127,
  stops the script with `baseline-diff: cannot run DIFF=...` and status 1
  instead of an empty diff and the "N baseline(s) differ" line.

### Decision 3: `diagnostic-delta` cleans up on every exit and counts the tree it builds

- **Context**: `must()` called `process.exit(1)`, which skips the
  `finally` that removes the base worktree and the copied binaries. The
  head side is built from the working tree, but the baseline count read
  `git diff base..HEAD`.
- **Alternatives considered**: (a) Call the cleanup before each exit;
  every future exit path would have to remember it. (b) Throw from
  `must()` and turn a rejected `main()` into status 1 at the top level, so
  the existing `finally` runs. For the count, (a) build HEAD in a second
  worktree, doubling the build; (b) count what is built: `git diff base`
  against the working tree plus untracked files under the baseline paths.
- **Decision and rationale**: (b) for both. The report says when the head
  side includes working-tree changes, and a tree with changes on the base
  revision is no longer reported as "nothing to compare".

## Work log

- 2026-09-30: Reproduced the three defects with the probe sketches in
  `target/probe7-cli/cases/05`–`07`.
- 2026-09-30: Replaced `filtered()` with `filtered_by`; added
  `tests/baseline_tracking.rs`.
- 2026-09-30: Rewrote how `baseline-diff` runs a tool, and how
  `diagnostic-delta` fails and counts baselines; added
  `npm/scripts/baseline-tools.test.mjs` with scratch repositories and a fake
  `cargo`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/baseline_tracking.rs`
  (`every_libtest_filter_form_marks_the_run_filtered`) and
  `npm/scripts/baseline-tools.test.mjs` (four tests).
- **Observed failure**: With the previous reading of the arguments moved
  into `filtered_by` unchanged, `every_libtest_filter_form_marks_the_run_filtered`
  failed at `["--skip=editor"]`. With both scripts reversed, all four node
  tests failed: `diff: tests/baselines/reference/a case.ts: No such file
  or directory` (run from `tests/`); status 0 for
  `DIFF=no-such-diff-tool-xyz`; `.tt-dev/delta-base` still listed by
  `git worktree list` after the failed build; and "12 output(s) changed,
  but the change touches no baseline" with the baseline edited in the
  working tree it built.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`
- [x] Baseline changes reviewed and committed with the change (none)

## Result

The tracking header, `baseline-diff`, and `diagnostic-delta` now agree with
libtest's, git's, and the working tree's view of the run. Changed files:
`tests/common/baseline.rs`, `tests/baseline_tracking.rs`,
`scripts/baseline-diff`, `scripts/diagnostic-delta`,
`npm/scripts/baseline-tools.test.mjs`.
