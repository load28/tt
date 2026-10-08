# TASK-795: Fix the round-nine editor findings

> **Superseded in part by TASK-796**: Decisions 9 (E6) and 10 (E8) were decided there: boolean literal members come from the checker, and payload fields and tag definitions follow the scrutinee's type. Decision 11 (E9) moved to its own task.

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-795: Fix the round-nine editor findings`

## Purpose

The ninth audit found eleven editor findings (E1–E11) on `c48ad20e`. This
task fixes the ones that contradict TypeScript's answer for the same
program or the documented behaviour. Three (E6, E8, E9) are left for a
decision, because each needs a change to a documented design.

## Scope

- Included: E1, E2, E3, E4, E5, E7, E10, E11.
- Excluded: E6, E8, E9 (Decisions 9–11).

## Decisions

### Decision 1: An optional call keeps the user's callee (E1)

- **Context**: `o?.m(try r())` lowers to `if (o != null) { …; v = o?.m(t); }`.
  The callee, the receiver in the test and the receiver of `.call(p)`
  were generated text, so hover, definition, references and rename on
  `o` and `m` had nothing to map, and TypeScript's diagnostics on the call
  fell back to the construct.
- **Decision and rationale**: The callee is copied from the source, with
  only captured parts read from their slots (`member_callee_text`). A
  receiver read again (the `!= null` test, `.call(receiver)`) is copied
  as well; rename already merges edits a lowering writes more than once
  into one source edit. The `(` that opens the arguments is copied from
  the source (the last token of the authored text before the first
  argument), so signature help finds its call.

### Decision 2: A `try`'s test and failure exit are anchors of their own (E2)

- **Context**: Every TS2322 inside a `try` anchor was restated as "the
  `Err` this `try` propagates does not fit", including a value that does
  not fit where it is used and a non-`Result` operand.
- **Alternatives considered**: Telling the cases apart by the message
  text is a heuristic that contract 3 forbids.
- **Decision and rationale**: The lowering anchors the test
  (`AnchorKind::TryTest`) and the failure exit (`AnchorKind::TryExit`)
  separately. The Err sentence is keyed on the exit; a 2322, 2360 or
  2361 on the test is "`try` needs a `Result`"; anything else on the
  `try` is TypeScript's own message.

### Decision 3: A module specifier's target is the module's start or whole text (E3)

- **Context**: TypeScript answers definition on a module specifier with
  the target file's start (`0:0-0:0`) or its whole text. In a projected
  `.tt` file neither offset is copied text, so the answer was dropped.
- **Decision and rationale**: In the projection, offset 0 and the
  projection's end are the module's ends, so `0..0` maps to the source's
  start and `0..end` to the whole source.

### Decision 4: Diagnostic mapping is logarithmic in the mappings (E4)

- **Context**: `mapper::diagnostic_origin` scanned every mapping for each
  diagnostic, and every anchor for each mapping: cubic in the file size
  when each function carries a restated diagnostic.
- **Decision and rationale**: Mappings are in output order
  (`in_output_order`), so the covering chunk is found by binary search.
  Only the last chunk before the span can satisfy the "missing token"
  rule, so one pass over the anchors decides it.

### Decision 5: A captured operand's read maps back to the operand (E5)

- **Context**: A JSX element whose tag or attribute is captured because a
  child is a `match` reads the capture (`<$tt_v1 n={$tt_v2}>`). A
  diagnostic on it got a one-character range.
- **Decision and rationale**: A captured source operand's read is
  recorded as a relocated operand. A diagnostic span made of copied text
  and such reads, back to back in the output and in source order, maps to
  the source text they stand for (`mapper::relocated_origin`), on the CLI
  and in the editor.

### Decision 6: An arm without `=>` is completed through the probe (E7)

- **Context**: `match (ab) { is | }` has no arm the parser can finish, so
  the completion probe found no projection and the plain reading answered
  nothing.
- **Decision and rationale**: The parser reports where the `=>` of the arm
  being written would go (`arrowless_arm_end`, from the parser's own arm
  walk). A cursor in such an arm is asked through the probe, and the probe
  closes the arm with ` => 0`, as it closes open brackets.

### Decision 7: No tt keyword after an unterminated statement (E10)

- **Context**: After `val ` (and any expression on the same line) a new
  statement cannot begin: no automatic semicolon is inserted without a
  line terminator (ECMA-262 §12.10.1). The lexer's recovery still marked
  the next word as a statement start.
- **Decision and rationale**: The lexer records that fact on the token
  (`unterminated_before`), and no tt keyword fits there.

### Decision 8: A `try` declaration ends with the user's `;` (E11)

- **Context**: `const s = try rr(1);` lowers its binding as
  `const s = $tt_t0.value;`, all generated after the name, so the outline
  range ended at the name.
- **Decision and rationale**: The binding statement's `;` is the
  statement's own and is copied from the source.

### Decision 9: E6 waits for a decision

- **Context**: Pattern completion for a boolean scrutinee offers no
  `true`/`false`. The design (TASK-607) takes literals from TypeScript's
  completion of `(scrutinee) === `, and TypeScript offers booleans there
  only as keywords, for every type.
- **Decision and rationale**: Offering them needs a checker question the
  completion path does not ask today, so the choice is left to the
  maintainer.

### Decision 10: E8 waits for a decision

- **Context**: A hand-written union that shares a tag with a visible
  variant gets the variant's fields and symbol. This is the evidence rule
  behind K8 (TASK-794 Decision 9).
- **Decision and rationale**: It is decided together with K8.

### Decision 11: E9 waits for a decision

- **Context**: Each edit re-settles the contextual storage type of every
  match in the file. With 1000 matches an edit costs about 3.7 s, almost
  all of it in the TypeScript processes answering the batched contextual
  query.
- **Decision and rationale**: Re-using answers across edits needs to know
  which slots an edit cannot affect, which changes how contextual types
  are settled. The choice is left to the maintainer.

## Work log

- 2026-10-08: Reproduced E1–E11 with the audit binary (`c48ad20e`).
- 2026-10-08: Changed `src/codegen/core/emitter/{host,result,expression,mod}.rs`,
  `src/codegen/core/authored.rs`, `src/lib/mapped.rs` (E1, E2, E5, E11);
  `src/engine/semantics/{translate,tests}.rs`,
  `tests/baselines/reference/api/ttc.api.txt` (E2);
  `src/engine/language/service/targets.rs` (E3);
  `src/typescript/mapper.rs`, `src/engine/projection.rs`,
  `src/engine/language/service.rs` (E4, E5);
  `src/parser/{partial,mod}.rs`, `src/engine/language/project/completion.rs`
  (E7); `src/lexer/facts.rs`, `src/lexer/facts/statements.rs`,
  `src/engine/completions.rs` (E10).
- 2026-10-08: Measured E4 with 200/400/800 functions: 2.2/7.9/42.1 s
  before, 1.8/3.4/7.8 s after (`ttc --check-types`, release build).
- 2026-10-08: Regenerated the case and editor baselines and reviewed them:
  the emitted TypeScript is unchanged; 124 `.map.txt` files gain source
  mappings, 9 `.types` files gain hover answers on optional calls, and
  four `.errors.txt` files and one editor baseline now underline the
  expression TypeScript reports on instead of the enclosing construct.
  Two completion baselines now answer through the probe in an unfinished
  arm.

## Issues and resolutions

### Issue 1: The "needs a `Result`" rule lost the codes it had on the test

- **Symptom**: `engine::projection::tests::a_type_error_on_a_constructs_glue_is_reported_in_tts_words`
  failed: a 2339 on `"value" in $tt_t0` was no longer translated.
- **Cause**: The new `TryTest` key listed only 2322, 2360 and 2361, while
  the rule on the whole `try` anchor had covered 2339, 2551 and 2571 on
  the test.
- **Resolution**: The `TryTest` key covers all six codes.

### Issue 2: The public API baseline gained the two anchor kinds

- **Symptom**: `the_rust_api_matches_its_baseline` failed.
- **Cause**: `AnchorKind` is public.
- **Resolution**: Regenerated `tests/baselines/reference/api/ttc.api.txt`.

## Regression test (fails before the fix)

Each test was run with `src/` stashed back to `854664ad`.

- **Path**: `tests/cases/editor/optionalCallAroundATtValue.tt`
- **Observed failure**: hover, definition and rename on `o` were empty.
- **Path**: `tests/cases/editor/tryValueMismatchIsNotAnErrTypeError.tt`
- **Observed failure**: the value mismatches read "the `Err` this `try`
  propagates does not fit".
- **Path**: `tests/cases/editor/definitionOnATtModuleSpecifier.tt`
- **Observed failure**: `definition: 0 location(s)` on every specifier.
- **Path**: `src/typescript/mapper.rs::tests::diagnostic_origins_cost_one_pass_over_the_anchors_each`
- **Observed failure**: `4020000 -> 32080000` entries for 200 and 400
  diagnostics (the old algorithm, counted the same way).
- **Path**: `tests/cases/editor/diagnosticOnACapturedJsxTag.ttx`
- **Observed failure**: TS2741 at `4:46-4:47 "B"`.
- **Path**: `tests/cases/editor/diagnosticOnAnElementWithACapturedAttribute.ttx`
- **Observed failure**: TS7026 at `2:46-2:47 "<"`.
- **Path**: `tests/cases/editor/completionAfterIsBeforeTheArrow.tt`
- **Observed failure**: `completion: 0 item(s)` at both markers.
- **Path**: `src/engine/completions.rs::tests::a_tt_keyword_is_offered_where_its_construct_can_begin`
- **Observed failure**: `val ‸` offered all seven tt keywords.
- **Path**: `tests/native/cases_06.rs::a_declaration_initialized_by_try_spans_its_initializer_in_the_outline`
- **Observed failure**: the range of `s` was `"s"`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change

## Result

E1, E2, E3, E4, E5, E7, E10 and E11 are fixed. E6, E8 and E9 wait for a
decision (Decisions 9–11).
