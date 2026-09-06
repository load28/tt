# TASK-329: Audit mixed scoped siblings and nested match composition

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-329: complete calls from a final scoped argument`

## Purpose

Extend TASK-324's binding-free sibling coverage to owners containing payload
bindings, declaration-bearing arms, and scoped matches beside other arguments.

## Scope

- Included: A scoped match in the final argument position of a call whose
  earlier arguments are ordinary expressions or other tt values.
- Excluded: Reopening the repaired binding-free sibling defect; the widening
  of captured earlier arguments and of non-final scoped positions, split to
  TASK-333 after reproduction.

## Decisions

### Decision 1: Preserve evaluation order across the entire expression owner

- **Context**: The inline sibling plan requires every participating value to
  be expression-compatible. A scoped sibling (payload bindings, declarations)
  falls outside that proof, so those owners fell back to unannotated join
  slots and lost contextual typing.
- **Alternatives considered**: Delay all arm values until every subject has
  run; independently rewrite siblings; special-case argument positions.
- **Decision and rationale**: Keep each subject and selected arm value at its
  original evaluation point and extend the existing completion proof rather
  than adding a second mechanism. The schedule already captures everything
  that evaluates before the value, so the arm's call can read those captures
  and the authored order is reproduced exactly.

### Decision 2: Complete only from the final argument position

- **Context**: Moving the call into a dispatch runs it after the match's
  subject and arm. Anything the authored call evaluates *after* the value
  would then run too early.
- **Alternatives considered**: Complete from any argument position and
  re-order the remaining arguments (changes observable order); complete only
  from single-argument calls (the previous, narrower proof).
- **Decision and rationale**: `CallCompletionFacts` accepts a call whose
  **final** non-spread argument is exactly the value, with no spread in any
  position. Earlier arguments are already scheduled to evaluate before the
  dispatch, so the arm re-reads them from their capture slots — a sibling tt
  value through its join slot, a proven-inert argument re-emitted in place.
  A match in a non-final position keeps its join slot unchanged.

### Decision 3: Erase the claimed call frame only outside structural emission

- **Context**: A completed call claims its authored frame so the remaining
  statement walk does not emit it twice. With a sibling tt value inside that
  frame, the sibling's own dispatch must still read its authored subject and
  arm source, which sit inside the same frame.
- **Alternatives considered**: Widen the claim to the sibling's spans (loses
  the sibling's source); track claimed frames by byte ranges in the emitter.
- **Decision and rationale**: The replacement records that it is a claim, and
  the claim applies only when no structured value is being emitted. Structural
  emission of any value therefore keeps reading authored source, while the
  plain statement walk still sees the frame as consumed exactly once.

## Work log

- 2026-09-05: Reproduced the sibling matrix against the TASK-328 compiler.
  `pair(made, match …)`, `pair(make(), match …)` and `trio(made, 7, match …)`
  each report TS7006 on the arm callbacks plus TS2345 at the call; the
  reproduction and its output are in the scratch directory recorded below.
- 2026-09-05: Widened the completion proof to the final argument position and
  built the arm's invoke prefix from the schedule's captured inputs. Added the
  claim flag so a sibling inside the claimed frame still emits its own source.
- 2026-09-05: The first draft also completed calls whose *earlier* argument
  was a bare object literal. That literal is captured before the dispatch and
  its capture is unannotated, so `kind: "item"` widens to `string`. Verified
  against the TASK-328 binary that this widening predates this task; it is not
  a regression and is recorded as TASK-333 rather than papered over.
- 2026-09-05: Re-scoped the typing matrix to earlier arguments whose types do
  not depend on the capture (`made`, `make()`, `state ? made : make()`), which
  fail on TASK-328 and pass here. Added runtime coverage for later-subject
  mutation, abrupt completion in a later subject, and receiver order, plus a
  compile test and an emitted snapshot pinning both the completed final
  position and the unchanged non-final position.

## Issues and resolutions

### Issue 1: Mixed scoped siblings lacked validated contextual composition

- **Symptom**: A payload-binding or declaration-bearing match beside another
  argument reported TS7006 on its arm callbacks and TS2345 at the call.
- **Cause**: The owner-level inline proof requires all roots to be
  expression-compatible, and the completion proof accepted only a
  single-argument call, so these owners fell back to unannotated join slots.
- **Resolution**: The final-argument completion (Decision 2) delivers the arm
  value directly into the authored call. Verified for typing, runtime order,
  later-subject mutation, and abrupt completion.

### Issue 2: Captured earlier arguments and non-final positions still widen

- **Symptom**: `pair({kind: "item", run: x => x}, match …)` still reports
  TS7006/TS2345 — for the *earlier* argument's literal, not the match. A match
  in a non-final argument position keeps its own join slot and its arms are
  still not contextually typed.
- **Cause**: Preserving evaluation order requires evaluating everything before
  the value first, and an unannotated `const` capture erases the contextual
  type the authored position would have supplied.
- **Resolution**: Out of this task's scope; recorded with its reproduction as
  [TASK-333](./TASK-333-captured-argument-contextual-types.md). Emitted
  semantics are unchanged and the boundary is pinned by the tests above.

## Verification

- [x] Matrix covers scoped/plain permutations and `.tt`/`.ttx` boundaries: 4
  match families × 3 earlier-argument shapes × 2 hosts, with native oracles,
  all failing on TASK-328 and passing here
- [x] Earlier arm effects precede later subjects, including later-subject
  mutation and abrupt completion
  (`scoped_sibling_completions_preserve_order_and_slots`)
- [x] Generated-name hygiene and strict unused checks (existing suites)
- [x] Reviewed the `contextual-sibling-completion` snapshot; it type-checks
  under the pinned TypeScript and existing fixtures are byte-identical
- [x] Live `.tt`/`.ttx` editor diagnostics, hover, completion, and
  invalid-member ranges for the sibling form
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/program_syntax/protocol.rs`,
`src/codegen/core/planning.rs`,
`src/codegen/core/emitter/{mod,source}.rs`, `tests/compile/cases_02.rs`,
`tests/integration/contextual.rs`,
`tests/fixtures/emit/contextual-sibling-completion/{input.tt,expected.ts}`,
`editors/vscode/server/src/test/engine.test.ts`,
`docs/design/mixed-source-composition-matrix.md`, and the task records.
Follow-up: [TASK-333](./TASK-333-captured-argument-contextual-types.md).
