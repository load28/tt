# TASK-718: Hold the editor answers to `tsgo --lsp` over TypeScript's fourslash tests

> Follow-up: TASK-729 classifies D1 as by design (TypeScript names a module after the file it holds). TASK-730 found D2 to be this harness's: Decision 2 renamed the source a declaration map names; a declaration map's sources now count as reached. TASK-731 found D3 to be this harness's too: the twin held the `node_modules` declaration files open and the tt side did not; both sides now hold the same TypeScript documents open.

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-718`

## Purpose

The editor cases (TASK-639, TASK-675, TASK-685) hold the engine's answers
at markers to a TypeScript twin, over cases written here. TypeScript keeps
thousands of language-service tests of its own. Contract 1 says a `.ts`
file renamed `.tt` is still the same program, so the editor should answer
at every one of those tests' markers as `tsgo --lsp` answers on the `.ts`
file. This task runs them.

## Scope

- Included: `typescript_fourslash_tests_answer_as_their_twins` in
  `tests/editor_cases.rs` (a reader for the Go fourslash tests, their
  conversion into editor cases, and a server-only run of them), the
  `server_only` flag on a case, URI decoding of locations in the runner,
  `tests/fourslash-differences.txt`, the fourslash tree in
  `tests/typescript-cases.json` and `scripts/fetch-typescript-cases`, the
  nightly step, and `CONTRIBUTING.md`.
- Excluded: fixing the defects found (below, with repros, for tasks from
  TASK-719 on); verbs other than hover, completions, definition,
  references, rename, signature help, and semantic tokens (document
  highlights, call hierarchy, code fixes, formatting, inlay hints); a test's
  questions after its first edit.

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`
(the `gitHead` of the pinned package).

- The repository has no `tests/cases/fourslash` any more: typescript-go
  was merged under `tsc/`, and its fourslash tests are Go,
  `tsc/internal/fourslash/tests/*_test.go` (4,352 tests, tree
  `e22a52dabdd2807bbc1e61e24dba1683629ec330`). Each holds the test's files
  as a Go string constant `content` and the questions as calls on
  `f := fourslash.NewFourslash(t, capabilities, content)`. These are the
  only usable source at the pin, and the manifest now pins that tree.
- `tsc/internal/fourslash/test_parser.go`: `ParseTestData` splits units
  with `testrunner.ParseTestFilesAndSymlinksWithOptions` and
  `AllowImplicitFirstFile` (content before the first `// @Filename` is a
  file named after the test, leading blank lines kept); `parseFileContent`
  strips one leading space from every line when all have one
  (`chompLeadingSpace`), and reads `[|ranges|]`, `/*name*/` markers (name
  characters `[A-Za-z0-9$_]`; anything else makes it a comment), and
  `{| "name": ... |}` object markers; ranges are sorted by start, then by
  end descending; `emitThisFile` and `noOpen` are per-file options.
- `tsc/internal/fourslash/fourslash.go`: `newFourslash` (the default file
  `/<testName>.ts`, `getBaseFileNameFromTest`; options `target`
  latest, `jsx: preserve`, `skipDefaultLibCheck`, then the test's global
  options; `SkipUnsupportedCompilerOptions` in `harnessutil.go` skips
  `target: es5`, `module` AMD/UMD/System, `moduleResolution` node10 and
  classic, `outFile`, `baseUrl`, and `esModuleInterop`,
  `allowSyntheticDefaultImports` or `alwaysStrict` false);
  `VerifyCompletions` (a marker name, a list, or `nil` for the caret);
  `lookupMarkersOrGetRanges` (no marker means every range) for
  `VerifyBaselineFindAllReferences` and `VerifyBaselineGoToDefinition`;
  `VerifyBaselineHover` and `VerifyBaselineSignatureHelp` (every named
  marker); `VerifyBaselineRename` and `VerifyBaselineRenameAtRangesWithText`
  (ranges by text); `VerifyQuickInfoAt`, `VerifyQuickInfoIs`,
  `VerifySignatureHelp` (at the caret `GoToMarker` placed).
- TASK-639's runner (`run`, `twin_answer`, `parity_view`) and TASK-685's
  list format.

## Decisions

### Decision 1: Read the Go tests mechanically, and ask only what they ask before they edit

- **Context**: The tests are Go programs; a run needs the files, the
  markers, and the questions.
- **Alternatives considered**: (a) An older TypeScript checkout's
  `tests/cases/fourslash/*.ts`: not the pinned commit, and a second pin.
  (b) Running the Go tests: needs Go and the typescript-go build, and
  compares against TypeScript's baselines, not against ttc. (c) Reading the
  Go source: the `content` constant is a string literal (raw and
  interpreted pieces joined by `+`), and the calls are statements of the
  test function.
