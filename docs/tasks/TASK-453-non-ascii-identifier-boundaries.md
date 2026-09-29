# TASK-453: Never read a tt keyword out of the middle of a non-ASCII identifier

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A TypeScript identifier that ends in `try` after a non-ASCII letter was claimed as a tt `try`: `const étry = (n: number) => n; console.log(étry(2));`, `名try(1)`, and `x.étry(1)` failed with `lowering-plan-failed` instead of passing through. The same token stream feeds every tt keyword decision, so any keyword could be carved out of such a word.

## Scope

- Included: Identifier boundaries in the byte scanner (`src/scanner.rs`) and the lexer (`src/lexer.rs`), the byte scanners that start words with the same predicate (`src/scanner.rs`, `src/codegen/core/emitter/helpers.rs`), and the parser's name reader (`src/parser/cursor.rs`).
- Excluded: Treating non-ASCII white space as trivia. Those code points keep lexing as they did before this task (opaque one-byte tokens that separate words). Allowing non-ASCII names or bindings in tt constructs (`docs/ai/tt.md` keeps them ASCII).

## Decisions

### Decision 1: A non-ASCII code point is identifier material unless ECMA-262 makes it white space or a line terminator

- **Context**: The scanner decided on ASCII bytes only. `is_ident_start`/`is_ident_char` rejected every byte at or above 0x80, so the lexer emitted `é` as two one-byte `Punct` tokens and then started a new `Ident` at `t`, which the parser read as the `try` keyword. A trailing non-ASCII letter (`tryé`) only worked by accident: the keyword is still a separate `Ident`, but the next token is not an operand.
- **Alternatives considered**:
  - Refuse to start an identifier right after any byte at or above 0x80. That fixes `étry` but splits `x try` wrongly the other way: ECMA-262 §12.2 makes U+00A0 white space, so `try` there is a whole word. It also leaves the non-ASCII prefix as `Punct` tokens, which every expression-boundary predicate reads as a non-value.
  - Decode identifiers with full Unicode `ID_Start`/`ID_Continue` tables. That needs tables the crate does not carry, and it is not needed: outside strings, comments, templates, regexes, and JSX text, a non-ASCII code point in valid TypeScript is one of three things. ECMA-262 §12.7 (`IdentifierPart`: `ID_Continue`, `$`, ZWNJ, ZWJ) makes it part of an identifier. §12.2 (`WhiteSpace`: ZWNBSP U+FEFF and any `Zs` code point) and §12.3 (`LineTerminator`: U+2028, U+2029) make it a separator. Anything else is a syntax error that TypeScript reports.
- **Decision and rationale**: `identifier_char_len` looks at the whole code point that a UTF-8 lead byte starts. It returns the code point's length unless the code point is white space (Rust's `char::is_whitespace`, the Unicode `White_Space` property, which contains every `Zs` code point plus U+2028 and U+2029) or U+FEFF. A continuation byte, a truncated sequence, or a sequence that crosses the scan end is never identifier material. `starts_identifier` and `ident_end` use it. The code point is consumed whole, so no scanner decision ever splits a multibyte character. This follows the scanner contract: bytes still pass through opaquely, and the only question asked of a non-ASCII code point is whether it separates words. The lexer and every byte scanner that starts a word (`find_matching`, `is_primary_expression`, `member_name_end`, `contains_await`, and `generic_param_names`) use the same predicate. `contains_await`, for example, no longer sees `await` inside `éawait`.

### Decision 2: tt names and bindings stay ASCII

- **Context**: With Decision 1, `café` is a single `Ident` token. Before, a match binding written `Some(prïx)` failed as `malformed-match` only because the lexer produced no identifier. `docs/ai/tt.md` documents that identifiers inside tt constructs are ASCII, and parts of resolution (`variant_of_type`, `entry_of_type`) read type names as ASCII.
- **Alternatives considered**: Accept non-ASCII tt names. This is a language surface change with its own resolution and tooling work, and it is outside this task.
- **Decision and rationale**: `Cursor::eat_ident`, the parser's single reader of tt names and bindings, accepts only an ASCII `Ident`, so those positions behave as before. TypeScript expressions inside a construct (a `try` operand, a scrutinee) are host text and may use any identifier. `const n = try étry();` now lowers; before, it failed with `source-not-typescript`. One observable change: `variant Café { … }` is now a located `malformed-variant` error rather than an unclaimed lookalike that failed the output self-check. That is the TRAP rule `docs/ai/tt.md` already states for variants.

## Work log

- 2026-09-28: Reproduced `lowering-plan-failed` for `étry(2)` with the pre-change binary. With the same binary, `ématch (1)` followed by a line-broken `{ }`, `f(1, éval [0])`, `évariant` and `éresult` on their own lines passed through: their claims need a following shape those files did not have. They share the token stream, so they are covered by the same fix and by the regression test.
- 2026-09-28: Added `starts_identifier` and `identifier_char_len` to `src/scanner.rs`, rewrote `ident_end` on top of them, and switched the lexer and the byte scanners listed in Decision 1 to `starts_identifier`. Documented the new `TokenKind::Ident` extent. Restricted `Cursor::eat_ident` to ASCII names. Updated `docs/ai/tt.md`.
- 2026-09-28: Added a scanner unit test (letters, ZWNJ, an astral letter, each non-ASCII white space and line terminator, truncated sequences), passthrough tests for `try`/`match`/`val`/`flow`/`variant`/`result`/`else` suffixes and for non-ASCII white space between words, and a compile test for a `try` whose operand is a non-ASCII identifier.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `non_ascii_identifiers_ending_in_a_tt_keyword` covers the reported repro, which failed with the previous binary (`lowering-plan-failed`).

## Result

Changed `src/scanner.rs`, `src/lexer.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/helpers.rs`, `src/parser/cursor.rs`, `docs/ai/tt.md`, `tests/passthrough.rs`, `tests/compile/cases_07.rs`, `docs/tasks/TASK-453-non-ascii-identifier-boundaries.md`, and `docs/tasks/INDEX.md`.
