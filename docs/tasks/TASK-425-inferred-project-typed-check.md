# TASK-425: Check a requested file outside the configuration in its inferred project

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

For an open document outside the tsconfig `include`, `typedCheck` with
`includeTypes: true` reported no TypeScript diagnostic and no `val`
built-in-mutator diagnostic, while `tsDiagnostics` reported TS2322 for the
same buffer. The extension merges typed results with `replacesTypes`, so the
empty typed answer removed the service's TS2322 from the editor.
`ttc --check-types other/x.tt` was silent in the same way.

## Scope

- Included: The roots the typed check sends to the TypeScript host
  (`src/typescript/backend.rs`, `src/typescript/native.rs`,
  `src/typescript/contextual.rs`, `src/engine/project.rs`) and how the host
  answers a root the configured program does not contain
  (`src/typescript/host.mjs`).
- Excluded: Declaration emit (`--types`) still emits only modules of the
  configured program. Unrequested candidate files outside `include` still
  receive no checker question (`files_outside_tsconfig_do_not_receive_typed_queries`).

## Decisions

### Decision 1: A requested or open file is answered by its default project

- **Context**: The host answered every question from the configured
  project and filtered each question to modules that project contains, so a
  file outside `include` had no answers at all. TASK-352 Decision 6 had
  covered the file's match coverage from declarations and had rejected
  adding the file to the configured program, because overriding `include`
  makes ttc disagree with `tsc` about what the project is. The language
  service does something else: TypeScript's project system gives every open
  file a default project, which is the configured project that contains it
  or, when none does, an inferred project for it (the TypeScript API's
  `UpdateSnapshotParams.openFiles`: "if found, that configured project is
  loaded and becomes the file's default project. Otherwise the file is
  loaded into the inferred project"; `Snapshot.getDefaultProjectForFile`).
  That is how `tsDiagnostics` reported TS2322.
- **Alternatives considered**:
  - Add the file to the configured program: rejected for the reason
    TASK-352 Decision 6 gives.
  - Keep reporting nothing for such a file: the typed answer then
    contradicts the service for the same buffer and deletes its diagnostic
    in the editor.
- **Decision and rationale**: The query carries `roots`: the modules of the
  files the caller named explicitly (`Project::named`, the file inputs of
  `open_project`) and of the documents open in the project (its overlays),
  which is the set an editor has opened. A file found under a named
  directory is not a root: a directory input is a project check, and there
  the configuration's `include` and `exclude` decide, as `tsc -p` does
  (`extended_config_patterns_preserve_exclusions_and_authored_config`). The
  host keeps the configured project as it is. A root the configured program
  does not contain is opened with `openFiles` (and closed with `closeFiles`
  once it stops being a root), and its default project answers the same
  questions for that module: its syntactic and semantic diagnostics, the
  contextual slot, exhaustiveness, `Result` shape and `val` symbol
  questions. The answered modules are reported in `projectModules`, so the
  report treats the file as checked. Configuration, program and global
  diagnostics come only from the configured project. The contextual pass
  sends the same roots, so the annotation facts and the check come from the
  same project and the host does not reopen the file between them. This
  narrows TASK-352 Decision 6, and a note at the top of that record says so;
  its fallback still covers files no checked project contains.

## Work log

- 2026-09-27: Reproduced: `ttc --check-types other/x.tt` exited 0 with no
  output; the server's `typedCheck` answered `[]` while `tsDiagnostics`
  answered TS2322 at 0:6.
- 2026-09-27: Read the TypeScript API's `UpdateSnapshotParams` and
  `Snapshot` declarations for the default-project rule.
- 2026-09-27: Added `Query::roots`, `Project::roots`, `Project::named`
  (set in `Engine::open_project`), the `roots` job field,
  the `materialize` parameter, and the per-project answering in the host
  (`contextual` and `answer` over the configured project and each default
  project of an outside root).
- 2026-09-27: Added
  `a_requested_file_outside_the_configuration_is_checked_in_its_inferred_project`
  (`tests/native/cases_03.rs`), covering the CLI with the file named, the
  CLI with only `src` named (the outside file stays unchecked), and the
  server after another document opened the project first.

## Issues and resolutions

### Issue 1: A file opened after the project existed was not a requested input

- **Symptom**: While drafting the roots from `Project::requested` alone, a
  server session that opened `src/a.tt` first and then checked
  `other/x.tt` would not have treated `x.tt` as a root: the project was
  opened for `a.tt`, and `requested` holds only the inputs it was opened
  with.
- **Cause**: `requested` records what an emitting run writes, not what an
  editor has open.
- **Resolution**: Roots also include every document open in the project,
  which is the language service's own definition of the files that get a
  default project. The regression test opens `src/a.tt` before `other/x.tt`.

### Issue 2: The TASK-352 regression test pinned the declaration fallback

- **Symptom**: `a_named_file_the_project_excludes_is_still_checked`
  (`tests/native/cases_01.rs`) failed: `ttc --check-types lib/outside.tt`
  on `match (C.A) { A => 1 }` reported nothing and exited 0.
- **Cause**: The file is now answered by the checker, and the checker's
  alphabet is the narrowed type: `C.A` is `{ kind: "A" }`, so the match is
  exhaustive. The same source inside `include` gives the same answer; the
  test's scrutinee only failed because the declaration fallback ignores
  narrowing.
- **Resolution**: The test now matches over a parameter `c: C`, whose type
  is the whole union, and its comment states the new contract. The output of
  `--check-types` for that file is identical inside `src/` and in `lib/`
  (`match is not exhaustive: missing "B"` at 2:28, exit 1).

### Issue 3: A directory input put excluded files into their inferred project

- **Symptom**: With roots taken from `Project::requested`,
  `extended_config_patterns_preserve_exclusions_and_authored_config`
  (`tests/workflow_repairs.rs`) failed: `ttc --check-types src` reported
  TS2322 in `src/excluded.tt`, which the configuration excludes.
- **Cause**: `requested` also holds every file collected under a named
  directory, so an excluded file there became a root.
- **Resolution**: Roots come from `Project::named`, the explicitly named
  files, plus open documents (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: all suites passed.
- [x] `./scripts/ci extension`: 204 tests passed, none skipped.
- [x] `node scripts/check-task-index`
- [x] With the source changes reverted, the new test fails
  (`--check-types other/x.tt` exits 0); with them it passes.
- [x] `ttc --server`: `typedCheck` of the open outside document reports
  `ts2322` at 1:19 and `val-mutation` at 3:1, matching `tsDiagnostics`;
  after `closeDocument` a one-off `typedCheck` still reports both.

## Result

Changed `src/engine/mod.rs`, `src/engine/project.rs`,
`src/engine/semantics/report.rs`,
`src/typescript/backend.rs`, `src/typescript/contextual.rs`,
`src/typescript/host.mjs`, `src/typescript/native.rs`,
`tests/native/cases_01.rs`, `tests/native/cases_03.rs`, and the note at the top of
`docs/tasks/TASK-352-developer-surface-structural-fixes.md`.
