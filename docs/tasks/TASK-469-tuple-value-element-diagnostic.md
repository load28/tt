# TASK-469: Report a literal or `is` tuple element as what it is

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`match (a, b) { (1, "x") => "one-x", _ => "other" }` failed with `tt \`match\` could not be parsed` at the `match` keyword and the help "write `match (<scrutinee>) { <pattern> => <body> }`; a tuple pattern must match the scrutinee arity". The arity is correct; the actual problem is that tuple pattern elements are tag patterns or `_` (`docs/ai/tt.md`, "NOT allowed inside tuple patterns (v1)"), and `1` is a literal pattern. The diagnostic pointed at the wrong construct and described a problem the program does not have.

## Scope

- Included: The malformed-match diagnostic built by the match parser (`src/parser/matches.rs`), the `malformed-match` explanation (`src/diagnostics.rs`), compile tests, and a rendered-diagnostic fixture.
- Excluded: Admitting literal or `is` patterns in tuple elements (a language change; the documented v1 surface keeps them out). The separate defect found while checking the help's advice (see Issue 1).

## Decisions

### Decision 1: Classify the failing element structurally at the diagnostic site

- **Context**: The match parser is all-or-nothing: `parse_tuple_elems` returns `None` for any element that is not a tag pattern or `_`, and the committed-but-malformed branch then chooses a message. An arity mismatch never reaches that branch: a tuple pattern of the wrong length parses and sema reports `match-tuple-arity` over the complete tuple pattern. So the arity clause of the generic help was never the cause of a parse failure.
- **Alternatives considered**:
  - Accept literal elements in the tuple grammar and reject them in sema. That changes the parse contract of every tuple match (lowering, exhaustiveness, editor recovery) to deliver one message.
  - Search the body text for digits or quotes inside parentheses. A textual heuristic, excluded by contract 3.
  - Thread a failure reason out of every tuple parser function. Much wider change for one diagnostic.
- **Decision and rationale**: When a committed match fails, `tuple_value_element` walks the arm starts of the body (the first token and each token after a top-level comma), and for each parenthesized pattern with a top-level comma splits its elements and runs the real pattern parsers on each: an element that `parse_literal_alternatives` or `parse_instance_alternatives` consumes completely is a value pattern. The first one becomes the error, spanning exactly that element: "a literal pattern cannot be a tuple pattern element" or "an `is` pattern cannot be a tuple pattern element", with the help "tuple pattern elements are tag patterns or `_`; test this value in an arm guard or a nested `match`". It replaces both the keyword-level error and the arm-recovery error ("invalid match arm") when one of the other arms parsed; the recovery node is unchanged. The generic help keeps only the form, `write \`match (<scrutinee>) { <pattern> => <body> }\``. The `malformed-match` explanation now states the tuple element rule.

## Work log

- 2026-09-28: Reproduced with `ttc --check` on `match (a, b) { (1, "x") => ..., _ => ... }`: the error sat on `match` with the arity help. Checked that a length mismatch is reported by sema (`tuple_match_arity_mismatch_is_an_error`), never by the parser.
- 2026-09-28: Added `TupleValueElement`, `tuple_value_element` and `value_element` and used them in both malformed branches of `parse_match` (`src/parser/matches.rs`); reworded the generic help; extended the `malformed-match` explanation (`src/diagnostics.rs`).
- 2026-09-28: Checked the advice: `(None, _) if n === 1 => 0` compiles and emits the guard; a literal element of a variant/number tuple and an `is Date` element each report at the element.
- 2026-09-28: Tests: `a_value_pattern_in_a_tuple_element_is_reported_at_that_element` and `an_unparseable_match_help_does_not_claim_an_arity_problem` (`tests/compile/cases_06.rs`); `tuple_patterns_do_not_accept_literals` now pins the new message and the literal's position (`tests/compile/cases_07.rs`); new fixture `tests/fixtures/diagnostic/tuple-value-element/` generated with `UPDATE_EXPECT=1 cargo test --test snapshot` and reviewed (the caret covers `404`, the help is the new advice).

## Issues and resolutions

### Issue 1: A guarded all-wildcard tuple arm emits `if ()`

- **Symptom**: While checking the help's guard advice, `match (a, b) { (_, _) if a === 1 && b === "x" => "one-x", _ => "other" }` failed verification ("Expression expected"); `--no-verify` shows `if () { if (a === 1 && b === "x") ... }`.
- **Cause**: The tuple arm's element test is empty when every element is `_`; this is in tuple-arm lowering, not in the diagnostic.
- **Resolution**: Out of scope for this diagnostic task; fixed by TASK-478. A guard on an arm with at least one tag element, as the help describes, lowers correctly.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/parser/matches.rs`, `src/diagnostics.rs`, `tests/compile/cases_06.rs`, `tests/compile/cases_07.rs`, `tests/fixtures/diagnostic/tuple-value-element/`, this record, and `docs/tasks/INDEX.md`. A literal or `is` pattern inside a tuple pattern is reported at that element with advice that fits it, and no malformed-match help claims an arity problem.
