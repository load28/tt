# TASK-713: Make the tt-only typed check the full check's tt layer

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-713`

## Purpose

TASK-700 Issue 5: `ttc --check-types --tt-only` and the server's
`typedCheck` (tt-only unless `includeTypes`) drop `result-return-nested`,
which plain `ttc --check-types` and the editor report. `--tt-only` is
documented as the tt layer of `--check-types`, so the two must report the
same tt diagnostics by one rule.

## Scope

- Included: how the typed report applies `CheckRequest::tt_only`
  (`src/engine/semantics/report.rs`, `src/engine/semantics.rs`,
  `src/engine/project.rs`), a CLI test, and
  `tests/oracle-failures.txt`.
- Excluded: what the typed check reports otherwise.

## Sources

- TASK-700 Issue 5 and TASK-701 Issue 3 (the defect and where it shows).
- `ttc --help`: `--tt-only` reports "the tt layer of --check-types".

## Decisions

### Decision 1: The tt layer is a filter over the one report, by code

- **Context**: The report decided per section whether a tt-only check
  should see it: the TypeScript diagnostics and the project diagnostics
  were skipped, and so was the `result-return-nested` section, because the
  checker answers it. A section a later change adds has to remember the
  rule again.
- **Alternatives considered**: (a) Drop the one `if !tt_only` around the
  result-shape section: fixes this code, keeps the rule spread over the
  sections. (b) Build the whole report and keep, for a tt-only check, the
  diagnostics whose code is a tt rule (`DiagnosticCode::parse`): one rule,
  in one place, that holds whatever answered a tt rule.
- **Decision and rationale**: (b), as `Diagnostic::states_tt_rule` and one
  `retain` before the report is sorted. Translating TypeScript's
  diagnostics for a tt-only check costs a pass over the checker's answers,
  which the check already has; it does not ask the checker anything more.

## Work log

- 2026-10-01: Reproduced with the 49 `result-return-nested` cases
  (`TT_CASES=result-return-nested cargo test --test case_baselines` without
  the `tests/oracle-failures.txt` line).
- 2026-10-01: Made `tt_only` a filter, documented `CheckRequest::tt_only`,
  added `tt_only_reports_the_full_check_without_typescripts_diagnostics`,
  and removed the listed defect.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: the 49 cases under
  `tests/cases/conformance/diagnostics/result-return-nested/` and
  `result-return-nested_explain1` (`cargo test --test case_baselines`), and
  `tests/cli/cases_01.rs`,
  `tt_only_reports_the_full_check_without_typescripts_diagnostics`.
- **Observed failure**: Without the change in `src/`: "49 case(s) failed:
  result-return-nested_explain1: `typedCheck` reports nothing ... was to
  report result-return-nested at main.tt:5:12-5:23 and reports nothing",
  and the CLI test's `--tt-only` report had the `val-mutation` diagnostic
  but not the `result-return-nested` one the full check printed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the full gate is recorded in TASK-715.
- [x] Baseline changes reviewed and committed with the change (none).

## Result

Changed `src/engine/semantics/report.rs`, `src/engine/semantics.rs`,
`src/engine/project.rs`, `tests/cli/cases_01.rs`, and
`tests/oracle-failures.txt`. Every surface that asks for the tt layer gets
the full check's tt diagnostics.
