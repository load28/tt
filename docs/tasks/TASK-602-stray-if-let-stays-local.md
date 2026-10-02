# TASK-602: Recover an unfinished `if let` only as far as TypeScript reads its statement

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-602: Recover an unfinished if let only as far as TypeScript reads its statement`

## Purpose

An `if let` typed before its `{` (`if let Some(value: w) = find(id)` with
more statements after it, or just `if let Some(`) broke IntelliSense for
the rest of the file: completion in a later function answered nothing,
hover, definition and signature help answered `null`, the outline stopped
at the `if let`, and the service reported `'}' expected` at the end of the
file and a false unused parameter. The same unfinished `if (find(id.))` in
a `.ts` file costs TypeScript one local syntax error, and its condition
keeps member completion.

## Scope

- Included: The recovery nodes the parser records for an `if let` that did
  not parse (`iflets::stray_if_let_recoveries`), a recovery placeholder that
  keeps an operand (`RecoveryKind::OperandHead`), and its projection in the
  recovered source and the host-grammar masks.
- Excluded: Where `stray-if-let` is reported and what it says (TASK-599
  changes where an `else if` chain reports it; this task keeps every
  diagnostic as it was), and how a parsed `if let` lowers.

## Decisions

### Decision 1: The recovery ends where TypeScript ends the `if` statement

- **Context**: `recovery_statement_span` ran from `if` to the closer of
  the first `{` anywhere after it — for an `if let` with no block, the body
  of the next function — or to the end of the region. The faithful
  projection (TASK-527, TASK-561) replaced all of it with one `;`, so the
  rest of the file, and the enclosing function's `}`, were gone from the
  served text.
- **Alternatives considered**: (a) Stop at the first line break: a head
  written over several lines (`if let Some(v) =` then the operand) would
  lose its operand, and a head with an open list would still be cut where
  TypeScript is not. (b) Recover to the end of the enclosing block: the
  statements after the `if let` would still be blank, which is what broke
  navigation, the outline, and the unused-variable check.
- **Decision and rationale**: The head is read as TypeScript reads an `if`
  condition and an open argument list (TypeScript `parser.ts`,
  `parseIfStatement`, and `parseDelimitedList`, which abandons a list at a
  token that starts an element of an enclosing context through
  `isInSomeParsingContext`): it ends at a then-block `{`, a `;`, a statement
  boundary (ECMAScript 2025 §12.10, automatic semicolon insertion, as the
  lexer's `boundary_before` fact records it), a closer of a bracket it did
  not open, or a statement keyword directly in a parenthesis or index — the
  same synchronization TASK-528 gave an unterminated `try` operand and a
  pipeline step. A then-block runs to its closer (or to the end of the
  region when it has none, as TypeScript's block does), and an `else`
  continuation (a block or another `if`) belongs to the statement, as
  ECMAScript's `IfStatement` production (§14.6) makes it. The recovery
  covers nothing past that.

### Decision 2: The operand stays the user's expression

- **Context**: With the head `if let Some(v) = find(id.)`, the user is
  typing the operand; TypeScript's twin `if (find(id.))` answers member
  completion and signature help there. A statement placeholder over the
  whole head would erase the operand.
- **Alternatives considered**: (a) `;` over the whole statement (the old
  placeholder): no IntelliSense in the operand. (b) Keep the operand as a
  bare expression statement behind `;`: an operand beginning with
  `function` or `class` would then read as a declaration. (c) `const {} =`
  before it: a declaration is not a statement in every position (`else`,
  a loop body), and destructuring checks the operand's type.
- **Decision and rationale**: When the head reached its `=` and has an
  operand, only `if let <pattern> =` is replaced, by `void` padded with
  spaces (`RecoveryKind::OperandHead`), so the operand is the operand of a
  `void` expression statement (ECMAScript §13.5.2, `UnaryExpression`), with
  every byte still mapped to the source. The then-block and its `else`
  continuation become one empty statement, because the pattern's bindings
  that the block reads are not declared. A head that never reached its `=`
  (`if let Some(`) is one empty statement. The one operand `void` reads
  differently from an `if` condition is an unparenthesized arrow function,
  which an `if let` never matches on; it is recorded here as the limit.

## Work log

- 2026-09-30: Reproduced with the probe harness (`target/probe5-editor`,
  `il3.tt`): every question after the `if let` answered nothing and the
  outline was empty. The projection dump showed the recovery
  `(172, 298)` — from `if` to the end of the next function.
- 2026-09-30: Added `stray_if_let_recoveries`, `if_extent`, and
  `is_binding_eq` (`src/parser/iflets.rs`), called from the parse loop in
  place of `recovery_statement_span` (`src/parser/parse.rs`, removed);
  `RecoveryKind::OperandHead` (`src/ast.rs`), its `void` placeholder in
  `recover_source` (`src/lib/compile.rs`) and in the host masks
  (`src/parser/host.rs`).
- 2026-09-30: Re-ran the harness: later functions complete, hover and
  navigate; the outline lists every declaration; `find(id.|)` in the head
  completes `string` members and shows `find(...)`'s signature; the only
  diagnostics are `stray-if-let` and, while `id.` is unfinished,
  TypeScript's own `Identifier expected.`.
- 2026-09-30: Tests: `a_stray_if_let_recovers_only_the_statement_typescript_reads`
  (`tests/compile/cases_05.rs`),
  `an_unfinished_if_let_leaves_the_rest_of_the_file_served` and
  `the_operand_of_an_unfinished_if_let_is_served`
  (`tests/native/editor_service.rs`). Both engine tests failed before the
  change.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

## Result

Changed `src/parser/iflets.rs`, `src/parser/parse.rs`, `src/parser/host.rs`,
`src/ast.rs`, `src/lib/compile.rs`, `tests/compile/cases_05.rs`,
`tests/native.rs`, `tests/native/editor_service.rs`,
`docs/design/lsp-architecture.md`, and the task index. An unfinished
`if let` costs one local recovery, and its operand is served.
