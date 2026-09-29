# TASK-486: Place the runtime import after the parsed directive prologue

> Updates [TASK-219](./TASK-219-generated-code-readability.md) where
> it placed the runtime import with a byte scan of the directive prologue:
> the prologue now comes from the parsed program.

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The `@tt/runtime` import must follow a directive prologue (`docs/ai/tt.md`),
because an import above `"use client"` demotes the directive to a string
expression. The byte scan that found the prologue got two shapes wrong:

- A directive followed by a comment on its line
  (`"use client" // client component`, `"use client" /* c */;`) was not
  recognized, so the import went above it and demoted it.
- A string that continues on the next line (`"use client"` then
  `.length;`, or `+ 1;`) was taken for a directive, and the import was
  inserted inside the expression, which failed verification
  (`verify-failed`) or split the expression.

## Scope

- Included: The directive-prologue fact in the program syntax model
  (`src/program_syntax/collector.rs`, `src/program_syntax/projection.rs`),
  its transport through the evaluation file and lowering plan
  (`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`), and the
  import position in codegen (`src/codegen/core/planning.rs`,
  `src/codegen/core/mod.rs`).
- Excluded: Where the import goes when there is no directive; that is
  unchanged (after a byte-order mark and a hashbang line).

## Decisions

### Decision 1: The directive prologue comes from the parsed program

- **Context**: ECMA-262 §11.2.1 defines a Directive Prologue as the longest
  sequence of leading ExpressionStatements that each consist entirely of a
  StringLiteral followed by a semicolon, where the semicolon may be
  inserted (§12.10). Whether the string is followed by `.length` or `+ 1`
  on the next line is a question the grammar answers, not the bytes after
  the quote.
- **Alternatives considered**: (a) Extend the byte scan to skip comments
  and look ahead past line breaks. It would have to reproduce automatic
  semicolon insertion, which is the parser's job. (b) Use the tt token
  stream and the shared automatic-semicolon predicate. It is always
  available but still approximates the grammar. (c) Read the SWC module
  that `ProgramSyntax` already parses for every file that needs a runtime
  import.
- **Decision and rationale**: (c). `directive_prologue_end` takes the
  leading `Stmt::Expr` items whose expression is exactly `Lit::Str` (a
  parenthesized string is `Expr::Paren` and is not a directive, as in the
  specification) and maps the last one's end to a source byte. The fact
  travels `ProgramSyntax` → `EvaluationFile` → `LoweringPlan`, the same
  path as the generated-name and shadowed-global facts.

### Decision 2: The statement end is its last copied source byte

- **Context**: When a tt construct follows a semicolon-free directive, the
  projection writes a `;` of zero source width at the construct's start
  (TASK-391), so SWC's statement span covers the line break and that
  synthetic `;`. Mapping the span end produced a position past the line
  break, and the import was written with an extra blank line.
- **Decision and rationale**: If the statement's last projected byte is
  copied source, it is the author's `;` and the statement ends after it;
  otherwise the statement ends at the string literal's end.

### Decision 3: Codegen keeps the directive's line together

- **Context**: A comment written after the directive on its line belongs
  to that line.
- **Decision and rationale**: `module_import_position` starts at the
  directive end and skips spaces, tabs, and comments (a block comment may
  span lines). At a line break the import goes on the next line; at the end
  of the file it goes at the end with a line break; if code follows on the
  same line, the import goes right after the directive statement, preceded
  by a line break. With no directive, the position is after a byte-order
  mark and a hashbang line (ECMA-262 §12.5), as before. An editor mapping
  of a buffer that is not TypeScript has no parsed program and so no
  directive fact; it uses that same position.

## Work log

- 2026-09-28: Reproduced both reports with `ttc -o`: (A) the import above
  `"use client" // …` and `"use client" /* c */;`; (B) `verify-failed` for
  `.length;` and a split `+ 1;`. Also found that `"use client"; code` on
  one line wrote the import after the whole line, inside the code.
- 2026-09-28: Added the prologue fact and the codegen position, and
  replaced the byte scan (`directive_prologue_end`, `skip_trivia`,
  `string_literal_end` in `src/codegen/core/planning.rs`).
- 2026-09-28: Added regression tests in `tests/compile/cases_11.rs` for
  trailing line and block comments, a multi-line block comment, several
  directives with `'b'` on its own line, a directive at the end of the
  file, a directive before a `variant`, and three strings that are not
  directives (`.length`, `+ 1`, parenthesized).

## Issues and resolutions

### Issue 1: An extra blank line before the import after a semicolon-free directive

- **Symptom**: `"use client"` followed by a statement `match` emitted a
  blank line between the directive and the import.
- **Cause**: The mapped end of the SWC statement included the projection's
  zero-width automatic semicolon (Decision 2).
- **Resolution**: Decision 2; the `variant` case in the test covers a
  construct that is projected right after the directive.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed, snapshots
  unchanged; the TASK-219 directive and shebang tests still pass.

## Result

Changed `src/program_syntax/collector.rs`,
`src/program_syntax/projection.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`,
`src/codegen/core/mod.rs`, and `tests/compile/cases_11.rs`.
