# TASK-800: Fix the crashes the scheduled mutation pass found

- **Status**: In progress
- **Started**: 2026-10-13
- **Completed**: —
- **Commit**: —

## Purpose

The scheduled CI run has failed since 2026-10-04, so no Nightly has been
published. One failing job is the exhaustive regression sweep:
`tests/fuzz_regressions.rs::the_case_corpus_survives_deletion_and_typing`
found five crashes no list entry covers (run 38233947903, job
114759903855). Each is fixed at the layer that broke its contract.

## Scope

- Included: the five crashes, each committed as a crash input under
  `fuzz/regressions/compile_any_bytes/`.
- Excluded: the typed-parity failure of the same run (TASK-799).

| Input | Pipeline | Crash |
|-------|----------|-------|
| `3b85010032ac492f.tt` | `compile_projection_report` (editor host emit) | `src/codegen/rope.rs`: Result return start has no matching end |
| `9b2177c8b8b7016a.tt` | `compile_projection_report` (editor host emit) | `LayoutScopeMissing` |
| `6989ca1641a0926c.tt` | `compile_projection_report` (editor host emit, planning) | `ReferenceModeUnsupported` |
| `ee29e196684ae9d1.tt` | `compile_projection_report` (editor host emit) | `SourceEmittedTwice` |
| `2b887068d381d643.tt` | `emit_mapped_with_kind` | `SourceReordered` |

## Decisions

### Decision 1: A missing unary operand sits after its operator

- **Context**: `return result{const e()=(try())else{return-;};}`. In the
  editor host emission the `return` argument's span was empty and began at
  the `-`, so the Result return mark closed where it opened, and the edit
  after the argument replaced the `-` itself.
- **Alternatives considered**: Accepting an empty Result return range in
  `src/codegen/rope.rs` would silence the check and still drop the `-` from
  the output. Skipping exits with an empty argument in the collector would
  treat a recovery artefact as a shape of the language.
- **Decision and rationale**: The span came from the vendored parser's
  recovery: `parse_unary_expr` placed the missing operand at
  `cur_pos() - 1`, which for `-;` is the operator's own start, so the
  `UnaryExpr` covered nothing. It now places it at the operator's end
  (`prev_span().hi`), where TypeScript's `parseMissingNode` places it
  (the full start of the next token). Upstream swc has the same code; the
  change is recorded in `vendor/swc_ecma_parser/TT-PATCH.md`.

### Decision 2: An arm `return` on its own line under an unbraced header is braced

- **Context**: `if (c)⏎  return 10;` in a block arm. The exit rewrite in
  `emit_body_with_exits` opened `{ ` for an exit that requires a block but
  closed it only in the same-line form, so the own-line form emitted an
  unbalanced brace (`verify-failed` on valid tt); the own-line `return;`
  form ignored `requires_block` and emitted `$v = undefined;⏎break;`, whose
  `break` ran whatever the condition. The fuzz input reached the first
  through the editor host emission.
- **Alternatives considered**: Rejecting such returns would reject valid tt.
- **Decision and rationale**: Both own-line forms now wrap the rewritten
  exit in `{ … }` when it requires a block, as both same-line forms and
  the Result-region rewrite already do.

### Decision 3: An owner block closes where an edit consumes the owner's end

- **Context**: `if (c)⏎  return twice(match …)⏎;return 30;` — the hoisted
  prelude opens the unbraced body's block, and `close_owner_blocks_at`
  closed it only when a copied source range reached the owner's end. Here
  the exit edit consumes the end and the next statement's edit starts
  there, so no range was copied and the block stayed open
  (`LayoutScopeMissing`).
- **Alternatives considered**: Extending the exit edit's text with the
  brace would duplicate the owner-block bookkeeping the emitter already
  keeps (`opened_owner_blocks`/`closed_owner_blocks`).
- **Decision and rationale**: `source_rope_with_edits` closes the owner
  blocks ending at each edit's end, as `source_range_rope` does at each
  copied range's boundaries; a block closes once (`closed_owner_blocks`).

### Decision 4: A re-checked value outside every planned operation stays at the boundary

- **Context**: `x && match …, g.m?.(match …, h?.(match …))`.
  `plan_conditional_operations` demoted the `g.m?.(…)` argument because its
  operation could not be planned whole. A later pass substitutes the
  earlier operation's join slot into the owner's values and recomputes
  their capability, which promoted the value back to `StatementRegion`;
  `validate_reference` then found a member callee of an unplanned optional
  call (`ReferenceModeUnsupported`). Without the earlier `x && match` the
  same call reported `match-placement`.
- **Alternatives considered**: Re-running operation planning after the
  substitution would plan a group the planner already refused.
- **Decision and rationale**: After the recomputation, a value with a
  conditional step that no planned operation consumes takes
  `ConditionalOperationNotStructurable`, the rule
  `plan_conditional_operations` states ("never half-lowered"). The file
  now reports the placement error it reports without the earlier value.

### Decision 5: A template substitution the schedule captures is not captured again

- **Context**: `a |> f(`${p}${match …}`)`. `emit_operand` captured the
  substitutions before a lowered value itself
  (`template_substitution_captures`), and the nested value's schedule
  captures the same `${p}` as a `TemplateSubstitution` input, so `p` was
  emitted twice (`SourceEmittedTwice`).
- **Alternatives considered**: Dropping the schedule's capture would leave
  the evaluation plan's order to an emitter-local rule.
- **Decision and rationale**: `emit_operand` skips a substitution whose
  source an input of the nested values' scheduled steps captures; the
  planned capture and its slot are used, as without a pipeline. The two
  spans are compared up to trivia: the planned input is the host
  expression's span, while the Core interpolation's span runs from `${` to
  `}` and keeps the whitespace and comments around the expression, so a
  planned input counts when it lies inside the substitution with only
  trivia (`scanner::skip_ws_comments`) on either side.

