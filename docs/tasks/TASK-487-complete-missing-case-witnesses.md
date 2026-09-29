# TASK-487: Report every missing case and an exact total

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see the `TASK-487:` commit on this branch

## Purpose

A non-exhaustive match named only some of its holes, so the "add the missing
arms" fix did not make the match exhaustive and the stated total was wrong.
`docs/ai/tt.md` promises that each hole is reported as a pattern that can be
pasted back and that every missing tuple combination is an error.

## Scope

- Included: complete witness enumeration in `src/analysis/usefulness.rs`, an
  exact total (or an honest lower bound) carried on `Coverage`, the shared
  message wording, the arm-insertion edit on both the default and the typed
  pipelines, regression tests, and `docs/ai/tt.md`.
- Excluded: compressing witnesses with `_` beyond what the analysis already
  did (a tuple position no arm tags, an untested payload field), reachability
  (unchanged), literal-match exhaustiveness (the checker names those).

## Decisions

### Decision 1: Split by every constructor when reporting witnesses

- **Context**: `usefulness` handled a column that some constructor is
  missing from by specializing only the default matrix and pairing the
  result with the missing constructors. That is Maranget's shortcut for
  deciding usefulness, and it is complete as a yes/no answer, but as a list
  of witnesses it drops every hole under a constructor that is written yet
  not fully covered: `(Y, Q)` over `{X, Y} × {P, Q}` lost `(Y, P)`, and
  `Ok(value: Some(value: v))` lost `Ok(value: None())`.
- **Alternatives considered**: (a) keep the shortcut and add a second pass
  after applying the fix — the fix would still be wrong and the total still
  wrong; (b) split by every constructor in both reachability and reporting —
  correct but slower for reachability, which only needs one witness;
  (c) split by present constructors only when every witness is wanted, as
  Maranget (JFP 2007, section 5) and rustc's pattern analysis do.
- **Decision and rationale**: (c). The search takes a mode: `All` for
  exhaustiveness and `Any` for reachability. In `All`, an incomplete column
  yields, in declaration order, each missing constructor paired with the
  default matrix's witnesses and each present constructor's specialized
  witnesses. The branches are disjoint and cover every unhandled value, so
  one arm per witness closes the match. `Any` keeps the old shortcut. When
  the default matrix has no witnesses, the column cannot have any either,
  so both modes return early there. A row made only of wildcards ends the
  search at once.

### Decision 2: Count witnesses exactly and cap only the list

- **Context**: the message printed the length of a list capped at 40 as the
  total ("6 combinations in total" when there were 8, "40" for every large
  product).
- **Alternatives considered**: raising the cap (still wrong past it,
  exponential in width); compressing everything to `_` to keep lists short
  (changes the case naming the language guide promises).
- **Decision and rationale**: `usefulness::Missing` carries the first 40
  witnesses together with the exact number of witnesses and how many are
  certain. A product of a missing constructor with the rest multiplies
  counts without building the product. `Coverage` gains `total`,
  `certain_total`, and `exact`, and `non_exhaustive_message` states the
  total it is given rather than a list length.

### Decision 3: Bound the work and say so when it is exceeded

- **Context**: splitting by present constructors is exponential in the worst
  case. A benchmark with random arms took 57 s on a 20-wide, 100-arm match
  (the old code took 0.7 s).
- **Alternatives considered**: no bound (unacceptable); a hard error like
  rustc's pattern complexity limit (drops a useful answer).
- **Decision and rationale**: a step budget of 4,096 recursive calls per
  exhaustiveness question. Past it, the search stops splitting present
  constructors once a witness is already known. The Maranget shortcut still
  decides whether the match is exhaustive, so no hole is lost as a yes/no
  answer, but the count becomes a lower bound: `exact` is false and the
  message reads `… (at least N combinations in total)`. Measured
  afterwards: 1.2 s for the same 20-wide, 100-arm match in release, most of
  it process start-up and the rest of the check.

