# TASK-493: Document the missing-case enumeration bound as a named analysis limit

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-493`

## Purpose

The architecture audit that followed TASK-487 judged its step budget
structural but undocumented. A bound that no design document names reads as
a workaround, so this task records it where match analysis is designed.

## Scope

- Included: a limits entry in `docs/design/match-analysis.md` §6 naming
  `STEP_BUDGET` and `WITNESS_BUDGET`, what degrades, and what never does.
- Excluded: any change to the bounds or to the analysis.

## Decisions

### Decision 1: Keep the bound and document it instead of removing it

- **Context**: Enumerating every missing combination is exponential in the
  number of tuple positions. TASK-487 measured a 20-wide, 100-arm tuple at
  57 s before the bound and about 1.2 s with it.
- **Alternatives considered**: removing the bound (unbounded time on wide
  tuples); a time-based cutoff (nondeterministic output across machines).
- **Decision and rationale**: keep the deterministic step count. The
  exhaustive-or-not verdict comes from Maranget's usefulness check, which does
  not enumerate, so the bound never changes which programs are accepted; only
  the listed holes degrade to a stated lower bound with the `_` fix. rustc
  applies the same kind of named pattern-complexity limit.

## Work log

- 2026-09-28: Read `src/analysis/usefulness.rs` (`WITNESS_BUDGET = 40`,
  `STEP_BUDGET = 4_096`) and the TASK-487 record; added the limits entry to
  `docs/design/match-analysis.md`. The lower-bound wording is already pinned
  by `a_tuple_too_wide_to_enumerate_states_a_lower_bound_and_offers_only_the_wildcard`
  in `tests/compile/cases_11.rs`.

## Issues and resolutions

None.

## Verification

- [x] `node scripts/check-task-index`
- [x] Documentation-only change; no code or test changed.

## Result

`docs/design/match-analysis.md` names the enumeration bound as an analysis
limit. No behaviour changed.
