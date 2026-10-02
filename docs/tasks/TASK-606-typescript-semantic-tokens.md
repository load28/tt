# TASK-606: Classify a `.tt` file's source as TypeScript does, with tt's tokens over its constructs

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-606: Classify a .tt file's source as TypeScript does, with tt's tokens over its constructs`

## Purpose

A `.tt` file got none of TypeScript's semantic highlighting. The extension
owns `textDocument/semanticTokens` for `.tt`/`.ttx` (`TT_OWNED_FEATURES`),
so the native TypeScript provider does not run there, and tt's provider
returned only the parser's tokens for tt constructs. In the equivalent `.ts`
file TypeScript classifies `area` as `function.declaration`, `s` as
`parameter`, `scale` as `variable.readonly.local`, `Math` as
`variable.defaultLibrary`, and so on; in `.tt` all of that was the TextMate
grammar's guess.

## Scope

- Included: The engine's semantic tokens over the served projection
  (`Project::semantic_tokens`, `source_tokens`, `merge_tokens` in
  `src/engine/language/`), the service client's semantic-token capability and
  legend (`src/typescript/service.rs`), the server method
  `documentSemanticTokens` (`src/server.rs`), the adapter's legend and
  provider (`editors/vscode/server/src/server.ts`, `engine.ts`), and tt's
  token for a variant field's name (`src/engine/tokens.rs`).
- Excluded: Feature ownership (semantic tokens stay tt-owned), range
  requests (`semanticTokens/range`) and deltas, which the provider has never
  offered.

## Decisions

### Decision 1: The engine asks TypeScript for the served projection's tokens and maps them by the emit mapping

- **Context**: Only the engine knows the projection. The adapter must stay a
  protocol layer (`docs/design/lsp-architecture.md`).
- **Alternatives considered**: (a) Release `textDocument/semanticTokens`
  back to the native extension: the native provider runs over the content
  mapper's projection and knows nothing of tt's constructs, so the parser's
  claim of a split `flow` head or its denial of a function named `match`
  would be lost, and a second provider cannot be layered over the first for
  one document in VS Code (one provider answers a document; the
  `DocumentSemanticTokensProvider` API has no merge). (b) Classify in the
  adapter from hover or definition answers: a second classifier that would
  disagree with TypeScript's.
- **Decision and rationale**: The engine requests
  `textDocument/semanticTokens/full` (LSP 3.17, "Semantic Tokens") from the
  service for the served text. The data uses the relative encoding (delta
  line, delta start, length, type index, modifier bit set) and is decoded
  against the `SemanticTokensLegend` the service returned in its
  `initialize` result; the client declares the LSP 3.17 standard types and
  modifiers in `textDocument.semanticTokens` so the server can encode them,
  with `multilineTokenSupport` and `overlappingTokenSupport` false. A token
  is the user's only when `to_source_span` maps its whole span, that is,
  every byte was copied from the source (the emit mapping); a token on glue
  (the scrutinee temporary `$tt_m`, a variant's generated declarations) is
  dropped, and one the emission copied more than once is reported once.

### Decision 2: tt's tokens win where they overlap, keeping TypeScript's modifiers on the same classification

- **Context**: The parser owns the classification of tt's constructs
  (TASK-093); TypeScript classifies what the emission copied, which inside a
  construct is sometimes the same text (a pattern binding is copied into a
  destructuring).
- **Alternatives considered**: (a) TypeScript's token wins: it would call a
  `match` denied by the parser whatever the emission made of it, and there
  is no TypeScript token at all for a case tag. (b) tt's token wins
  outright: a pattern binding would lose `declaration`, `readonly`, and
  `local`, which TypeScript gives `const { x } = o`.
- **Decision and rationale**: A service token that overlaps a tt token is
  dropped. When the two cover the same range with the same type, the tt
  token takes the service's modifiers: tt decides what the text is,
  TypeScript adds what it knows about it. Tokens are sorted by position.

### Decision 3: The adapter's legend is the LSP standard lists plus TypeScript's `local`

- **Context**: The legend is fixed in the `initialize` result, before any
  engine or service exists, and both tt's and TypeScript's tokens are
  encoded against it.
- **Alternatives considered**: Declare only tt's seven types (the old
  legend): every TypeScript type or modifier outside them would be dropped.
- **Decision and rationale**: The legend lists the LSP 3.17
  `SemanticTokenTypes` and `SemanticTokenModifiers` in their specification
  order, which the VS Code Semantic Highlight Guide documents as the
  standard ones themes style, and TypeScript's own `local` modifier
  (`services/classifier2020.ts`, `TokenModifier.local`), which the native
  TypeScript provider reports. A token whose type is not in the legend is
  not sent, as LSP requires every index to be into the legend.

### Decision 4: A variant field's name is tt's `property` token

- **Context**: A variant field is declared only in glue (the union and the
  constructor each declare it, `declared_names`), so no TypeScript token
  maps to it, and TypeScript classifies its two declarations differently
  (`property` in the union, `parameter` in the constructor).
- **Alternatives considered**: Map the service token of the first glue
  declaration: the answer would depend on the order the emission writes the
  two declarations in.
- **Decision and rationale**: The parser classifies the field name as it
  classifies the variant's name and tags: `property`, the classification
  TypeScript gives a property of an object type.

## Work log

- 2026-09-30: Reproduced with the probe harness (`tok.cjs` over `t3.tt` and
  `t4.ts`): tt returned `enum`, `enumMember`, `keyword`, `variable`, and
  `property` tokens only; tsgo returned `function.declaration`,
  `parameter.declaration`, `type`, `variable.declaration.readonly.local`,
  `variable.defaultLibrary`, and `property.readonly.defaultLibrary`.
- 2026-09-30: Added the legend to `Service` and the capability to the
  initialize request, `Project::semantic_tokens` with `source_tokens` and
  `merge_tokens`, the `documentSemanticTokens` method, the adapter legend and
  provider (falling back to the parse-only tokens when the engine cannot
  serve the file), and the field token.
- 2026-09-30: Re-ran the harness: `area` is `function.declaration`, `s` is
  `parameter`, `Math.PI` is `variable.defaultLibrary` /
  `property.readonly.defaultLibrary`, pattern bindings are
  `variable.declaration.readonly.local`, and no token lands on glue.
- 2026-09-30: Tests: `semantic_tokens_classify_the_source_as_typescript_does_with_tt_constructs_over_it`
  (`tests/native/editor_service.rs`),
  `tt_tokens_replace_the_service_tokens_they_overlap`
  (`src/engine/language/tests.rs`), the field token in
  `claimed_constructs_report_their_tokens` (`src/engine/tokens.rs`), and the
  extension test "semantic tokens carry TypeScript's classification of the
  source under tt's" (`server.test.ts`), whose decoder now reads the full
  legend.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native semantic_tokens`, `cargo test --lib tokens`
- [x] `node --test server/out/test/server.test.js` (semantic tokens cases)
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/typescript/service.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/engine/language/tests.rs`, `src/engine/mod.rs`, `src/engine/tokens.rs`,
`src/server.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/server.ts`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/test/server.test.ts`, `editors/vscode/README.md`,
`docs/design/lsp-architecture.md`, and the task index. A `.tt` file is
highlighted with TypeScript's classification of its source and tt's
classification of its constructs.
