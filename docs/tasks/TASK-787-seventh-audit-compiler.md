# TASK-787: Fix the seventh audit's remaining compiler findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-787: Fix the seventh audit's remaining compiler findings`

## Purpose

The seventh audit of the compiler (at `a71c5729`) reported seventeen
findings. TASK-784 fixed the first two; this task fixes the other fifteen
(numbered as the audit numbered them).

## Scope

- Included: findings 3–17: wrong output or evaluation order (3–9),
  internal errors on valid and invalid programs (10–14), false
  `match-placement` diagnostics (15, 16), and cubic compile time (17).
- Excluded: the audit's editor findings (a separate task).

## Decisions

### Decision 1: A value is the whole argument or branch only when its span is (findings 3)

- **Context**: `f?.(match (s) { ... } ? "a" : "b")` returned the match's
  value. The optional-call and ternary planners took a value with no
  evaluation step between it and its argument or branch as being that
  argument; a value in a conditional's test has no step of its own.
- **Decision and rationale**: A value is the whole argument (or branch)
  only when its source span equals the argument's; otherwise the argument
  is rebuilt around it (`src/evaluation_ir/planning.rs`).

### Decision 2: A rebuild never applies a replacement it sits inside (findings 4, 5, 6)

- **Context**: Copying an operand inside a replaced operation applied the
  operation's replacement: a callee capture read the `||` result slot
  (`const $tt_v3: typeof obj = ($tt_v7)`), an optional call's argument lost
  its parentheses (`$tt_v2$tt_v0`), and a branch lost `+ "x"`.
- **Decision and rationale**: The emitter keeps a stack of the spans it is
  rebuilding (a capture's operand, a branch or argument rebuilt around its
  values); a replacement that contains the innermost one is not applied
  inside it (`rebuilds_inside` in `src/codegen/core/emitter/source.rs`).

### Decision 3: A template's substitutions before a lowered value are evaluated first (finding 7)

- **Context**: A template that is a match arm's value (or a guard, or a
  block arm's `return`) lowered its nested match before evaluating the
  substitutions written before it. ECMA-262 evaluates substitutions left
  to right (`TemplateLiteral` evaluation, §13.2.8.6), applying `ToString`
  to each in turn for an untagged template, and passes them unconverted to
  a tag.
- **Decision and rationale**: Before the lowered value, every earlier
  substitution that is not inert is captured: as `` `${e}` `` for an
  untagged template, so `ToString` runs at its place, and as `(e)` for a
  tagged one (`template_substitution_captures`).

### Decision 4: An optional call's argument before a later value is captured whole (finding 8)

- **Context**: In `g.mm?.(f(match ... |> F, L("x")), match ...)` the first
  argument's call and `L("x")` ran after the second argument's match.
- **Decision and rationale**: An argument that holds tt values and comes
  before the last one is captured whole after its values
  (`PlannedOperand::Composed::capture`), as a plain argument before a value
  already is.

### Decision 5: Spreading an object literal with a getter is observable (finding 9)

- **Context**: `{ ...{ get g() { ... } }, x: match ... }` ran the getter
  after the match: the spread operand was judged inert because creating
  the literal is, but `CopyDataProperties` calls the getter. A JSX spread
  attribute was captured as a plain value, so its properties were copied
  only when the element was created.
- **Decision and rationale**: An object spread of a literal with a getter
  has effects (`object_spread_effects`); a JSX spread attribute is an
  `ObjectSpread` input, captured as `{ ...x }`. TypeScript gives a spread
  operand the object literal's contextual type without requiring it to be
  assignable, so the typed check annotates no capture whose use is a
  spread (`spreadsProperties` in `src/typescript/host.mjs`).

### Decision 6: A nested `try` shares its owner's captures (finding 10)

- **Context**: `match (s) { A => f(match (s) {...}, try R(1)), ... }` failed
  with `SourceEmittedTwice`: the `try` was planned as a nested region with
  captures of its own, so `f` and the match's argument were captured twice.
- **Decision and rationale**: A nested `try` region is planned with its host
  owner's value slots and source captures (`group_captures`).

### Decision 7: Only tt that needs statements keeps source out of a branch (findings 11, 12, 13, 16)

- **Context**: A skipped branch or a later optional-call argument holding a
  pipeline written inline (`c ? (1 |> F) : match ...`, `g?.(match ..., 1 |> F)`)
  was refused as an expression boundary; with a statement-form value in the
  other branch it ended in an internal error.
- **Decision and rationale**: The relocation test asks only about tt that
  has a statement form (`lowered_spans`); an inline pipeline is written
  where its source is copied.

### Decision 8: An arm's nested match inherits an owned value's capability (finding 15)

- **Context**: A match with a match arm, inside a call in a scrutinee, a
  pipeline head, an `if let` subject or a let-else initializer, reported
  "no sound statement region": the outer value was owned by the statement
  decision, so its capability was not among the direct values'.
- **Decision and rationale**: Owned values' capabilities are consulted too
  (`owned_capabilities`).

### Decision 9: `match` after a complete expression on its line is not a construct (finding 14)

- **Context**: `console.log(n)match (g) { ... } |> (show)` is not
  TypeScript, but ttc claimed the match (the facts machine starts a new
  statement there to recover), projected it as `console.log(n)(...)`, a
  call, and emitted `return $tt_v2$tt_v0;`, or failed with an internal
  error.
