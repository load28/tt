# TASK-481: Project a jump that leaves a `result` block so only its crossing is reported

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A `break` inside a `result` block that targets a loop outside it reported
`result-break-crossing` plus a redundant `source-not-typescript` at the same
byte. The same cascade followed an unlabeled `continue` and a labeled
`break`/`continue`, and a crossing jump in a `result` block inside a
template interpolation reported only `source-not-typescript`.

## Scope

- Included: The outward-control query (`src/flow/mod.rs`), a HIR and Core
  IR fact for the jumps that leave a Result body (`src/hir/`,
  `src/core_ir/`), the analysis projection's region function
  (`src/program_syntax/projection.rs`), the sema callers
  (`src/sema/checker.rs`), and `docs/ai/tt.md`.
- Excluded: Allowing a jump to leave a `result` block; it remains a tt
  error. Match-arm crossings keep `match-control-crossing`, which already
  blocks the projection.

## Decisions

### Decision 1: The region function re-opens the jump targets the body leaves to

- **Context**: The analysis projection gives a Result region statement
  positions with an immediately called `(() => { ... })()`. A source
  `break`, `continue`, or labeled jump whose target lies outside the block
  has no target inside that function, so SWC rejected it and the
  projection failure became `source-not-typescript`. TASK-450 met the same
  shape for `yield` and fixed it by giving the region function the kind
  the source statement needs (`function*`) instead of hiding the second
  error.
- **Alternatives considered**: (a) Add the crossing codes to
  `blocks_projection`. That drops the cascade but also stops planning the
  rest of the file, so an unrelated `try-placement` elsewhere would vanish
  until the jump is fixed; TASK-450 rejected the same option for `yield`.
  (b) Rewrite the jump in the projection. The jump sits inside copied
  source bytes, so this would need a new Core IR node for ordinary
  TypeScript statements. (c) Carry the outward jumps as a fact and open a
  target for them inside the region function.
- **Decision and rationale**: (c), following TASK-450. HIR lowering asks
  flow for the jumps that leave the statement body
  (`flow::outward_jump_labels`): `None` when there are none, otherwise the
  distinct labels they name. `ResultRegion::outward_jumps` carries it, and
  the projection opens `label: ... for (;;) {` inside the region function
  and closes it after the synthetic `return`. An iteration statement is a
  valid target for unlabeled `break` and `continue`, and each label on it
  is a valid target for `break label` and `continue label`. The projection
  is only parsed and walked; nothing is emitted from it, and the synthetic
  return span is recorded explicitly instead of being derived from the
  closing text's length. Sema still reports each crossing from the same
  flow query.

### Decision 2: The outward-control query lexes the body it is given

- **Context**: Sema passed the whole file's tokens, in which a template
  literal is one token, so a crossing jump in a `result` block inside a
  template interpolation was never found.
- **Decision and rationale**: `outward_controls_in_span` lexes its own span,
  as `program_diverges_in_span` already does for the same reason. The
  crossing is now reported inside templates, for Result bodies and match
  arms alike, and the HIR fact uses the same query.

## Work log

- 2026-09-28: Reproduced unlabeled `break` and `continue`, labeled `break`
  and `continue` to a loop and to a labeled block, a jump from a nested
  labeled loop to an outer label, a `switch` target, generator and async
  owners, a pipeline head, and a template interpolation. Confirmed jumps
  to targets inside the block and in nested functions were already fine.
- 2026-09-28: Changed `OutwardControl` to carry the label name, added
  `outward_jump_labels`, the HIR and Core IR fields, and the projection
  frame; updated the sema callers.
- 2026-09-28: Added a flow unit test, compile tests for every case above
  (each reports only its crossing, located at the jump), a test that an
  unrelated `try-placement` in the same file is still reported, and a test
  that jumps owned inside a block still compile.

## Issues and resolutions

### Issue 1: A crossing inside a template interpolation was not reported

- **Symptom**: `` `${result { ... break; ... }}` `` in a loop reported only
  `source-not-typescript`.
- **Cause**: The outward-control query read the file's token stream, where
  the template is one opaque token.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/flow/mod.rs`, `src/flow/tests.rs`, `src/hir/mod.rs`,
`src/hir/lower.rs`, `src/core_ir/mod.rs`, `src/core_ir/lower.rs`,
`src/program_syntax/projection.rs`, `src/sema/checker.rs`, `docs/ai/tt.md`,
and `tests/compile/cases_11.rs`. A `break`, `continue`, or labeled jump
leaving a `result` block reports only its crossing diagnostic.
