# TASK-708: Retire `stray-result`, which no `result` block can reach

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-708`

## Purpose

TASK-700 (Issue 3) found that no program reports `stray-result`:
`program.stray_results` (`src/parser/parse.rs`) is created empty and never
filled, so the checker's loop over it never runs, while `ttc explain` lists
the code and its explanation describes a "`result` block was claimed but
could not be parsed". Either the detection exists as the documentation
describes, or the code and its dead path go.

## Scope

- Included: the `DiagnosticCode::StrayResult` variant, its wire name,
  explanation, and projection-blocking entry (`src/diagnostics.rs`), its
  numbered slot (kept, retired), `Program::stray_results` (`src/ast.rs`),
  its creation in the parser and its loop in `src/sema/checker.rs`, the
  unused second return value of `parse_result_block`
  (`src/parser/results.rs`), the unit tests that pin code numbers and the
  code count, `tests/diagnostic-codes-without-cases.txt`, the public API
  baseline, and `docs/ai/tt.md`.
- Excluded: how an unbalanced `result {` is reported (it is a TypeScript
  syntax error and is reported as `source-not-typescript`; where TypeScript
  places it is TASK-707's and TASK-711's question).

## Sources

- `docs/ai/tt.md` (result block): "`result` is contextual: a block is
  claimed only when a speculative parse finds a tt `try` whose nearest
  lexical Result scope is that block", and a block without one "stays a
  TypeScript identifier plus block statement".
- `src/parser/results.rs`: the speculative parse of the body is the
  parser's infallible statement-list parse, so a block whose braces balance
  is either claimed whole or passed through; with unbalanced braces it is
  passed through.
- TASK-476 (Decision 1): `NUMBERED_CODES` is append-only and a retired code
  keeps its slot (`Numbered::Retired`), so `tt5` keeps meaning
  `stray-result` and `ttc explain` says it is retired, as it does for
  `result-missing-keyword` (tt8) and `result-tail-semicolon` (tt33).
- rustc retires an error code the same way: `compiler/rustc_error_codes`
  keeps the number, and the explanation of a code that is no longer emitted
  says so (`#### Note: this error code is no longer emitted by the
  compiler.`).

## Decisions

### Decision 1: Retire the code instead of adding a reporting site

- **Context**: The claim rule decides a `result` block by a successful
  speculative parse. There is no state in which a block is claimed and then
  fails to parse, which is what the code describes.
- **Alternatives considered**: (a) Report `stray-result` for an unbalanced
  `result {` whose partial body holds a `try`: the text is a TypeScript
  syntax error that TypeScript reports itself ("'}' expected"), and
  `source-not-typescript` already reports it, so this would be a second
  diagnostic for one cause or would replace TypeScript's own verdict with a
  tt one, against the error-layer contract (AGENTS.md, contract 2). (b) Keep
  the code listed as unreachable: an explained code no program can report
  documents a rule that does not exist, and the dead field invites a
  reporting site that would duplicate (a). (c) Retire it.
- **Decision and rationale**: (c). The variant, its explanation, the
  `Program` field, the parser vector, and the checker loop are removed; the
  slot becomes `Numbered::Retired("stray-result")`, so `ttc explain
  stray-result` and `ttc explain tt5` print that the code is retired, and
  every later number is unchanged. `DiagnosticCode` is `#[non_exhaustive]`;
  a consumer that named the variant no longer compiles, as with the two
  earlier retirements, and the public API baseline records the change.

## Work log

- 2026-10-01: Confirmed the claim rule in `src/parser/results.rs` and that
  `stray_results` has no writer (`git log -S stray_results.push` ends at the
  commit that introduced scoped `result` blocks).
- 2026-10-01: Removed the variant and the dead path; retired the slot;
  pinned `retired("tt5")` and the new count (48) in
  `src/diagnostics/tests.rs`; removed the line from
  `tests/diagnostic-codes-without-cases.txt`; reworded `docs/ai/tt.md`
  (diagnostics paragraph); `UPDATE_EXPECT=1 cargo test --test public_api`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/diagnostics/tests.rs`
  (`code_numbers_are_stable_and_start_at_one`,
  `every_rule_is_listed_once_and_explained`) and `tests/case_baselines.rs`
  (`every_diagnostic_code_has_cases`, with the without-cases line removed).
- **Observed failure**: Without the change in `src/`, `retired("tt5")` was
  `None` where `Some("stray-result")` was expected, `ALL.len()` was 49, not
  48, and `every_diagnostic_code_has_cases` reported "`stray-result` has no
  case that expects it".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib` (412 passed), `cargo test --test public_api`,
  `TTC_REQUIRE_TSGO=1 TT_CASES=result-no-success cargo test --test
  case_baselines` (the code checks over every case file): passed. The full
  gate runs once at the end of the batch (see TASK-712).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/diagnostics.rs`, `src/diagnostics/tests.rs`, `src/ast.rs`,
`src/parser/{parse,results}.rs`, `src/sema/checker.rs`,
`tests/diagnostic-codes-without-cases.txt`,
`tests/baselines/reference/api/ttc.api.txt`, and `docs/ai/tt.md`.
`stray-result` (tt5) is retired; every other code keeps its number.
