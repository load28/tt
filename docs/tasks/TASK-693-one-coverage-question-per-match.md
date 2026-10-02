# TASK-693: Decide once which coverage question a match asks

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-693`

## Purpose

Under `--check-types`, a match mixing tag and literal patterns reported a
spurious `match-not-exhaustive` ("missing true") next to
`match-mixed-patterns` (TASK-642 Issue 2). The untyped pass skipped
coverage for such a match; the typed pass did not.

## Scope

- Included: the rule that decides whether and how a single-scrutinee match
  is checked for exhaustiveness (`src/analysis/coverage.rs`), its two
  consumers (the untyped `Coverage` reported by `src/sema`, and the typed
  probes of `src/probe.rs` that `--check-types` asks the checker), the
  sema suppression list it replaces, a case file,
  `docs/design/match-analysis.md`, `docs/ai/tt.md`.
- Excluded: tuple matches (a literal element already contributes no row,
  in both passes: `tuple_rows`); the codegen of a mixed match (TASK-642).

## Decisions

### Decision 1: One function names the question, and both passes ask only it

- **Context**: Three places decided the question separately. Sema pushed a
  mixed match and a match with an `is` arm onto `coverage_suppressed`,
  which only `report_coverage` read. `probe::collect` returned early for a
  `_` or `is` arm and otherwise classified the match by its **last**
  patterned arm, so a mixed match became a literal question (literal last)
  or a tag question (tag last). `coverage_of` checked only for a `_` arm.
  The typed pass therefore asked a question the untyped pass had ruled
  out: TASK-642's case reported `missing true`, and a tag-last mixed match
  over `variant V { A, B }` reported `missing "B"`.
- **Alternatives considered**: (a) Make the typed report read sema's
  suppression list: the typed report runs in the engine over probes built
  from the AST, not from sema's output, so the list would have to cross
  the engine boundary, and the rule would still live in three places.
  (b) Repeat the mixed-pattern test in `probe::collect`: the rule written
  twice is what went wrong.
- **Decision and rationale**: `analysis::coverage_question(expr)` returns
  `Tags`, `Literals`, or `None` (a `_` arm, an `is` arm, tag patterns mixed
  with literal or `is` patterns, or no pattern). `coverage_of` and
  `checked_coverage` compute a `Coverage` only for `Tags`, and
  `probe::collect` emits the literal or tag probe the question names.
  Sema's `coverage_suppressed` list is gone: a match the checker rejects
  as mixed asks no question in either pass, and the resolution-error
  boundary (`match_has_resolution_error`) is unchanged. This follows
  `docs/design/match-analysis.md` §5 ("one rule, one implementation"),
  which the analysis module already owns for the coverage computation.

## Work log

- 2026-09-30: Read TASK-642 and its case
  `tests/cases/compiler/mixedPatternsInEveryPosition.tt`, whose
  `--check-types` section pinned the spurious error.
- 2026-09-30: Traced the three decisions (Decision 1) and added
  `CoverageQuestion`/`coverage_question`; removed `MatchRows::wildcard`,
  which only encoded part of the question, and `Checker::coverage_suppressed`.
- 2026-09-30: Added `tests/cases/compiler/mixedPatternsAskNoCoverage.tt`
  (tag last, literal last, `is` last); `UPDATE_EXPECT=1 TT_CASES=ixedPattern
  cargo test --test case_baselines`; `cargo test --lib`; `cargo test --test
  compile --test snapshot --test case_baselines --test native`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/mixedPatternsAskNoCoverage.tt` and
  `tests/cases/compiler/mixedPatternsInEveryPosition.tt`
  (`cargo test --test case_baselines`).
- **Observed failure**: without the changes under `src/`, both
  `.errors.txt` baselines are out of date: the `--check-types` section
  gains `error[match-not-exhaustive]: match is not exhaustive: missing "B"`
  at `mixedPatternsAskNoCoverage.tt:8:24` and `match on literal union is
  not exhaustive: missing true` at `:9:28` and at
  `mixedPatternsInEveryPosition.tt:4:9`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (targeted: `--lib`, `compile`, `snapshot`,
  `case_baselines`, `native`); the full gate is recorded in TASK-692.
- [x] Baseline changes reviewed and committed with the change: the
  `mixedPatternsInEveryPosition.errors.txt` `--check-types` section loses
  only the `match-not-exhaustive` error.

## Result

Changed `src/analysis/coverage.rs`, `src/analysis/mod.rs`, `src/probe.rs`,
`src/sema.rs`, `src/sema/checker.rs`, `src/sema/coverage.rs`,
`docs/design/match-analysis.md`, `docs/ai/tt.md`,
`docs/tasks/TASK-642-mixed-pattern-match-dispatch.md` (Issue 2 points
here), `docs/tasks/INDEX.md`; added
`tests/cases/compiler/mixedPatternsAskNoCoverage.tt` and its baseline;
updated `mixedPatternsInEveryPosition.errors.txt`.
