# TASK-307: Close Result review follow-ups

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-307: close Result review follow-ups`

## Purpose

Close the actionable review follow-ups from PR #99 by pinning the complete-input
recovery boundary and documenting the structural Result propagation ABI.

## Scope

- Included: Assert that complete statement-position matches never emit the
  editor-only `$tt_recovery` boundary, and document how custom Result values are
  classified during propagation.
- Excluded: Change the Result runtime representation, add a cache for flow
  analysis without performance evidence, or explain ordinary JavaScript object
  stringification.

## Decisions

### Decision 1: Pin the recovery invariant at the complete Result regression

- **Context**: The recovery IIFE is valid only when an incomplete editor owner
  provides neither an owner rewrite nor a value slot.
- **Alternatives considered**: Add a fixture that only searches generated output,
  or assert the invariant on the existing complete statement-match regression.
- **Decision and rationale**: Extend the existing regression because it already
  exercises the production Result statement path whose output must never contain
  `$tt_recovery`.

### Decision 2: State the structural ABI at the propagation surface

- **Context**: Code generation classifies Result success with `"value" in value`,
  but the user guide only documented the standard constructors' field shapes.
- **Alternatives considered**: Leave custom values implicit, or document the
  exact structural test next to `try` propagation.
- **Decision and rationale**: Document the exact test so hand-written Result
  values cannot silently disagree with compiler lowering.

## Work log

- 2026-08-30: Ran `./scripts/doctor`, fetched `origin/main`, and created the task
  branch from commit `233bd81`.
- 2026-08-30: Added the complete-input recovery assertion in `tests/compile.rs`
  and documented the structural propagation ABI in `docs/ai/tt.md`.
- 2026-08-30: Verified the focused compile regression and the complete
  `./scripts/ci` gate.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Complete Result programs now have an explicit regression against editor-only
recovery emission, and the user guide defines the structural ABI required by
hand-written Result values.
