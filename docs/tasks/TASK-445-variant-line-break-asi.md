# TASK-445: Keep `variant` followed by a line break an expression statement

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`variant` is an ordinary TypeScript identifier. When a line break follows it, TypeScript's automatic semicolon insertion ends the statement there, so

```ts
variant
Foo
{ A }
```

is three statements: `variant;`, `Foo;`, and the block `{ A }`. `tsc --strict` accepts it. ttc claimed it as `variant Foo { A }` and emitted a `type Foo`/`const Foo` declaration, which broke the rule that every valid TypeScript file compiles to itself. `declare\nvariant Foo { A }` had the same problem: ASI ends the statement after `declare`, so the variant is not ambient.

## Scope

- Included: The variant claim in `src/parser/parse.rs` and `src/parser/variants.rs`, the shared line-break query in `src/parser/cursor.rs`, and the variant-body test in the flow statement splitter (`src/flow/syntax.rs`).
- Excluded: The flow splitter's general lack of ASI between an identifier and a `{` on the next line (`foo\n{ return 0; }` inside a let-else `else` is not seen as a block). That limitation predates this task and affects every identifier, not only `variant`.

## Decisions

### Decision 1: Follow TypeScript's restricted productions for contextual declaration keywords

- **Context**: TypeScript's parser treats `declare`, `type`, `namespace`, `module`, `abstract`, and similar contextual keywords as declaration starts only when the next token is on the same line (`nextTokenIsIdentifierOrKeywordOnSameLine` and `nextTokenCanFollowModifier` in `parser.ts`). Otherwise the word is an identifier and ASI applies.
- **Alternatives considered**: Rejecting the multi-line form with a tt diagnostic would make valid TypeScript fail to compile. Requiring the whole declaration on one line would reject `variant Foo {\n A\n}`, which TypeScript tolerates for `enum`.
- **Decision and rationale**: `variant` claims a declaration only when no line terminator (LF, CR, U+2028, U+2029) separates it from the name, and `declare` makes a variant ambient only when `variant` is on the same line. This is the same rule TypeScript applies to `type Foo = ...` and `declare`. A comment containing a line break counts as a line break, as it does in TypeScript. The flow model's `variant_or_enum_body` applies the same condition so it never claims a statement boundary the parser did not claim.

## Work log

- 2026-09-27: Reproduced `variant\nFoo\n{ A }` and `variant /* a\n */ Foo\n{ A }` compiling to a variant declaration; confirmed `tsc --strict --noEmit` accepts the input.
- 2026-09-27: Added `line_break_before` to the parser cursor and checked it in `parse_variant` and at the `declare variant` / `export declare variant` claim sites.
- 2026-09-27: Applied the same condition in `variant_or_enum_body`.
- 2026-09-27: Added `variant_followed_by_a_line_break_is_an_expression_statement` and `declare_followed_by_a_line_break_does_not_declare_the_variant` to `tests/passthrough.rs`. Both fail without the parser change.

## Issues and resolutions

- **Symptom**: Twelve `tests/integration.rs` cases failed on the first full run with `No space left on device`. **Cause**: The shared disk filled during the run; it was an environment problem, not a code one. **Resolution**: Freed this worktree's incremental build cache and re-ran the integration target (166 passed) and the targets cargo had skipped after the failure (all passed). No code change was needed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/parser/cursor.rs`, `src/parser/parse.rs`, `src/parser/variants.rs`, `src/flow/syntax.rs`, `tests/passthrough.rs`, and `docs/tasks/INDEX.md`.
