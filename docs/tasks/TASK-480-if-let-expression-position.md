# TASK-480: Report an `if let` in any expression position as `if-let-placement` alone

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`const x = if let Some(v) = o { v };` inside a function reported
`lowering-plan-failed` instead of the documented `if-let-placement`, and an
`if let` in a template interpolation, scrutinee, or expression arm reported
both. `if let` is a statement; wherever it stands in place of a value, the
located `if-let-placement` must be the only diagnostic.

## Scope

- Included: The parser's position fact for `if let`
  (`src/ast.rs`, `src/parser/parse.rs`, `src/parser/iflets.rs`), the
  pipeline step and `try` operand scans that delimit an operand
  (`src/parser/pipes.rs`, `src/parser/tries.rs`), the sema check
  (`src/sema/checker.rs`), the diagnostic's projection class and
  explanation (`src/diagnostics.rs`), and `docs/ai/tt.md`.
- Excluded: Giving `if let` a value form. It stays a statement; `match` is
  the value construct.

## Decisions

### Decision 1: The parser records whether the `if` starts a statement

- **Context**: The parser claims `if let` at any token, because `if let`
  is never TypeScript. Sema judged placement only by the kind of program
  the statement sat in (`Ctx::Expr` without a user function), so an
  initializer, argument, operand, `return` operand, or concise arrow body
  in an ordinary statement stream passed sema. The analysis projection then
  wrote the statement's block where a value stands and SWC rejected it,
  which surfaced as `lowering-plan-failed`. A concise arrow body even
  compiled, silently turning the arrow into a block body that returns
  `undefined`.
- **Alternatives considered**: (a) Judge the position in sema from the
  surrounding text. Sema has no statement-boundary model; the parser
  already owns one. (b) Project a misplaced `if let` as an immediately
  called function so SWC parses it. That invents a value form the language
  does not have, and emission would then write a block where a value
  stands. (c) Record the position in the parser with the same rule that
  separates a `try` expression from a `try` statement.
- **Decision and rationale**: (c). `IfLetStmt::expression_position` is set
  when the `if` is the root of an expression region, does not start a
  statement (`starts_statement`, with an automatic-semicolon boundary
  counting as a start), sits in a `for` update, or follows an object
  member's `:`. Sema reports `if-let-placement` for it, in addition to the
  existing expression-region rule.

### Decision 2: A misplaced `if let` has no TypeScript projection

- **Context**: The file still has to reach the typed pass and editors
  without a cascade from the projection.
- **Decision and rationale**: `IfLetPlacement` joins `blocks_projection`,
  the class of diagnostics whose construct has no TypeScript form (as
  `stray-if-let` already is). The parser records a misplaced `if let` as an
  expression recovery node, so the editor projection replaces the whole
  construct with a value placeholder and keeps type information for the
  rest of the file. This does not filter a cascade after the fact: the
  lowering is not attempted because the construct has nothing to lower to.

### Decision 3: Operand scans keep an `if let` whole

- **Context**: `o |> if let ...` and `if let ... { } |> f` reported
  `stray-pipe` as well, and `try if let ...` reported only
  `source-not-typescript`, because the `try` scan took the keyword `if` as
  an identifier operand and left `let ...` behind.
- **Decision and rationale**: A pipeline step that starts with a complete
  `if let` spans the construct (`iflets::if_let_end`, like the `match` and
  `result` skips); an expression-position `if let` does not reset the
  pipeline head start, so the head includes it; and a `try` operand cannot
  start with a statement-only keyword. Each sub-program then finds the
  `if let` at the root of an expression region.

## Work log

- 2026-09-28: Reproduced `lowering-plan-failed` for initializer, argument,
  `return`/`throw` operand, array/object element, binary operand, and
  method-body cases; both diagnostics for template, scrutinee, and
  expression arm; a silent compile for a concise arrow body; `stray-pipe`
  for unparenthesized pipeline heads and steps; and `source-not-typescript`
  for `try if let`.
- 2026-09-28: Added the parser position fact, the sema condition, the
  projection class, the recovery node, the operand scans, the explanation,
  and the language reference line.
- 2026-09-28: Added compile tests for every position above, for statement
  positions that must keep compiling (after an automatic semicolon, after
  a concise arrow, as an unbraced body, labeled, in a function in a
  template, in a block arm), for the `try` operand rule, and for the
  explanation; added a content-mapper case for the recovered projection.

## Issues and resolutions

### Issue 1: A statement after an automatic semicolon read as an operand

- **Symptom**: `const x = 1` followed by `if let ...` on the next line
  reported `if-let-placement`.
- **Cause**: `starts_statement` looks only at the previous token.
- **Resolution**: An automatic-semicolon boundary before the `if`
  (`flow::asi_boundary_at`) also starts a statement.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `ttc explain if-let-placement` names the value positions.

## Result

Changed `src/ast.rs`, `src/parser/parse.rs`, `src/parser/iflets.rs`,
`src/parser/pipes.rs`, `src/parser/tries.rs`, `src/sema/checker.rs`,
`src/diagnostics.rs`, `docs/ai/tt.md`, `tests/compile/cases_11.rs`, and
`tests/content_mapper.rs`. An `if let` in any expression position reports
exactly one located `if-let-placement`.
