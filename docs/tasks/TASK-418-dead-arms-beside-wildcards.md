# TASK-418: Compute dead arms in matches that have a `_` arm

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttHints` (`src/engine/hints.rs`) returned no unreachable-arm hints for any
match that contained a `_` arm. `MatchAnalysis::coverage` is `None` whenever a
wildcard arm exists, and the dead arms were stored inside `Coverage`. As a
result, `(Circle, Nope) => 3` in
`match (s, w) { (Circle, _) => 1, (_, Nope) => 2, (Circle, Nope) => 3, _ => 4 }`
and `Has(s: Circle(radius: q)) => 2` in
`… Has(s: Circle(radius: r)) => 1, Has(s: Circle(radius: q)) => 2, _ => 5` were
not dimmed.

docs/ai/tt.md (match, guards): "A dead arm the duplicate rule misses (nested
pattern or tuple combination already covered) is NOT an error — it compiles,
and the editor dims it (engine `ttHints`)." The rule has no exception for a
`_` arm.

## Scope

- Included: `src/analysis/coverage.rs`, `src/analysis/patterns.rs`,
  `src/analysis/mod.rs`, `src/analysis/tests.rs`, and `src/engine/hints.rs`.
- Excluded: exhaustiveness. With a `_` arm, a match stays unchecked (tt.md:
  "With `_`: unchecked"). `coverage` is still `None` there, in both the
  parse-only path and the typed `checked_coverage` path.

## Decisions

### Decision 1: Move reachability from `Coverage` to `MatchAnalysis`

- **Context**: Reachability asks whether an arm adds anything to the arms
  before it. That question exists for every match whose tags identify a known
  variant. Exhaustiveness asks whether the arms leave anything uncovered, and
  that question does not arise once a `_` arm exists. Storing the first answer
  inside the second hid it whenever the second did not arise.
- **Alternatives considered**: (a) Make `coverage` `Some` for matches with a
  wildcard. That changes what sema, completion (`covered`), and the typed pass
  read as "checked", which the task forbids. (b) Add a second copy of the dead
  arms beside `Coverage::unreachable`. That gives two sources for one fact.
- **Decision and rationale**: The new field `MatchAnalysis::unreachable` is
  computed by the same `unreachable_arms` usefulness pass.
  `Coverage::unreachable` is removed. `match_rows` no longer returns `None` for
  a wildcard. Instead it records `wildcard` and adds each unguarded `_` arm to
  the per-arm rows as a `Wild` row. A bare tuple `_` becomes a row of `Wild`
  cells. The matrix used for missing witnesses (`rows`) still excludes those
  rows, so it still selects the subject variant as before. `coverage` is still
  withheld when `wildcard` is set, and `checked_coverage` skips such matches as
  before. The only consumer of the old field was `hints.rs`.

### Decision 2: Judge the `_` arm itself by the same rule

- **Context**: A `_` arm after arms that already cover every case of the
  resolved variant matches nothing new.
- **Decision and rationale**: The `_` arm is judged by the same usefulness
  test as every other arm. There is no special case in either direction. The
  alphabet is the same one `ttc --check` trusts when it reports a missing case
  as an error. A `_` after partial coverage stays live
  (`match (e) { A => 1, _ => 3 }` has no hint).

## Work log

- 2026-09-27: Confirmed the cause: `match_rows` and `tuple_coverage_of` both
  returned `None` on a wildcard arm. Implemented both decisions.
- 2026-09-27: Added `a_trailing_wildcard_does_not_hide_dead_arms` (the tuple
  and nested repros) and
  `a_wildcard_after_arms_that_cover_every_case_is_dead_and_a_live_one_is_not`
  in `src/engine/hints.rs`. Both fail with the old hint source
  (`coverage.unreachable`) and pass with `analysis.unreachable`. Updated
  `unreachable_arms_are_computed_but_not_an_error` for the moved field.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --lib --test compile --test snapshot --test practical_diagnostics --test native --test cli`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` and `./scripts/ci extension` (the final run at the end of TASK-421)

## Result

Dead arms are hinted whether or not the match has a `_` arm. Exhaustiveness
results are unchanged. The public field `Coverage::unreachable` is now
`MatchAnalysis::unreachable`.
