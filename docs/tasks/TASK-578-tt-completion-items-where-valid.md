# TASK-578: Offer tt completion items only where they are valid, ranked as TypeScript ranks

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-578: Offer tt completion items only where they are valid, ranked as TypeScript ranks`

## Purpose

At a general position the extension put every visible variant (sortText
`0…`) and every keyword snippet in front of TypeScript's answer, wherever
the cursor was: a JSX attribute name (`<Row ‸ />`), an object literal key
(`const cfg: Cfg = { ‸ }`), an import specifier
(`import { ‸ } from "./orders.tt"`, with `Option` and `Result` first though
the module exports neither), and a value position in a file that imports
no built-in, where accepting `Option` gave "Cannot find name". tsgo lists
only what is valid there, the names in scope first.

## Scope

- Included: which tt items the general-position branch of
  `connection.onCompletion` adds, the engine decision behind it
  (`engine::tt_keywords_at`, the `keywords` field of `ttCompletions`), and
  the two lexer facts it reads.
- Excluded: pattern completion, the member branch (a variant's
  constructors after `Variant.`), and the snippet texts.

## Decisions

### Decision 1: A variant's name at a general position is TypeScript's entry

- **Context**: The extension listed `declarations`' variants — local,
  imported and built-in — and dropped TypeScript's entries of the same
  labels. The emission declares every local and imported variant (a type
  alias and a constant), so TypeScript already lists each of them wherever
  it is in scope and valid, ranked by its conventions; a built-in is in
  scope only when imported (`docs/ai/tt.md`: built-ins give checking only),
  and then it is an ordinary import TypeScript lists too.
- **Alternatives considered**: Have the engine decide where a variant name
  is valid and rank tt's own item by TypeScript's rank for a declaration
  in scope. That is a second answer to a question TypeScript's list already
  answers exactly, and at an object literal key or an import specifier it
  would still have to repeat TypeScript's contextual rules to stay out.
- **Decision and rationale**: The general-position branch adds no variant
  names. TypeScript's own entries stand, with their rank, resolve data and
  import edits. `Option` in a file that imports nothing is then whatever
  TypeScript has in scope (the DOM's `Option` constructor), never a tt item
  that claims the built-in is. Without a TypeScript toolchain no name is
  completed at a general position, variant or otherwise; pattern and
  member completion still answer from tt's table.

### Decision 2: The engine decides which tt keywords can begin at the position

- **Context**: The keyword snippets (`variant`, `match`, `try`, `flow`,
  `result`, `let-else`) are tt's and TypeScript cannot rank or place them.
  TypeScript's own keyword filter does not help: it offers `if` at
  `const x = ‸` as at a statement start (measured with tsgo through the
  engine).
- **Alternatives considered**:
  - Infer the position from TypeScript's answer (keywords present or not).
    That reads a grammar decision off another tool's output, and cannot
    tell a statement from an expression position.
  - Classify the token before the cursor in the extension. A second reading
    of the grammar beside the lexer's, which already models it.
  - Parse with SWC at a probe. A buffer being typed often does not parse.
- **Decision and rationale**: The lexer's facts machine already records,
  per token, the grammar position it stands in; it is the model every
  consumer reads (`src/lexer/facts.rs`). Two facts are added:
  `OPERAND_START` (an expression expected an operand and the token began
  it) and `MODIFIED` (the token continues a statement `export`, `declare`
  or decorators began). `tt_keywords_at` reads the facts of the word being
  typed, or of a probe word spliced at the cursor when there is none, in
  the innermost template interpolation: `variant` where a statement begins
  or after `export`; `try` and `let-else` where a statement begins; `match`,
  `flow` and `result` where a statement or an unmodified operand begins.
  Nothing in a comment or a literal, at a member access, or in a pattern.
  `ttCompletions` answers them as `keywords`, and the extension shows the
  matching snippets and nothing else of tt's.

### Decision 3: The snippets carry TypeScript's keyword rank

- **Decision and rationale**: Each keyword carries `sortText` `15`,
  TypeScript's `SortText.GlobalsOrKeywords`, and the extension prefixes it
  like every TypeScript entry (`215`), so the snippets sort among
  TypeScript's keywords, after the names in scope (`211`). A TypeScript
  entry with a snippet's label is still replaced by the snippet.

## Work log

- 2026-09-29: Reproduced each reported position with `ttc --server`
  (`target/probe/kw.cjs`): TypeScript answered `a b` for the object key,
  `a` for the JSX attribute, `Shape area type` for the import specifier,
  and the extension put the variants and snippets before them.
- 2026-09-29: Measured TypeScript's keyword entries at statement, expression,
  type, object key, JSX attribute and import specifier positions (Decision
  2).
- 2026-09-29: Added the facts (`src/lexer/facts.rs`,
  `src/lexer/facts/{expressions,statements}.rs`), `TtKeyword` and
  `tt_keywords_at` (`src/engine/completions.rs`, exported from
  `src/engine/mod.rs`; `PROBE_NAME` in `src/engine/language.rs` is now
  shared), the `keywords` field (`src/server.rs`, `engine.ts`) and the
  general-position branch (`server.ts`). Probed twenty positions.
- 2026-09-29: A first server test asserted that TypeScript offered no
  `Option` at a value position; its entry is the DOM's `Option` global, a
  valid name. The test asserts the entry is TypeScript's instead.
- 2026-09-29: Tests: `a_tt_keyword_is_offered_where_its_construct_can_begin`
  (`src/engine/completions.rs`) and "tt items are offered only where they
  are valid, ranked as TypeScript ranks keywords" (`server.test.ts`), which
  fails with the old branch (`<Row : Option in [...]`). Updated
  `docs/design/lsp-architecture.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib`
- [x] `editors/vscode`: `npm run compile`; `server`, `completion` and
  `engine` tests passed.
- [x] Full suites with TASK-579 (see that record).

## Result

Changed `src/lexer/facts.rs`, `src/lexer/facts/expressions.rs`,
`src/lexer/facts/statements.rs`, `src/engine/completions.rs`,
`src/engine/mod.rs`, `src/engine/language.rs`, `src/server.rs`,
`editors/vscode/server/src/{engine,server}.ts`,
`editors/vscode/server/src/test/server.test.ts` and
`docs/design/lsp-architecture.md`. tt's keyword snippets appear only where
their construct can begin and rank among TypeScript's keywords; variant
names are TypeScript's entries.
