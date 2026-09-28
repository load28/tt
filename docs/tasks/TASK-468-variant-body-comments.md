# TASK-468: Keep comments written inside a variant body

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttc -p` dropped every comment written inside a `variant` body: JSDoc on a case (`/** A round shape. */ Circle(radius: number)`), JSDoc on a payload field, and `// trailing` or own-line comments. The emitted type alias and constructor object are what TypeScript answers hover, completion and signature help from, so the documentation a user wrote on a case or field never reached the editor. The same gap produced invalid output: a line comment after the last field or case without a trailing comma (`h: number // height` before `)`) was swallowed into the field's type text and emitted as `h: number // height }`, commenting out the rest of the member and the constructor signature.

## Scope

- Included: Comment trivia on variant cases and payload fields in the AST and parser (`src/ast.rs`, `src/parser/variants.rs`), carried through HIR (`src/hir/mod.rs`, `src/hir/lower.rs`) and Core IR (`src/core_ir/mod.rs`, `src/core_ir/lower.rs`) to variant emission (`src/codegen/core/emitter/helpers.rs`). Tests, a reviewed emit fixture, and `docs/ai/tt.md` §variant.
- Excluded: Comments between a field's tokens other than around the type (between the name and `?`/`:` they are kept as trailing comments of the field; inside the type they were and remain part of the type text). The engine's hover split of signature and documentation (see Issue 2).

## Decisions

### Decision 1: Attach each comment to the nearest case or field, following TypeScript's leading/trailing rule

- **Context**: A comment in the body has no syntax node of its own; emission is a reshaped declaration (a union alias and a constructor object), so byte passthrough of the body is not available and every comment needs an owner to be placed with.
- **Alternatives considered**:
  - Emit the body's comments as one block before the declaration. Loses which case or field a comment documents, so hover gains nothing.
  - Only keep JSDoc. Silently drops ordinary comments, the reported defect.
- **Decision and rationale**: The parser records, per case and per field, `leading` and `trailing` comments, using the rule TypeScript's own emitter applies (`getLeadingCommentRanges`/`getTrailingCommentRanges`): in the gap between two elements, comments before the first line break belong to the preceding element (trailing) and the rest to the following one (leading); comments before the first element lead it; comments after the last element trail it. Each comment keeps its text, its source column, and whether a line break precedes it. The scan reads only the gap bytes between tokens, which the lexer guarantees are whitespace, commas and comments, and judges ASCII bytes only. A field's type text now ends at its last token instead of the stopping `,`/`)`, so a trailing comment is no longer part of the type; comments between the type's tokens stay in the text as before.

### Decision 2: Place comments in the union, and repeat only JSDoc on the constructors

- **Context**: A case becomes both a union member and a constructor property; a field becomes both a member property and a constructor parameter. TypeScript shows JSDoc from the declaration a symbol resolves to: `Shape.Circle` resolves to the constructor property, a narrowed `s.width` to the member's property signature, and signature help for `Shape.Rect(` to the parameter (a `ParameterDeclaration` is a JSDoc container).
- **Alternatives considered**:
  - Emit every comment in both places. Duplicates ordinary remarks, which are source annotations rather than documentation, and clutters the output.
  - Emit JSDoc only before the union member. TypeScript does not attach JSDoc to a type literal in a union, so quick info on the constructor would still show nothing.
- **Decision and rationale**: The union is the declaration's primary rendering and receives every comment at its position: case comments before (leading) or after (trailing, after the `;` for the last member) the `| { ... }` line, field comments around the member property. A member whose fields carry comments is laid out one property per line so a `//` comment cannot swallow code; members without comments keep the one-line form, so existing output is unchanged. JSDoc comments (`/** ... */`) are additionally written on the case's constructor property and on the field's constructor parameter; a parameter list with documented fields is laid out one parameter per line. Multi-line comments are re-indented by stripping the source column's indentation from continuation lines, so `--no-banner` output stays aligned. Verified with the pinned TypeScript language server through the engine: hover on `Shape.Circle` and on a narrowed `s.width`, and signature help in `Shape.Rect(`, include the written documentation.

## Work log

- 2026-09-28: Reproduced with `ttc -p --no-banner` on a variant with case JSDoc, field JSDoc, trailing and own-line comments: all were dropped. Found that `Rect(w: number, h: number // height\n)` emitted `h: number // height }` and `(w: number, h: number // height): Shape`, invalid TypeScript.
- 2026-09-28: Added `Comments`/`Comment` to `VariantCase` and `Field` (`src/ast.rs`); `gap_comments` and `attach_comments` in `src/parser/variants.rs`, and ended a field's type text at its last token; carried the trivia on `VariantData`/`FieldData` and `AdtVariant`/`AdtField`; rewrote the union and constructor emission in `emit_adt` with `push_comment` and `push_trailing_comments`.
- 2026-09-28: Tests: `a_comment_after_the_last_field_or_case_stays_a_comment`, `comments_inside_a_field_type_stay_in_the_type`, `only_doc_comments_are_repeated_on_constructors`, `variant_comments_keep_crlf_line_endings_valid` (`tests/compile/cases_07.rs`); `variant_case_and_field_docs_reach_hover_and_signature_help` (`tests/native/cases_03.rs`); new fixture `tests/fixtures/emit/variant-comments/` (plain and ambient variants) generated with `UPDATE_EXPECT=1 cargo test --test snapshot` and reviewed line by line: no trailing whitespace, JSDoc continuation lines aligned, the last member's `;` before its trailing comment. Existing emit fixtures are unchanged.
- 2026-09-28: Documented the placement in `docs/ai/tt.md` §variant.

## Issues and resolutions

### Issue 1: A trailing line comment after the last field broke the output

- **Symptom**: `h: number // height` followed by `)` emitted `| { kind: "Rect"; w: number; h: number // height }`.
- **Cause**: The type text ran from the `:` to the stop byte, which is the closing `)` when there is no trailing comma, so it included the comment.
- **Resolution**: The type text ends at the type's last token; the comment is the field's trailing comment and is emitted on the field's own line.

### Issue 2: The engine's hover glues documentation onto the signature

- **Symptom**: The new native test's hover answer had `signature: "(property) Circle: (radius: number) => ShapeA circle around the origin."` and an empty `documentation`.
- **Cause**: The engine requests hover with `contentFormat: ["plaintext", "markdown"]`, and tsgo's plaintext answer concatenates the signature and the JSDoc with no separator, which `split_hover` cannot split. This is independent of variant emission; the documentation does reach the answer.
- **Resolution**: The test checks the complete hover answer. Fixed by TASK-479, which requests Markdown hover and splits at its code fence; the test now asserts `signature` and `documentation` separately.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/ast.rs`, `src/parser/variants.rs`, `src/hir/mod.rs`, `src/hir/lower.rs`, `src/core_ir/mod.rs`, `src/core_ir/lower.rs`, `src/codegen/core/emitter/helpers.rs`, `tests/compile/cases_07.rs`, `tests/native/cases_03.rs`, `tests/fixtures/emit/variant-comments/`, `docs/ai/tt.md`, this record, and `docs/tasks/INDEX.md`. Every comment in a variant body reaches the emitted TypeScript at its case or field, and case and field JSDoc reaches hover and signature help.
