# TASK-763: Validate editor buffers per file, lazily and cancellably

- **Status**: In progress
- **Started**: 2026-10-05
- **Completed**: —
- **Commit**: —

## Purpose

Every keystroke made the engine re-check and re-materialize the whole
project for every open document. Adopt TypeScript's language-server
structure (per-file checks, supersession, referenced-file closures) so that
editor validation does the work the current revision needs.

## Scope

- Included: request supersession in `ttc --server` and the VS Code adapter,
  per-file typed checks, contextual materialization scoped to a file's
  reference closure, design and regression coverage.
- Excluded: command-line and build checks (unchanged), debounce tuning,
  toolchain changes.

## Decisions

### Decision 1: Follow TypeScript's language server, not a timing change

- **Context**: Measured with the real adapter, ten keystrokes 350 ms apart in
  a 30-module chain project settled 5.9-7.4 s after the last one. Most of a
  validation was whole-project materialization (1.7-1.8 s) and a
  whole-program semantic check per open document.
- **Alternatives considered**: A longer debounce; caching the last answer;
  restricting validation to the active document; TypeScript's structure.
- **Decision and rationale**: The first three hide or delay the work and
  leave stale answers. TypeScript's server checks one file at a time, stops
  a pending check when the document changes, and knows each file's
  dependencies. See [the design](../design/editor-validation-scheduling.md).

### Decision 2: A per-file typed check ignores other files' syntax

- **Context**: The whole-program typed check stopped semantic diagnostics
  when any file had a syntax error.
- **Alternatives considered**: Keep the batch stop rule; check per file.
- **Decision and rationale**: TypeScript's `semanticCheck(file)` reports a
  file's semantic diagnostics regardless of other files, and the batch rule
  hid real type errors in the open file. The command line keeps `tsc`'s rule.

### Decision 3: A whole-project request after a file's may materialize

- **Context**: `scaling_tests` held that a whole-project request after
  hovers asks the checker nothing. A hover now settles only the hovered
  file's reference closure.
- **Alternatives considered**: Materialize the whole project for every
  question (the cost this task removes); keep the scoped and whole
  materializations apart (a hover after a whole-project request would
  materialize again).
- **Decision and rationale**: The first whole-project request settles the
  modules outside the closures already settled, and a repeated one asks
  nothing. Both kinds of run record, for each module, the digest of every
  served file it read, so either reuses the other's result while those
  files, the module's closure and the disk generation are unchanged. The
  test states this contract.

## Work log

- 2026-10-05: Measured per request with temporary timing (not committed):
  contextual materialization 1.5-1.8 s and the first typed check's backend
  ask 0.27-0.83 s per edit; later requests reused the program.
- 2026-10-05: Supersession: settle time 6.0 s → 5.4-6.1 s range improved
  about 10% (three runs each). Found and fixed an adapter defect: the engine
  client converted a `superseded` answer into `{ result: undefined }`.
- 2026-10-05: Per-file typed check: the scoped backend ask fell to 60-160 ms.
- 2026-10-05: Scoped materialization: settle time 1.6-2.3 s; the typed check
  of an unaffected module 0.19 s.
- 2026-10-05: Review found Issues 2-3; the full suite found Issue 4 and the
  protocol and API baselines (new `scope` and `supersedable` parameters,
  `Project::update_scoped`, `Project::check_file`), reviewed and accepted.

## Issues and resolutions

### Issue 1: Superseded answers crashed the typed layer

- **Symptom**: `TypeError: Cannot read properties of undefined (reading
  'backendError')` in the adapter; no final publish.
- **Cause**: The line reader built every non-error answer as `{ result }`.
- **Resolution**: The reader keeps `superseded` answers as such.

### Issue 2: Local declaration files were not global-scope roots

- **Symptom**: Found in review: a non-module `globals.d.ts` naming a type of
  a tt module that no module imports was not in another module's closure.
- **Cause**: The closure excluded declaration files from the files that
  affect the global scope; TypeScript's `isFileAffectingGlobalScope` does not.
