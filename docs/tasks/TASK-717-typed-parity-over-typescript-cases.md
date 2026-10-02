# TASK-717: Hold `ttc --check-types` to the pinned `tsc` over TypeScript's own test cases

> Follow-up: TASK-726, TASK-727 and TASK-728 fix D1, D2 and D3; TASK-729 classifies D4 as by design (`tsc --runExternalCode` writes the same name). TASK-730 adds a declaration map's sources to the oracle's reachability (Decision 3).

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-717`

## Purpose

TASK-638 holds every TypeScript unit of TypeScript's own test cases to
byte-identical passthrough (contract 1). Nothing checked what the typed
layer then says about that TypeScript: contract 2 makes a user's TypeScript
errors TypeScript's to report, so a `.tt` project holding a case's units
must report exactly what the pinned `tsc` reports for the same `.ts`
project.

## Scope

- Included: a second differential in `tests/corpus.rs`
  (`typescript_cases_type_check_as_typescript_does`), the oracle
  `tests/typescript-diagnostics.mjs`, the shared helpers in
  `tests/common/typescript_cases.rs`, `tests/typed-parity-differences.txt`,
  the nightly `typescript-parity` job in `.github/workflows/ci.yml`, and
  `CONTRIBUTING.md`.
- Excluded: fixing the defects the run finds (recorded below with a
  minimal repro each, for tasks from TASK-719 on), declaration emit
  (`ttc --types`), and the editor (TASK-718).

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`,
the `gitHead` of `typescript@7.1.0-dev.20260826.1` (`package.json`).

- `tsc/internal/compiler/program.go`, `GetDiagnosticsOfAnyProgram`
  (lines 1942-1995): what `tsc` reports, in stages. Configuration-file
  diagnostics; syntactic diagnostics; only when there were none, program
  (option) diagnostics, then global diagnostics; only when there were still
  none, semantic diagnostics followed by global diagnostics again; and,
  under `noEmit` with declarations on and no diagnostic so far, declaration
  diagnostics. `GetGlobalDiagnostics` (line 1426) reports only what the
  internal checker pool has produced, which before semantic checking is
  nothing.
- `tsc/internal/execute/tsc/emit.go`, `emitFilesAndReportErrors` (lines
  78-130): the CLI gathers those diagnostics, adds emit diagnostics, and
  sorts and deduplicates them (`compiler.SortAndDeduplicateDiagnostics`).
- `tsc/internal/api/session.go`, `handleGetGlobalDiagnostics` (lines
  3769-3797): the API's `getGlobalDiagnostics` forces a full semantic pass
  first, so the oracle cannot ask it before the semantic stage the way
  `tsc` does.
- `tsc/internal/testrunner/test_case_parser.go`: units by `// @filename`,
  settings by ``(?m)^\/{2}\s*@(\w+)\s*:\s*([^\r\n]*)`` (`extractCompilerSettings`:
  lower-cased name, the last value wins, a trailing `;` removed), and a
  `tsconfig.json` unit used as the case's configuration.
- `tsc/internal/testrunner/compiler_runner.go` (lines 283-330): units
  resolve against `/.src`; the root files are every unit, or only the last
  one when the case has `@noImplicitReferences` or the last unit contains
  `require(` or a `/// <reference path`.
- `tsc/internal/testutil/harnessutil/harnessutil.go`: `CompileFilesEx`
  leaves `.json` and `.tsbuildinfo` units out of the root files (lines
  125-133); `harnessCommandLineOptions` (lines 340-402) are the options that
  are not compiler options; `GetFileBasedTestConfigurations` and
  `splitOptionValues` (lines 1038-1200) expand a comma-separated value of a
  non-list option into one configuration per value (`*` and `!x` select
  from every value), at most 25.

## Decisions

### Decision 1: The oracle is the pinned TypeScript's API, gathered in `tsc`'s stages, and audited against `tsc`

