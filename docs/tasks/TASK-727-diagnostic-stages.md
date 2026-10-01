# TASK-727: Stop at the stage where `tsc` stops

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-727`

## Purpose

TASK-717 D2: `tsc` reports a program in stages and stops before the type
errors when an earlier stage reported anything; the typed pass gathered
every stage at once, so `ttc --check-types` and the server's `typedCheck`
reported the checker's diagnostics beside an option error (`jsxFactory:
"id1 id2"` gave TS5067 and TS2322, `tsc` TS5067 alone), a syntax error in a
hand-written `.ts` file, or a `.tt` file whose TypeScript does not parse.

## Scope

- Included: the order and short-circuit of TypeScript's diagnostics in
  `src/typescript/host.mjs`, for every surface that reports them (the CLI,
  the server's `typedCheck`, and through it the editor's typed layer); the
  two facts the engine must give the host for it (`Query::unparsed_documents`,
  `Query::syntax_blocked`); regression cases; `docs/ai/tt.md`;
  `tests/typed-parity-differences.txt`.
- Excluded: the declaration stage (TASK-728); the language service's
  per-document diagnostics, which `tsgo --lsp` gathers without stages.

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/compiler/program.go`, `GetDiagnosticsOfAnyProgram` (lines
  1942-1995): configuration-file parsing diagnostics; syntactic
  diagnostics; only when none was added after the configuration's, program
  (option) diagnostics, then, unless `listFilesOnly`, global diagnostics;
  only when still none was added, semantic diagnostics and global
  diagnostics again. A content mapper's locationless failure is appended
  with the syntactic stage.
- `tsc/internal/compiler/checkerpool.go`, `GetGlobalDiagnostics` (line
  460): the first global stage reports what creating the checkers produced.
- `tsc/internal/api/session.go`, `handleGetGlobalDiagnostics` (lines
  3771-3799): the API runs a full semantic pass before answering, so the
  host cannot ask for the first global stage without checking.
- `tsc/internal/ls/diagnostics.go`, `getAllDiagnostics`: the language
  service answers one document's syntactic and semantic diagnostics
  together, without stages.

## Decisions

### Decision 1: The host gathers the program's diagnostics in `tsc`'s stages

- **Context**: The host asked every class at once and reported them all.
- **Alternatives considered**: (a) Filter the semantic diagnostics in the
  report when a program diagnostic exists: the report would decide a
  TypeScript rule from codes after the fact. (b) Leave the CLI as it is
  and document the difference: contract 2 makes these TypeScript's
  diagnostics, reported as `tsc` reports them.
- **Decision and rationale**: The host follows `GetDiagnosticsOfAnyProgram`
  for each program it answers: the configuration's diagnostics; the
  syntactic ones; the program's only when no syntactic one was reported;
  the semantic ones (and the specifier check that stands for TS2307 on a
  lowered module's served name, a semantic diagnostic in `tsc`) and the
  global ones only when neither was; nothing is asked under
  `listFilesOnly`. A stage stops the next only with what is reported: an
  inferred project's positionless option diagnostics are not reported
  (TASK-388 Decision 3), so they stop nothing. The first global stage is
  not asked: the API checks the whole program to answer it
  (`session.go`), which would report a global diagnostic that `tsc` finds
  only while checking and so stop a stage `tsc` runs; the global
  diagnostics after the semantic stage are asked as before. This is the
  same limitation the oracle has (TASK-717 Decision 1).

### Decision 2: A file ttc blocks because its TypeScript does not parse is a syntactic diagnostic of the program

- **Context**: A `.tt` file whose TypeScript does not parse is blocked
  (`verify-failed`, `source-not-typescript`, `DiagnosticCode::restates_typescript_syntax`)
  and served as `export {};`, so TypeScript sees no syntax error, while
  `tsc` on the twin reports only the syntax error.
- **Alternatives considered**: Serve the unparsed text and let TypeScript
  report it: the engine blocks the file precisely so that the tt
  restatement is the one report (TASK-527).
- **Decision and rationale**: The engine names those modules
  (`Query::syntax_blocked`), and the host counts one the program contains
  as a syntactic diagnostic it does not report.

### Decision 3: An open document that does not parse keeps its type errors

- **Context**: TASK-561 Decision 1 checks a document held open whose
  TypeScript does not parse through its faithful projection, so the
  editor keeps the checker's facts while the syntax error is transient.
  Counting its syntax error would stop the semantic stage for it.
- **Alternatives considered**: (a) Answer the editor with the language
  service's ungated diagnostics for every document: the server's
  `typedCheck` is one answer for the editor and for other consumers (and
  TASK-717's server surface), and the other files of the program are not
  documents being edited. (b) Count it: undoes TASK-561.
- **Decision and rationale**: Such a document's syntactic diagnostics are
  reported but stop no stage (`Query::unparsed_documents`), as
  `getAllDiagnostics` answers a document. A file read from disk is still
  blocked and stops the stages (Decision 2).

## Work log

- 2026-10-01: Reproduced D2 with TASK-717's repro and with a `.ts` file
  holding `export const y = (1;` beside a `.tt` type error (`tsc`: TS1005
  only; ttc: TS2322 as well).
- 2026-10-01: Read `GetDiagnosticsOfAnyProgram`, `checkerpool.go`,
  `session.go` and `ls/diagnostics.go`; staged the host's `answer`, added
  `unparsedDocuments` and `syntaxBlocked` to the job
  (`src/typescript/backend.rs`, `src/typescript/native.rs`,
  `src/engine/projection.rs`).
- 2026-10-01: Added three cases; ran the D2 corpus cases, the CLI,
  native, sidecar, integration suites, every case baseline, and the
  extension's tests; updated `docs/ai/tt.md` and the list.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/anOptionErrorStopsBeforeTheSemanticStage.tt`,
  `tests/cases/compiler/aSyntaxErrorInHandWrittenTypeScriptStopsBeforeTheSemanticStage.tt`,
  `tests/cases/compiler/aTtFileThatIsNotTypeScriptStopsBeforeTheSemanticStage.tt`
  (`cargo test --test case_baselines`).
- **Observed failure**: with the previous host and engine, all three
  `.errors.txt` baselines differed by an added
  `+ error[ts2322]: Type 'string' is not assignable to type 'number'.` at
  `a.tt:1:14` under `ttc --check-types`.

## Verification

- [x] The nine D2 corpus cases (`TTC_TYPED_FILTER`, `TTC_TYPED_CASES=all`):
  every one agrees with `tsc`; seven lines removed, one
  (`bundlerImportTsExtensions.ts`) left with its by-design TS6054 wording,
  and the five lines TASK-726 kept are gone.
- [x] `RUST_TEST_THREADS=4 TTC_REQUIRE_TSGO=1 cargo test --test native
  --test cli --test sidecar --test case_baselines --test integration`: all
  pass (every case baseline unchanged but the new cases).
- [x] Extension tests (`node --test server/out/test/*.test.js
  client/out/test/*.test.js` with `target/debug` on `PATH`): 238 pass,
  including TASK-561's open-document check.
- [x] The full gate is recorded in TASK-731.

## Result

Changed files: `src/typescript/host.mjs`, `src/typescript/backend.rs`,
`src/typescript/native.rs`, `src/engine/projection.rs`, `docs/ai/tt.md`,
three cases and their baselines, `tests/typed-parity-differences.txt`,
`docs/tasks/INDEX.md`, and this record. Every D2 difference is gone.
Remaining: the first global stage (Decision 1) cannot be asked through the
API.
