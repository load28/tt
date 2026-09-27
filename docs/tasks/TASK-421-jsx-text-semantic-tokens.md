# TASK-421: Treat JSX text as opaque when classifying semantic tokens

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`semanticTokens` on a `.ttx` buffer returned a `function` token for the word
`match` inside the JSX text `<p>match (x) is fun</p>`. docs/ai/tt.md states that
JSX tags and text stay opaque. JSX text is not code, so it has no identifiers
to classify.

## Scope

- Included: the lookalike denial in `src/engine/tokens.rs`.
- Excluded: the claimed-construct tokens, which come from the parse. The parse
  already ran under the buffer's surface kind.

## Decisions

### Decision 1: Read lookalikes off the file's own token stream, lexed once under its kind

- **Context**: `deny_lookalikes` re-lexed each verbatim segment with
  `lexer::lex`, which always uses `SourceKind::TypeScript`. Under that kind
  JSX text is ordinary tokens, so `match (` in text looked like a call. The
  parse, by contrast, used `parse_with_kind`.
- **Alternatives considered**: Pass the kind to the per-segment re-lex. A
  verbatim segment can begin partway through JSX, for example after a claimed
  `{match …}` child. Lexing that fragment on its own starts outside the
  element, so the fragment's JSX text would still be read as code.
- **Decision and rationale**: `semantic_tokens_with_kind` lexes the whole
  buffer once with `lex_with_kind(source, 0, len, kind)`, the same lexer the
  parser uses. `deny_in` then classifies identifiers that lie inside each
  verbatim span, descending into template interpolations that overlap the
  span. JSX text arrives as `JsxRaw`. JSX expression containers are still
  lexed as code, so `<p>{match(x)}</p>` is still reclassified as a call.

## Work log

- 2026-09-27: Reproduced the defect with `semantic_tokens_with_kind(…, Tsx)`.
  Implemented the decision.
- 2026-09-27: Added `jsx_text_stays_opaque_and_jsx_expressions_are_still_read`.
  It fails with the old per-segment TypeScript lex and passes with the fix.
  The existing token tests pass unchanged.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib engine::tokens`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (full suite, after TASK-416–421)
- [x] `./scripts/ci extension`
- [x] `node scripts/check-task-index`

## Result

JSX text in `.ttx` produces no semantic tokens. Changed `src/engine/tokens.rs`.
