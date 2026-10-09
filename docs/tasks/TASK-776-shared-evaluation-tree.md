# TASK-776: Lower an expression's tt values over one shared evaluation tree

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: —

## Purpose

TASK-774 left compiling many tt values in one expression quadratic in their
number: every value carries its own copy of the steps between it and its
owner, so `n` values in one array or one `+` chain cost `n²` steps and
inputs in every stage that reads schedules. The user chose to restructure
the lowering the way TypeScript handles the same problem: the generators
transform spills the operands evaluated before a `yield` in one top-down
pass, guided by a subtree flag (`transformers/generators.ts`,
`visitLeftAssociativeBinaryExpression` and `cacheExpression`, keyed on
`TransformFlags.ContainsYield`), so its work is linear in the expression.

## Scope

- Included: the evaluation protocol (`src/program_syntax/`), the schedules
  and their planning and validation (`src/evaluation_ir/`), and the target
  planning and emitter that read them (`src/codegen/core/`).
- Excluded: any change to what the compiler emits, except the generated
  name numbering decision 5 changes.

## Decisions

### Decision 1: A value's steps are a path in a tree shared by every value of the expression

- **Context**: Every tt value carried a vector of the steps between it and
  its owner, built and resolved for it alone; values under the same
  operations held equal copies.
- **Alternatives considered**: (a) Trimming each value's step to the inputs
  after the previous value (TASK-774 decision 1): it changes the steps two
  values of one conditional operation share and was reverted. (b) A
  persistent list per value, linked from the innermost step outward, with
  one link per (enclosing link, frame, the operand the value sits in): the
  values under one operand of one frame share that link and everything
  outside it, which is TypeScript's own shape (`Node.parent`).
- **Decision and rationale**: (b). `src/chain.rs` holds the list
  (`Chain`, and `ChainSlice` for a prefix or suffix of one) and
  `protocol_step` is split into `step_selection` (which operand of the frame
  holds the value) and the step that operand gives, so the link is built
  once per (outer link, frame, selection) — a loop test's step names the
  value itself and is keyed by it as well. The planned schedule reuses the
  resolution of a shared suffix: resolving it again would read the same
  slots and allocate nothing (a value whose call may complete reserves
  names for its inputs and is resolved alone, as before). Every step a
  value has is the step it had before, so every consumer reads the same
  sequence.

### Decision 2: Facts about a value's steps are summarized on the shared links

- **Context**: With the steps shared, the stages that read a schedule still
  walked every step of every value: the conditional count, the sole
  conditional step, reference loss, capture admission, the outermost step,
  the loop-test count, and the order validation.
- **Alternatives considered**: (a) Caching each answer per value: the walk
  that fills the cache is the quadratic part. (b) A summary stored on each
  link when it is built, combined from the step and the summary of the link
  outside it, the way TypeScript's `TransformFlags` aggregate a subtree's
  facts bottom-up (`computeTransformFlagsForNode`).
