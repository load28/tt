# TASK-797: Bound the checker requests of contextual storage

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-09
- **Commit**: `TASK-797: Bound the checker requests of contextual storage`

## Purpose

E9 (TASK-795 Decision 11): each edit in a file with 1000 matches cost
seconds in the TypeScript host. The maintainer chose to fix it. This task
corrects the diagnosis TASK-796 Decision 6 recorded and fixes the cause.

## Scope

- Included: the number of checker requests the contextual pass sends per
  slot.
- Excluded: K9 (files outside `include` are projected); it gets its own
  task.

## Decisions

### Decision 1: The cost is the number of checker requests, not projection

- **Context**: TASK-796 Decision 6 said E9, like K9, needs projection on
  demand. Timing the host's contextual pass on the 1000-match file showed
  otherwise: almost all of an ask is synchronous checker requests, about
  43 µs each, and each slot sent dozens of them — many the same question
  about the same node or type.
- **Alternatives considered**:
  - Reusing answers across edits needs to know which slots an edit cannot
    affect; type ids are scoped to one snapshot, so answers cannot be
    carried to the next one without a dependency model.
  - Projection on demand does not change how many requests each slot
    sends.
- **Decision and rationale**: Within one ask, the checker's answers that
  depend only on their arguments are kept (`memoizedChecker` in
  `src/typescript/host.mjs`): a node's symbol and type, keyed by node, and
  `isTypeAssignableTo`, `getWidenedType` and `getBaseTypeOfLiteralType`,
  keyed by type id. The nodes every slot reads are asked for up front in
  one batched request (the API's array overloads of `getSymbolAtLocation`
  and `getTypeAtLocation`, `node_modules/typescript/dist/api/sync/api.d.ts`),
  through the existing `batched` probe. The answers are the checker's own,
  so the emitted output does not change.

### Decision 2: The host reports how many requests the pass sent

- **Context**: A regression test needs a count that does not depend on
  timing.
- **Decision and rationale**: The ask's answer carries
  `contextualRoundTrips`, and `contextual::materialize` adds it to the
  `contextual round trips` work counter that scaling tests read.

## Work log

- 2026-10-08: Instrumented the host's contextual pass (temporary, removed)
  and counted the checker requests per slot.
- 2026-10-08: Changed `src/typescript/host.mjs`, `src/typescript/backend.rs`
  (`Answers::contextual_round_trips`), `src/typescript/native.rs`,
  `src/typescript/contextual.rs`; added a scaling test.
- 2026-10-08: Measured with the 1000-match file (release build, editor
  session): five edits 18.8 s → 5.6 s, first completion 5.6 s → 1.9 s;
  the contextual ask's first round 580 → 345 ms, its second round 2050 →
  726 ms.
- 2026-10-08: Marked the E9 part of TASK-796 Decision 6 as superseded.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs::contextual_storage_costs_a_bounded_number_of_checker_requests_per_slot`
- **Observed failure**: with the memo and up-front batch removed,
  `906 -> 1806` requests for 20 and 40 matches (45 per match; the bound is
  16). With them: `267 -> 527`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change (none:
  the answers are the checker's own, so no output changed)

## Result

The contextual pass keeps the checker's per-node and per-type answers for
one ask and batches the nodes every slot reads, so its requests grow by
about 13 per match instead of 45. E9's edit cost dropped about threefold.
K9 is left for TASK-798.
