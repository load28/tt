# TASK-461: Offer payload field completions only inside a pattern

> **Superseded in part by TASK-492.** Decision 1's token-stream reading in `src/engine/completions.rs` (`unclaimed_site`, `arm_start`, `pattern_head`, `match_body`) moved into the parser as `parser::pattern_site_at` and `parser::arm_headers`, which use the parser's own match head, arm walk, and pattern heads. A match with an identifier scrutinee now has arm positions too.

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttCompletions` answered payload field names for any `(` that followed an identifier. Constructor calls (`Shape.Circle(¦)`, `Option.Some(¦)`, `Shape.Rect(w, ¦)`), a plain function named like a tag (`Rect(q, ¦)`), a call inside an arm body, and a case's field list in a `variant` declaration all received the case's fields. Because the VS Code server returns tt items alone whenever the engine answers any (`editors/vscode/server/src/server.ts`), TypeScript's own argument completions disappeared at those positions.

## Scope

- Included: How the engine decides that a position is a pattern position (`src/engine/completions.rs`), for payload field lists, nested pattern tags, tuple slots, and the arm and `if let` positions that share the same decision; the engine's unit tests that pointed at a declaration by accident.
- Excluded: The VS Code adapter, whose rule (a non-empty tt answer replaces the TypeScript list) is right once the engine only answers at pattern positions, and the completion items themselves. Comments and literals as completion sites are TASK-462.

## Decisions

### Decision 1: Parsed constructs decide positions inside them; text being typed is read with the pattern grammar

- **Context**: A payload field belongs only in a pattern: a match arm's pattern, an `if let` pattern, or a let-else pattern. The old context test was "the innermost unclosed `(` follows an identifier", which every call satisfies. Completion is, however, also asked while the construct is unfinished (`match (s) { Rect(`), and the parser claims only complete constructs, so a parse-only answer would lose the positions the completion exists for.
- **Alternatives considered**: Filtering calls by what precedes the tag (`.`, `=`, `new`) would be a list of token shapes and would still accept `Rect(q, ¦)` at a statement start and calls inside arm bodies. Answering only from the parse would stop completing every unfinished pattern the existing tests and the extension suite cover (`match (user) { Ad`, `match (s) { Rect(w, `).
- **Decision and rationale**: `parsed_at` walks the program and asks the innermost parsed tt construct containing the position: an arm's `pattern_span` or the slot between arms is a pattern site, an `if let` or let-else alternative span is a pattern site, and every other part of the construct (scrutinee, guard, arm body, bound expression, blocks) is an expression unless a nested construct claims the position. Only when no parsed construct contains the position does `unclaimed_site` read the token stream, and it recognizes a pattern only by the grammar that introduces one: the arm grammar of a `match (…) {` body (pattern, optional `if` guard, `=>`, body up to a top-level `,`, as the parser and its arm recovery use), or the alternatives after `if let` or after a let-else's declaration keyword whose first alternative has parens. `site_context` then reads the pattern's own tokens for the field list, nested tag, tuple slot, or tag position. In LSP 3.17 terms the server answers `textDocument/completion` with its own `CompletionItem`s only where tt owns the syntax; everywhere else the TypeScript list stands.

## Work log

- 2026-09-28: Reproduced with the hunter's `t6.py` and `cp3.txt`: `Shape.Circle(¦)`, `Shape.Rect(w, ¦)`, `Option.Some(¦)`, and `Rect(q, ¦)` all returned the payload fields.
- 2026-09-28: Replaced `innermost_open` and `enclosing_match_body` in `src/engine/completions.rs` with the site model: `parsed_at`/`segment_at`/`arms_at`/`if_let_at` over the AST, `unclaimed_site`/`arm_start`/`pattern_head`/`match_body` over the token stream, and `site_context` for the position inside the pattern. `tt_completions_at` now lexes and parses once with `lex_and_parse_with_kind`.
- 2026-09-28: Four existing unit tests failed; see Issue 1. Pointed them at the positions they describe.
- 2026-09-28: Added `call_arguments_are_not_payload_positions`, `a_variant_declaration_is_not_a_payload_position`, and `payload_positions_follow_the_pattern_that_introduces_them`. The hunter's `cp1`–`cp3` constructor and call cases now return no tt items.

## Issues and resolutions

### Issue 1: Unit tests had been completing inside the variant declaration

- **Symptom**: `a_payload_position_offers_the_cases_fields`, `a_nested_position_offers_the_fields_variant`, `payload_positions_work_in_every_construct_that_has_a_pattern`, and `ambiguous_tags_preserve_fields_and_deduplicate_insertions` returned no items after the change.
- **Cause**: Their needles (`"Rect("`, `"Wrap(inner: "`, `"{ Shared("`) first occur in the `variant` declaration, whose field list the old rule treated as a pattern. The tests passed because of the bug this task fixes.
- **Resolution**: The needles now name the pattern they describe (`"{ Rect("`, `"match (o) { Wrap(inner: "`, `"if let Wrap(inner: "`, `"match (x) { Shared("`), and `a_variant_declaration_is_not_a_payload_position` pins the declaration.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`

## Result

Changed `src/engine/completions.rs`. Payload fields, nested tags, and tuple slots are offered only inside a match arm pattern, an `if let` pattern, or a let-else pattern, whether the construct parses or is still being typed; constructor and function call arguments, arm bodies, guards, and variant declarations get no tt items, so the editor keeps TypeScript's completions there. TASK-462 builds on the same site model for arm positions and comments.
