# TASK-482: End a statement at a line break after a postfix operator or a restricted production

> Superseded by [TASK-491](./TASK-491-token-facts-statement-boundaries.md): `asi_boundary_at`, `expression_ends_at`, `word_ends_expression`, and `restricted_production_ends_at` are deleted; the lexer records automatic semicolons, restricted productions included, on each token. The excluded follow-up (a type-argument `>` that ends an `as` type) is fixed there: the lexer's type grammar knows where a type ends.

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

An `if let` on the line after `q++`, `q--`, `q!`, `const g = [1] as const`,
a bare `return`, or a bare `yield` was reported as `if-let-placement`, also
inside a `.ttx` arrow body such as `onClick={() => { … }}`. JavaScript reads
each of those lines as a complete statement: `if` cannot continue an
expression, so a semicolon is inserted before it, and `return`/`yield` are
restricted productions whose operand may not follow a line terminator
(ECMA-262 §12.10.1). The shared automatic-semicolon model did not recognize
any of these tokens as the end of a statement.

## Scope

- Included: The shared predicate `crate::flow::asi_boundary_at` and its
  helper `line_break_after_expression` in `src/flow/syntax.rs`, which every
  consumer uses: the parser's expression-head tracking and `if let`
  position fact (`src/parser/parse.rs`), pipeline step scans
  (`src/parser/pipes.rs`), concise-arrow ends and flow statement splitting
  (`src/flow/`), and the program-syntax projection's boundary semicolon
  (`src/program_syntax/projection.rs`).
- Excluded: A type-argument `>` that ends an `as` type
  (`o as Array<number>` followed by a line break). Telling that `>` from a
  relational operator needs a type-position model the token scan does not
  have; it is not one of the reported shapes and is left as a follow-up.

## Decisions

### Decision 1: Recognize postfix operators by their operand, not by the operator token alone

- **Context**: `token_ends_expression` judges one token. `++`, `--`, and
  `!` lex as single-byte punctuation, and each is either a prefix operator
  (which owes an operand) or a postfix operator (which ends an expression).
  ECMA-262 §13.4 makes an update operator postfix only when no line
  terminator separates it from its operand, and TypeScript applies the same
  rule to the non-null assertion `!`.
- **Alternatives considered**: (a) Treat every `++`/`--`/`!` before a line
  break as an expression end. `x = !` followed by `y` on the next line
  would then end the statement early. (b) Decide the operator from its
  operand: the operator is postfix exactly when the token before it ends an
  expression on the same line.
- **Decision and rationale**: (b). `expression_ends_at` walks back through
  a chain of postfix operators (`p.a!!`, `a!++`) iteratively, so a long run
  of operators cannot deepen the stack, and ends at the operand. An
  adjacent `++`/`--` pair is one operator, matching how the existing
  continuation rule already reads a line-leading `++`.

### Decision 2: A word after `.`/`?.` is a property name, and `as const` ends an assertion

- **Context**: `const` is in `NON_VALUE_WORDS`, so `[1] as const` looked
  unfinished. A reserved word after a member dot (`{ return: 1 }.return`)
  is an IdentifierName property (ECMA-262 §13.3), not a keyword.
- **Decision and rationale**: `word_ends_expression` returns true for any
  word that follows `?.` or a `.` that is not the last dot of a spread
  `...`, and for `const` directly after `as`. The existing rule that a type
  operator word (`as`, `satisfies`, …) before a line break still owes its
  operand is kept.

### Decision 3: `return` and `yield` end the statement at a line break regardless of the next token

- **Context**: ECMA-262 §12.10.1 lists `return [no LineTerminator here]
  Expression` and `yield [no LineTerminator here] AssignmentExpression`, so
  `return` followed by `(x)` on the next line is two statements, even
  though `(` would otherwise continue an expression.
- **Alternatives considered**: Adding `return` and `yield` to the
  expression-ending words would make `return` followed by `(x)` a call.
- **Decision and rationale**: A separate `restricted_production_ends_at`
  counts as a line-break boundary before any next token. `yield` is treated
  as the keyword everywhere, as `NON_VALUE_WORDS` already does, because tt
  emits module (strict-mode) code where `yield` is reserved. A word after a
  member dot is excluded. `throw` is not included: a line break after
  `throw` is a syntax error, not an inserted semicolon. `break` and
  `continue` already end an expression as identifiers.

## Work log

- 2026-09-28: Reproduced every reported shape with `ttc --check`; `q--`
  failed as well, contrary to the report. `break`/`continue` already
  compiled.
- 2026-09-28: Rewrote `line_break_after_expression` over
  `expression_ends_at` and `restricted_production_ends_at`, and made
  `asi_boundary_at` skip the continuation test after a restricted
  production. The line-break test runs first, so the operand walk only runs
  at line breaks.
- 2026-09-28: Added regression tests in `tests/compile/cases_11.rs`: an
  `if let` after each boundary (including the `.ttx` arrow), pipeline heads
  after `q++`, `p!`, `as const`, and `return`, and the continuing forms
  `q++` + `+ …` and `p!` + `(…)` that must still report placement. The
  first two tests fail on the previous predicate.

## Issues and resolutions

### Issue 1: A reserved word used as a property name still read as a keyword

- **Symptom**: After the first change, an `if let` after
  `const k = { return: 1 }.return` was still reported as
  `if-let-placement`.
- **Cause**: `token_ends_expression` rejects every word in
  `NON_VALUE_WORDS` without looking at the preceding member dot.
- **Resolution**: Decision 2; the property-name case is part of the test.

### Issue 2: A byte-order mark panicked the line-break test

- **Symptom**: `the_banner_never_displaces_a_shebang_or_a_byte_order_mark`
  failed with an internal compiler error: `start byte index 1 is not a char
  boundary; it is inside '\u{feff}'`.
- **Cause**: Running the line-break test first (so the operand walk only
  runs at line breaks) reached `line_break_before_tokens` for the token
  after a byte-order mark, whose lexed span ends inside the mark. The
  function sliced the `str` between the two tokens; the old predicate only
  got there after the mark had already failed `token_ends_expression`.
- **Resolution**: `line_break_before_tokens` searches the bytes between the
  tokens for `\n`, as the repository's rule for scanners requires (ASCII
  bytes decide, multi-byte UTF-8 is opaque).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed; the TASK-391,
  TASK-451, and TASK-480 tests are unchanged and green. No snapshot changed.

## Result

Changed `src/flow/syntax.rs` and `tests/compile/cases_11.rs`. Every
reported shape compiles and each `if let` stays a statement.
