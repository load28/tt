# TASK-499: Read a parenthesized arrow return type the way TypeScript does

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

An arrow function whose return type is parenthesized had its body read as a type, a regression from TASK-491. `export const parse = (s: string): (number | undefined) => {⏎  const n = s |> Number⏎  return n⏎}` reported `stray-pipe` (48dff35 compiled it), and with `--no-verify` `const f = (): (void) => { const v = 1⏎ v |> o.m; }` emitted wrong code. Older variants of the same misreading: `const f = (): (void) => {}⏎/a|>b/.test(...)` was rejected, and `let f = (): (() => void) => {}⏎/ val const q = 2 /.test("")` (and JSX text in `.ttx`) was silently rewritten.

## Scope

- Included: The token-facts machine's type grammar (`src/lexer/facts/types.rs`): which `(` in a type opens a function type's parameters. The keyword table in `src/lexer/facts.rs` gains TypeScript's `isIdentifier` split.
- Excluded: Constructor types (`new (…) =>`, `abstract new`) and the other contextual type words, which TASK-503 covers.

## Decisions

### Decision 1: Apply TypeScript's `isUnambiguouslyStartOfFunctionType`

- **Context**: `Machine::type_atom` marked every `(` as a possible function-type parameter list, and `Machine::ty` let an `=>` after the closing `)` continue the type. For `(): (A | B) => {`, the arrow's `=>` became a function type's arrow and its body a type literal, so the statements inside it had no statement facts.
- **Alternatives considered**: (a) End the type at `=>` when the type is an arrow function's return type. TypeScript does not: `(): (a: A) => void => …` has a function return type, and `(): (A) => {}` is a function type followed by a syntax error (TS1131). (b) Apply TypeScript's own lookahead.
- **Decision and rationale**: (b). TypeScript's `parseFunctionOrConstructorTypeToError` reads a `(` as function-type parameters only when `isStartOfFunctionTypeOrConstructorType` holds, which for `(` is `isUnambiguouslyStartOfFunctionType`: `()`, `(...`, or `skipParameterStart` (modifiers accepted by `nextTokenCanFollowModifier`, then a binding identifier, `this`, or a binding pattern that parses without errors) followed by `:`, `,`, `?`, `=`, or `) =>`. Anything else is `parseParenthesizedType`. `Machine::starts_function_type` implements that rule over the bytes after the `(`, with the peek mechanism the machine already uses for contextual keywords; `binding_pattern_end` follows `parseArrayBindingPattern` and `parseObjectBindingPattern`. The `Type` field `after_paren` is renamed `parameters`.

### Decision 2: TypeScript's identifier set is part of the keyword table

- **Context**: `skipParameterStart` accepts any token TypeScript's `isIdentifier` accepts, which excludes only the words up to `LastReservedWord`; strict-mode future reserved words (`public`, `let`, `static`, …), `await`, and `yield` are identifiers to its parser. The machine's `reserved` table includes them.
- **Decision and rationale**: `reserved` is now `keyword` (TypeScript's reserved words) plus the strict-mode words, so the machine still has one keyword table and `(public) => void` is a function type as in TypeScript.

## Work log

- 2026-09-28: Reproduced the reported inputs with the TASK-498 build (`43d603f`); confirmed with the pinned TypeScript that `(): (A) =>`, `(): ([a]) =>`, `(): ({a}) =>`, and `(): (public) =>` read as function types (TS1131 after them) while `(["a"])` and `(readonly string[])` are parenthesized types.
- 2026-09-28: Added `starts_function_type` and `skip_parameter_start` to `Machine`, and `can_follow_modifier`, `binding_identifier_end`, `binding_element_end`, `binding_pattern_end`, and `expression_end` (`src/lexer/facts/types.rs`); split `keyword` out of `reserved` (`src/lexer/facts.rs`).
- 2026-09-28: Added `tests/compile/cases_14.rs` (`an_arrow_body_after_a_parenthesized_return_type_is_a_body`, `a_statement_after_an_arrow_with_a_parenthesized_return_type_keeps_its_text`, `a_function_return_type_still_continues_to_its_own_arrow`), `an_arrow_function_with_a_parenthesized_return_type_passes_through` (`tests/passthrough.rs`), and the shapes to the SWC oracle's known cases (`src/lexer/facts/tests.rs`). With the old rule forced back, the first two compile tests fail. Documented the rule in `docs/design/compiler-architecture.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 1543 tests, 0 failures.
- [x] `./scripts/ci extension`: exit 0; 210 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: exit 0.
- [x] The facts oracle (`the_machine_reads_known_shapes_as_swc_does`, `the_machine_reads_the_corpus_as_swc_does`) agrees.

## Result

Changed `src/lexer/facts.rs`, `src/lexer/facts/types.rs`, `src/lexer/facts/tests.rs`, `tests/compile.rs`, `tests/passthrough.rs`, `docs/design/compiler-architecture.md`, and `docs/tasks/INDEX.md`; added `tests/compile/cases_14.rs` and this record. A parenthesized type is a function type's parameter list exactly where TypeScript's lookahead says so.
