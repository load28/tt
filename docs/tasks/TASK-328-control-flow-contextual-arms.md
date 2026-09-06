# TASK-328: Preserve contextual typing across control-flow and cleanup arms

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-328: complete calls at control-flow arm exits`

## Purpose

Investigate and repair contextual typing beyond the expression and linear-return
arm proofs in TASK-324 without changing cleanup or exception semantics.

## Scope

- Included: Conditional and multiple returns, loops, `switch`, and labeled
  statements in match arms consumed by a completable call; the semantic
  boundary for try/catch/finally and resource-disposal-bearing arms.
- Excluded: Broader host-call forms (TASK-327, complete), sibling composition
  (TASK-329), and matches inside larger argument expressions (TASK-332).

## Decisions

### Decision 1: Represent completion and cleanup boundaries explicitly

- **Context**: Moving a consumer into an arm can move it inside a handler or
  before a finalizer/disposal action.
- **Alternatives considered**: Reuse linear-return lowering indiscriminately,
  flatten lexical scopes, or wrap matches in callbacks; these can change
  observable behavior.
- **Decision and rationale**: The projected arm block carries an explicit
  cleanup-freedom fact: no `try`, `with`, or `using` anywhere in its statement
  tree outside nested functions. Statements are the complete search space —
  a statement can only re-enter an expression through a function body, and
  exits are never collected across a function boundary. Every exit of a
  cleanup-free, never-completing block may carry the consuming call at its
  own authored `return`; a cleanup-bearing arm keeps the consumer outside the
  arm because calling earlier would land inside the arm's handler and ahead
  of its finalizers. This replaces the narrower linear-prefix proof, whose
  single-return restriction was subsumed.

### Decision 2: Seed exit labels from a generated identifier

- **Context**: Widening the proof let completed calls meet `break`-capturing
  arm statements (loops, `switch`) for the first time. The region label was
  derived from the continuation's assignment target, and a discarded
  completion's target was the invoke prefix — `$tt_v1(` — producing an
  unparsable label.
- **Alternatives considered**: Sanitize the prefix text (string-shape
  guessing), or forbid `break`-capturing arms for discarded completions
  (an arbitrary asymmetry).
- **Decision and rationale**: The invoke continuation carries an explicit
  label seed — the result slot, or the callee slot for a discarded call —
  which is always a generated identifier. Caught before commit by the
  emit-verify gate on the new matrix cells.

## Work log

- 2026-09-05: Reproduced the recorded forms with strict checking: an
  `if (other) return …; return …;` arm, a `try/finally` arm, and a
  `try/catch` arm each fail with TS7006/TS2345 through the join slot.
- 2026-09-05: Replaced the linear-return exit proof with per-exit facts:
  `HostExit::{body, call_safe}`, computed from an arm-block scope stack and a
  recursive statement scan in the projected AST. Removed the now-unconsumed
  `linear_return_body` plumbing. `completable_decision_arms` accepts
  never-completing blocks whose every owned exit is call-safe.
- 2026-09-05: The widened matrix caught invalid emitted labels for loop/switch
  arms of discarded completions (`$tt_y_v1(:`); gave the invoke continuation
  an explicit identifier label seed (Decision 2).
- 2026-09-05: Extended the strict matrix (7 match families × 10 hosts ×
  tt/ttx with native oracles), added runtime suites for authored-exit call
  timing, loop fallthrough, throwing consumers, and try/finally / try/catch
  ordering, compile tests pinning the labeled region and the cleanup-arm
  slot path, an editor case for a multi-return arm, and the unit-test matrix
  for call-safe exits.

## Issues and resolutions

### Issue 1: Non-linear arm completion was outside the validated contextual path

- **Symptom**: Contextually typed values returned from conditional/multiple
  returns, loops, or `switch` arms reported TS7006/TS2345 under strict
  checking of the emitted TypeScript.
- **Cause**: The linear-return proof accepted only one value return after a
  restricted statement prefix.
- **Resolution**: The cleanup-freedom proof (Decision 1) lets every authored
  return in such arms perform the call; verified for typing, runtime order,
  and abrupt completion.

### Issue 2: try/catch/finally and disposal arms must not complete the call

- **Symptom**: The same TS7006/TS2345 remains for cleanup-bearing arms.
- **Cause**: In authored semantics the consumer runs after the arm's
  finalizers and outside its handlers; completing the call at the `return`
  would reorder cleanup and re-home consumer exceptions.
- **Resolution**: Deliberately out of the proof; runtime tests pin the
  authored order (`arm, finally, call`; consumer exceptions uncaught by arm
  handlers). Contextual typing there requires annotations or a typed-backend
  slot annotation, recorded as the remaining boundary.

## Verification

- [x] Strict contextual typing for each control-flow family across tt/ttx
  hosts with native TypeScript oracles
- [x] Consumer exceptions stay outside arm handlers; finalizers run in
  original order (`cleanup_bearing_arms_keep_the_consumer_outside_the_arm`)
- [x] Call timing at authored exits, loop fallthrough, and labeled-region
  emission (`control_flow_arm_completions_*`)
- [x] Live editor diagnostics/hover/completion for a multi-return arm
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/program_syntax.rs`,
`src/program_syntax/{collector,visit,tests}.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/{mod,host,pattern}.rs`,
`tests/compile/cases_02.rs`, `tests/integration/contextual.rs`,
`editors/vscode/server/src/test/engine.test.ts`,
`docs/design/mixed-source-composition-matrix.md`, and the task records.
Cleanup-bearing arms remain outside the completion by design (Issue 2);
nested-tt-bearing arms remain with TASK-329/TASK-332.
