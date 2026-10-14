# TASK-804: List the incremental differences the TypeScript service causes

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-804: List the incremental differences the TypeScript service causes`

## Purpose

After TASK-801, 12 of the nightly incremental sweep's 81 differing samples
still differed in content. Each was traced to the pinned TypeScript
(`@typescript/typescript-linux-x64` 7.1.0-dev.20260826.1), not to ttc. The
maintainer chose to record them, as `tests/typed-parity-differences.txt`
records typed-parity differences, rather than change the pinned TypeScript
or work around the service in ttc.

## Scope

- Included: `tests/incremental-differences.txt` and its reading in
  `tests/incremental.rs`.
- Excluded: upgrading TypeScript; the engine.

## Decisions

### Decision 1: A line names a unit, not a sample

- **Context**: A sample's label carries a seed derived from the case's
  index in the corpus, so adding a case renames later samples (the
  `importedVariantExhaustiveness` sample had different labels on 0776bbb8
  and on this branch).
- **Decision and rationale**: A line names `<case>#<unit>`, which survives
  corpus growth, and covers every sample of that unit.

### Decision 2: `stable` and `unstable`

- **Context**: Some differences reproduce every time; others come and go
  on identical inputs.
- **Decision and rationale**: A `stable` line fails a run of every sample
  (`TT_INCREMENTAL=all`) when none of the unit's samples differs, so a fixed
  TypeScript removes it; an `unstable` line accepts either outcome. A full
  run also fails on a line that names no unit of the corpus. A line is
  `by-design` (citing docs/) or `defect` (naming its TASK), as in the
  typed-parity list.

## Work log

- 2026-10-14: Reran each of the 12 content-differing samples six times:
  `editorRecoveryLexicalGuards.ttx` (namespace 5/6, nestedNamespace 4/6
  and 1/6, conflict 1/6), `editorRecoveryStillRejectsIncompleteHost.tt`
  missingInitializer (0/6 and 4/6), `noSubstitutionTemplateDynamicImportIsRewritten2.tt`
  (6/6, 0/6, 0/6), `flow-first-step-method_*` (0/6 and 0/6).
- 2026-10-14: Evidence per unit, below; wrote the list and its reading.
  `TT_INCREMENTAL_ONLY` reruns of a listed sample pass; the default sample
  passes.

## Issues and resolutions

### Issue 1: A name several script files declare answers nondeterministically

- **Symptom**: `editorRecoveryLexicalGuards.ttx` (three script files that
  each declare `good` and `bad`) differs in completion (`bad` with sort
  text 11 or 15, `good` present or not); `editorRecoveryStillRejectsIncompleteHost.tt`
  (two script files declaring `flag`, `broken`, `later`) differs in which
  file definition of `later` goes to.
- **Cause**: With `didOpen`/`didChange` and completion logged in
  `typescript::service`, two fresh workspaces sent byte-identical messages
  and tsgo answered `bad:15, good:15` in one and `bad:11` in the other.
- **Resolution**: Listed `unstable`.

### Issue 2: An unresolved dynamic import's definition depends on earlier edits

- **Symptom**: `noSubstitutionTemplateDynamicImportIsRewritten2.tt`, final
  text ``const m = import(`../view.ttx = import({ type: 'module' } });``:
  the edited project answers definition at 0:21 with nothing, a fresh one
  with a location in the nonexistent module.
- **Cause**: Reproduced with `tsgo --lsp` alone (a Node driver, no ttc):
  opened with the final text it answers a location; opened with
  ``const m = import(`../view.ttx = import{ type: 'module' } });`` and
  changed (whole-document `didChange`) to the final text, it answers `[]`.
- **Resolution**: Listed `stable`.

### Issue 3: A 134-byte file costs the checker 4.5 GB

- **Symptom**: `verifyPreflightsUnbalancedDelimitersBeforeSwc.tt`: the
  fresh check failed with `Unexpected EOF while reading from child process`
  after about 550 s; the edited project's did not.
- **Cause**: The pinned `tsc -p` alone takes 23 s and a 4.5 GB resident set
  on the file; beside the sweep's other workers the process dies.
- **Resolution**: Listed `unstable`. The same run's `flow-first-step-method`
  samples timed out (`textDocument/diagnostic timed out after 8s`, the
  content mapper's 5 s `initialize`) while it ran, and passed 6/6 alone;
  they are not listed.

## Regression test (fails before the fix)

Not applicable: the change fixes no ttc bug; it records differences the
TypeScript service causes.

## Verification

- [x] `TT_INCREMENTAL=all TT_INCREMENTAL_ONLY=<listed sample>` for the
  `noSubstitution…#8186ecf6d407bc71` and `nestedNamespace…#68379a725fd9558`
  samples
- [x] `cargo test --release --test incremental` (default sample)
- [ ] `TT_INCREMENTAL=all cargo test --release --test incremental` (full run, with TASK-801)

## Result

`tests/incremental-differences.txt`, `tests/incremental.rs`.
