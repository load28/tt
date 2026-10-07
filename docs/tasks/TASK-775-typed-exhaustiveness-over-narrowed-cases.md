# TASK-775: Judge a match's exhaustiveness on the typed path by the cases its value can still be

- **Status**: Cancelled
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: see `git log --grep TASK-775`

## Purpose

The fifth audit's K4 (TASK-773 decision 21): after
`if (s.kind === "Rect") return 0;` a match on `s` needs a `Rect` arm (the
declared cases), which `--check-types` then reports as TS2678. The user
first chose to have the typed surfaces read the scrutinee's narrowed type
instead, so that such an arm would not be required there. This task tried
it and was cancelled when that rule turned out to be the one TASK-725
decision 1 had removed.

## Scope

- Included: the typed report's match coverage
  (`src/engine/semantics/report.rs`), its documentation (`docs/ai/tt.md`),
  and a case pinning both surfaces.
- Excluded: the untyped coverage, which stays the declared cases.

## Decisions

### Decision 1: Keep TASK-725's rule; cancel this task

- **Context**: The implementation answered each match the checker named
  from the narrowed alphabet and dropped the declared verdict for it. The
  full suite then changed one baseline,
  `aMatchCoversItsDeclaredCasesWhateverNarrowingRemoved.errors.txt`, a
  case TASK-725 wrote to pin the opposite rule. TASK-725 decision 1 gave
  two reasons: a CI that runs `--check-types` passes a program the build
  rejects, and the editor's text layer reports the declared hole anyway
  (`editors/vscode/server/src/diagnostics.ts`, `mergeTyped` replaces only
  what the typed layer states), so `--check-types` would be the one
  surface that disagrees.
- **Alternatives considered**: (a) Go ahead and reverse TASK-725
  decision 1, changing `mergeTyped` as well, and accept that the typed
  surfaces pass programs the build rejects. (b) Keep the declared cases on
  every surface; `_` is the one arm every surface accepts, and TS2678 on a
  narrowed-away arm already carries that advice (TASK-773 decision 21).
- **Decision and rationale**: Put to the user with TASK-725's reasons, the
  user chose (b). The code, documentation, and case of this task are
  reverted; TASK-773 decision 21 and TASK-725 decision 1 stand.

## Work log

- 2026-10-07: Read the typed report's coverage flow and
  `analysis::checked_coverage`; implemented the narrowed verdict and added
  `aCaseNarrowedAwayBeforeTheMatchIsNotRequiredWhenTyped` (the build
  reported `Rect` missing, `--check-types` did not).
- 2026-10-07: The full suite failed only on
  `aMatchCoversItsDeclaredCasesWhateverNarrowingRemoved` (its
  `--check-types` section lost the `Rect` hole). Found TASK-725 decision 1,
  asked the user, and reverted the task's changes on their answer.

## Issues and resolutions

### Issue 1: The chosen rule was one an earlier task had removed on purpose

- **Symptom**: The case TASK-725 added to pin "the declared cases on every
  surface" changed.
- **Cause**: The K4 question offered the narrowed rule without citing
  TASK-725 decision 1, which had rejected it.
- **Resolution**: The user was asked again with TASK-725's reasons and
  kept TASK-725's rule; this task's changes are reverted.

## Regression test (fails before the fix)

Not applicable: the task is cancelled and changes no behavior.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the code is back to the commit before this task)
- [x] No baseline changes remain

## Result

Cancelled. `src/engine/semantics/report.rs` and `docs/ai/tt.md` are as
before this task, and its case is removed.
