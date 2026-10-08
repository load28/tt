# TASK-789: Recover the first-snapshot time this branch lost to main

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-789: Recover the first-snapshot time this branch lost to main`

## Purpose

The `performance` check on PR #142 failed at `6462aac7`:
`project_first_snapshot` took 38.24 ms against `main`'s 33.74 ms (13.3%
median, 11.8% fastest, over the 10% budget). This task finds where the
branch spends the extra time and removes it without changing any output.

## Scope

- Included: the work `benches/compile.rs` measures (projection, lowering,
  emission) on this branch compared with `main` (`1cc08aa9`).
- Excluded: speeding up what `main` already pays for; behaviour changes.

## Decisions

### Decision 1: Measure instructions, per function, against main

- **Context**: Wall time on a shared runner is noisy; the question is which
  code grew.
- **Decision and rationale**: Both revisions' bench binaries, built with
  line tables, ran one iteration under callgrind. A small parser of the
  callgrind output compared each function's callees between the two. The
  branch ran 2,236.8M instructions against `main`'s 2,042.0M (+9.5%). The
  growth came from a few places, each removed below.

### Decision 2: Build the projection's segment index once

- **Context**: `ProgramSyntax::build_with` built a `ProjectionSegments`
  for the completion, `if`-test and anonymous-function passes, and the
  parent collector built a second one from a copy of the same segments.
- **Decision and rationale**: The collector takes the index the earlier
  passes used, which also shares their memo of mapped spans.

### Decision 3: Insert declarations by moving the shorter side of the rope

- **Context**: `Rope::insert_declarations_at_source` split the piece deque
  at the insertion point and re-appended the whole tail: 2.1M → 25.6M once
  the rope's pieces became a `VecDeque`.
- **Decision and rationale**: `insert_run` moves whichever side of the
  insertion point is shorter, which for a prelude is the few pieces before it.

### Decision 4: Find a span's mapping without collecting candidates

- **Context**: `look_up_source_span` collected the segments starting (or
  ending) at a position and those containing it into vectors, sorted and
  deduplicated them, and took the first that mapped.
- **Decision and rationale**: The first match in segment order is the
  lowest-indexed match, so `first_at_start`/`first_at_end` visit the
  candidates in place (`SpanIndex::each_starting_at`, `each_ending_at`,
  `each_containing`) and keep the lowest index that answers.

### Decision 5: A parent path is its folded facts

- **Context**: TASK-776 made parent paths shared chains so a deep path
  costs one link per edge. Each edge was an `Arc` allocation, and dropping
  the overlays' paths freed them one by one (+18M). Outside tests, nothing
  reads a path's edges: evaluation context reads only the facts folded
  over them and the path's length.
- **Decision and rationale**: `ParentPath` is now `{ len, facts }`, which
  is `Copy`, so extending a path is one fold with no allocation, and depth
  still costs nothing per edge. Test builds also keep the edges in an
  arena (`ParentEdges`), so the tests that read a path's kinds still can.

### Decision 6: Hash projection positions with a position hasher

- **Context**: The mapped-span memo and the parent collector's
  projected-span maps spent more on SipHash and rehashing than on their
  lookups.
- **Alternatives considered**: Dropping the memo would make nested values
  map the same span repeatedly (TASK-774), which the scaling test pins.
- **Decision and rationale**: `position_hash::PositionHasher` is the
  multiply-rotate hash rustc uses for its own integer keys. These keys are
  byte offsets the compiler computed, not adversarial input. The memo is
  sized to the segment count up front.

## Work log

- 2026-10-08: Profiled both revisions; wrote `cgq.py`/`cgdiff.py` (scratch)
  to compare callees.
- 2026-10-08: Decisions 2–6, measuring after each step: 2,236.8M →
  2,199.5M → 2,185.6M → 2,154.8M → 2,121.1M; with Issue 1 fixed,
  2,118.9M (`main` 2,042.0M; now +3.8%).

## Issues and resolutions

### Issue 1: Removing a fast path removed the exact-span answer with it

- **Symptom**: `tests/compile.rs` `every_value_region_crosses_every_host_protocol_class`
  reported `try-placement` for `ready && (try (match (flag) { ... }))`,
  which `89a3f72a` compiles, and the case
  `aTryAroundAPipelineInALogicalOperandLowersOnce` emitted different code.
- **Cause**: A sorted exact-span table tried in front of the memo saved
  nothing (no measured change), and removing it also removed
  `look_up_source_span`'s own first rule: a span some segment was written for
  exactly maps to that segment's whole source, which mapping its two ends
  separately does not reproduce (a placeholder's source is not its
  projection's length). Found by removing the other changes one at a time in
  a worktree.
- **Resolution**: Restored the rule as `ProjectionSegments::exactly`, which
  visits the segments starting at the span's start in place.

## Regression test (fails before the fix)

Not applicable: no output changes; the work is measured by
`benches/compile.rs`, whose comparison against `main` is the CI
`performance` check, and the existing scaling tests keep the projection
linear.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`: 31 of 31 test binaries pass.
- [x] No baseline changed: the emitted code, maps and diagnostics are the
  same as `89a3f72a`'s.

## Result

The bench's instruction count fell from 2,236.8M to 2,118.9M (`main`
2,042.0M). Changed files: `src/position_hash.rs` (new), `src/lib.rs`,
`src/span_index.rs`, `src/codegen/rope/builder.rs`, `src/program_syntax.rs`,
`src/program_syntax/{parents,visit,collector,projection,protocol,tests}.rs`,
`src/program_syntax/projection/segments.rs`. The CI `performance` check on
the pushed commit is the remaining confirmation.