- **Decision and rationale**: (b). `src/evaluation_ir/summary.rs` holds
  `StepsSummary` and `InputsSummary`; each reader takes the summary when the
  schedule is a whole chain and walks the steps otherwise, so the answer is
  the same either way. A summary built from a chain that was not summarized
  (a test's edited schedule) is marked unknown and never read. The order
  validation and the reference validation remember the links and input
  segments they already checked.

### Decision 3: A value's parent path is a shared chain with folded facts

- **Context**: Every overlay copied its whole AST parent path, and
  `EvaluationContext::from_path`, `owner_reach`, `frequency_within_owner`,
  `value_role`, and `host_continuation` scanned it: quadratic in depth (the
  part TASK-772 decision 2 left unchanged).
- **Alternatives considered**: (a) Comparing the cached path with the
  current one: finding where they diverge is itself a walk. (b) A parent
  chain like TypeScript's `Node.parent`, each edge carrying the facts of the
  path down to it, folded from its parent's facts the way a parser's
  `contextFlags` are carried down as it descends.
- **Decision and rationale**: (b). `src/program_syntax/parents.rs` holds
  `ParentPath`. The collector marks the path depth at every expression,
  statement, pattern, and JSX child and builds the chain for a mark only
  when a value under it is recorded, so a subtree without values costs
  nothing and every edge is folded once. Each fact the scans computed is a
  fold: the evaluation owner is the last owner edge, the facts local to it
  restart at that edge, and the reach from a host owner compares the last
  loop and unmodeled-conditional edge indices with the owner's edge (an
  optional chain's base waits two edges to learn whether it is modeled).
  The host owners are a shared chain as well, and the facts that read the
  edges around a host owner (an iteration statement, a label, an unbraced
  body) are taken when the owner is entered.

### Decision 4: Span and step queries over an owner's values are indexed

- **Context**: Planning an owner compared every value with every enclosing
  statement-capable value, and the target plan and nested relocations listed
  the inputs of every step of every value.
- **Decision and rationale**: `span_index::innermost_containers` answers
  the shortest strictly enclosing span for all values at once (a sweep with
  a Fenwick tree keyed by span end, ties broken by list order as
  `min_by_key` does). The steps a child keeps inside its outer value are
  counted once per shared link. The relocated spans are a set, so each
  shared link and input segment contributes once (`Chain::fresh`,
  `ChainSlice::fresh`, `Segments::fresh`).

### Decision 5: A completed call reserves names only for the step it re-emits

- **Context**: A value whose call may complete reserved a name for every
  inert input on its whole path and was resolved alone, so values nested as
  `f(match, f(match, …))` allocated names quadratic in the nesting. The
  completion re-emits only the call step's inputs
  (`call_completion_plan`); the other reserved names were never read.
- **Alternatives considered**: (a) Keeping every reservation, so the output
  stays byte for byte: the allocations themselves are quadratic. (b)
  Reserving names for the steps up to and including the call step and
  resolving the rest of the path through the shared links.
- **Decision and rationale**: (b), chosen by the user. The generated names
  after a reservation are numbered lower, so one baseline renames its
  temporaries (`runtimeAPropagatedCallAroundAMatchDoesNotShareTheMatchSSlot`
  `.ts` and `.map.txt`); its runtime output is unchanged. A logical
  operation whose condition sits outside the call step allocates its
  condition name when it is planned, as it already did for every other
  value.

## Work log

- 2026-10-07: Implemented decision 1. Release timings with line tables:
  1,600 matches joined by `+` take 0.92 s (1.5 s before); the flat shapes
  (1,600 matches in one array) stay quadratic, because the step for an
  operand still lists every earlier operand.
- 2026-10-07: Shared each frame's input segments (`Segments`) and the
  sibling chain of an ordered frame, so an operand's step extends the one
  before it.
- 2026-10-07: Implemented decision 2.
- 2026-10-07: Implemented decision 3 and decision 4. Instruction counts
  (callgrind, release with line tables) for 400 and 1,600 matches: a flat
  array 146M and 589M, a `+` chain 158M and 638M, call arguments 147M and
  590M — 4.0 times the work for 4 times the values in each shape (before
  the task: 180M and 1,163M for the array, 312M and 3,156M for the chain).
- 2026-10-07: Added `compiling_does_linear_work_in_the_tt_values_of_one_expression`
  with work counters on protocol links, parent edges, and planned steps.
- 2026-10-07: Implemented decision 5 and added the nested-call shape to the
  scaling test; with every inert input reserved it fails with `planned
  evaluation steps: 5050 units for n matches but 20100 for 2n`.

## Issues and resolutions

### Issue 1: The disk filled during the full suite

- **Symptom**: `No space left on device` while compiling the test binary.
- **Cause**: Stale build directories from earlier tasks.
- **Resolution**: Removed them and ran the suite again.

### Issue 2: Shapes this task does not make linear

- **Symptom**: 200 and 800 matches nested as `f(match, f(match, …))` take
  341M and 4,548M instructions; 200 and 800 matches nested in arms take
  259M and 3,304M.
- **Cause**: A value whose call may complete reserves a name for every
  inert input on its whole path (`resolve_schedule`), so the names, and the
  numbering of every later name, are quadratic in the nesting; keeping the
  output byte for byte requires them. Nested arms indent each level, so the
  output itself is quadratic in size (1.8 MB for depth 200).
- **Resolution**: The user chose to reserve names only for the call step
  (decision 5): 200 and 800 nested calls now take 90M and 357M. The nested
  arms stay proportional to their output, which is the indentation of each
  level.

## Regression test (fails before the fix)

Not applicable: the task fixes no behaviour; it changes how much work the
compiler does. The scaling test
`src/lib/scaling_tests.rs::compiling_does_linear_work_in_the_tt_values_of_one_expression`
pins the linear work; with the collector rebuilding each value's parent path
from the root it fails with `parent path edges: 45216 units for n matches
but 170416 for 2n`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`--test-threads=4`): every suite passes
- [x] Every case baseline, snapshot, and fixture unchanged except the
  renamed temporaries of decision 5

## Result

Complete. Compiling the tt values of one expression is linear in flat
arrays, `+` chains, call arguments, nested calls, and nested statements:
steps and input segments are shared links (`src/chain.rs`,
`src/program_syntax/protocol.rs`), schedule facts are link summaries
(`src/evaluation_ir/summary.rs`), parent paths are folded chains
(`src/program_syntax/parents.rs`), owner span queries are indexed
(`src/span_index.rs`), and a completed call reserves names for its own step
only. Changed files: `src/chain.rs`, `src/span_index.rs`,
`src/program_syntax.rs`, `src/program_syntax/{collector,parents,projection,protocol,visit,tests}.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,planning,summary,validation,tests}.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/planning/rewrites.rs`,
`src/lib/scaling_tests.rs`, and the two renamed baselines of decision 5.