- **Context**: The comparison needs each diagnostic's range. `tsc
  --pretty false` prints a start position only.
- **Alternatives considered**: (a) `tsc --pretty false`: no end, so a range
  difference is invisible. (b) `tsgo --lsp` pull diagnostics: the editor's
  answer, which neither stages nor reports program-level diagnostics as
  `tsc` does. (c) The API client the pinned package ships
  (`dist/api/sync/api.js`), the same checker `tsc` runs, gathered the way
  `GetDiagnosticsOfAnyProgram` gathers it.
- **Decision and rationale**: (c), in a long-lived Node process per worker
  (`tests/typescript-diagnostics.mjs`) that opens and closes one project per
  case. It reports the configuration, syntactic, program, semantic, global
  and declaration diagnostics in `tsc`'s stages, deduplicated, each with
  its file, its 0-based line and UTF-16 character range, code and message
  (a message chain flattened with two spaces per level, as `tsc` prints
  it). The first global stage is not asked: the API forces a full check
  there (`session.go`), while `tsc` sees no global diagnostic before
  checking. So that the oracle cannot drift from `tsc`, the first 40 cases
  a run compares are also run through `tsc -p`, and its printed lines
  (file, line, column, code, the message's first line) must be the
  oracle's.

### Decision 2: A case runs as a project whose `tsconfig.json` holds the case's options

- **Context**: TypeScript's harness compiles a case in a virtual file
  system with the case's settings. `ttc --check-types` checks a project on
  disk against its `tsconfig.json`.
- **Alternatives considered**: Mapping a hand-picked list of options:
  every option the corpus uses (about 120) would need its type written down
  here.
- **Decision and rationale**: The case's directory stands for both `/` and
  the harness's `/.src` (a case with two units at one path after that is
  skipped). Each setting that is not a harness option is typed by the
  pinned TypeScript's own `parseCommandLine` (a boolean, a list, a number,
  an enum name, a path made relative to the case, or, for a
  `tsconfig.json`-only option such as `paths`, its JSON). A non-list value
  with commas is the harness's configuration matrix; the configurations are
  tried in order and the first one the pinned TypeScript does not reject as
  removed (TS5102, TS5108: TypeScript 7 removed `target: es5`,
  `module: amd`, `outFile`, and others) is compared, as one of the
  harness's own configurations. `noEmit` is always on, because `ttc
  --check-types` writes nothing. The root files are the harness's.

### Decision 3: Which units become `.tt`/`.ttx`

- **Context**: A `.tt` module is imported by its extension
  (`docs/ai/tt.md`, "Import `.tt`/`.ttx` files by relative path WITH
  extension"), so renaming a unit that another unit imports as `./b` turns
  the case into a different program.
- **Decision and rationale**: A `.ts`/`.tsx` unit (not a declaration file,
  not under `node_modules`) is renamed unless another unit, or itself,
  reaches it: the oracle resolves each unit's import specifiers and module
  augmentations through the checker (the module symbol's declaration is the
  target file) and its `/// <reference path>`s against the units. Every
  renamed unit must also pass through (TASK-638's verdict); `.js`, `.json`,
  `.d.ts`, `.mts` and `.cts` units are written unchanged on both sides. A
  case is compared only when the pinned TypeScript reports no syntax error
  and no TS1xxx diagnostic, TASK-638's definition of a TypeScript-valid
  unit.

### Decision 4: Both surfaces, compared as TypeScript's terms

- **Context**: `ttc --check-types` prints rendered text; the server's
  `typedCheck` returns JSON from the same `Project::check`.
- **Decision and rationale**: The server answers `typedCheck` with
  `includeTypes` for the project, and each diagnostic becomes (file, range,
  code, message): the file relative to the project with a renamed unit
  written by its `.ts` name, the 1-based line and UTF-16 column range,
  `ts2322` as `TS2322`, and the project's directory written `$DIR` in the
  message (a renamed unit's path written by its `.ts` name). A diagnostic
  with no range is compared without its file. The command line, run in the
  same project, must print the same diagnostics as the server (code, the
  message's first line, file, line, and column in code points). The two
  lists are compared as multisets.

### Decision 5: One tracked list, keyed by case, with a signature

- **Context**: TASK-638's lists name the observed verdict so a changed
  difference is noticed; TASK-685's list names a class and a reason.
- **Decision and rationale**: `tests/typed-parity-differences.txt`: the
  case, a tab, the signature the failure prints (`+TS2307` reported only by
  ttc, `-TS4118` only by TypeScript, `~TS5069@range` and `~TS2694@message`
  the same diagnostic placed or worded differently, `transport` when the
  command line and the server disagree), a tab, `by-design` (with the
  document under `docs/` that states it) or `defect` (with the task that
  records the repro), a tab, and the reason. The run fails on an unlisted
  difference, on a listed case whose signature changed, on a listed case
  that now agrees or is now skipped, and, when every case runs, on a listed
  case that is not in the corpus.

### Decision 6: A fixed-seed sample per pull request, every case nightly

- **Decision and rationale**: `cargo test` runs 80 cases drawn with seed
  `0x7474747970656421` (`TTC_TYPED_CASES=<n>|all`, `TTC_TYPED_CASES_SEED`,
  `TTC_TYPED_FILTER=<fragment,...>`; with `all`, `TTC_TYPED_SHARD=i/n` runs
  every n-th case, so a long run can be split and resumed), on four
  workers. The nightly
  `typescript-parity` job runs every case in a release build.

## Work log

- 2026-10-01: Reset the worktree onto `claude/ecstatic-dijkstra-qw5pf9`
  (`1ce50c0`), ran `npm ci` at the root and in `editors/vscode`, fetched the
  corpus, built the extension's server.
- 2026-10-01: Read the pinned `program.go`, `emit.go`, `session.go`,
  `test_case_parser.go`, `compiler_runner.go` and `harnessutil.go`; probed
  the API client's diagnostics (`pos`/`end` and `startPosition` are UTF-16)
  and `parseCommandLine` (option types).
- 2026-10-01: Wrote the oracle and the test; the first 60-case run showed
  the configuration diagnostics of removed options (Issue 1) and a
  transport mismatch the test itself caused (Issue 2).
- 2026-10-01: Runs of 400 and 1,200 cases; Issues 3 to 5.
- 2026-10-01: First run over every case (6,752 s, debug build, sharing the
  machine): 67 differences; Issues 6 and 7, and program-membership
  reachability (Decision 3).
- 2026-10-01: A container restart killed the second full run; added
  `TTC_TYPED_SHARD` and ran the four shards; classified the 58
  differences into `tests/typed-parity-differences.txt`.

## Issues and resolutions

### Issue 1: Most `es5` configurations stop at TS5108

- **Symptom**: `@target: es5, es2015` cases reported only TS5108 from
  `tsc`, while ttc reported TS5108 and the checker's diagnostics.
- **Cause**: TypeScript 7 removed those values, and `tsc` stops after a
  program diagnostic (Sources).
- **Resolution**: Decision 2's choice of the first configuration TypeScript
  accepts; a case with none is skipped. The staging difference itself is
  Defect 2.

### Issue 2: The test read a project diagnostic's place differently on the two surfaces

- **Symptom**: `transport` for every case with a configuration diagnostic.
- **Cause**: The server reports such a diagnostic with the
  `tsconfig.json` path and line 0, which the test wrote as `-`, while the
  command line prints `--> tsconfig.json`.
- **Resolution**: The test writes the path for both.

### Issue 3: The oracle skipped semantic checking whenever a global diagnostic existed

- **Symptom**: `types.forAwait.es2018.3.ts` showed four TS2495 only in
  ttc's list; `tsc -p` prints them.
- **Cause**: The API's `getGlobalDiagnostics` checks the whole program
  first (`session.go`), so asking it before the semantic stage reported
  TS2318 early and the staging dropped the semantic stage.
- **Resolution**: The oracle does not ask the first global stage (Decision
  1).

### Issue 4: Renamed units that their own or another unit's augmentation names

- **Symptom**: `duplicateIdentifierRelatedSpans_moduleAugmentation.ts`
  (`declare module "./a"`) and `unusedLocalsStartingWithUnderscore.ts` (a
  file that imports itself) reported TS2664 and TS2307 only in ttc's list.
- **Cause**: Reachability looked at import specifiers of other units only.
- **Resolution**: Module augmentations count, and so does a self-import.

### Issue 5: `package.json` units as root files

- **Symptom**: TS6054 for `package.json`.
- **Cause**: The harness leaves `.json` units out of the root files.
- **Resolution**: So does the test.

### Issue 6: The server checked a file the configuration leaves out

- **Symptom**: `exportsAndImports4-es6.ts` and `modulePreserve2.ts`
  reported TS1202 through the server only (`+TS1202 transport`), and
  `conformance/directives/multiline.tsx` TS2578.
- **Cause**: When the last unit uses `require(`, the harness's root is that
  unit alone; a renamed unit that no root reaches is not in the program,
  and the test asked `typedCheck` about it. The server checks a requested
  document as a root (`docs/ai/tt.md`: a file named as an input is checked
  as a root), so it saw a file `tsc` never compiles.
- **Resolution**: Only root files are renamed, and a root is renamed only
  when the program built from the other roots does not contain it (the
  oracle's `files` request), which also catches the implicit JSX runtime
  import of `jsxImportSource` that no import specifier names.

### Issue 7: Units named with a drive letter

- **Symptom**: `commonSourceDir1.ts` reported TS6053 for `A:/foo/bar.tt`.
- **Cause**: `A:/foo/bar.ts` is outside any directory the case can stand
  for, and the message named the renamed file.
- **Resolution**: Such a case is skipped as "a unit outside the case's
  root".

## Defects found (not fixed here)

Each is listed in `tests/typed-parity-differences.txt` with `TASK-717`;
the repros are `.ts`/`.tt` projects with the `tsconfig.json` shown, checked
with `tsc -p` and `ttc --check-types --project tsconfig.json .`.

- **D1: a configuration diagnostic loses its range.** `tsc` places an
  option error at the option in `tsconfig.json`; ttc reports it with no
  position (`--> tsconfig.json`; the server answers line 0). Repro:
  `{"compilerOptions":{"jsxFactory":"id1 id2","noEmit":true},"files":["a.tt"]}`
  with `a.tt` holding `export const x: number = "s";` — `tsc` prints
  `tsconfig.json(1,36): error TS5067`, ttc `error[ts5067]` with no line.
  Cause: `src/typescript/host.mjs` sends every diagnostic whose file is a
  configuration file to `projectDiagnostics`, which carry no position
  (`src/engine/semantics/report.rs`). Signatures `~TS5053@range`,
  `~TS5069@range`, `~TS5110@range`, `~TS5067@range`, and others.
- **D2: ttc reports the checker's diagnostics where `tsc` stops.** `tsc`
  reports program and global diagnostics and skips the semantic stage when
  any exists (`GetDiagnosticsOfAnyProgram`); the typed pass gathers every
  stage at once. The same repro: `tsc` reports only TS5067, ttc also
  `error[ts2322]` at `a.tt:1:14`. A JavaScript root without `allowJs`
  (TS6504) or a `package.json` in `files` (TS6054) stops `tsc` the same
  way. Signatures with a `+` beside a configuration or program diagnostic.
- **D3: declaration diagnostics are not reported.** Under `declaration`
  (or `composite`) and `noEmit`, `tsc` reports declaration diagnostics when
  nothing else was reported; `ttc --check-types` never asks for them.
  Repro: `{"compilerOptions":{"declaration":true,"isolatedDeclarations":true,"noEmit":true},"files":["a.tt"]}`
  with `export const x = Math.random() ? 0 : 1;` — `tsc` prints
  `a.ts(1,14): error TS9010`, ttc nothing. Signatures `-TS9007`,
  `-TS9010`, `-TS2883`, `-TS4023`, `-TS4094`, `-TS4118`, `-TS5088`,
  `-TS7056`, `-TS2527`, and others.
- **D4: TypeScript names a `.tt` module with its extension.** A module's
  name in a message is its path without a TypeScript extension, and `.tt`
  is not one. Repro: `a.tt` holding `export declare class P {}`,
  `export declare namespace P {}`, `export type T = P.Missing;` — `tsc`
  on `a.ts` says `Namespace '"<dir>/a".P'`, ttc `Namespace '"<dir>/a.tt".P'`
  (TS2694; also TS2322's `import("<dir>/f.tt")`). The same cause names an
  auto-imported default export `fooBarTt` in the editor (TASK-718 D1).

One class is by design: TS6054 and TS6231 list `.tt` and `.ttx` among the
supported extensions, because the project registers them through a
content mapper (`docs/ai/tt.md`, the `contentMappers` entry with
`"extensions": [".tt", ".ttx"]`).

## Regression test (fails before the fix)

Not applicable: this task adds a differential test and its oracle and
fixes no compiler bug. The checks the test makes are shown under
Verification.

## Verification

- [x] Every case (`TTC_TYPED_CASES=all`, four shards `TTC_TYPED_SHARD=i/4`,
  debug build): 12,831 cases; 8,721 compared, 58 differ (all listed after
  classification), 4,110 skipped (1,396 TypeScript syntax or grammar
  errors, 1,339 only removed configurations, 936 no `.ts`/`.tsx` unit, 140
  own `tsconfig.json`, 108 every root imported, 93 harness file-system
  options, 32 not passing through, 53 option, path, or encoding reasons).
  Time: 1,476 + 1,997 + 2,210 + 2,385 = 8,068 s on four cores shared with
  another agent's builds (load average 12 to 17); the first full run, less
  contended, took 6,752 s. The nightly job runs a release build.
- [x] The PR sample, 80 cases: 55 compared, 0 differ, 72.6 s at load
  average 15 (400 cases took 161 s on a quieter machine, about 0.4 s a
  case); `cargo test --test corpus` passes its three tests.
- [x] The oracle audit (`tsc -p` on the first 40 compared cases) passed in
  every run after Issue 3.
- [x] Negative checks: with `compiler/2dArrays.ts` listed and the line of
  `compiler/arrayFakeFlatNoCrashInferenceDeclarations.ts` removed, the run
  failed with "compiler/2dArrays.ts now type-checks as TypeScript does" and
  "ttc's typed check differs from the pinned tsc ... list it".
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] The full gate is recorded in TASK-718.

## Result

Changed files: `tests/corpus.rs`, `tests/common/typescript_cases.rs`,
`tests/common/mod.rs`, `tests/typescript-diagnostics.mjs`,
`tests/typed-parity-differences.txt`, `.github/workflows/ci.yml` (the
nightly `typescript-parity` job, with TASK-718's step), `CONTRIBUTING.md`
(with TASK-718's paragraph), `docs/tasks/INDEX.md`, and this record. The
58 listed differences: 54 defects (D1 17, D2 9, D3 31, D4 3; a line may
name two) and 4 by design. Follow-ups: tasks for D1 to D4.
