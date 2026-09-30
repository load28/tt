# SWC parser dependency patch

Source: crates.io `swc_ecma_parser` 45.0.0, upstream commit
`9170cd59ebf735925c00cf0fbe615d1feb181431`, directory `crates/swc_ecma_parser`.
The dependency is Apache-2.0 licensed; see LICENSE.

Local change: `src/lexer/mod.rs`, `read_jsx_entity`'s numeric conversion
returns an optional code point and preserves unrecognized references as text.
Empty digits, arithmetic overflow, and code points above U+10FFFF previously
returned None and were unwrapped. Both hexadecimal and decimal references now
use the same literal representation as unknown named references. TypeScript
accepts these sources; inventing a syntax error would break compatibility.

The entity scanner stops at non-entity characters before
advancing, preserving JSX delimiters and UTF-8 text for the enclosing scanner.

Local change: `src/parser/expr.rs`, `parse_assignment_expr` grows the stack
through `maybe_grow` the way `parse_stmt` already does, so expression nesting
(parentheses, arguments, array and object elements) is bounded by memory
rather than by the calling thread's stack. `tests/cli.rs` in the parent
repository covers deeply nested input end to end.

Local change: `src/parser/expr.rs`, `parse_paren_expr_or_arrow_fn` continues
a block-bodied arrow function with a binary operator (reporting TS1005, as
TypeScript does for `() => {} / 2`) only when no line break precedes the
operator. Upstream made the exception only for Flow and `<`. An
`ArrowFunction` is an `AssignmentExpression`, never the left operand of a
binary operator (ECMA-262 §15.3), so after a line break the operator is the
offending token of §12.10.1 and an automatic semicolon ends the statement:
`() => {}⏎/x/g.exec("x")` is two statements, the second starting with a
regular expression, and `() => {}⏎+1` likewise. TypeScript's parser accepts
both (`canParseSemicolon` after the arrow function). Upstream rejected them
with "Expected a semicolon"; the ident-parameter form (`x => {}`) already
returned before the operator. acorn had the same defect
(acornjs/acorn#475). `tests/swc_arrow_asi.rs` and `tests/passthrough.rs` in
the parent repository cover it (TASK-497).

Local change: `src/parser/typescript.rs`, `parse_ts_import_type` reads type
arguments only when no line break precedes the `<` (or `<<`), as
`parse_ts_type_ref` and `parse_ts_type_query` already did. TypeScript's
`parseImportType` reads them through `parseTypeArgumentsOfTypeReference`,
which requires `!scanner.hasPrecedingLineBreak()` and rescans `<<` as `<`.
Upstream read `let x: typeof import("x")⏎<any>y` as `import("x")<any>`
followed by `y` ("Expected a semicolon"), and in TSX read the `<b>` of a JSX
element on the next line as a type argument list; TypeScript ends the
declaration at the line break. No upstream SWC issue for this shape was
found. `tests/swc_import_type_arguments.rs` and `tests/passthrough.rs` in
the parent repository cover it (TASK-502).

Local change: `src/parser/stmt.rs`, `parse_for_head` reads a `using` or
`await using` declaration as a `for` statement's initializer
(`for (using r = open(); ; )`), which ECMAScript explicit resource management
(ES2026, `ForStatement : for ( [lookahead ≠ let [] LexicalDeclaration
Expression ; Expression ) Statement` with `LexicalDeclaration : UsingDeclaration
| AwaitUsingDeclaration`) and TypeScript 5.2 accept. Upstream read only the
`for (using x of xs)` form and rejected the initializer ("Expected ';'");
upstream's own TypeScript conformance run still excludes
`awaitUsingDeclarationsInFor.ts`, and its AST had no initializer that holds a
using declaration. The declaration is `VarDeclOrExpr::UsingDecl`, a variant the
vendored `swc_ecma_ast` adds (`vendor/swc_ecma_ast/TT-PATCH.md`). `using`
stays an identifier where no binding identifier follows it on the same line
(`for (using; ;)`, `for (using = 1; ;)`, `for (using of xs)`).

Local change: `src/parser/stmt.rs`, `parse_if_clause` reads a plain
`FunctionDeclaration` as an `if` statement's consequent or alternative
(`if (c) function f() {}`), the production of ECMA-262 Annex B.3.4
("FunctionDeclarations in IfStatement Statement Clauses"). TypeScript's parser
reads it as a function declaration in every file; upstream reported
"Declaration is not allowed". A generator is not part of the production and is
still rejected.

Local changes from TASK-641, each a form TypeScript's parser at the pinned
commit (`5739027c`, `tsc/internal/parser`) reads and upstream rejected:

- `src/parser/stmt.rs`, `parse_for_head`: `of` is the binding of an
  `await using` declaration (`for (await using of of xs)`). TypeScript's
  `parseForOrForInOrForOfStatement` excludes `of` only after a plain `using`
  (`nextTokenIsBindingIdentifierOrStartOfDestructuringOnSameLineDisallowOf`),
  as ECMA-262's `[lookahead ≠ using of]` does. Upstream fixed the same in
  swc-project/swc#12354.
- `src/parser/expr.rs`, `parse_args_or_pats_inner`: a first element that is
  an accessibility or `readonly` modifier followed by `as` is an expression,
  not a parameter modifier (`(readonly as number)`), the check TypeScript's
  `isParenthesizedArrowFunctionExpressionWorker` makes for
  microsoft/TypeScript#44466.
- `src/lexer/state.rs`, `scan_jsx_attribute_value`: whitespace, line breaks
  included, is skipped between `=` and a JSX attribute string, so a string
  after a space reads as a JSX string that may span lines, as TypeScript's
  `ScanJsxAttributeValue` does.
- `src/parser/module_item.rs`, `parse_named_export_specifier`:
  `export { type "x" as "y" } from "m"`, the type-only form of an
  arbitrary module namespace name, which the import side already read.
- `src/parser/module_item.rs`, `parse_export`: `export @dec abstract class`,
  decorators after `export` on an abstract class, as on a plain class.
- `src/parser/module_item.rs`, the three import attribute sites: `with`
  may follow a line break; the no-line-break restriction stays on the legacy
  `assert` (TypeScript's `tryParseImportAttributes`; upstream
  swc-project/swc#12356).

`tests/swc_typescript_grammar_gaps.rs` in the parent repository tests them.

`tests/jsx_entities.rs`, `tests/swc_arrow_asi.rs`,
`tests/swc_import_type_arguments.rs`, `tests/swc_for_using_and_if_function.rs`,
and `tests/swc_typescript_grammar_gaps.rs` in the parent repository test the
dependency directly.
The direct path dependency also applies when ttc is built by the standalone
fuzz workspace. Remove this vendored copy only after an upstream version
passes these regressions without the patch. This copy retains upstream source,
including upstream unsafe blocks; the patch introduces no unsafe code.
