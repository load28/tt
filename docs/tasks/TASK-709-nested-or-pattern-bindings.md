# TASK-709: Count a nested alternative's bindings when or-pattern alternatives are compared

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-709`

## Purpose

TASK-700 (Issue 4) found that
`match (o) { Done(value: Some(value: v)) | Failed(error: v) => v, _ => 0 }`
reports `match-nested-in-or-pattern` and also `match-or-binding-mismatch`
("`v` is bound in `Failed(...)` but not in `Done(...)`"). The second report
is false: `Done(value: Some(value: v))` binds `v`. One fault (a nested
pattern in an or-pattern) must give one diagnostic.

## Scope

- Included: the binding set sema compares or-pattern alternatives by
  (`src/sema.rs`, `binding_set` and `binding_mismatch`) and its three callers
  in `src/sema/checker.rs` (a match arm, an element of a tuple pattern, and
  the `if let` / let-else alternatives); two compiler cases; the listed
  defect in `tests/oracle-failures.txt` and its matrix baseline.
- Excluded: the TypeScript-layer cascade `ttc --check-types` reports on such
  a match (Issue 2), and the or-pattern rules themselves, which are
  unchanged.

## Sources

- `docs/ai/tt.md` (Patterns): an or-pattern's alternatives "must bind same
  (field,name) set" because they share one emitted destructuring, and a
  nested pattern cannot be combined with an or-pattern for the same reason.
- The Rust Reference, "Patterns" > "Or-patterns", and rustc's E0408 ("An
  or-pattern was used where a variable binding is not consistently bound
  across patterns"): the names an alternative binds are every identifier
  pattern in it, at any depth.

## Decisions

### Decision 1: Compare names, nested leaves included, when an alternative is nested

- **Context**: `binding_set` took an alternative's top-level bindings and
  skipped any binding with a nested pattern, on the stated assumption that a
  nested pattern never reaches it because or-patterns reject it. They do
  reject it, but only by reporting `match-nested-in-or-pattern`; the
  comparison still ran and saw no `v` in `Done(...)`.
- **Alternatives considered**: (a) Skip the binding comparison whenever an
  alternative is nested: it would hide a real second fault, alternatives
  that bind different names (`Done(value: Some(value: v)) | Failed(error:
  w)`), which the author has to fix as well once the arm is split.
  (b) Compare (field path, name) pairs with nested paths (`value.value`):
  `v` from `value.value` and `v` from `error` would still differ and report
  `match-or-binding-mismatch` for the same fault the nested rule names,
  because the same-field requirement exists only for the shared
  destructuring that the nested rule already rules out. (c) Compare names
  with nested leaves when an alternative is nested, and (field, name) pairs
  otherwise.
- **Decision and rationale**: (c). The (field, name) rule follows from the
  shared destructuring, so it applies exactly when the alternatives can
  share one (none is nested); when one is nested, that is reported once by
  `match-nested-in-or-pattern`, and what remains a separate fault is a name
  the arm body reads that one alternative does not bind, which is Rust's
  E0408 rule and counts every leaf. Flat or-patterns compare exactly as
  before.

## Work log

- 2026-10-01: Reproduced with `ttc --check` on a match arm, a tuple
  element, and an `if let`; each reported both codes.
- 2026-10-01: Changed `binding_set` and `binding_mismatch` to take the
  alternative and whether the alternatives share a destructuring, and the
  three callers to pass `!alts.iter().any(has_nested)`.
- 2026-10-01: Added `tests/cases/compiler/nestedOrPatternAlternativeBindsItsLeaves.tt`
  (`@expectDiagnostic`, four positions) and
  `nestedOrPatternAlternativeBindsOtherNames.tt` (`@expectErrors`, the real
  second fault); removed the defect line from `tests/oracle-failures.txt`;
  `UPDATE_EXPECT=1 TT_CASES=match- cargo test --test case_baselines`
  rewrote the one matrix baseline that held the false report.
- 2026-10-01: The full case run (`TT_MATRIX_CASES=all`) moved two more
  baselines, `ifLetNestedPatternsCannotCombineWithOr.errors.txt` and
  `nestedPatternInsideOrPatternIsAnError.errors.txt`: their alternatives
  do bind different names, and the message now names the nested leaf
  (`v` is bound in `Some(...)` but not in `None(...)`; `v` in `Ok(...)`
  but not in `Err(...)`, where it named `error` before); the first also
  reports the mismatch on the command line, which skipped it before because
  the nested alternative's empty set equalled `None()`'s.

## Issues and resolutions

### Issue 1: The false report

- **Symptom**: as in Purpose, in a match arm, a tuple pattern's element, and
  an `if let`.
- **Cause**: `binding_set` filtered out bindings with a nested pattern.
- **Resolution**: Decision 1.

### Issue 2: `--check-types` also reports TS2339 on such a match

- **Symptom**: For an or-pattern whose flat alternative binds a field
  (`... | Failed(error: v)`), `ttc --check-types` reports
  `error[ts2339]: match on a tag pattern needs a value with a kind
  discriminant ...` on the scrutinee besides `match-nested-in-or-pattern`.
- **Cause**: The rejected arm is still projected with one shared
  destructuring for both alternatives, so TypeScript sees `value` read from
  `Failed`. It is a TypeScript-layer report, which the diagnostics oracle
  does not judge, and it was there before this task (the matrix baseline
  holds it).
- **Resolution**: Not changed here; recorded as a follow-up.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/nestedOrPatternAlternativeBindsItsLeaves.tt`
  and `nestedOrPatternAlternativeBindsOtherNames.tt` (`cargo test --test
  case_baselines`), and the matrix case
  `match-nested-in-or-pattern_bindingAlternative_declarationInitializer`.
- **Observed failure**: Without the change in `src/`, the first case
  failed its oracle: "`ttc --out-dir` was to report
  match-nested-in-or-pattern at ...:9:22, ...:12:41, ...:15:26, ...:18:10
  and reports" those four plus `match-or-binding-mismatch` at 9:52, 12:41,
  15:56, and 18:40; the second case's `.errors.txt` differed (the
  `if let` message named `w` from the other side, as the nested `v` was not
  counted).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 TT_CASES=match- cargo test --test case_baselines`
  (every match diagnostics case, including the full matrix of
  `match-nested-in-or-pattern` and `match-or-binding-mismatch`): passed.
  The full gate runs once at the end of the batch (see TASK-712).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/sema.rs`, `src/sema/checker.rs`, two compiler cases and their
baselines, the baselines of two existing compiler cases, the matrix baseline
`match-nested-in-or-pattern_bindingAlternative_declarationInitializer.errors.txt`,
and `tests/oracle-failures.txt`. A nested alternative in an or-pattern is
reported once; alternatives that bind different names are still reported.
