# TASK-462: Offer case tags only at arm positions, never in arm bodies or comments

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttCompletions` offered the subject's case tags and `_` inside arm-body expressions (`Circle(r) => [1, ¦]`, `Circle(r) => r | ¦`, `Circle(r) => (x: number, ¦) => 1`) and inside comments (`// note: Rect(¦`). The VS Code server then returned those items alone, hiding TypeScript's completions for the expression, and a comment got code completions at all.

## Scope

- Included: Arm-position recognition in `src/engine/completions.rs`, and a lexer query for whether a position is inside a comment (`src/lexer.rs`).
- Excluded: The VS Code adapter. Its rule, that a non-empty tt answer replaces the TypeScript list, is correct once the engine answers only at tt positions. Payload positions are TASK-461.

## Decisions

### Decision 1: An arm position is a place the arm grammar expects a pattern

- **Context**: `enclosing_match_body` tracked braces only, and the arm check took any `{`, `,` or `|` directly inside a match body as the start of an arm. A `,` inside `[…]` or a parameter list, or a `|` in a body expression, satisfied it, because the check never asked whether the previous arm was finished or whether the position was in a pattern at all.
- **Alternatives considered**: Adding bracket kinds to the brace walk would fix `[1, ¦]` but not `r | ¦`, where the `|` is at the body's top level. Only the arm grammar tells a body's `|` from an or-pattern's.
- **Decision and rationale**: The arm positions go through the site model TASK-461 introduced for payload positions, so one grammar decides both. For a parsed match, the arm's `pattern_span` and the slots between arms are pattern positions, and a guard or body is an expression. For a match still being typed, `arm_start` walks the body with the parser's arm grammar: pattern, optional `if` guard, `=>`, and a body that ends at a top-level `,`. A tag position exists only while the current arm is in its pattern, at its start or after a top-level `|`. A top-level `,` after a finished body opens the next arm, and a top-level `,` also ends an unfinished arm, as `parse_arm_list` recovery does. TASK-461's commit already routed arm positions through this model; this task records the decision and pins the arm-body and guard cases with regression tests.

### Decision 2: Comments and literals are not completion sites

- **Context**: A comment is trivia, so the token stream has no trace of it. A position inside `// note: Rect(` inside a parsed match body landed in the slot between arms.
- **Alternatives considered**: Searching the line for `//` would misread `//` inside strings, regexes, and templates, and would not see block comments that span lines.
- **Decision and rationale**: `inside_text` asks the lexer's own facts before any construct is asked. A position strictly inside a non-identifier token (a string, template, regex, or JSX text) is inside a literal. A position in the gap between two tokens is checked by `lexer::comment_at`, which scans only that trivia gap with the rules `lex_with_kind` uses to skip comments. A position is inside a comment from just after its opening delimiter to its end: the line end, the `*/`, or the end of the gap for an unterminated block comment. LSP 3.17 leaves which positions complete to the server (`textDocument/completion`). tt's items are names in tt syntax, and a comment holds no syntax.

## Work log

- 2026-09-28: Reproduced with the hunter's `t6.py` and `cp1.txt`–`cp3.txt`. The arm-body array, the `|` and `||` bodies, the arrow-function parameter list, and the line comment inside the match all returned `Circle`/`Rect`/`Point`/`_`.
- 2026-09-28: Confirmed that after TASK-461's site model the arm-body cases return nothing, and only the comment case still returned cases.
- 2026-09-28: Added `comment_at` to `src/lexer.rs` and `inside_text` to `src/engine/completions.rs`. `context` asks it first. Updated the module documentation.
- 2026-09-28: Added `arm_bodies_and_guards_are_expressions` and `comments_and_literals_are_not_completion_sites`, covering parsed and unfinished matches, and the positive cases after a finished arm and after a closed comment. The hunter's `cp1`–`cp3` runs now return tt items only at pattern positions.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`

## Result

Changed `src/lexer.rs` and `src/engine/completions.rs`. Case tags and `_` are offered only where an arm's pattern may start: after the body's `{`, after a finished arm's `,`, or after an or-pattern's `|`. Arm bodies, guards, comments, and literals get no tt items, so the editor keeps TypeScript's completions in expressions and offers nothing tt-specific in comments.
