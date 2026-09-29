# TASK-492: Completion reads the parser's arm structure

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`src/engine/completions.rs` had its own copy of the match arm grammar for text that does not parse yet (`unclaimed_site`, `match_body`, `arm_start`, `pattern_head`, `arm_tags`). It ran alongside the parser's `parse_arm_list`, `recover_match_arms`, and `expr_body_end`, and the two had drifted. For example, `match_body` accepted only `match (…) {`, while the parser also commits to a match with an identifier scrutinee (`match s {`). The fix makes the parser the only owner of that grammar. Completion reads only what the parser outputs.

## Scope

- Included: A parser query module (`src/parser/partial.rs`). The match head, arm walk, arm pattern, and single-pattern head rules it uses, which are extracted from the sub-parsers and shared with them (`src/parser/matches.rs`, `iflets.rs`, `lets.rs`, `parse.rs`). Removal of the duplicated grammar from `src/engine/completions.rs`. Regression tests.
- Excluded: The pattern-internal position logic in `site_context` (payload field list, nested tag, tuple slot). It reads a single pattern's own tokens, not the arm grammar. The expression scanners' `skip_match_shape` in `src/parser/cursor.rs` also stays as it is. It steps over a complete `match (…) { … }` only, and changing it would change which `if let` and let-else expressions parse.

## Decisions

### Decision 1: Parser-owned position queries, not a new AST node

- **Context**: Completion needs two answers for text that does not parse: which pattern a position is in (the arm and its part, or an `if let`/let-else pattern), and which arms of the body are already written.
- **Alternatives considered**: (a) Add a partial-match recovery node to the AST. That would mean claiming `match … {` text that the parser leaves verbatim today. For the uncommitted shapes (`match (user) { Ad`, which has no `=>`), it would also mean recording a construct that is not tt yet, which is contract 1 territory, only so that an editor could read it. (b) A parser query over the token stream that uses the parser's own rules.
- **Decision and rationale**: Option (b). `parser::pattern_site_at(src, tokens, before)` returns a `PatternSite` (`Arm { open, start }` or `Single { start }`). `parser::arm_headers(src, tokens, open)` returns the finished arms of a body, each with its pattern as parsed by the arm pattern grammar and whether it is guarded. Completion keeps using the AST for constructs that parse (`parsed_at`) and uses these queries otherwise. It no longer has any grammar of its own.

### Decision 2: One arm walk for the strict list, the recovering list, and partial text

- **Context**: Three places delimited arms: the parser's strict list, its recovery skip, and the completion walk. The completion walk also tracked the `if` guard and the `=>`.
- **Alternatives considered**: Expose a completion-only walk from the parser. That would still leave two walks in the parser.
- **Decision and rationale**: `matches::outline_arms` is the only walk. An arm ends at a `,` outside every bracket, and it records its top-level `if` and `=>`. A bracket left open runs to the end, so a pattern or argument list being typed stays in its arm. `parse_arm_list` (recovery) and `parse_strict_arm_list` parse each outlined arm on its own sub-cursor and require the arm grammar to consume all of it. For balanced input the arm boundaries are the ones the old cursor loop found, because a successfully parsed arm always ends at a top-level `,` or at the end of the list. The whole test suite, including the recovery spans and the body-count regression, is unchanged.

### Decision 3: Match and single-pattern heads are parser predicates

- **Context**: `match_body` recognized only `match (…) {`. `pattern_head` walked backwards over alternatives to `if let` or `const|let|var` without the parser's dotted-name and reserved-tag rules.
- **Decision and rationale**: `matches::match_body_open` gives the body brace for both committed shapes. `parse_match` uses it to decide whether to commit, and a parenthesized scrutinee now takes its body brace from after the closing paren, not from the first `{` in the scrutinee. `parse::match_keyword_at` is the undotted-or-spread keyword rule, and the main loop uses it. `iflets::if_let_pattern` and `lets::let_else_pattern` are the head rules. The main loop and `if_let_end` use them before calling the sub-parsers, and `parse_let_else` no longer repeats the checks. The partial query reads alternatives forward from the head with `matches::parse_alternative`.

## Work log

- 2026-09-28: Read `src/engine/completions.rs`, `src/parser/matches.rs`, `iflets.rs`, `lets.rs`, `parse.rs`, and `cursor.rs`, and TASK-461/462.
- 2026-09-28: Added `match_body_open`, `ArmPart`, `ArmOutline`, `outline_arms`, `list_arms`, `parse_whole_arm`, and `parse_arm_pattern` to `matches.rs`. Rewrote `parse_arm_list` and `parse_strict_arm_list` over the outline. Added `if_let_pattern`, `let_else_pattern`, `match_keyword_at`, and `Parser::new`, and routed the main loop through them.
- 2026-09-28: Added `src/parser/partial.rs` with `pattern_site_at` and `arm_headers`. Deleted `unclaimed_site`, `match_body`, `matching_open`, `ArmPart`, `arm_start`, `pattern_head`, and the token-walking `arm_tags` from `completions.rs`. `arm_tags` now reads `arm_headers`.
- 2026-09-28: Added regression tests: `an_identifier_scrutinee_match_has_arm_positions`, `an_unclosed_body_keeps_its_finished_arms_as_evidence`, and `only_tag_patterns_are_arm_evidence` (completions), plus four tests in `parser::partial` covering both match heads, arm parts, single-pattern heads, and the arm walk.
- 2026-09-28: Marked TASK-461 and TASK-462 as superseded in part. Added a paragraph on `parser/partial.rs` to `docs/design/compiler-architecture.md`.

## Issues and resolutions

### Issue 1: An `is` arm was read as naming the tag `is`

- **Symptom**: In `match (s) { is Circle => 1, Po|`, the old token walk took `is` as a tag, found no variant declaring it, and offered only `_`.
- **Cause**: `arm_tags` took the first identifier of every finished arm as a tag, whatever the pattern kind.
- **Resolution**: `arm_headers` parses each finished pattern with the arm pattern grammar, and only `Pattern::Tags` counts as evidence. This is pinned by `only_tag_patterns_are_arm_evidence`.

### Issue 2: Expected labels in the first draft of the new tests

- **Symptom**: The new tests expected only the subject's cases where no arm was written yet, and one needle (`"r, "`) first matched inside the variant declaration.
- **Cause**: With no finished arm there is no evidence, so every visible case is offered (as `every_prefix_retains_cases_without_arbitrary_variant_selection` already pins). The needle was ambiguous.
- **Resolution**: The no-evidence cases now assert membership. The evidence cases write a finished arm, and the needles name the arm (`"=> r, "`).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension` (208/208)

## Result

Changed `src/parser/partial.rs` (new), `src/parser/matches.rs`, `src/parser/iflets.rs`, `src/parser/lets.rs`, `src/parser/parse.rs`, `src/parser/mod.rs`, `src/engine/completions.rs`, `docs/design/compiler-architecture.md`, and the TASK-461/462 records. The parser owns the match head, the arm walk, the arm pattern, and the single-pattern heads. Completion reads the AST for constructs that parse and the parser's partial queries for text that does not. A match with an identifier scrutinee now has arm positions, and an `is` or literal arm no longer counts as tag evidence.
