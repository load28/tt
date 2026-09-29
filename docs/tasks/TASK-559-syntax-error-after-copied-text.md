# TASK-559: Report a syntax error after an arm body where TypeScript puts it

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-559: Report a syntax error after an arm body where TypeScript puts it`

## Purpose

A syntax error at the end of a match arm's body (`Circle(radius) =>
radius.,`, `radius *,`) was reported at the `match (...)` head as
`1003 Identifier expected. (in code ttc generated for this construct)`.
TypeScript reports it at the `,` after the body, and the error is about the
user's text.

## Scope

- Included: How a service diagnostic span in glue is mapped back
  (`mapper::diagnostic_origin`), which the editor and the typed report
  share.
- Excluded: How the arm is emitted.

## Decisions

### Decision 1: A span that starts where copied text ends, in glue no construct opens there, is the position after that text

- **Context**: TypeScript reports a missing token at the token it found in
  its place, with that token's width. In the served projection the body
  `radius.` is copied and followed by the arm's glue `;`, so the range was
  `[end of radius., +1)` over the `;`. It is inside the match's anchor, so
  the diagnostic went to the anchor.
- **Alternatives considered**: (a) Decide by the diagnostic code whether it
  is a syntax error: the error-code heuristic the error-layer contract
  forbids (TASK-527). (b) Emit the arm's terminator with a space or
  parenthesis before it: TypeScript still reports at the next glue token.
  (c) Map every span starting at a chunk's end to that end: a type error on
  a construct's generated node that starts right after copied text (the
  `$tt_ap(` of `const y = x |> f`) would lose its anchor and its
  translated message.
- **Decision and rationale**: As the cursor model reads a position between
  a copied chunk and glue on the chunk's side when the question is about
  what precedes it (TASK-538, `Affinity::Preceding`), a span that starts
  exactly where a copied chunk ends, covers no copied byte, and is not the
  start of any anchor's glue is that position: `Exact { start, end }` at
  the chunk's source end. The editor then gives an empty span the
  character it points at, which is the `,` TypeScript names in the `.ts`
  twin. A span that starts a construct's glue keeps its anchor.

## Work log

- 2026-09-29: Reproduced through `ttc --server` for `radius.,` and
  `radius *,` in `.tt` and `.ttx`, and with the body as the last arm. The
  service's range was one byte over the glue `;` following the copied
  body.
- 2026-09-29: Added the rule to `diagnostic_origin`
  (`src/typescript/mapper.rs`). The answers are now 3:27 `1003` and 3:28
  `1109`, at the `,`.
- 2026-09-29: Tests: `a_token_right_after_copied_text_is_the_position_after_it`
  (`src/typescript/mapper.rs`) and
  `a_syntax_error_in_a_match_arm_is_reported_where_typescript_puts_it`
  (`tests/native/cases_10.rs`), which fails without the rule. The full
  Rust suite, with its anchor and translation tests, passes unchanged.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed.

## Result

A syntax error at the end of an arm body is reported at the token after
it, in TypeScript's words. Changed `src/typescript/mapper.rs` and
`tests/native/cases_10.rs`.