### Decision 4: Offer the arm edit only when it is complete

- **Context**: an edit that writes 40 of 100 holes does not close the match.
- **Decision and rationale**: both pipelines pass the witness arms to
  `non_exhaustive_suggestions` only when the count is exact and the list
  holds all of them (on the typed path, all certain witnesses). Otherwise
  only the final `_` arm is offered.

## Work log

- 2026-09-28: Reproduced all three symptoms with `ttc --check` on the
  cases from the task: `missing (X, P), (X, Q)`, `(6 combinations in total)`
  for 8 holes, and only `"Err"` for the nested Result/Option match.
- 2026-09-28: Rewrote the witness search in `src/analysis/usefulness.rs`
  (`Missing`, `Mode`, `Search`, `split`, `missing_head`, early return on an
  all-wildcard row and on an empty default matrix).
- 2026-09-28: Added `Coverage::{total, certain_total, exact}` and
  `Coverage::of` in `src/analysis/coverage.rs` and `src/analysis/mod.rs`;
  the single-match candidate choice now compares totals.
- 2026-09-28: `non_exhaustive_message` takes the total and whether it is
  exact (`src/diagnostics/suggestions.rs`); updated
  `src/sema/coverage.rs` and `src/engine/semantics/report.rs`.
- 2026-09-28: Benchmarked old and new release binaries on random wide
  tuple matches; added the step budget after the blow-up described below.
- 2026-09-28: Added regression tests and updated `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: Exponential time on wide random tuple matches

- **Symptom**: `ttc --check` on a 20-wide, 100-arm tuple match took 57 s,
  and 5.7 s at 14 wide with 60 arms.
- **Cause**: splitting by every present constructor in every column
  multiplies the branches, the known worst case of complete witness
  enumeration.
- **Resolution**: early returns on an all-wildcard row and on an empty
  default matrix, plus the step budget from Decision 3. The same inputs now
  finish in about 1.2 s.

### Issue 2: A first timing looked constant across widths

- **Symptom**: every width took about 3.6 s in the first benchmark.
- **Cause**: `ttc --check <file>` also checked the other generated files
  in the same scratch directory, so every run included the slowest one.
- **Resolution**: the benchmark writes and removes each file in its own
  directory.

## Verification

- [x] `cargo fmt --check` (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (exit 0)
- [x] `cargo test --test snapshot`: no fixture changed.
- New tests: `tests/compile/cases_11.rs`
  (`a_tuple_hole_under_a_written_constructor_is_reported_and_fixed_in_one_step`,
  `a_tuple_hole_count_is_every_missing_combination`,
  `a_nested_hole_under_a_written_constructor_is_reported_with_the_missing_case`,
  `a_wide_tuple_counts_its_holes_exactly_without_listing_them_all`,
  `a_tuple_too_wide_to_enumerate_states_a_lower_bound_and_offers_only_the_wildcard`),
  `tests/native.rs`
  (`typed_missing_arms_list_every_hole_under_a_written_constructor`), and
  `src/diagnostics/tests.rs` (`long_lists_truncate_the_same_way_on_both_paths`).
  The paste-back tests apply the edit and require `ttc::analyze` (or the
  typed server) to report nothing.

## Result

Changed files: `src/analysis/usefulness.rs`, `src/analysis/coverage.rs`,
`src/analysis/mod.rs`, `src/diagnostics/suggestions.rs`,
`src/diagnostics/tests.rs`, `src/sema/coverage.rs`,
`src/engine/semantics/report.rs`, `tests/compile/cases_11.rs`,
`tests/native.rs`, `docs/ai/tt.md`, `docs/tasks/INDEX.md`, and this record.
A match now reports every hole with an exact total (or an explicit lower
bound for very wide matches), and the "add the missing arms" edit makes the
match exhaustive in one step on both the default and the typed paths.
