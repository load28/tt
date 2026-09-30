# TASK-646: Color tt keywords as TypeScript's, with semantic tokens only where the grammar cannot decide

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-646: Color tt keywords as TypeScript's, with semantic tokens only where the grammar cannot decide`

## Purpose

TASK-639's `semanticTokensOverTtConstructs` baseline recorded `match` and
`result` as `keyword` semantic tokens but not `variant`, `val`, or `try`.
The question was which layer colors tt's keywords, and whether a keyword
the semantic layer leaves out is a defect: in a `.ts` file TypeScript's
keywords are colored by the TextMate grammar alone.

## Scope

- Included: `src/engine/tokens.rs` (which keywords are reported, as what),
  `VariantDecl::keyword_off` (`src/ast.rs`, `src/parser/variants.rs`), the
  modifiers of tt's own tokens in `merge_tokens`
  (`src/engine/language/service.rs`) and in the parse-only `semanticTokens`
  method (`src/server.rs`, `editors/vscode/server/src/engine.ts`,
  `server.ts`), the extension's `semanticTokenScopes` contribution
  (`editors/vscode/package.json`), the extension tests, the editor cases,
  `docs/design/lsp-architecture.md` and the extension README.
- Excluded: The TextMate grammar (it already scopes every tt keyword as the
  TypeScript grammar scopes the TypeScript keyword of the same role), the
  denial of look-alike calls (TASK-093), and TypeScript's own tokens
  (TASK-606).

## Sources

- LSP 3.17, "Semantic Tokens" (`textDocument/semanticTokens`):
  `SemanticTokenTypes` includes `keyword`, `modifier` and `operator`, and
  `SemanticTokenModifiers` includes `declaration`; a server reports the
  tokens it knows, and the client combines them with its own coloring.
- VS Code, "Semantic Highlight Guide"
  (code.visualstudio.com/api/language-extensions/semantic-highlight-guide):
  "Semantic highlighting is an addition to syntax highlighting"; "The
  editor applies the highlighting from semantic tokens on top of the
  highlighting from grammars"; an extension maps a token selector
  `(*|tokenType)(.tokenModifier)*` to TextMate scopes per language with the
  `semanticTokenScopes` contribution (its example is TypeScript's own
  `property.readonly` mapping).
- VS Code `src/vs/platform/theme/common/tokenClassificationRegistry.ts`
  (main, read 2026-09-30): the default map styles `keyword` as
  `keyword.control` and `operator` as `keyword.operator`; no default exists
  for `modifier`; a selector matches its own type id even when the type is
  not registered (`getTypeHierarchy`).
- typescript-go `internal/ls/semantictokens.go` (main, read 2026-09-30):
  `tokenTypeKeyword` is in the legend but no classification produces it;
  tokens come from `classifySymbol` over identifiers. TypeScript's keywords
  reach the editor only through the grammar.