- **Resolution**: Every local non-module file, declaration files included,
  is a root. `typescript::native::contextual_tests::
  a_reference_closure_follows_imports_and_global_declarations` pins the
  closure.

### Issue 3: References and renames read a file-scoped snapshot

- **Symptom**: Found in review: a reference in an importer of the asked file
  is outside that file's closure, so the importer's storage could be left
  unsettled while TypeScript searched it.
- **Cause**: Every service question served a snapshot scoped to the asked
  file.
- **Resolution**: A service question names what it reads. References and
  renames settle the whole project; questions about one file keep the scope.

### Issue 4: A whole-project materialization did not reach the file cache

- **Symptom**: `scaling_tests` showed a hover after a whole-project request
  materializing again.
- **Cause**: The whole-project path read the backend's stable generation
  after `contextual_emits` had already taken it, so it never recorded its
  results; and validity compared a digest of the request's roots, which a
  hover and a whole-project request name differently.
- **Resolution**: The whole-project path reads the generation its
  materialization recorded. Validity is per file read: a result is current
  while every served file it read is unchanged, the module's closure reads
  no served file it did not, and the disk generation is the same.

### Issue 5: A test failed only in a worktree whose `node_modules` was a link

- **Symptom**: `dependencies_of_a_configured_project_are_only_its_inputs`
  listed TypeScript's library files.
- **Cause**: The scratch worktree linked `node_modules` to another checkout,
  so the library files lay outside the project root. Not a defect of the
  change; with a copied `node_modules` the test passes.
- **Resolution**: None needed.

### Issue 6: A closed, unsaved host buffer kept its importers' types

- **Symptom**: Found in review: after closing an unsaved `dep.ts`, a scoped
  request kept annotations computed from the discarded buffer.
- **Cause**: A file the materialization read that was no longer served
  counted as unchanged, and closing a buffer does not advance the disk
  generation.
- **Resolution**: Every served file a materialization read must still be
  served with the same text.

### Issue 7: The closure ask rebuilt the checker's program on every request

- **Symptom**: Found in review: each file-scoped request sent every
  module's lowered text for the closure, then the materialized text for the
  check, so the checker's program changed twice per request.
- **Cause**: The closure was asked from lowered text and never kept.
- **Resolution**: The closure is asked from the texts that will be served
  (each module's unchanged materialization) and kept while those texts,
  sources, roots and the disk generation are unchanged.

### Issue 8: Whole-project records copied every path for every module

- **Symptom**: Found in review: memory and validity checks grew with the
  square of the module count.
- **Cause**: Each module's record held its own copy of the files read.
- **Resolution**: Modules materialized together share one record, and a
  request checks each record once.

### Issue 9: A disk change during materialization returned unrefined text

- **Symptom**: Found in review: if the disk changed while a closure was
  materialized, the target was served without its materialization.
- **Cause**: The results were recorded under the later generation and then
  judged against the earlier one.
- **Resolution**: The request falls back to the whole project.

The review's question whether a file-scoped check loses program
diagnostics placed in the file was checked against the native compiler:
`TS6053` (a missing `/// <reference path>`) and `TS2688` (a missing type
reference) are reported by both the whole-program and the file-scoped check.

## Regression test (fails before the fix)

- **Path**: `src/typescript/native.rs`,
  `a_reference_closure_follows_imports_and_global_declarations`.
- **Observed failure**: With declaration files excluded from the global
  roots (Issue 2), the closure was `leaf.ts`, `middle.ts`, `target.ts`,
  without `globals.d.ts` and the module it names.

- **Path**: `src/lib/scaling_tests.rs`,
  `a_file_question_after_closing_an_unsaved_dependency_reads_the_disk`.
- **Observed failure**: With a file no longer served counted as unchanged
  (Issue 6), the target kept `let $tt_v0: (boolean) | (number[])` from the
  closed buffer instead of the disk's `string`.

Issues 1, 3, 4, 7, 8 and 9 were found before this task's first commit. Equivalence of
scoped and whole-project materialization is pinned by
`engine::scoped_tests`; supersession by the `server` unit tests.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change
- [ ] `./scripts/ci`

## Result

Pending.
