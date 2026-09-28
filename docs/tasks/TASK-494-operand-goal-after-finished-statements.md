# TASK-494: Expect an operand after a statement the grammar has completed

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

TASK-491 made the token facts machine decide whether a `/` starts a regular expression and a `<` a JSX element. After a finished nested statement or a declaration's `}`, the machine answered "no operand", so the lexer read the `/` as division and the `<` as less-than. The code after it was then misread and silently miscompiled: `export function f() {}⏎/ val const q = 2 /.test("")` erased the `val` inside the regular expression, `.ttx` `export function F() {}⏎<b> val const q = 2 </b>;` erased the `val` in JSX text, and `if (!s) return;⏎/\`/.test(s) && s; return match (x) {…}` failed verification.

## Scope

- Included: The machine's operand decision (`src/lexer/facts.rs`) for every statement form; the `[Yield]` grammar parameter, so `yield` at a line end in a generator is a complete `YieldExpression`; an SWC oracle for regular-expression and JSX classification; regression tests.
- Excluded: The `await` operand heuristic (not a statement completion question) and nested template-interpolation regions, which start a fresh machine and do not know an enclosing generator.

## Decisions

### Decision 1: The machine's own transitions decide the operand goal

- **Context**: `Machine::operand_expected` looked only at the top frame. A statement or declaration frame that the grammar has finished (`Stmt::Done`, `DeclState::Done`, an `if` after its consequent, a `try` after a block, a label after its body, a type after an annotation, a `break` after its label, an `import` after its source) stays on the stack until the next token arrives and makes it hand the token down. Such a frame answered "no operand".
- **Alternatives considered**: (a) Add the missing frames to the top-frame table. Some frames complete only depending on the next token and its line break (automatic semicolon insertion, §12.10.1; a type's same-line `<`), so a table either duplicates each frame's transition or is wrong. (b) Pop finished frames eagerly after each token. That covers only frames whose completion does not depend on the next token (`else`, `catch`, `finally`, ASI), so a second mechanism would still be needed, and it changes the `ends_expression` fact of a function expression's `}`. (c) Ask the grammar: offer the byte, as a punctuator with its line break, to a copy of the stack, and expect an operand when an expression waiting for one receives it.
- **Decision and rationale**: (c). It is the definition of ECMA-262 §12's goal symbol: `InputElementRegExp` applies where the syntactic grammar permits an operand. Frames the grammar has completed hand the probe down through the same `step` functions that hand the real token down, so the decision cannot drift from the transitions, and every statement form is covered at once, including ones completed only by ASI before the byte (`let c: number⏎/re/`, `type T = number⏎`, `declare const b: number⏎`, `break x⏎`, `debugger⏎`, `import "a"⏎`) and the do-while rule (`do {} while (0) /re/`). A function or class expression's frame hands the byte to its expression, which is past an operand, so `const f = function () {}⏎/ 2` stays division. The per-frame `operand_expected` methods of `Stmt`, `SwitchBody`, and `MatchBody`, and the special cases TASK-491 Issue 1 added for a `match` arm body and `export default`, are deleted; those positions reach an expression through their transitions. The probe copies the stack only for a `/`, a `<` in TSX, and a `.` before a digit.

### Decision 2: Statement lists carry the `[Yield]` parameter

- **Context**: `yield` at the end of a line in a generator is a complete `YieldExpression` (`yield [no LineTerminator here] AssignmentExpression`), which no binary operator continues, so a `/` on the next line starts a new statement. The machine read `yield` there as an identifier, which a `/` continues as division.
- **Alternatives considered**: Keep guessing from the next token. The next token cannot tell a generator's `yield` from an identifier named `yield`.
- **Decision and rationale**: Each `Frame::List` records `Yield::{Inherited, Identifier, Operator}`: a file, an ordinary function's, arrow's, or constructor's body, a namespace body, and a class static block set `Identifier`; a generator body sets `Operator`; blocks inherit (§15.5). `open_function_body(kind)` records the brace facts and sets the parameter together; `open_block` and `open_body` open the other lists. `yield` is an operator only where the nearest deciding list is a generator's body; there, with no operand on its line, it ends like an arrow with a block body (`After::Closed`), so only a `,` or a closer continues it. Elsewhere `yield` is an identifier.

### Decision 3: The oracle also compares regular-expression and JSX positions

- **Context**: The statement-span oracle did not see the class of bug: most misreadings left statement spans equal.
- **Decision and rationale**: `lexer::trace` now returns a `Trace` with the statement spans, the start of every regular expression the lexer read, and the start of every JSX element it read. The SWC side collects `Regex` literals and `JSXElement`/`JSXFragment` expressions. Equal sets mean every other `/` is division and every other `<` an operator or a type bracket, as SWC reads them. The known shapes, the corpus, and the new `an_operand_begins_after_a_finished_statement` test all compare the full reading.

## Work log

- 2026-09-28: Reproduced the three reported inputs with the branch head (`f794466`) and a battery of 48 statement forms, each followed on the next line by `/ val const q = 2 /.test("")` (`.tt`) and `<b> val const q = 2 </b>` (`.ttx`): 92 of 96 cases erased the `val` or failed to verify; with the fix, all pass (the one intended division case, a function expression followed by `/`, is division as ECMA-262 reads it).
- 2026-09-28: Replaced `operand_expected` with the probe (`src/lexer/facts.rs`), passed the byte position and line break from the lexer (`src/lexer.rs`), added `Yield` to `Frame::List` and `open_body`/`open_block`/`open_function_body(kind)` (`src/lexer/facts/expressions.rs`, `statements.rs`), and moved `yield` onto the parameter.
- 2026-09-28: Extended the trace and the oracle (`src/lexer.rs`, `src/lexer/facts/tests.rs`); added `an_operand_begins_after_a_finished_statement`, `tests/compile/cases_13.rs`, and `a_regex_or_element_after_a_finished_statement_passes_through` in `tests/passthrough.rs`. Updated `docs/design/compiler-architecture.md` and noted the superseded Issue 1 resolution at the top of the TASK-491 record.
- 2026-09-28: Ran the oracle on the wider corpus of TASK-491: `TTC_FACTS_CORPUS=/opt/node22/lib/node_modules:/opt/node22/lib/node_modules/npm/node_modules:/opt/node22/lib/node_modules/eslint/node_modules:/opt/node22/lib/node_modules/ts-node/node_modules cargo test --release --lib the_machine_reads_the_corpus` — statement spans and regular-expression and JSX positions agree on all 2931 of 3013 files SWC parses.

## Issues and resolutions

### Issue 1: SWC rejects a regular expression after an arrow function with a block body

- **Symptom**: The oracle case `const g = () => {}⏎/ a /.test("")` failed with "SWC rejects the case".
- **Cause**: SWC reads the `/` as a division continuing the arrow function and reports an error; TypeScript ends the declaration by ASI and reads a regular expression, which is what the machine does.
- **Resolution**: The case was removed from the oracle test, which only holds cases SWC parses. The machine keeps TypeScript's reading.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 40 suites, 1498 tests, 0 failures.
- [x] `./scripts/ci extension`: exit 0; 209 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: exit 0.
- [x] The oracle: all known shapes, the finished-statement shapes, the TSX shapes, and the default and wider corpora agree with SWC on statement spans and on regular-expression and JSX positions.

## Result

Changed `src/lexer.rs`, `src/lexer/facts.rs`, `src/lexer/facts/{expressions,statements,tests}.rs`, `tests/compile.rs`, `tests/passthrough.rs`, `docs/design/compiler-architecture.md`, the TASK-491 record, and `docs/tasks/INDEX.md`; added `tests/compile/cases_13.rs` and this record. The machine's operand goal is decided by its own transitions, so a `/` or `<` after any statement the grammar has completed begins an operand, and the SWC oracle now checks that classification on every corpus file.
