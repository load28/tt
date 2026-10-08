# TASK-790: Fix the round-eight compiler findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-790: Fix the round-eight compiler findings`

## Purpose

The eighth audit of `ttc` found ten compiler defects (C1–C10) and one
editor finding whose cause is in the parser (E5: compile time doubling with
each nested pipeline head). This task fixes each in the layer responsible
for it.

## Scope

- Included: C1–C10 and E5, each with a regression test.
- Excluded: the other round-eight CLI and editor findings, which later
  tasks take.

## Decisions

### Decision 1: A `try` declaration's binding ends at its last token

- **Context**: C1. `const v //C\n = try ok(3);` emitted
  `const v //C = $tt_t0.value;`: the binding span ran to the `=`, so the
  comment was copied into the binding and the generated `= …` landed inside
  it.
- **Decision and rationale**: `parse_try_decl` ends the binding at the last
  token before `=`. The emitter writes the binding, then the gap up to the
  `=` through `push_gap`, which keeps a comment and ends its line, then
  `= `. One helper (`push_propagate_binding`) serves the statement form, the
  region form, and a `for` initializer.

### Decision 2: The printer ends a copied line comment's line

- **Context**: C2. Under a comment directive the printer keeps a
  statement's glue on one line. A `// c` copied from an arm was followed by
  generated `; break; }` on the same line, which the comment swallowed
  (`verify-failed`).
- **Decision and rationale**: The target file knows where each copied `//`
  comment ends. While the last printed source ends one, the printer starts a
  new line before the next generated text. The comment ended the line in
  the source too, so the directive covers the same line it did there.

### Decision 3: Placement checks skip values owned by a nested function

- **Context**: C3. `c ? match … : () => match …` reported
  `match-placement`: the arrow's match was counted as a value of the
  conditional's branch, though it runs in its own function.
- **Decision and rationale**: `TtSpans` records each tt value's owner span.
  `overlaps_tt` ignores a value whose owner lies inside the branch, because
  that value is evaluated by its own owner, not by the branch.

### Decision 4: A discarded comma operand is not stored as a value

- **Context**: C4. `c ? (try r(), 1) : 2` emitted `$tt_v0 = $tt_t0.value`
  followed by `($tt_v0, 1)`, which TypeScript reports as an unused left
  operand (ts2695).
- **Decision and rationale**: The planner's discarded-value set also covers
  the active values of conditional operations. A discarded value is
  evaluated for its effect and contributes no text.

### Decision 5: An or-pattern's own alternatives are covered

- **Context**: C5. In `Circle(r) | Circel(r) => r`, the unknown case
  `Circel` was answered with `Circle`, which the same arm already matches.
- **Decision and rationale**: `cases_covered_beside` reads every
  alternative of an or-pattern, and the arm holding the misspelled name
  counts even when guarded. The suggestion becomes `Circles`.

### Decision 6: A `try` operand that TypeScript cannot read is the source's error

- **Context**: C6, C7. `try r(1 2)` reported `verify-failed`, and a missing
  `;` before `const a = try r();` reported `lowering-plan-failed`.
- **Decision and rationale**: The projection hides an opaque `try` operand
  as an operand part, so TypeScript's parse of the source reads it, and the
  propagation placeholder is no longer parenthesized, so a missing `;`
  before it is where TypeScript stops. Both are now `source-not-typescript`
  at TypeScript's position.

### Decision 7: Smaller parser and message fixes

- **Context and decision**:
  - C8: `match (/*c*/)` was parsed as a scrutinee holding only a comment.
    An empty scrutinee is now judged by its tokens (`malformed-match`).
  - C9: `...result { … }` was not claimed. The last dot of a spread is no
    longer read as member access (`cursor::spread_ends_at`), which also
    replaces `follows_spread_operator`.
  - C10: TypeScript's message already ends with a period, and the verify
    and source-error messages appended a second one.

### Decision 8: A pipeline is parsed only once its claim survives

- **Context**: E5. Each pipeline head was parsed when the pipeline was
  claimed. A later pipeline whose head contains it rewound the claim and
  parsed its own head, which found the inner pipeline again, so nested heads
  took 2^depth parses (depth 22: 3.9 s).
- **Alternatives considered**: A memo of expression parses keyed by token
  range needed a clone of each cached tree and remained cubic (depth 400:
  27 s).
- **Decision and rationale**: `parse_pipeline` now returns a `PipeScan`
  (the head and step token ranges). The token loop keeps scans with its
  segments and parses them when the loop ends, so a rewound claim costs
  nothing. Depth 400 now compiles in 130 ms, with the same output.

## Work log

- 2026-10-08: Reproduced C1–C10 and E5 with the audit binary (`6dc87196`).
- 2026-10-08: Fixed C1–C10 (`src/parser/{tries,matches,cursor,parse}.rs`,
  `src/codegen/core/emitter/{result,host}.rs`, `src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`, `src/codegen/core/{mod,planning}.rs`,
  `src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,planning}.rs`,
  `src/program_syntax/projection.rs`, `src/resolve/mod.rs`,
  `src/verify.rs`).
