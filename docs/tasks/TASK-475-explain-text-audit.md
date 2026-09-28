# TASK-475: `ttc explain` texts match what the compiler does

> Partly superseded by [TASK-495](./TASK-495-type-argument-bracket-facts.md): `generic_angle_close` is deleted. Whether a `<` in a scrutinee opens type arguments is the token facts' decision (TypeScript's `canFollowTypeArgumentsInExpression`), and `split_scrutinees` balances brackets through `Token::opens_bracket`/`closes_bracket`.

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttc explain match-nested-in-or-pattern` recommended `Ok(value: Some(v) | None())`, which does not parse (`malformed-match`). `docs/ai/tt.md` allows no or-pattern combined with a nested pattern, and element-level alternation exists only in tuple positions. `ttc explain try-placement` did not say that `try` is rejected when its target is a constructor, a generator, or a class static block (`src/lib/compile.rs`, `docs/design/try-result-scopes.md` §4). An explanation is where a user goes after a diagnostic, so a wrong one sends them to a second error.

## Scope

- Included: Every explanation in `src/diagnostics.rs`, checked against `ttc` behaviour; the scrutinee splitter in `src/parser/matches.rs`, which contradicted the tuple advice; `docs/ai/tt.md` where it carried the same inaccuracies; tests.
- Excluded: The two unrelated defects the audit found (Issues 2 and 3). Each explanation is accurate for them; the behaviour is not, and it belongs to a separate task.

## Decisions

### Decision 1: Audit by running every claim, not by reading

- **Context**: The explanations contain examples and statements about what is accepted and rejected. Reading them against the code misses runtime wording and parser details.
- **Alternatives considered**: Extract code blocks from the explanations and compile them automatically. Most examples are fragments (`x |> (n => n + 1)`, arm lists) that need a different prelude each, so a generic extractor would test the prelude more than the claim.
- **Decision and rationale**: Wrote one probe `.tt` file per claim (about 60) in the scratchpad and ran the debug `ttc -p` over them, comparing codes and messages with each explanation. The wrong claims are fixed, and the fixed ones are held by `explained_examples_behave_as_their_explanations_say`, which asserts both the explanation text and the compiler's answer for the same example.

### Decision 2: Fix the scrutinee splitter instead of the advice

- **Context**: `malformed-match` said scrutinees with a top-level `<` or `>` need parenthesizing. In fact `match (a < b, v)` and `match (a < b, c > d)` split correctly, and the one ambiguous case, `a < b, c > (d)`, is the generic call `a<b, c>(d)` in TypeScript too. But parenthesizing did not help: `match ((a < b), (c > (d)))` still had one scrutinee, because `generic_angle_close` looked for the closing `>` across the parentheses that enclose the `<`.
- **Alternatives considered**: Reword the advice to "rewrite the comparison". That documents a parser defect as a language rule.
- **Decision and rationale**: `generic_angle_close` now tracks `(`/`[` nesting: a closer with no opener after the `<` ends the search (the `<` cannot be a type-argument list that leaves its group), and a `>` inside a nested group is not the list's close. The advice moved to `match-tuple-arity`, which is the code the ambiguous case reports, and names the exact case and the fix.

## Work log

- 2026-09-28: Probed every explanation. Inaccurate: `match-nested-in-or-pattern` (non-parsing recommendation), `try-placement` (constructor, generator, async generator and static-block targets missing), `match-not-exhaustive` (the hole is rendered `missing "Ok(value: None())"`, not `None`), `malformed-match` (the `<`/`>` advice, see Decision 2), and `val-mutation` (it omitted the `--check-types` report of built-in mutator calls). The other 42 matched their probes, examples included; `result-return-nested` (typed only), `lowering-plan-failed` and `other` were checked against their reporting sites instead.
- 2026-09-28: Rewrote the five texts in `src/diagnostics.rs`; fixed `generic_angle_close` in `src/parser/matches.rs`; corrected the same statements in `docs/ai/tt.md` (exhaustiveness hole, tuple scrutinees, `try` placement).
- 2026-09-28: Tests: `explained_examples_behave_as_their_explanations_say` (`tests/compile/cases_11.rs`) and two more scrutinee lists in `tuple_match_accepts_comparison_expression_subjects` (`tests/compile/cases_01.rs`).

## Issues and resolutions

### Issue 1: Parenthesized comparisons still merged tuple scrutinees

- **Symptom**: `match ((a < b), (c > (d))) { (A, _) => 1, _ => 2 }` reported `match-tuple-arity` (2 elements, 1 scrutinee).
- **Cause**: `generic_angle_close` scanned from `<` to any `>` followed by `(`, ignoring the `)` that closed the `<`'s own group.
- **Resolution**: Nesting-aware scan (Decision 2).

### Issue 2: `if let` as a declaration initializer reports `lowering-plan-failed`

- **Symptom**: `function h() { const x = if let Some(value: v) = o() { v }; }` reports `lowering-plan-failed` rather than `if-let-placement`; inside a template interpolation both are reported.
- **Cause**: Not investigated in this task; the placement check does not see an initializer as an expression position, and the lowering fails afterwards.
- **Resolution**: Left for a follow-up task. The `if-let-placement` explanation is accurate.

### Issue 3: `break` crossing a `result` block also reports `source-not-typescript`

- **Symptom**: `for (;;) { const r = result { const a = try g(); break; }; }` reports `result-break-crossing` and then `source-not-typescript` for the same `break`.
- **Resolution**: Left for a follow-up task.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/diagnostics.rs`, `src/parser/matches.rs`, `tests/compile/cases_01.rs`, `tests/compile/cases_11.rs`, `docs/ai/tt.md`, this record, and `docs/tasks/INDEX.md`. Every `ttc explain` text now describes what the compiler does, and parenthesizing a comparison splits tuple scrutinees as the text says.
