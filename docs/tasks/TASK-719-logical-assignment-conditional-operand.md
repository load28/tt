# TASK-719: Run a tt value in a logical assignment's right operand only when the assignment needs it

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-719`

## Purpose

`o.a ??= try read(k)` called `read` and could return its `Err` although
`o.a` was set, and `o.b ||= match (sideEffect()) {...}` evaluated the
scrutinee although `o.b` was truthy: the right operand of a logical
assignment was hoisted unconditionally. ECMA-262 §13.15.2 evaluates the
target's reference once, reads it (`GetValue`), and evaluates the right
operand (then `PutValue`) only when that value does not decide the result.
TASK-522 left this as Issue 2: lowering it as a conditional then lost
TypeScript's narrowing of the target. The round-8 probe found it again;
the position matrix had no logical or compound assignment.

## Scope

- Included: the protocol of a logical assignment's right operand
  (`src/program_syntax.rs`, `program_syntax/collector.rs`, `visit.rs`,
  `protocol.rs`), its plan (`src/evaluation_ir.rs`,
  `evaluation_ir/planning.rs`), its emission (`src/codegen/core/planning.rs`,
  `codegen/core/emitter/host.rs`), the placement messages
  (`src/lib/compile.rs`), `docs/ai/tt.md`, `docs/design/program-lowering.md`
  §7.6, a note atop TASK-522, two cases, and five matrix positions.
- Excluded: a logical assignment whose value is used (Decision 2).

## Decisions

### Decision 1: A logical assignment statement is a conditional operation over its target

- **Context**: TASK-522 captured the target into a slot and tested the
  slot, and TypeScript stopped narrowing the target: it narrows a reference
  through a `const` alias only in narrow cases, and through a stored copy
  under `==`/`===` null tests not at all (`docs/design/program-lowering.md`
  §7.10, checked there with the pinned `tsc`).
- **Alternatives considered**: (a) Guard the hoisted value with a test and
  keep the authored `o.a ??= $v`: reads the target twice (a getter or Proxy
  observes it), and the slot is used where TypeScript cannot prove it
  assigned. (b) Lower every logical assignment as a whole operation with a
  stored operand and a result slot, as `l ?? v` is: correct order, but
  `??=` loses the target's narrowing after the statement (the failure
  TASK-522 met). (c) Test the target where it is written and assign it in
  the branch: `if (o.a == null) { <values>; o.a = v'; }`, `if (!o.b) {...}`,
  `if (o.c) {...}`, with the target's object and computed key evaluated
  first (captured unless an identifier or `this`, TASK-522 Decision 3).
- **Decision and rationale**: (c), as
  `PlannedConditionalKind::LogicalAssignment`, planned from a conditional
  protocol step (`ConditionalBranch::LogicalAssignmentRight`) whose
  condition is the target (`EvaluationInputMode::LogicalAssignmentTarget`,
  with its object and key as the reference's parts). The target is read
  once, by the test, as `GetValue(lref)`; the right operand runs and is
  assigned only on the path ECMA-262 runs it; and TypeScript narrows the
  target after the `if` as it does after the operator (on one path the test
  narrows it, on the other the assignment does). Checked with the pinned
  `tsc` (`u ??= try read(k); use(u)` and `o.a ??= ...; o.a` type-check), and
  at runtime by the case's getter, which runs once.

### Decision 2: A logical assignment whose value is used is a placement error

- **Context**: When the assignment's value is used, the skipped path's
  value is the target's value, which only a second read (a getter or Proxy
  runs again) or the stored copy of Decision 1 (b) can supply.
- **Alternatives considered**: (a) A second read: changes what a getter
  observes. (b) The stored copy: `??=` loses the target's narrowing after
  the expression, a false TypeScript error on valid code (contract 2).
  (c) Reject the `try` or `match` there and keep the documented rule.
- **Decision and rationale**: (c), as the coordinator's brief allows when
  no form avoids a type trick: `ExpressionBoundaryReason::LogicalAssignmentValue`,
  reported as `try-placement` or `match-placement` with the help "write the
  logical assignment as a statement of its own". The syntax layer records
  whether the assignment is an expression statement's whole expression
  (parentheses aside); the capability decision reads that fact. A `result`
  block there still runs in place, inside the operand, so it is evaluated
  only when needed.

### Decision 3: The matrix gets the three statements, a used `??=`, and `+=`

- **Context**: No value position put a construct on the right of a logical
  or compound assignment.
- **Decision and rationale**: `nullishAssignment`, `orAssignment`, and
  `andAssignment` assign `note("target", target).value op= <value>` where
  `flip()` alternates a target that decides the result and one that does
  not, so every construct runs both paths against its twin;
  `logicalAssignmentValue` uses `(slot ??= <value>)`'s value (rejected for
  `match` and `try`); `compoundAssignment` appends with `+=`. A `flow`
  skips `compoundAssignment` as it skips `templateLiteral` (the value is a
  function, whose source text differs from the twin's). Adding value
  positions re-picks the all-pairs companions of the value constructs'
  later forms, so many generated cases were renamed; their baselines were
  regenerated and the old ones deleted.

## Work log

- 2026-10-01: Reproduced the probe's case; read TASK-522 Issue 2 and
  `docs/design/program-lowering.md` §7.6 and §7.10.
- 2026-10-01: Implemented Decisions 1 and 2; the first computed-key run
  evaluated `obj()` and `key()` again in the test (Issue 1).
- 2026-10-01: Added `tests/cases/compiler/logicalAssignmentShortCircuitsTtValue.tt`
  (from the probe, with a getter target and a computed key) and
  `logicalAssignmentValueHoldingTtValueIsAPlacementError.tt`; added the
  positions (`tests/matrix/positions.mjs`, `tests/matrix/flow.mjs`);
  `node scripts/generate-cases`; `UPDATE_EXPECT=1 TT_MATRIX_CASES=all` for
  `case_baselines` and `editor_cases`; deleted the baselines of removed
  cases.
- 2026-10-01: Updated `docs/ai/tt.md` ("try"), `docs/design/program-lowering.md`
  §7.6, and TASK-522.

## Issues and resolutions

### Issue 1: A captured object and key were evaluated again in the test

- **Symptom**: `obj()[key()] ??= try read(k)` captured `$tt_v8 = (obj())`
  and `$tt_v9 = (key())`, then tested `obj()[key()] == null`.
- **Cause**: The target's captured parts get source replacements from the
  steps of the operation's values; the condition is not one of those steps.
- **Resolution**: The plan registers the captured parts of a logical
  assignment's condition as replacements too, so the test and the
  assignment read `$tt_v8[$tt_v9]`.

### Issue 2: `compoundAssignment` printed a function's source under `flow`

- **Symptom**: Eight `flow_*_compoundAssignment_*` cases printed
  `"start:(...a) => g(f(...a))"`, which their twins do not.
- **Cause**: A composed function appended to a string prints its source,
  which is the runtime helper's in the program and an arrow in the twin.
- **Resolution**: `flow` skips the position, as it skips `templateLiteral`.

### Issue 3: The protocol test's operation names missed the new branch

- **Symptom**: The final gate's `cargo clippy --all-targets` stopped with
  E0004 in `src/program_syntax/tests.rs`: `ConditionalBranch::
  LogicalAssignmentRight { .. }` not covered.
- **Cause**: The unit test names every host operation by exhaustive
  `match`, and the new branch was added after the record's commit had been
  verified with `cargo test --test case_baselines` only.
- **Resolution**: A follow-up commit names it
  `conditional-logical-assignment-right`.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/logicalAssignmentShortCircuitsTtValue.tt`;
  `tests/cases/compiler/logicalAssignmentValueHoldingTtValueIsAPlacementError.tt`;
  the matrix cases `*_nullishAssignment_*`, `*_orAssignment_*`,
  `*_andAssignment_*`
- **Observed failure**: with the non-test changes reverted,
  `TT_CASES=logicalAssignmentShortCircuits cargo test --test case_baselines`
  reported "2 case(s) failed" with its `.ts`, `.map.txt`, and `.stdout`
  baselines differing (the right operand hoisted before the statement:
  `nullish({ a: 1 }, -1)` returned the `Err` of a `read` that should not
  run), and `logicalAssignmentValueHoldingTtValueIsAPlacementError`
  failed with its `.map.txt` out of date: the `try` and `match` were
  hoisted out of the assignments instead of reported.

## Verification

- [x] `TT_MATRIX_CASES=all` `case_baselines` and `editor_cases`: see TASK-725
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

`target ??= <tt value>;`, `||=`, and `&&=` statements run the value, and
propagate a `try`'s `Err`, only when the assignment needs it, reading the
target once and keeping TypeScript's narrowing; one whose value is used is
a placement error.