- The extension's grammar, tokenized with vscode-textmate and oniguruma as
  VS Code does: `variant` is `storage.type.variant.tt` (TypeScript's `class`
  and `enum` are `storage.type.*`), `val` is `storage.modifier.val.tt`
  (TypeScript's `readonly` is `storage.modifier`), `match`, `result` and
  `flow` are `keyword.control.*.tt`, `try` is `keyword.control.trycatch.ts`
  in both of tt's forms, a pattern's `is` is
  `keyword.operator.is-pattern.tt`, and `if let`/`let-else` use
  TypeScript's `if`, `let` and `else` scopes.

## Decisions

### Decision 1: Keywords are the grammar's, as TypeScript's are; a semantic keyword token only where the grammar cannot decide

- **Context**: The grammar colors every tt keyword in every form it can
  see. A TextMate rule matches within one line, so a construct whose
  deciding token sits on a later line is invisible to it. The probe
  `export variant Shape\n{ Circle(r: number), Point }` compiles (the parser
  allows the `{` on the next line) and the grammar scopes `variant` as
  `variable.other.readwrite.alias.ts`; the same holds for a `flow` head
  whose first `|>` is on the next line (TASK-093). `val` is lifted only
  with `const`/`let`/`var` or its binding on the same line (`docs/ai/tt.md`:
  "`val` only in front of `const|let|var` or a parameter, same line"), so
  `val\nconst k = 1;` is not tt and the grammar agrees with the parser on
  every `val` it lifts. `try` is a reserved word: no reading of it as an
  identifier exists.
- **Alternatives considered**: (a) Report every tt keyword (`variant`,
  `val`, `try`, `if let`, `let-else` too), the uniform reading of the
  finding: TypeScript reports none of its keywords, and every token would
  override a grammar scope that is already right with the theme's style for
  the token type, which differs from the grammar's scope for `variant` and
  `val` (`keyword` falls back to `keyword.control`, while `class` is
  `storage.type`). (b) Report no keywords: the parser's claims of a split
  `flow` or `variant` would be lost, and the grammar would leave them plain.
- **Decision and rationale**: A tt keyword is reported only when it is an
  identifier in TypeScript and its construct can continue on a later line:
  `match`, `result`, `flow`, `variant`, and a pattern's `is`. `try`, `val`,
  `if let` and `let-else` are left to the grammar. This removes the bare
  form's `try` token, which the declaration form (`const n = try g()`) never
  had, so the two forms now color alike.

### Decision 2: A reported keyword is styled with the scope the grammar gives the same word

- **Context**: A semantic token replaces the grammar's style for its range.
  `match`, `result` and `flow` as `keyword` style as `keyword.control`,
  which is the grammar's own family (`keyword.control.match.tt`); `variant`
  as plain `keyword` would style as a control keyword, where the grammar
  and TypeScript's `class` use `storage.type`; `is` as `keyword` styled as a
  control keyword, where the grammar says `keyword.operator`.
- **Alternatives considered**: (a) A custom token type (`storageType`) with
  a `semanticTokenTypes` contribution: other LSP clients do not know it.
  (b) Map `keyword` itself for tt: `match` and `variant` would share one
  style.
- **Decision and rationale**: Standard types and modifiers only.
  `variant` is `keyword` with the `declaration` modifier, and the extension
  contributes `semanticTokenScopes` for `tt` and `ttx` mapping
  `keyword.declaration` to `storage.type.variant.tt`; `is` is `operator`,
  whose default is `keyword.operator`; `match`, `result` and `flow` stay
  `keyword`. The extension test "a tt keyword's semantic token is styled as
  the grammar scopes the same word" resolves each reported token through
  the contribution and VS Code's defaults and requires the grammar's scope
  for that word to be the same scope or under it, in `.tt` and `.ttx`.

### Decision 3: tt's own tokens carry modifiers, merged with TypeScript's

- **Context**: tt's tokens had a type only; TASK-606 gave a tt token the
  service's modifiers when both classified the same range the same way.
- **Decision and rationale**: `SemanticTokenKind::modifiers` names the
  modifiers the parser adds (`declaration` for `variant`); `merge_tokens`
  starts from them and adds the service's, and the parse-only
  `semanticTokens` method sends them (`modifiers`), which the adapter
  forwards (an older compiler's answer without the field reads as none).
  The declaration's `variant` keyword offset is recorded by the parser
  (`VariantDecl::keyword_off`), not searched for in the text.

## Work log

- 2026-09-30: Tokenized a probe file with the extension's grammar through
  vscode-textmate and oniguruma (every tt keyword form on one line, then
  `variant` and `val` split across lines); compiled the split forms with
  `ttc --check` and read the output.
- 2026-09-30: Read VS Code's token classification registry and semantic
  highlight guide, the LSP 3.17 token types, and typescript-go's semantic
  token classifier.
- 2026-09-30: Changed `tokens.rs`, `VariantDecl`, `merge_tokens`, the
  server's `semanticTokens`, the adapter, and `package.json`; added the unit
  test `contextual_keywords_are_reported_and_reserved_words_are_left_to_the_grammar`,
  the extension grammar test, the `variant` assertions in the extension's
  semantic token tests, and the editor case `semanticTokensForTtKeywords`;
  regenerated the editor baselines (`semanticTokensOverTtConstructs` gains
  `2:8-2:15 "variant" keyword [declaration]`).

## Issues and resolutions

### Issue 1: `val` split from its declaration was assumed to be tt

- **Symptom**: The first version reported `val` as a `modifier` token, and
  the unit test failed on `val\n  const k = 1;`.
- **Cause**: The probe had compiled, but as TypeScript: `val` on its own
  line is an expression statement, and `ValModifierKind::Declaration` is
  "on one line".
- **Resolution**: `val` is left to the grammar (Decision 1), as the
  comment in `tokens.rs` already said.

### Issue 2: A native test pinned the token list without the declaration keyword

- **Symptom**: The first full gate failed
  `semantic_tokens_classify_the_source_as_typescript_does_with_tt_constructs_over_it`:
  the answer began with `("variant", "keyword.declaration")`.
- **Cause**: The intended change (Decision 1); the test lists every token.
- **Resolution**: The expected list includes the new token.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/semanticTokensForTtKeywords.tt`
  (`tests/editor_cases.rs`); unit test
  `engine::tokens::tests::contextual_keywords_are_reported_and_reserved_words_are_left_to_the_grammar`.
- **Observed failure**: With `tokens.rs`, `service.rs` and `server.rs`
  restored to the previous revision, `TT_CASES=semanticTokensForTtKeywords
  cargo test --test editor_cases` failed with "modified baseline ...
  semanticTokensForTtKeywords.baseline is out of date": the stored
  `2:8-2:15 "variant" keyword [declaration]` was missing from the answer,
  and the answer had `10:28-10:30 "is" keyword` for the stored `operator`
  and an extra `12:52-12:55 "try" keyword` for the bare `try`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test`
  and `node scripts/check-baselines --tracking <dir>`: 46 test binaries, 1763 passed, 0 failed, no `SKIP`; "baselines: 129 compared, none unused". The first run failed one test,
  `semantic_tokens_classify_the_source_as_typescript_does_with_tt_constructs_over_it`
  (`tests/native/editor_service.rs`), whose expected list now starts with
  `("variant", "keyword.declaration")` (Issue 2)
- [x] Extension: `npm run compile` and `node --test
  "server/out/test/*.test.js" "client/out/test/*.test.js"`: 232 tests, 232 passed
- [x] `./scripts/ci agents`: passed (warnings: rolldown not on PATH, doctor reports the checkout not ready; both environmental)
- [x] Baseline changes reviewed and committed with the change

This gate covers TASK-644 to TASK-646.

## Result

Changed `src/engine/tokens.rs`, `src/ast.rs`, `src/parser/variants.rs`,
`src/engine/language/service.rs`, `src/server.rs`,
`editors/vscode/package.json`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`, the extension tests
(`grammar.test.ts`, `server.test.ts`), `tests/native/editor_service.rs`, `editors/vscode/README.md`,
`docs/design/lsp-architecture.md`, the editor case
`semanticTokensForTtKeywords` and two baselines, and the task index. tt's
keywords are colored by the grammar as TypeScript's are; the parser reports
a keyword only where a line-based grammar cannot decide it, styled as the
grammar styles the same word.