- **Decision and rationale**: (c). `go_test` reads the test name, the
  `content` literal, the capabilities argument (a test with custom client
  capabilities is skipped, since the answer's shape depends on them), and
  every `f.Method(...)` statement with its arguments. A test whose body has
  `for`, `if` or `switch` is skipped. The calls are taken in order up to
  the first one that is not on a read-only list (an edit, a code fix
  applied, `Configure`) or that chains `.AndApplyCodeAction`, since a later
  question is about a buffer the case does not hold. The verbs map to the
  runner's: quick info and hover to `hover`, `VerifyCompletions` to
  `completions`, the definition and references baselines to `definition`
  and `references`, the rename verbs with no preferences to `rename`,
  signature help to `signatureHelp`, `VerifySemanticTokens` to
  `semanticTokens` over every renamed file. A range a verb names becomes a
  marker `rangeN` at its start; the anonymous marker `/**/` is named
  `anonymous`.

### Decision 2: The same renaming and options as TASK-717

- **Decision and rationale**: The units are written under the case's
  directory with fourslash's base options and the test's global options in
  a `tsconfig.json`, converted by the same oracle as TASK-717
  (`tests/typescript-diagnostics.mjs`). A `.ts`/`.tsx` file that no other
  file reaches by an import, an augmentation, or a reference is renamed
  `.tt`/`.ttx` (`docs/ai/tt.md`: a `.tt` module is imported by its
  extension); the original files are the twin. Questions at markers in a
  file that is not renamed are not asked: they are not questions about a
  `.tt` file. A test whose harness needs symlinks, `@currentDirectory`,
  `@tsc`, or its own `tsconfig.json` is skipped. Unlike the compiler cases,
  a test whose file does not parse is kept: completion at an unfinished
  expression is what many of these tests ask.

### Decision 3: Server only, compared through `parity_view`, listed like the matrix

- **Context**: TASK-685 Decision 5 asks generated cases through the server
  transport only; the engine-versus-server agreement is held by the
  written cases.
- **Decision and rationale**: A converted test is a `Case` with
  `server_only` (the flag TASK-685's `matrix` field implied) and the twin;
  `run` asks `ttc --server` and `tsgo --lsp`, and `parity_view` compares.
  Nothing is written as a baseline: the tests are third-party source, and a
  difference is printed in the failure. `tests/fourslash-differences.txt`
  lists each differing question as the matrix's list does (test name with
  `*`, verb and marker, `by-design` with a document under `docs/` or
  `defect` with its task, reason); the run fails on an unlisted difference
  and on a listed one that agrees, and, when every test runs, on a line
  that names no question.

### Decision 4: Sample and cost

- **Decision and rationale**: 60 tests drawn with seed
  `0x7474666f7572736c` per pull request (`TT_FOURSLASH=<n>|all`,
  `TT_FOURSLASH_SEED`, `TT_FOURSLASH_FILTER`, `TT_FOURSLASH_VERBOSE=1`
  prints each skip), eight workers as the editor cases use; every test in
  the nightly `typescript-parity` job.

## Work log

- 2026-10-01: Found that the pinned commit holds the fourslash tests as Go
  only; read `test_parser.go`, `fourslash.go` and the verbs' argument
  conventions; counted the calls over all tests.
- 2026-10-01: Added the fourslash tree to the manifest and refetched;
  wrote the reader, the conversion and the test.
- 2026-10-01: Runs of 8, 150 and 600 tests, then every test; Issues 1
  and 2; classified the 51 differing questions into
  `tests/fourslash-differences.txt`.
- 2026-10-01: Second run over every test with the list: passes.

## Issues and resolutions

### Issue 1: tsgo's percent-encoded URIs read as another file

- **Symptom**: `referencesIsAvailableThroughGlobalNoCrash` differed with
  `node_modules/%40types/...` on TypeScript's side only.
- **Cause**: The runner stripped `file://` from an LSP URI without decoding
  it.
- **Resolution**: `Files::name` percent-decodes the URI.

### Issue 2: A side-effect import did not keep its target `.ts`

- **Symptom**: `goToDefinitionScriptImport` answered a location at
  `$DIR/scriptThing` with no extension on the tt side.
- **Cause**: `import "./scriptThing"` has no import clause, so the checker
  gives its specifier no module symbol, the oracle did not count the file
  as reached, and the test renamed it: the `.tt` project's import no longer
  resolved. (The engine then answers go to definition on the unresolved
  specifier with the path of a file that does not exist; observed, not
  compared, since the twin cannot have the same unresolved import.)
- **Resolution**: A relative specifier with no module symbol is resolved
  against the units with TypeScript's extension candidates.

## Defects found (not fixed here)

Each is listed in `tests/fourslash-differences.txt` with `TASK-718`.

- **D1: a `.tt` module's default export is auto-imported as
  `someModuleTt`** (`completionsImport_defaultAndNamedConflict`,
  `completionsImport_default_anonymous`,
  `completionsImport_jsxOpeningTagImportDefault`). Repro: `fooBar.tt`
  holding `export default function () {}` and `main.tt` holding `fooB`;
  completion at the end offers `fooBarTt` where the `.ts` twin offers
  `fooBar`. TypeScript derives the name from the file name with a known
  extension removed, and `.tt` is not one; TASK-717's D4 is the same cause
  in a diagnostic message.
- **D2: go to definition into a declaration file does not follow its
  declaration map** (`declarationMapGoToDefinition`,
  `declarationMapsGoToDefinitionRelativeSourceRoot`). Repro: `index.ts`
  with a class, its `indexdef.d.ts` and `indexdef.d.ts.map` (as the tests
  write them), and `mymodule.tt` calling `instance.methodName(...)`
  through `./indexdef`; the engine answers `indexdef.d.ts`, `tsgo --lsp`
  answers the method in `index.ts`.
- **D3: find all references at a module specifier leaves out the
  declaration file** (`findAllRefsTripleSlashRef1`). Repro:
  `node_modules/@types/react/index.d.ts` holding `export type JSX = {};`
  and `index.tt` holding `import type { JSX } from "react";`; references
  at `"react"` answer only the specifier, where TypeScript also answers the
  declaration file.

The by-design lines cite `docs/ai/tt.md` (a `.tt` module is imported with
its extension, so path completion offers `x.tt`; a `/// <reference path>`
is not a `.tt` import; a `package.json` `imports` pattern that names `.ts`
files does not name the renamed `.tt` unit) and
`docs/design/lsp-architecture.md` (the engine materializes `@tt/std` and
`@tt/runtime` under the project's `node_modules`, so path completion lists
`node_modules`).

## Regression test (fails before the fix)

Not applicable: this task adds a differential test and fixes no compiler
bug.

## Verification

- [x] Every test (`TT_FOURSLASH=all`, debug build): 4,352 tests; 1,829
  compared, 5,808 questions, 51 differ (all listed), 2,523 skipped (1,677
  ask nothing this runner asks at a marker in a renamed file, 384 have no
  renameable file, 174 bring a `tsconfig.json`, 118 custom client
  capabilities, 93 options the oracle or the fourslash harness rejects or
  skips, 29 symlinks, and 48 that are not readable as a plain test: no
  `content` literal, control flow, files outside the root, two files at one
  path). 746.6 s on four cores shared with TASK-717's run and another
  agent's builds; the first full run took 499 s.
- [x] The PR sample, 60 tests: 30 compared, 73 questions, none differ,
  12.8 s.
- [x] An unlisted difference fails ("differs from the TypeScript twin, and
  tests/fourslash-differences.txt does not list it"), and a listed question
  that agrees fails ("agrees with the TypeScript twin now"), seen after
  Issue 2's fix.
- [x] `cargo test --test editor_cases`: both tests pass, 159 s; no editor
  baseline changed with the URI decoding.
- [x] Full gate over TASK-717 and TASK-718 (branch at `1ce50c0`, which
  `claude/ecstatic-dijkstra-qw5pf9` had not advanced past): `cargo fmt
  --check`; `cargo clippy --all-targets -- -D warnings`;
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast` (1,263 passed, 0 failed, no `SKIP`, 23.5 minutes on the
  shared machine); `node scripts/check-baselines --tracking <dir>`
  ("5352 compared, ... none unused"); `./scripts/ci agents`.

## Result

Changed files: `tests/editor_cases.rs`, `tests/fourslash-differences.txt`,
`tests/typescript-cases.json`, `scripts/fetch-typescript-cases`,
`docs/tasks/INDEX.md`, and this record; the nightly step and the
`CONTRIBUTING.md` paragraph were committed with TASK-717. The 51 listed
questions: 45 by design (path completion offers `x.tt`, lists the
materialized `node_modules`, or does not offer a `.tt` file in a
`/// <reference path>` or under a `package.json` `imports` pattern that
names `.ts`) and 6 defects (D1 3, D2 2, D3 1). Follow-ups: tasks for D1 to D3.
