# TASK-558: Read the member-completion context from the token stream

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-558: Read the member-completion context from the token stream`

## Purpose

Completion of a member name after a receiver that is not a plain name
(`nm.trim().ma`, `foo().t`, `xs[0].t`, `"abc".len`, `k |> .t`) listed tt's
`Option`, `Result` and keyword snippets first, and String's `match` method
was replaced by the tt `match` snippet. TypeScript lists the members only.

## Scope

- Included: How the member context of a completion is decided
  (`engine::member_access_at`), the `ttCompletions` answer that carries it,
  and the extension's completion handler.
- Excluded: The members themselves, which TypeScript answers as before.

## Decisions

### Decision 1: The engine reads the member context from the lexer's tokens

- **Context**: The extension decided it from text: `memberAccessAt`
  accepted only an identifier right before the `.`, and `atMemberAccess`
  looked only at the character before the cursor. With a name being typed
  after `trim().`, neither held, so the general list (tt items, then
  TypeScript's, with tt labels replacing TypeScript's) was answered.
- **Alternatives considered**: (a) Widen the text scan in `analysis.ts` to
  walk back over the identifier and accept any receiver: a second reading
  of the syntax next to the compiler's lexer, over a mask that has its own
  rules for strings, regexes, templates and JSX. (b) Let `completion`
  answer whether the service found members: the extension must know the
  context before it decides whether to add tt items, and a pattern
  position would then pay for a TypeScript request.
- **Decision and rationale**: `member_access_at` lexes the buffer with the
  compiler's lexer and answers when the name being typed (or none yet)
  directly follows a `.` or `?.` token, not a dot of `...`, outside
  comments and literals, inside template interpolations and JSX expression
  containers. It also answers the receiver when it is a path of names
  (`Result`, `ns.Shape`), through which a variant's constructors are
  offered. `ttCompletions`, the parse-only completion request, answers it as
  `member`; the extension relays it as the completion request's `member`
  flag and adds tt items only outside a member access.

### Decision 2: Remove the extension's masking and member scan

- **Context**: `maskNonCode` and `memberAccessAt` existed only for this
  decision.
- **Decision and rationale**: Both are removed with their tests; the JSX,
  template, and attribute cases those tests pinned are now cases of the
  engine's test. `analysis.ts` keeps the word at the cursor.

## Work log

- 2026-09-29: Reproduced through the language server with each reported
  receiver: `Option`, `Result`, and the `flow`, `let-else`, `match` snippets
  sorted first; the `match` item was the tt snippet.
- 2026-09-29: Added `MemberAccess` and `member_access_at`
  (`src/engine/completions.rs`, exported from `src/engine/mod.rs`), the
  `member` field of the `ttCompletions` answer (`src/server.rs`),
  `EngineTtCompletions` (`engine.ts`), and the handler change
  (`server.ts`); removed `maskNonCode`, `memberAccessAt` and
  `atMemberAccess`.
- 2026-09-29: Tests: `a_member_access_is_read_from_the_tokens_before_the_name`
  (`src/engine/completions.rs`), "a member name after any receiver
  completes members only" (`server.test.ts`, which fails with `member`
  withheld from the answer: `Option offered`), and the `member` field in
  `engine.test.ts`. Updated `docs/design/lsp-architecture.md`.

## Issues and resolutions

### Issue 1: An unterminated template interpolation is template text

- **Symptom**: The engine test's first form, `` `${obj.na `` at the end of
  the buffer, answered no member access.
- **Cause**: The lexer reads an interpolation without its `}` as template
  text while it is being typed (by design: a partial interpolation would
  give the parser overlapping spans).
- **Resolution**: The test asks inside a closed interpolation, as an editor
  with bracket closing has it.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib`
- [x] `editors/vscode`: `npm run compile`; `analysis`, `engine`,
  `completion` and `server` tests passed.

## Result

Completion of a member name answers TypeScript's members only, whatever
the receiver. Changed `src/engine/completions.rs`, `src/engine/mod.rs`,
`src/server.rs`, `editors/vscode/server/src/{analysis,engine,server}.ts`,
their tests (`analysis.test.ts`, `engine.test.ts`, `server.test.ts`), and
`docs/design/lsp-architecture.md`.
