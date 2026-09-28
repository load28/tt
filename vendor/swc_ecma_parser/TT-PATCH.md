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

`tests/jsx_entities.rs` and `tests/swc_arrow_asi.rs` in the parent repository
test the dependency directly.
The direct path dependency also applies when ttc is built by the standalone
fuzz workspace. Remove this vendored copy only after an upstream version
passes these regressions without the patch. This copy retains upstream source,
including upstream unsafe blocks; the patch introduces no unsafe code.