### Decision 6: A pipeline gap starts at the end of the whole operand

- **Context**: `match (o) { A => f }/*c*/() |> g`. The gap between a
  pipeline operand and the next step was measured from
  `structured_expr_span`, which for a `Sequence` operand is the span of its
  first structured value (the `match`), so the comment inside the operand
  was also written as a comment of the gap (`SourceEmittedTwice` in the
  statement form, `SourceReordered` in the editor's `emit_mapped`).
- **Alternatives considered**: Changing `structured_expr_span` would move
  every other user of the first structured value.
- **Decision and rationale**: `emit_apply` and `emit_apply_continued`
  measure operands with `operand_extent`, the whole source extent of a
  `Sequence` (`body_extent`) and `structured_expr_span` otherwise.

## Work log

- 2026-10-13: Saved the five minimized inputs from the CI log; their FNV
  names match. `every_committed_crash_input_replays_as_the_list_says`
  reports all five. A probe calling each pipeline of
  `ttc_fuzz::every_pipeline` separately located them (table above).
- 2026-10-13: Decision 1. `tests/swc_editor_recovery.rs::a_missing_unary_operand_sits_after_its_operator`.
- 2026-10-13: Reduced each remaining input to valid tt that crashes
  `ttc` itself: an own-line arm `return` under an unbraced `if` (verify
  failure; with `;` on the next line, `LayoutScopeMissing`), a `match`
  argument of `g.m?.(…)` after `x && match …` (`ReferenceModeUnsupported`),
  `a |> f(`${p}${match …}`)` and a comment inside a pipeline operand
  (`SourceEmittedTwice`). Decisions 2–6.
- 2026-10-13: Cases `tests/cases/conformance/evaluation/runtimeAnArmReturnOnTheLineAfterAnUnbracedHeaderStaysUnderIt.tt`,
  `tests/cases/compiler/aMatchArgumentOfAnOptionalCallAfterAnEarlierConditionalKeepsItsPlacementError.tt`,
  `tests/cases/conformance/evaluation/runtimeATemplateSubstitutionBeforeAMatchInAPipelineStepRunsOnce.tt`,
  and `tests/cases/compiler/aCommentInsideAPipelineOperandIsWrittenOnce.tt`
  (`UPDATE_EXPECT=1 TT_CASES=<name> cargo test --test case_baselines`). Each
  was compiled with `ttc` built from 0776bbb8 and crashed as listed below.

## Issues and resolutions

### Issue 1: Result return start has no matching end

- **Symptom**: `3b85010032ac492f.tt` panicked at `src/codegen/rope.rs`.
- **Cause**: The parser's recovered `UnaryExpr` for `-;` had the span
  `77..77` (projected), so the exit argument mapped to an empty span.
- **Resolution**: Decision 1.

### Issue 2: LayoutScopeMissing

- **Symptom**: `9b2177c8b8b7016a.tt` panicked in `validate_origin`.
- **Cause**: Decisions 2 and 3: an unbalanced exit brace and an owner block
  left open where two edits met.
- **Resolution**: Decisions 2 and 3.

### Issue 3: ReferenceModeUnsupported

- **Symptom**: `6989ca1641a0926c.tt` panicked in `validate_reference`.
- **Cause**: Decision 4: the capability recomputation undid the operation
  planner's demotion.
- **Resolution**: Decision 4.

### Issue 4: SourceEmittedTwice

- **Symptom**: `ee29e196684ae9d1.tt` panicked in
  `validate_source_preservation` at the `p` of `${p}`.
- **Cause**: Decision 5: two captures of one template substitution.
- **Resolution**: Decision 5.

### Issue 5: SourceReordered

- **Symptom**: `2b887068d381d643.tt` panicked in
  `validate_source_preservation` (editor `emit_mapped`) at the line comment.
- **Cause**: Decision 6: the gap after a `Sequence` operand began inside it.
- **Resolution**: Decision 6.

### Issue 6: The first fix of Issue 4 compared the spans exactly

- **Symptom**: The full mutation pass (`TT_MUTATIONS=all`) found
  `SourceEmittedTwice` again in a mutant of the new case:
  `return""|>p(\`${d(}${match(s){A=>}}\`)`, saved as
  `fuzz/regressions/compile_any_bytes/cb541c50f92feddd.tt`.
- **Cause**: The editor repair wrote `d()\n`; the planned capture covered
  `d()` (15..18) and the substitution `d()\n` (15..19), so the exact
  comparison missed it. Valid tt has the same shape: `${ head() /* first */ }`.
- **Resolution**: Decision 5's comparison up to trivia; the runtime case
  gained the `spaced` function, which crashed with the exact comparison
  (`SourceEmittedTwice` at bytes 476..477).

## Regression test (fails before the fix)

- **Path**: `tests/fuzz_regressions.rs::every_committed_crash_input_replays_as_the_list_says`
  (the five inputs and `cb541c50f92feddd.tt`), `tests/swc_editor_recovery.rs::a_missing_unary_operand_sits_after_its_operator`,
  and the four case files of the work log
- **Observed failure**: the replay test reported all five inputs crashing
  with the signatures of the scope table; the parser test reported
  `const broken = -;` with the span `(16, 16)` against `(16, 17)`; `ttc`
  from 0776bbb8 stopped on the case files with `LayoutScopeMissing`,
  `ReferenceModeUnsupported` (bytes 213..216), and `SourceEmittedTwice`
  (bytes 357..358 and 243..244).

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo test --release --test fuzz_regressions`
- [ ] Baseline changes reviewed and committed with the change

## Result

Pending.