- **Decision and rationale**: A `match` keyword that directly follows a
  token ending an expression, on the same line, cannot begin one
  (ECMA-262 inserts a semicolon only before a line terminator), so it is
  not claimed and the pipeline head does not restart there
  (`continues_an_expression` in `src/parser/parse.rs`); the text is then
  reported as `source-not-typescript`. A JSX raw run is not an expression
  end.

### Decision 10: An active value is found by its anchor, not by scanning (finding 17)

- **Context**: Nested block arms compiled in cubic time: each source
  replacement asked whether it contained any active value by computing
  every active value's anchor.
- **Decision and rationale**: The active-value stack keeps the anchors in
  an ordered map and answers the question with a range query
  (`ActiveExprStack::any_anchor_within`). At depth 800 the input takes
  1.3 s instead of 3.1 s; what remains grows with the output, which is
  quadratic in the depth.

## Work log

- 2026-10-08: Reproduced every finding with the audit's inputs at
  `4360558d`, fixed them in the order above, and checked each repro's
  output, `--check-types`, `tsc` on the output, and the run.

## Issues and resolutions

### Issue 1: A JSX attribute's match stopped being a construct

- **Symptom**: `aJsxTagIsReadBeforeTheAttributesALoweredValueFollows`
  reported `source-not-typescript` at `x={match (o) ...}`.
- **Cause**: Decision 9's first version treated the JSX raw run before the
  container as an expression end.
- **Resolution**: A JSX raw run is excluded.

### Issue 2: The JSX spread case reported TS2741

- **Symptom**: the new JSX case's capture was annotated with the whole
  props type and lacked `z`.
- **Cause**: TypeScript's contextual type of a spread operand is the
  enclosing literal's (`getContextualType` for `SpreadAssignment` and
  `JsxSpreadAttribute`), which the operand need not satisfy.
- **Resolution**: Decision 5's last sentence; the object-literal case pins
  the same rule with a declared type.

## Regression test (fails before the fix)

Each was run against `4360558d`.

- **Path**: `tests/cases/compiler/aValueInsideAnArgumentIsNotTheWholeArgument.tt` (finding 3)
- **Observed failure**: printed `"c":[0]` and `"j":[0]` instead of `["else"]`.
- **Path**: `tests/cases/compiler/aCalleeCaptureInsideALogicalOperandReadsTheCallee.tt` (finding 4)
- **Observed failure**: `TypeError: $tt_v3 is not a function`.
- **Path**: `tests/cases/compiler/aRebuiltOperandInsideACapturedOperandKeepsItsText.tt` (finding 5, 6)
- **Observed failure**: `ReferenceError: $tt_v2$tt_v0 is not defined`.
- **Path**: `tests/cases/compiler/templateSubstitutionsBeforeALoweredValueRunFirst.tt` (finding 7)
- **Observed failure**: `subject, subject, g.p` where `subject, g.p, subject` is right.
- **Path**: `tests/cases/compiler/templateSubstitutionsBeforeALoweredValueRunFirstInArmsAndGuards.tt` (finding 7)
- **Observed failure**: `subject, subject, g.p` where `subject, g.p, subject` is right.
- **Path**: `tests/cases/compiler/anOptionalCallArgumentRunsWholeBeforeALaterValue.tt` (finding 8)
- **Observed failure**: `subject1, F, subject2, x, f`.
- **Path**: `tests/cases/compiler/aSpreadOfAnInlineObjectWithAGetterRunsWhereItIsWritten.tt` (finding 9)
- **Observed failure**: the getter ran after `subject`.
- **Path**: `tests/cases/compiler/aJsxSpreadOfAnObjectWithAGetterRunsWhereItIsWritten.tt` (finding 9)
- **Observed failure**: the getter ran after `subject`.
- **Path**: `tests/cases/compiler/aMatchInAnArmBesideATryArgumentCapturesTheCalleeOnce.tt` (finding 10)
- **Observed failure**: internal error `SourceEmittedTwice`.
- **Path**: `tests/cases/compiler/aSkippedBranchOrLaterArgumentMayHoldAnInlinePipeline.tt` (finding 11, 12, 13, 16)
- **Observed failure**: internal error `ConditionalRegionLeft`.
- **Path**: `tests/cases/compiler/aMatchWithAMatchArmInsideAScrutineeArgumentIsLowered.tt` (finding 15)
- **Observed failure**: five `match-placement` errors.
- **Path**: `tests/cases/compiler/aMatchAfterACompleteExpressionOnItsLineIsNotAConstruct.tt` (finding 14)
- **Observed failure**: no diagnostic; the output held `return $tt_v2$tt_v0;`.
- **Path**: `src/lib/scaling_tests.rs::checking_an_active_value_does_constant_work_per_replacement_in_nested_block_arms` (finding 17)
- **Observed failure**: "value anchors: 44440 for n nested arms but 348080 for 2n".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed: `tryInConditionalOperand` now captures an
  optional call's first argument (`.toFixed(1)`) before the later `try`
  values run (decision 4); `aTryAroundAPipelineInALogicalOperandLowersOnce`
  renumbers its slots.
- [x] Callgrind on the benchmark module: 12.97M instructions, 12.92M on `4360558d`; the output is unchanged.

## Result

Complete. All fifteen findings are fixed, each pinned by a case or a scaling
test that fails on `4360558d`, and the full gate passes.
