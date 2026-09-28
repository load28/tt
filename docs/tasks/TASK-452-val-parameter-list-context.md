# TASK-452: Claim a `val` parameter modifier only inside a proven parameter list

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A file with no tt syntax that indexes a variable named `val` with a space before the bracket lost the variable name: `console.log(val [0]);` compiled to `console.log([0]);`, and `[1, val [0]]` and `g(1, val [1])` lost it the same way. That breaks the passthrough contract (every valid TypeScript file must pass through byte-for-byte).

## Scope

- Included: The parameter shape of `val::modifier_at` (`src/val.rs`), a parameter-list classifier in the flow syntax model (`src/flow/syntax.rs`), and the `val` syntax rule in `docs/ai/tt.md`.
- Excluded: The declaration shape (`val const|let|var`), which is unaffected, and the `val` semantic checks.

## Decisions

### Decision 1: Require the `(` that encloses the entry to open a formal parameter list

- **Context**: `modifier_at` accepted `val` followed by a binding (an identifier, `{`, `[`, or `...`) whenever it was preceded by `(` or `,`, optionally after parameter-property modifiers. The module's contract argument was that the shape cannot occur in valid TypeScript. That holds for an identifier, `{`, and `...`, but not for `[`: ECMA-262 §13.3 (`MemberExpression [ Expression ]`) allows whitespace between an object and its computed member, so `val [0]` is an element access and a legal first argument or array element. The `,` case did not even check which bracket the comma belonged to, so array literals were claimed too.
- **Alternatives considered**:
  - Refuse `val [` unless more tokens match a destructuring pattern (for example a `]` followed by `:` or `,`). This is another token-shape guess: `f(val [0], x)` and `([a, b]: T) => …` look the same at that depth, and it would break `val [a, b]` parameters that TypeScript allows without an annotation.
  - Parse the host with SWC to find parameter spans. The parser decides `val` in its main token loop, before any TypeScript projection exists, and the file may contain tt constructs SWC cannot parse; the flow syntax model already classifies function heads on the same token stream the parser uses.
- **Decision and rationale**: `modifier_at` now finds the bracket that encloses the `(` or `,` and claims `val` only when it is a `(` that `flow::opens_parameter_list` proves is a formal parameter list. The classifier follows where ECMA-262 and TypeScript put parameter lists:
  - `catch (` — `CatchParameter` (ECMA-262 §14.15).
  - `function [*] [name] [<T>] (` — `FunctionDeclaration`/`FunctionExpression`/generators (§15.2, §15.5), which also covers TypeScript's bodyless overload signatures.
  - An arrow head: the matching `)`, optionally followed by a return-type annotation, followed by `=>` with no line terminator in between (`ArrowParameters [no LineTerminator here] =>`, §15.3). A `(` that follows a token ending an expression is `Arguments` (§13.3), never an arrow head, so `c ? f(val [0]) : w => w` keeps `f(...)` a call.
  - A method, accessor, or constructor head (§15.4, §15.7): the matching `)`, optionally followed by a return-type annotation, followed by the `{` that the existing `function_body_brace` model classifies as a function body (not a control head, not a heritage call). A `{` directly after `:`, `|`, `&`, `<`, `,`, `(`, `[`, or `=>` begins an object type or literal operand, not a body, so `c ? f(val [0]) : { a: 1 }` stays a call.
  The return-type walk that `function_body_brace` already did is extracted as `annotated_paren` and shared, so the body classification and the parameter-list classification read an annotation the same way.

### Decision 2: Bodyless method signatures are not parameter lists for `val`

- **Context**: An interface or abstract method signature (`m(val x: T): void;`) has neither a body nor a `function` keyword, so the classifier cannot prove its list from the tokens around it.
- **Alternatives considered**: Classify every `name(` inside a class, interface, or type-literal body. That needs the enclosing brace's kind, which the token model does not carry, and it would add the ambiguity back for object literals whose members are call expressions.
- **Decision and rationale**: Leave them unclaimed. A `val` on a signature never had an effect: the mutation check needs a body, and the call check only consults same-file named functions, whose overloads the `function` rule still covers. `docs/ai/tt.md` states the rule.

## Work log

- 2026-09-28: Reproduced with `ttc -p` on the three-line file; the output dropped `val` from all three element accesses.
- 2026-09-28: Traced the claim to `val::modifier_at`: the parameter branch returned `Parameter` for any `(`/`,` before `val` followed by `[`.
- 2026-09-28: Added `opens_parameter_list`, `annotated_paren`, `operand_expected_before`, and `follows_function_keyword` to `src/flow/syntax.rs`; `function_body_brace` now uses `annotated_paren`. `modifier_at` checks the enclosing `(` through `enclosing_open`. Updated the contract wording in `src/val.rs` and `src/parser/parse.rs`, and the syntax rule in `docs/ai/tt.md`.
- 2026-09-28: Added `val_element_access_in_arguments_and_elements` to `tests/passthrough.rs` (call arguments, array elements, `new` arguments, a parenthesized expression, both ternary shapes, a call in a parameter default, control heads) and `val_array_pattern_parameter_is_erased_in_every_parameter_list` to `tests/compile/cases_07.rs` (function, arrow, async annotated arrow, generic generator, methods with a return type and with an Allman body, an object-literal method, `catch`, overloads).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] The passthrough repro failed with the previous binary (`console.log([0]);`) and passes now.

## Result

Changed `src/flow/syntax.rs`, `src/flow/mod.rs`, `src/val.rs`, `src/parser/parse.rs`, `docs/ai/tt.md`, `tests/passthrough.rs`, `tests/compile/cases_07.rs`, `docs/tasks/TASK-452-val-parameter-list-context.md`, and `docs/tasks/INDEX.md`.

Known limits, both left to the shared flow model rather than special-cased here: a parenthesized consequent of a conditional whose alternative is an arrow (`c ? (val [0]) : w => w`) reads as an annotated arrow head; a call-shaped `f(val [0])` followed by a bare block on the next line is read as an Allman method head, the same way `function_body_brace` reads it.
