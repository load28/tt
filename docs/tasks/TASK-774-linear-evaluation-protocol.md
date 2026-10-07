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

3. **A projected span is mapped to source once, by the segment index
   itself.** Every tt value nested in an expression maps the spans of the
   frames it shares with its siblings and ancestors (the step parents and
   their inputs), so `source_span_for_projection` ran once per step and
   input: 644,796 lookups for 800 matches joined by `+`, 18% of the
   compile. The mapping is a pure function of the segments, so
   `ProjectionSegments` memoizes it per projected span. The collector's
   own per-owner cache (`owner_sources`) was the same memo in one caller;
   it is removed in favour of the shared one.
4. **A capture's dependencies are a relation, not a list.** Evaluation
   planning stored, for each source capture, every other capture of the
   same owner inside it with an earlier slot, and the order validation
   only tested membership in that list. In an expression whose captures
   nest (a left-nested `+` chain), that is quadratic in time and memory.
   The plan now keeps each capture's owner group and span and each group's
   span-to-slot map, and `LoweringPlan::capture_depends_on` answers the
   same membership question in constant time.
5. **A value's steps stay one per enclosing operation.** A schedule lists
   each operation between the value and its owner, and the planner and
   emitter index, slice, and compare those lists (`steps()[..index]`,
   suffix equality for shared conditionals, `steps().is_empty()`), so a
   value nested `d` operations deep carries `d` steps. Sharing the outer
   steps between values (a persistent list) would change that contract in
   every stage that reads schedules; it is left as a proposal rather than
   folded into this task. TASK-772 decision 2 already accepted the
   per-value path in the collector for the same reason. With decisions 3
   and 4, 1,600 matches in one `+` chain compile in 1.5 s (from 2.9 s) and
   800 in 0.47 s (from 0.63 s); the remaining time is copying the step
   lists.

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
- 2026-10-07: A second profile of the same input attributed 18% to
  projection lookups (644,796 calls) and 22% to the order validation,
  most of it comparing spans against each capture's dependency list.
  Implemented decisions 3 and 4: the instruction count for 800 matches
  fell from 3.45 G to 1.36 G, and what remains is allocating and copying
  the per-value step lists (decision 5). Added
  `compiling_maps_each_projected_span_once_for_nested_tt_values`, which
  fails without the memo (`projection span lookups: 5651 units for n
  matches but 21301 for 2n`).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`
  (`compiling_does_linear_work_in_the_tt_values_one_expression_lists`)
- **Observed failure**: without decision 1 the test fails on
  `projection span lookups: 5252 → 20502` (200 values against 100).
- **Path**: `src/lib/scaling_tests.rs`
  (`compiling_maps_each_projected_span_once_for_nested_tt_values`)
- **Observed failure**: without decision 3 the test fails on
  `projection span lookups: 5651 units for n matches but 21301 for 2n`.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Every case baseline unchanged

## Result

In progress.
