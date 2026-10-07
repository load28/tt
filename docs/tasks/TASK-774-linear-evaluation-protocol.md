# TASK-774: Make the evaluation protocol linear in the tt values of an expression

- **Status**: In progress
- **Started**: 2026-10-07
- **Completed**: —
- **Commit**: —

## Purpose

TASK-773 decision 18 moved the fifth audit's K8 here: compiling an
expression that holds many tt values takes time quadratic in their number.
This task finds where the per-value work grows with the number of values and
removes that growth from the structure that causes it.

## Scope

- Included: the projection collector, the evaluation protocol, schedule
  resolution, and the order validation, for many tt values in one
  expression (as siblings, and nested).
- Excluded: any change to what the compiler emits. Every case baseline must
  stay byte for byte as it is.

## Decisions

1. **Each tt value of a frame lists only the inputs after the previous
   value's position.** An array, call, `new`, template, tagged template or
   JSX frame that holds many tt values gave each value a step listing every
   position before it, so the schedules of `n` values held `n²/2` inputs,
   and every later stage (schedule resolution, the order validation)
   walked them again. The earlier value of the same owner under the same
   frame already captures the positions before its own, so a later value's
   step starts at that position. The collector keeps, per owner and frame,
   the position of the latest listed value and passes it to
   `evaluation_protocol` as `earlier`. Alternatives rejected: deduplicating
   inputs after the schedules are built (it still builds the quadratic
   lists) and caching steps by frame (each value's step differs in its
   position and index). Two limits keep the emission the same: a value
   whose call the dispatch may complete keeps the full list, because the
   completion re-reads the whole argument list from its step; and the
   callee or tag is listed only by the first value, which is where it
   evaluates.
2. **Frames are shared, and positions are found by binary search.** The
   collector cloned every enclosing frame (with its position lists) into
   each value's path; frames are now `Rc`-shared. Positions in a frame are
   in source order and disjoint, so the position holding a value is found
   with `partition_point` instead of a scan.

## Work log

- 2026-10-07: Measured a release build with line tables. 1,600 matches in
  one array literal take 0.61 s (800: 0.24 s), in one call's arguments
  0.58 s, in one template 0.68 s, and joined by `+` 3.6 s (800: 0.72 s).
  Callgrind at 800 and 1,600 array elements shows every stage growing about
  4x: the collector's `finish` (244M → 980M instructions), the evaluation
  protocol (209M → 847M), schedule resolution (154M → 625M), and the order
  validation (51M → 202M).
- 2026-10-07: Implemented decisions 1 and 2. A new tick test,
  `compiling_does_linear_work_in_the_tt_values_one_expression_lists`
  (`src/lib/scaling_tests.rs`), compiles 100 and 200 matches in an array,
  a call, `new Array(...)`, an object's array property, a template, and
  object properties. Release timings at 1,600 values fall to 0.14–0.19 s
  (from about 0.6 s). Every case baseline and snapshot stays byte for byte
  the same.
- 2026-10-07: Profiled 800 matches joined by `+` (a deep left-nested
  chain): the collector's visit and `finish` dominate (`finish` about 52%,
  `source_span_for_projection` about 18%). The per-value paths clone every
  ancestor, which TASK-772 decision 2 accepted as depth-quadratic.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`
  (`compiling_does_linear_work_in_the_tt_values_one_expression_lists`)
- **Observed failure**: without decision 1 the test fails on
  `projection span lookups: 5252 → 20502` (200 values against 100).

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Every case baseline unchanged

## Result

In progress.