- 2026-10-08: Profiled E5 under callgrind, tried a parse memo (Decision 8),
  and replaced it with deferred pipeline parsing (`src/parser/pipes.rs`,
  `src/parser/parse.rs`).
- 2026-10-08: Added the regression cases and the scaling test, generated
  the baselines, and ran them against `6dc87196` in a worktree.

## Issues and resolutions

### Issue 1: An existing case's operand was not TypeScript

- **Symptom**: After Decision 6, `aTryExitingAResultFromAParameterInitializer`
  reported `source-not-typescript` instead of `try-placement`.
- **Cause**: The case wrote `try ()`. An empty parenthesized expression is
  not TypeScript, and the operand is now read by TypeScript's parse, which
  is the intended behaviour.
- **Resolution**: The case's operand is now `try read()`, with `read`
  declared, so it still tests the placement rule it was written for.

### Issue 2: A CLI test expected the old verify-failed report

- **Symptom**: `tests/cli.rs` `an_invalid_file_reports_the_same_diagnostic_with_typescript_installed`
  expected `verify-failed` for `(try result { if (b) { return 10; } return 1; })`
  in a function and `try-placement` for the same expression at module level.
  It got `source-not-typescript` at the `{` after `result` for both.
- **Cause**: The block holds no `try`, so `result` is not claimed and the
  operand is source text that TypeScript cannot parse. After Decision 6
  TypeScript's parse of the source reads that operand.
- **Resolution**: The test expects `source-not-typescript`, the code
  `docs/ai/tt.md` gives for TypeScript that does not parse in a file with
  tt constructs. The test still checks that both setups report the same
  thing.

### Issue 3: Nested pipeline heads still query their own text in codegen

- **Symptom**: With Decision 8, `top-level query bytes` and
  `yield context updates` still grow with depth squared for nested pipeline
  heads (130 ms at depth 400).
- **Cause**: Each step checks whether its input needs grouping from that
  input's emitted text, which contains every nested level.
- **Resolution**: Not changed here. The scaling test pins the parse count,
  which was the exponential term.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aCommentBeforeTheEqualsOfATryDeclarationKeepsItsInitializer.tt`
- **Observed failure**: `6dc87196` emitted `const v //C = $tt_t0.value;`; the `.map.txt` baseline differed.
- **Path**: `tests/cases/compiler/aLineCommentAtTheEndOfAMatchKeepsTheStatementEnd.tt`
- **Observed failure**: `verify-failed: unbalanced TypeScript delimiter`; the baselines differed.
- **Path**: `tests/cases/compiler/aMatchInsideAFunctionInAConditionalBranchIsNotMisplaced.tt`
- **Observed failure**: two `match-placement` errors; the baselines differed.
- **Path**: `tests/cases/compiler/aDiscardedCommaOperandInAConditionalBranchIsNotAValue.tt`
- **Observed failure**: the emitted code stored the discarded operand (`$tt_v0, 1`); the `.map.txt` baseline differed.
- **Path**: `tests/cases/compiler/anOrPatternAlternativeIsNotSuggestedForItsSibling.tt`
- **Observed failure**: `help: a case with a similar name exists: \`Circle\``; the `.errors.txt` baseline differed.
- **Path**: `tests/cases/compiler/invalidTypeScriptInATryOperandIsReportedAsSource.tt`
- **Observed failure**: `verify-failed` instead of `source-not-typescript`.
- **Path**: `tests/cases/compiler/aMissingSemicolonBeforeATryDeclarationIsReportedAsSource.tt`
- **Observed failure**: `lowering-plan-failed: the evaluation position 145..169 maps to no source`.
- **Path**: `tests/cases/compiler/aCommentOnlyMatchScrutineeIsMalformed.tt`
- **Observed failure**: `lowering-plan-failed: … Parenthesized expression cannot be empty`.
- **Path**: `tests/cases/compiler/aResultBlockAsASpreadOperandIsLowered.tt`
- **Observed failure**: `source-not-typescript: Expected ',', got '{'`.
- **Path**: `src/lib/scaling_tests.rs::parsing_does_linear_work_in_nested_pipeline_heads`
- **Observed failure**: `token range parses: 511 at depth 8 but 131071 at depth 16`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Each regression test above fails on `6dc87196` and passes here.

## Result

C1–C10 and E5 are fixed, and `docs/ai/tt.md` describes the or-pattern
suggestion rule and the spread operand of a `result` block. Changed files:
the sources in the work log, `src/lib/scaling_tests.rs`, nine cases under
`tests/cases/compiler/` with their baselines, the updated
`aTryExitingAResultFromAParameterInitializer` case and baselines, and
`docs/ai/tt.md`.
