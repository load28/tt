# TASK-579: Lex an unterminated template interpolation as an expression

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-579: Lex an unterminated template interpolation as an expression`

## Purpose

In `const at = new Date();\nconst s = \`returned ${at.‸` a `.` trigger
returned no items, and invoked completion after `at.ge` returned the
general list. tsgo gives Date's members. The member context comes from tt's
tokens, and the lexer read an interpolation without its `}` as template
text, so no `.` token was there.

## Scope

- Included: how the lexer reads an unterminated `${`
  (`lex_template`), what the parser and HIR take from it (the template's
  extent), the host syntax preflight's view of the open `${`, and the
  engine's walk into interpolations (`innermost_tokens`).
- Excluded: the completion answers themselves, which TypeScript gives once
  the member context is found.

## Decisions

### Decision 1: An unterminated `${` opens an interpolation that runs to the end

- **Context**: TypeScript's scanner reads `` `returned ${ `` as a template
  head and then scans expression tokens, whatever follows; the literal is
  reported unterminated. tt's lexer instead broke out of the template and
  kept everything from the `${` as one raw chunk (TASK-365, Decision 2).
  That decision answered a fuzz crash: recovery then emitted an
  interpolation through end-of-file *and* a raw chunk after it, whose
  delimiter extension in HIR (`start -= 1`) overlapped the interpolation's
  last byte, and codegen reported `SourceReordered`.
- **Alternatives considered**:
  - Keep the raw chunk and re-lex the text after an unterminated `${` in
    the engine for the member question. A second reading of the buffer for
    one surface, and every other token consumer (pattern context, keyword
    positions, semantic tokens) would still see text there.
  - Treat an unterminated template as closed at the end of the line. That
    is not how TypeScript reads it.
- **Decision and rationale**: The lexer does what TypeScript's scanner
  does: the interpolation's tokens are lexed to the end, and it is the
  template's last part — no raw chunk follows an interpolation that never
  closed, so the parts cover the source once, in order, and nothing
  overlaps. The AST `Template` now carries the token's span, and HIR takes
  the template's extent from it instead of from its raw chunks (which no
  longer reach the end). The TASK-365 regression still passes, and a
  randomized run of 9,000 buffers of template delimiters and tt syntax
  through `check`, `emitMap` and `ttCompletions` in `.tt` and `.ttx`
  answered no internal error. TASK-365's record says it is superseded.

### Decision 2: The open `${` is the unbalanced delimiter the check reports

- **Context**: With the interpolation's tokens now parsed, a tt construct
  inside it is claimed: `` `a ${x |> f `` was reported as
  `lowering-plan-failed` ("could not plan this construct"), where
  `(x |> f` is `source-not-typescript` ("unbalanced TypeScript
  delimiter") from the host syntax preflight.
- **Decision and rationale**: The preflight's bracket walk
  (`unbalanced_delimiter`) counts an interpolation that runs to its
  template's end as an open `{` at its `${`, as TypeScript expects a `}`
  there. Every unterminated interpolation is then reported at its `${`,
  as an unbalanced delimiter — `verify-failed` for plain TypeScript,
  `source-not-typescript` when the file has tt constructs — instead of
  SWC's message about whatever token came last.

### Decision 3: The engine walks into interpolations by their own spans

- **Context**: `innermost_tokens` entered a template only when the cursor
  was strictly inside the template token. An unterminated template ends
  where the file ends, which is where the cursor is.
- **Decision and rationale**: It enters an interpolation whose span holds
  the cursor, whatever the token's extent. For a closed interpolation the
  span ends at its `}`, so a cursor there is still inside it.

## Work log

- 2026-09-29: Reproduced with `ttCompletions` on
  `` `returned ${at.`` (`member: null`) and through the language server
  (`.` trigger: no items; invoked after `at.ge`: the general list).
- 2026-09-29: Read TASK-365's decision and the fuzz input it fixed, and the
  HIR raw-chunk delimiter extension that made the old recovery overlap.
- 2026-09-29: Changed `lex_template` (`src/lexer.rs`), added
  `Template::span` (`src/ast.rs`, `src/parser/parse.rs`) and used it in
  `lower_template` (`src/hir/lower.rs`). Compared `check` and `emitMap`
  answers for eight unterminated forms against the previous build; found
  Decision 2 and changed `src/lexer/validation.rs`; then Decision 3 in
  `src/engine/completions.rs`.
- 2026-09-29: Tests: `an_unterminated_interpolation_is_an_open_expression`
  (`tests/compile/cases_09.rs`: the diagnostic at the `${` and an identity
  projection), three unterminated cases in
  `a_member_access_is_read_from_the_tokens_before_the_name`,
  `a_member_in_an_unterminated_interpolation_completes_members`
  (`tests/native/cases_10.rs`, which fails with the old lexer: no member
  context), and an unterminated case in the server test "a member name
  after any receiver completes members only". Updated the TASK-365 fuzz
  regression's comment.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: `npm run compile`, then
  `node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

## Result

Changed `src/lexer.rs`, `src/lexer/validation.rs`, `src/ast.rs`,
`src/parser/parse.rs`, `src/hir/lower.rs`, `src/engine/completions.rs`,
`tests/compile/cases_09.rs`, `tests/native/cases_10.rs`,
`editors/vscode/server/src/test/server.test.ts`, and the TASK-365 and
TASK-558 records. A member typed in an interpolation whose `}` is not
written yet completes TypeScript's members, and the check reports the open
`${` where TypeScript expects its `}`.
