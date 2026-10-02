# TASK-547: Bind a function or class expression's name only inside itself

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A named function or class expression switched off `val` checks for the rest
of its scope in plain `ttc` and `ttc --check`:
`val const s = { a: 1 }; const g = function s() {}; s.a = 2;` reported
nothing, and so did `function f(val p) { const cb = function p() {}; p.a = 1; }`.
`ttc --check-types`, which pairs mutations with bindings by symbol identity,
reported both. ECMA-262 binds a function expression's name in a scope of its
own around the function (§15.2.5, `InstantiateOrdinaryFunctionExpression`)
and a class expression's name in the class scope (§15.7.15), so neither
declares anything in the scope the expression sits in.

## Scope

- Included: the token fact that tells a `function` or `class` declaration
  from an expression, and the `val` checker's scope model.
- Excluded: the typed path, which was already correct.

## Decisions

### Decision 1: The lexer's facts machine records declarations

- **Context**: `Checker::visit_ident` (`src/val/checker.rs`) declared the
  name after any `function` or `class` keyword in the current scope. Hoisting
  (`Checker::instantiate`) guessed a declaration from the token before the
  keyword (`;`, `}` or the start of the list), which also missed a
  declaration after an automatic semicolon.
- **Alternatives considered**: Looking back over modifiers and decorators in
  the checker repeats, with gaps (`@dec(x) class`, `export default`), the
  question the facts machine already answers when it pushes a declaration
  frame instead of an expression frame.
- **Decision and rationale**: `TokenFacts::declaration` marks the
  `function`/`class` keyword the machine reads as a statement
  (`src/lexer/facts.rs`, `src/lexer/facts/statements.rs`). The checker uses
  it for both the declaration at the keyword and hoisting, and the
  look-back helper is removed.

### Decision 2: An expression's name is an ordinary binding of its own scope

- **Context**: Inside the expression, the name does shadow an outer `val`
  binding (`const g = function s() { s.a = 2; }` mutates the function).
- **Decision and rationale**: The checker pushes a frame holding the name as
  an ordinary binding, from the keyword to the end of the function or class
  body (`Checker::own_scope_end`). A function's parameters are pushed after
  it, so a parameter of the same name still shadows the name, as in
  ECMA-262.

## Work log

- 2026-09-29: Reproduced both reported shapes on `c27ad13` with
  `compile` (no error) against `--check-types` (`val-mutation`).
- 2026-09-29: Added the `DECLARATION` fact and changed `src/val/checker.rs`.
- 2026-09-29: Added `val_binds_a_function_or_class_expression_name_only_inside_itself`
  (`tests/compile/cases_07.rs`): function, generator, async function, and
  class expressions, a class expression with heritage and type arguments, a
  parameter named like the expression, the name used inside the
  expression, a class declaration, and a function declaration hoisted after
  an automatic semicolon.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] The new test fails without the change (the expression cases compile
  without an error).

## Result

Changed `src/lexer/facts.rs`, `src/lexer/facts/statements.rs`,
`src/val/checker.rs`, `tests/compile/cases_07.rs`, `docs/tasks/INDEX.md`,
and this record.
