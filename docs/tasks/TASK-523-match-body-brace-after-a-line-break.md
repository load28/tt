# TASK-523: Read a line-broken brace after `match (…)` as a block statement

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

`declare function match(x: unknown): void;` followed by `match(x)` and a
block `{ (_: unknown) => 1 }` on the next line is valid TypeScript: a call
statement and a block. ttc reported `malformed-match`. With the block
`{ _ => 1 }` it compiled the text as a tt match and dropped the call. Every
valid TypeScript file must pass through (`docs/ai/tt.md`, contract 1).
`call_named_match_followed_by_a_block` and TASK-069 Issue 3 covered only
`{ 1 }`.

## Scope

- Included: the claim of a match body in the parser (`src/parser/matches.rs`,
  `src/parser/cursor.rs`), for every block content.
- Excluded: editor semantic-token classification of the `match` word
  (`src/engine/tokens.rs`), which colors a word and claims nothing.

## Decisions

### Decision 1: Only a brace on the head's line opens a match body

- **Context**: TASK-069 Issue 3 recorded that `match (x)` followed by `{`
  on the next line is valid TypeScript: after the call `match(x)`, no
  production continues with `{`, so a line terminator before it inserts a
  semicolon (ECMA-262 §12.10.1) and the brace opens a block. TASK-229 made
  the claim depend on whether the braces hold arms, which is the right test
  on one line (`class C { match(x) { y => y } }` is Decision 4's accepted
  ambiguity), but a line-broken block holding an arrow expression statement
  reads as arms and was claimed — or, when its first "pattern" was not one,
  reported as a malformed match. The lexer's token facts (TASK-491) already
  read the head this way: an expression's `{` after a line break is not a
  match body (`src/lexer/facts/expressions.rs`), so the parser and the
  lexer disagreed about the same tokens.
- **Alternatives considered**: (a) Claim a line-broken body only when its
  contents are not valid TypeScript statements. That needs a TypeScript
  parse of the block, and whether `{ A => 1, B => 2 }` is a match would
  depend on a trailing comma. (b) Follow TypeScript's reading, as TASK-445
  did for `variant`: a line terminator before the brace ends the head.
- **Decision and rationale**: (b). `opens_match_body` is the one test: the
  token is `{` and has no `line_break_before` fact (a line break inside a
  comment counts, as it does for ASI). `match_body_open` (both the
  parenthesized and the identifier-scrutinee near-miss), the complete parse,
  and `skip_match_shape` all use it. No test, fixture, example, or document
  in the repository writes a match body on the line after its scrutinee;
  `docs/ai/tt.md` now states the rule.

## Work log

- 2026-09-29: Reproduced both cases and `const v = match (x)` followed by
  `{ _ => 1 };` (also valid TypeScript, also claimed).
- 2026-09-29: Added `opens_match_body` and used it at the three places that
  find a match body's brace. Added
  `call_named_match_followed_by_a_block_of_any_content`
  (`tests/passthrough.rs`) with arrow, typed-arrow, comma, comment, `const`,
  and `return` forms. Updated `docs/ai/tt.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/parser/matches.rs`, `src/parser/cursor.rs`,
`tests/passthrough.rs`, and `docs/ai/tt.md`. A `match (…)` whose `{` follows
a line break passes through as a call and a block.
