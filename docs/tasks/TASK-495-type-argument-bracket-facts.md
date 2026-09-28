# TASK-495: Balance type-argument brackets through one token fact

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A comma inside generic type arguments broke every tt construct that splits or ends at a top-level token. With `declare function f<A, B>(v: any): any; declare function g<A, B>(v: any): Option<number>; type A = 1; type B = 2;`: `match (x) { 1 => f<A, B>(x), _ => 2 }`, `1 => new Map<A, B>()`, and a guard `1 if f<A, B>(x) => 1` reported `malformed-match`; `let Some(v) = g<A, B>(x) else { return 0 };` failed verification; `if let Some(v) = g<A, B>(x) {…}` reported `stray-if-let`; `x |> f<A, B>;` reported `source-not-typescript`. The one-argument forms worked.

## Scope

- Included: A token fact on each `<` that opens, and each `>` that closes, type arguments or type parameters; `Token::opens_bracket`/`closes_bracket`; every bracket walk over tokens (`src/parser/{matches,iflets,lets,pipes,tries,parse,imports,partial,cursor}.rs`, `src/flow/{scanner,syntax}.rs`, `src/val.rs`, `src/val/checker.rs`, `src/engine/completions.rs`, `src/lexer/queries.rs`); completion's arm evidence after such a guard (TASK-492).
- Excluded: The byte-level delimiter validator (`src/lexer/validation.rs`), which checks `( [ {` balance for the host-syntax error and has no `<` to balance, and `src/engine/semantics/translate.rs`, which balances bytes of TypeScript display text, not tokens.

## Decisions

### Decision 1: The facts machine owns the decision; bracket walks read it

- **Context**: The walks counted `( [ {` only. `split_scrutinees` alone knew `<…>`, through its own `generic_angle_close` guess (a `>` followed by `( [ . ? ! >`), and `binding_end` in `tries.rs` counted every `<` and `>`, comparisons included. The machine already decides whether a `<` opens type arguments (TASK-491: `type_arguments_end` and TypeScript's `canFollowTypeArgumentsInExpression`) and enters a type group for type parameters, annotations, and heritage clauses.
- **Alternatives considered**: (a) Teach each walk `<…>` with its own lookahead, as `generic_angle_close` did. That is a second and third owner of a decision the machine makes, and each copy disagreed with TypeScript somewhere. (b) Record the machine's decision on the tokens and make every walk ask the token.
- **Decision and rationale**: (b). `TokenFacts` gains `opens_type_arguments` (set by `open_type_group` for a `>` closer, the one place every type-argument and type-parameter list is entered, including a decorator's) and `closes_type_arguments` (set when that group consumes its `>`). `Token::opens_bracket` is `( [ {` or a `<` with the fact; `Token::closes_bracket` is `) ] }` or a `>` with the fact. Every token bracket walk balances through them; `find_close_at` counts only the matching pair as before, with a `<` opener requiring the fact. `generic_angle_close` is deleted, and `binding_end` no longer counts comparison operators.

### Decision 2: A scrutinee's type arguments follow TypeScript

- **Context**: `generic_angle_close` treated `f<A>, x` as a comparison followed by a second scrutinee. TypeScript 4.7 and later parse `f<A>` followed by `,` as an instantiation expression (`canFollowTypeArgumentsInExpression` accepts a token that cannot start an expression).
- **Decision and rationale**: The facts machine applies TypeScript's rule, so `match (f<A>, x)` has the scrutinees TypeScript reads. Comparisons are unchanged: `match (a < b, c > d)` and `match ((a < b), (c > (d)))` still split into two scrutinees, and `a < b, c > (d)` is still the generic call it is in TypeScript (TASK-475).

### Decision 3: `has_top_level_comma` sees type arguments

- **Context**: Its documentation said a `,` inside `a as Map<K, V>` counted as top level, which only ever kept a redundant pair of parentheses.
- **Decision and rationale**: It balances brackets through the same token methods, so the comma is inside the pair. No emit snapshot changed.

## Work log

- 2026-09-28: Reproduced the six reported inputs (and a nested `f<A, Map<A, B>>` guard, a two-step pipeline, and a tuple scrutinee `g<A, B>(x), x`) with the TASK-494 binary: all nine failed.
- 2026-09-28: Added the facts (`src/lexer/facts.rs`, `facts/expressions.rs`, `facts/types.rs`) and the token methods (`src/lexer.rs`); moved every walk listed in the scope onto them; rewrote `find_close_at` (`src/parser/cursor.rs`) and `split_scrutinees`, `body_reads_as_arms`, and `tuple_value_element` (`src/parser/matches.rs`); deleted `generic_angle_close`. The expression-start tracker in `src/parser/parse.rs` restores its frame at any closing bracket.
- 2026-09-28: Added `type_argument_brackets_are_recorded_on_their_angles` (`src/lexer/facts/tests.rs`), `a_comma_inside_type_arguments_is_not_an_arm_separator` (`src/parser/partial.rs`, the TASK-492 arm evidence after a guard with type arguments), `a_comma_inside_type_arguments_stays_inside_its_construct` and `a_comparison_comma_still_separates_scrutinees` (`tests/compile/cases_13.rs`), and `type_arguments_with_commas_pass_through` (`tests/passthrough.rs`). Updated `docs/design/compiler-architecture.md` and noted the superseded `generic_angle_close` decision at the top of the TASK-475 record.

## Issues and resolutions

### Issue 1: A stray closer of another kind ended a group

- **Symptom**: `variant_with_unbalanced_field_type_is_a_field_type_error` and two content-mapper recovery tests failed: `variant E { A(value: number]) }` reported an unbalanced delimiter instead of the field-type error.
- **Cause**: The first version of `find_close_at` balanced all bracket kinds together, so the `]` closed the `(`. The previous function counted only the opener's own pair, which recovery relies on.
- **Resolution**: `find_close_at` counts only the matching pair again; a `<` opener pairs with fact-marked `<` and `>` only.

### Issue 2: The lexer query test expected the old comma reading

- **Symptom**: `a_comma_counts_only_at_the_top_level` expected `a as Map<K, V>` to have a top-level comma.
- **Cause**: The expectation recorded the documented imprecision Decision 3 removes.
- **Resolution**: The case now expects no top-level comma, with `f<A, B>(x)` (none) and `a < b, c > d` (one) added.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 40 suites, 1503 tests, 0 failures.
- [x] `./scripts/ci extension`: exit 0; 209 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: exit 0.
- [x] The facts oracle (`the_machine_reads_the_corpus_as_swc_does`, in the test run) agrees on the default corpus; no emit snapshot changed.

## Result

Changed `src/lexer.rs`, `src/lexer/facts.rs`, `src/lexer/facts/{expressions,types,tests}.rs`, `src/lexer/queries.rs`, `src/parser/{cursor,iflets,imports,lets,matches,parse,partial,pipes,tries}.rs`, `src/flow/{scanner,syntax}.rs`, `src/val.rs`, `src/val/checker.rs`, `src/engine/completions.rs`, `tests/compile/cases_13.rs`, `tests/passthrough.rs`, `docs/design/compiler-architecture.md`, the TASK-475 record, and `docs/tasks/INDEX.md`; added this record. Whether a `<` opens type arguments is decided once, by the facts machine, and every bracket walk reads that decision from the token.
