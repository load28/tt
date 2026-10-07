# TASK-779: Share the pattern analysis' declaration entries across the scopes it narrows

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: —

## Purpose

CI's `performance` job fails on this branch against `main` (`1cc08aa9`):
`single_file` 14.4%, `project_first_snapshot` 14.0%, and
`project_recheck_one_file` 11.1% slower, each past its 10% budget.

## Scope

- Included: the cost the pattern analysis' scope filter adds.
- Excluded: any change to what the compiler reports or emits.

## Decisions

### Decision 1: A table narrowed to a scope shares its entries

- **Context**: `callgrind` over `benches/compile.rs` (two iterations,
  release with line tables) counts 2,978M instructions on this branch and
  2,513M on `main`, 18.5% more. The largest difference is the pattern
  analysis: `Table::visible_at` (TASK-768, TASK-773) runs for every match,
  tuple match, pattern site, and coverage check, and cloned every
  declaration entry with all its constructors and payload fields
  (`Vec<MatchConstructor>::clone` +258M, `Vec<PayloadField>::clone` +167M,
  `analysis::patterns::walk_grown` +359M inclusive).
- **Alternatives considered**: (a) A borrowed view type for the narrowed
  table: every function that takes `&Table` would need a second form.
  (b) Entries behind a reference count, so narrowing copies pointers, the
  way TypeScript's checker shares a `Symbol` among the scopes that see it
  instead of copying it into each.
- **Decision and rationale**: (b). `Table::entries` holds
  `Arc<Entry>`; `visible_at` filters the same entries as before and clones
  pointers. Its answers are unchanged, so every baseline is unchanged. The
  count falls to 2,716M (+8.1% over `main`).

## Work log

- 2026-10-07: Read the `performance` job of runs `37583922592` (before
  TASK-776: 13.6%, 12.2%, 9.5%) and `37600030738` (14.4%, 14.0%, 11.1%);
  built `benches/compile.rs` for this branch and for `1cc08aa9` with line
  tables and compared their `callgrind` profiles by function and by
  source file; changed `src/analysis/patterns.rs`.

## Issues and resolutions

### Issue 1: The rest of the difference is spread thin

- **Symptom**: After decision 1, no single function accounts for more than
  about 1% of the remaining 8.1%.
- **Cause**: The other differences come from the work of TASK-766 to
  TASK-777 (the parent chains and shared steps of TASK-776, the
  `VecDeque` rope of TASK-772, the projection span map of TASK-774), each a
  few tens of millions of instructions.
- **Resolution**: Recorded; CI's comparison on the pushed revision decides
  whether the budget holds.

## Regression test (fails before the fix)

Not applicable: the task changes no behaviour, only the time the pattern
analysis takes; CI's `performance` job is the measurement.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`--test-threads=4`): every suite passes, no baseline
  changes

## Result

Complete. Changed `src/analysis/patterns.rs`.
