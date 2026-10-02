# TASK-671: Report TypeScript that stops before a tt construct as the user's TypeScript

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-671`

## Purpose

In a `.ttx` file, an opening tag left unclosed while typing before a
`{match ...}` child (`<span` then `{match (s) { ... }}`) was reported, in
the editor and the CLI, as `error[lowering-plan-failed]: tt host lowering
could not plan this construct: the generated TypeScript for it does not
parse: Expected '</', got '('`, an internal failure over the whole match.
The `.tsx` twin gets TypeScript's TS1005 (`'...' expected`) at the token
after the `{`. The user must see TypeScript's parse error, or a located tt
error, never an internal lowering failure (TASK-640's rule for copied
text).

## Scope

- Included: `parse_failure_at` in `src/program_syntax/collector.rs`, the
  compiler case `unclosedJsxTagBeforeMatch.ttx`, and the editor case
  `matchAfterUnclosedJsxTag.ttx`.
- Excluded: swc's wording of its error (it names the placeholder's `(`);
  the editor shows TypeScript's own TS1005 instead (see Result).

## Decisions

### Decision 1: A parse that stops at the first byte of an outermost placeholder is the source's failure

- **Context**: The projection that models a tt-bearing file's TypeScript
  is copied source with each tt construct replaced by a placeholder in the
  smallest form of the syntactic category the parser claimed it in: a
  parenthesized name for a value (`($tt_syntax_expr_0)`), a block for a
  statement, a `const` for an item. A parse failure is classified by the
  byte it stopped at (`parse_failure`): copied text is the user's
  TypeScript (`source-not-typescript`), generated text is ttc's
  (`lowering-plan-failed`). Here swc stopped at the placeholder's `(`,
  because inside an opening tag a `{` can only begin a spread attribute
  (`{...expr}`), so no expression may start there.
- **Alternatives considered**:
  - Recognize JSX tags before a claimed construct in the parser and
    refuse the claim: a second model of JSX attribute grammar in tt's
    parser, and every other place where no value may stand would need its
    own rule.
  - Reword by the swc message: message inference, which the classifier
    rules out by design.
- **Decision and rationale**: When the failing byte is the first byte of a
  placeholder that no other placeholder encloses, only copied source
  precedes it, and the placeholder is the least that its category can be:
  what does not parse is the source text leading up to the construct.
  `outermost_placeholder_at` then classifies the failure as
  `source-not-typescript` at the construct's source start, where
  TypeScript's parser stops on the same text (TS1005 at `match` in the
  twin's position). A failure inside a placeholder, or at a placeholder
  nested in another's generated text, stays `lowering-plan-failed`, since
  there the generated text is at fault.

## Work log

- 2026-09-30: Added the probe's case as the editor case
  `matchAfterUnclosedJsxTag.ttx` (`@diagnostics: *`) and generated its
  baseline with the unfixed compiler: the published list had
  `lowering-plan-failed` over the match and no TS1005.
- 2026-09-30: Added `outermost_placeholder_at`; regenerated the editor
  baseline; added the compiler case `unclosedJsxTagBeforeMatch.ttx`.
- 2026-09-30: Ran the library, `compile`, `case_baselines`, `snapshot`,
  `fuzz_regressions`, `cli`, and `practical_diagnostics` suites.

## Issues and resolutions

### Issue 1: A `variant` written as a value was pinned as `lowering-plan-failed`

- **Symptom**: `a_generated_parse_failure_is_located_at_the_construct_that_generated_it`
  (`tests/compile/cases_11.rs`) failed: `const x = variant Dir { Up, Down };`
  and `f(variant Dir { Up, Down });` now report `source-not-typescript` at
  the variant.
- **Cause**: TASK-458 left a variant in expression position reporting
  `lowering-plan-failed` at the variant (its Scope, "Excluded"); the parse
  stops at the first byte of that variant's item placeholder, the same
  structure as this task's.
- **Resolution**: The same rule applies: no declaration may stand there,
  whatever ttc generates, so the report is the user's program not parsing
  at the variant. The test is renamed
  `a_construct_its_position_does_not_admit_is_reported_at_the_construct`
  and expects `source-not-typescript` at the same place; TASK-458 carries a
  note, and `docs/ai/tt.md` says so beside `lowering-plan-failed`.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/matchAfterUnclosedJsxTag.ttx`
  (`tests/baselines/reference/editor/matchAfterUnclosedJsxTag.baseline`)
  and `tests/cases/compiler/unclosedJsxTagBeforeMatch.ttx`
  (`tests/baselines/reference/unclosedJsxTagBeforeMatch.errors.txt`).
- **Observed failure**: Without the fix the editor case published
  `10:8-10:46 "match (s) { ... }" error lowering-plan-failed (ttc): tt host
  lowering could not plan this construct: the generated TypeScript for it
  does not parse: Expected '</', got '('` and no TS1005, and the CLI
  reported the same `lowering-plan-failed`; the committed baselines have
  TypeScript's `ts1005 '...' expected.` at `match` in the published list
  and `source-not-typescript` at 9:8 in the CLI.

## Verification

- [x] `UPDATE_EXPECT=1` for the two cases, baselines read.
- [x] `cargo test --lib --test compile --test case_baselines --test
  snapshot --test fuzz_regressions --test cli --test
  practical_diagnostics`: see the work log; all passed.
- [x] Baseline changes reviewed and committed with the change.

## Result

The CLI reports `error[source-not-typescript]: the TypeScript here does
not parse: ...` at the match; the editor publishes TypeScript's TS1005
there, since the service restates `source-not-typescript` (TASK-527) and
the typed pass states TS1005 in TypeScript's words. Changed files:
`src/program_syntax/collector.rs`, `tests/compile/cases_11.rs`, the two cases
and their baselines, `docs/ai/tt.md`, the note in TASK-458,
`docs/tasks/INDEX.md`, and this record.
