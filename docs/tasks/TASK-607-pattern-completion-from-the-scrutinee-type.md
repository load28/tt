# TASK-607: Complete an arm's pattern with what the scrutinee's type admits

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-607: Complete an arm's pattern with what the scrutinee's type admits`

## Purpose

Pattern completion in a literal `match` offered unrelated variant tags and
no literal: with `type Dir = "north" | "south"`, completion in
`match (d) { "north" => 1, | }` offered `_ Circle Err None Ok Point Some`.
The TypeScript equivalent `switch (d) { case "north": ...; case | }` offers
`"south"`. A string or number scrutinee must never be offered variant tags.

## Scope

- Included: The parse-level evidence a finished literal arm gives
  (`arm_tags`, `pattern_question` in `src/engine/completions.rs`), the typed
  arm-slot question (`Project::pattern_completions`,
  `discriminant_candidates` in `src/engine/language/project.rs`,
  `discriminant` and `arm_candidates` in `src/engine/language/service.rs`),
  the match scrutinee lookup (`scrutinee_at`,
  `src/engine/declarations.rs`), reading a text as one arm pattern
  (`parser::pattern_of`), the `patternCompletions` server method, the
  `pattern` flag of `ttCompletions`, the `Literal` completion kind, and the
  adapter's pattern branch.
- Excluded: Payload field completion and completions a generated switch
  contributes (TASK-608), tuple-match slots, `if let` and let-else patterns
  (their parse-level answer is unchanged), and nested payload patterns.

## Decisions

### Decision 1: A finished literal arm is evidence that no variant tag belongs in the match

- **Context**: `arm_tags` counted only tag arms as evidence (TASK-492), so a
  match whose finished arms were all literals had no evidence and every
  visible variant's cases were offered.
- **Alternatives considered**: Keep literal arms neutral and let the typed
  answer replace the list: without a toolchain the list would still be
  wrong.
- **Decision and rationale**: Literal and tag patterns never mix in one
  match (`Pattern::Literals` in `src/ast.rs`: the emitted discriminant is
  `$tt_m` or `$tt_m.kind`, and sema rejects a mix), so a finished literal
  arm says the match is over literals and the declaration table has no
  candidate there; only `_` is. An `is` arm stays neutral, as TASK-492
  decided. The TASK-492 record says which part this reverses.

### Decision 2: Ask TypeScript for the literals of the scrutinee's discriminant at a comparison

- **Context**: Which literals or tags a scrutinee admits is a fact about its
  TypeScript type (`docs/design/match-literal-patterns.md`: ttc must not
  grow a type system to answer it). The typed exhaustiveness pass asks the
  API server, but only inside a whole-project check; completion is asked of
  the language service.
- **Alternatives considered**: (a) Ask at a `case` label spliced into the
  emitted switch: a match lowers to a switch only without guards, nested
  tests, `is` tests, or several subjects (`core_ir::lower`,
  `MatchDispatch`), so the question would depend on the lowering form, and
  TypeScript's case tracker would hide a tag whose only arm is guarded,
  which tt does not count as covered. (b) Splice `$tt_probe` into the arm
  slot as the completion probe does: an identifier arm in a literal match
  mixes the families and nothing lowers, and in a tag match the probe
  becomes a string literal's content. (c) Run the typed pass: a
  whole-project check per keystroke.
- **Decision and rationale**: The engine lowers the buffer without the word
  being typed (and, when that still does not lower because the match has no
  arm yet, with a wildcard arm written in the slot: the arm grammar then
  reads a complete arm whose body is missing, TASK-605). The emit mapping
  gives the output the scrutinee was copied to; the question is that output
  with the scrutinee parenthesized and compared: `(scrutinee).kind ===
  $tt_probe` for a tag match (`VARIANT_TAG_FIELD`, the variant ABI's
  discriminant) and `(scrutinee) === $tt_probe` for a literal match, both in
  that order when no arm says which. TypeScript completes the right operand
  of an equality with the literals of the left operand's type
  (`services/completions.ts`: `getContextualType` returns the type of the
  left operand after an equality operator, and `getCompletionData` collects
  its literal constituents as `literals`, answered with kind `string`,
  LSP `CompletionItemKind.Constant`). The comparison stands where the
  scrutinee is evaluated, so the type is the narrowed one the match sees,
  whatever form the match lowers to. The question is served only for the
  request and the projection is served back before the answer is read.

### Decision 3: Keep an entry only when tt's arm grammar reads its label as one literal

- **Context**: The answer at an expression position also holds every name
  in scope and TypeScript's keywords.
- **Alternatives considered**: Filter by completion kind or sort text: a
  constant variable has the same kind, and a rank is not a statement about
  what the entry is.
- **Decision and rationale**: `parser::pattern_of` reads the label with the
  arm pattern grammar. A literal match keeps a label that is one literal
  alternative, as written (`"south"`, `2`); a tag match keeps a string
  literal whose value is itself a bare tag (`"Beta"` gives `Beta`).
  TypeScript's `true`/`false` keywords are offered at every expression
  position whatever the type, so keyword entries are not literal entries.

### Decision 4: A covered candidate stays in the list, sorted after the rest

- **Context**: TypeScript drops a `case` value another clause already has.
  tt's pattern completion keeps a covered tag and sorts it last, because a
  guard may repeat a tag.
- **Alternatives considered**: Drop covered literals as TypeScript's switch
  does: a literal arm can be guarded too, and tags and literals would then
  follow different rules.
- **Decision and rationale**: An unguarded arm's literal (compared by value,
  so `'north'` covers `"north"`) or tag marks the candidate covered; the
  adapter sorts it after the uncovered ones. A tag the declaration table
  knows keeps its case signature as detail. `_` is always offered.

## Work log

- 2026-09-30: Reproduced with the probe harness (`cmp.cjs k1.tt k1.ts c`):
  tt offered `_ Circle Err None Ok Point Some` at every literal-match slot;
  tsgo offered `"south"` in `switch (d) { case "north": break; case | }`.
- 2026-09-30: Checked the lowering (`ttc -p`): literal arms lower to
  `switch ($tt_m)`, tag arms to `switch ($tt_m.kind)`, and a
  `$tt_probe` spliced into a literal match's slot does not lower
  (`malformed-match`).
- 2026-09-30: Implemented the evidence change, the comparison question, the
  label filter, the server method, and the adapter branch; the adapter uses
  the parse-level list when the engine cannot serve the file.
- 2026-09-30: Re-ran the harness: `match (d) { "north" => 1, | }` offers
  `"south"`, `"north"` (covered) and `_`; a `string` scrutinee offers `_`.
- 2026-09-30: Tests: `pattern_completion_offers_what_the_scrutinee_type_admits`
  (`tests/native/editor_service.rs`: literal, number, string, tag, typed
  prefix, empty match, guarded arm, and `variant` scrutinees),
  `a_match_over_literals_offers_no_variant_tags`
  (`src/engine/completions.rs`), and the extension test "pattern completion
  offers what the scrutinee's type admits" (`server.test.ts`).

## Issues and resolutions

### Issue 1: `true` and `false` were offered for a string scrutinee

- **Symptom**: The first run offered `false` and `true` beside `"north"`
  and `"south"`.
- **Cause**: TypeScript offers its keywords at every expression position,
  and `true`/`false` parse as literal patterns.
- **Resolution**: Keyword entries are not literal entries (Decision 3).

### Issue 2: A match with no arm had no typed answer

- **Symptom**: `match (d) { | }` offered every visible variant's tags.
- **Cause**: A match body with no arm is not claimed, so nothing lowers and
  there is no scrutinee copy to ask about.
- **Resolution**: The question is built again with a wildcard arm written in
  the slot (Decision 2).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native pattern_completion`, `cargo test --lib completions`
- [x] `node --test server/out/test/server.test.js server/out/test/completion.test.js server/out/test/engine.test.js`
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/engine/completions.rs`, `src/engine/declarations.rs`,
`src/engine/workspace.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/parser/partial.rs`, `src/parser/mod.rs`, `src/server.rs`,
`tests/native/editor_service.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/test/server.test.ts`, `editors/vscode/README.md`,
`docs/design/lsp-architecture.md`, the TASK-492 record, and the task index.
An arm is completed with the literals or tags the scrutinee's type admits.
