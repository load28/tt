# TASK-639: Pin editor behaviour with fourslash-style cases over both transports

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-639`


> TASK-643 fixed Issue 4's lexer defect and restored the `attribute`
> marker inside the JSX tag of `plainTsx`.

## Purpose

Editor fixes here (TASK-556, 578, 602 to 613, 629 to 633) are pinned by
Rust tests that assert one property of one answer, through one of the two
ways an editor reaches the engine. TypeScript pins language-service
behaviour with fourslash files: a small project with `/*marker*/`s, a verb
per question, and the whole answer in a baseline; typescript-go additionally
checks its answers against TypeScript's and keeps the known differences in
a list its runs keep current. This task brings both to tt.

## Scope

- Included: `tests/editor_cases.rs`, `tests/cases/editor/` (19 cases, 7 with
  a TypeScript twin), `tests/baselines/reference/editor/`, the editor root in
  `scripts/check-baselines`, restricting `tests/case_baselines.rs` to
  `compiler/` and `conformance/`, `CONTRIBUTING.md` ("Adding an editor
  case"), and `AGENTS.md`.
- Excluded: code actions, inlay hints, document symbols, trigger-character
  completion (TASK-631), completion resolve, and the VS Code adapter's own
  merging of TypeScript and tt items (tested by the extension suite).

## Sources modelled

- microsoft/typescript-go at `16c25522e1230b69b11210cfad066d779e6319ba`:
  `internal/fourslash/test_parser.go` (`/*name*/` markers and `[|...|]`
  ranges removed from the content; units split by
  `testrunner.ParseTestFilesAndSymlinksWithOptions`; duplicate marker names
  rejected), `internal/fourslash/fourslash.go` (`VerifyBaselineHover`,
  `VerifyBaselineFindAllReferences`, `VerifyBaselineGoToDefinition`,
  `VerifyBaselineSignatureHelp`: one baseline section per marker), and
  `internal/fourslash/baselineutil.go` (`// === <command> ===` sections).
- The same commit's `internal/fourslash/_scripts/failingTests.txt` (385
  test names), `skip_if_failing.go` (`SkipIfFailing`, overridden by
  `TSGO_FOURSLASH_IGNORE_FAILING`), and `_scripts/updateFailing.mts`, which
  `hereby all-checks` runs to rewrite the list from a run of every test.
- `src/typescript/service.rs` here, for the `tsgo --lsp -stdio` handshake
  the engine itself uses (capabilities, `workspace/configuration` replies,
  the semantic-token legend), and `src/engine/language/service.rs`
  (`split_hover`) for how the engine reads TypeScript's hover markdown.

## Decisions

### Decision 1: One case format, the case runner's, with fourslash markers

- **Context**: The case runner (TASK-634) already parses `// @filename`
  units and `// @name: value` directives.
- **Alternatives considered**: fourslash's `////`-prefixed content with
  `verify.*()` calls in TypeScript: a second language inside the case and a
  TypeScript interpreter in the runner.
- **Decision and rationale**: The same directive syntax; the verbs are
  directives (`// @hover: a, b`, `// @semanticTokens: *`), markers are
  `/*name*/`, ranges are `[|...|]`, and all three are removed before the
  units are written, as fourslash removes them. An unknown directive, a verb
  naming no marker, and an unbalanced range fail the case. The runner owns
  `tests/cases/editor/`; `tests/case_baselines.rs` now reads only
  `compiler/` and `conformance/`.

### Decision 2: Ask the engine and the server the same question, and require the same answer

- **Context**: The server's JSON is built in `src/server.rs` from the same
  engine calls an embedding makes; nothing checked that the two agree.
- **Alternatives considered**: Only the server (what the extension reads):
  a drift between the API and the protocol would go unseen. Only the
  engine: the protocol layer untested.
- **Decision and rationale**: The runner opens every `.tt`/`.ttx` unit in a
  `ttc::engine::Workspace` and in a `ttc --server` process, asks each
  request of both (`hover` and `ttSymbol`; `completion`, `ttCompletions` and
  `patternCompletions`; `definition`; `references`; `prepareRename` and
  `rename`; `signatureHelp`; `documentSemanticTokens`; `tsDiagnostics`),
  builds the engine's JSON the way `src/server.rs` does, and fails on any
  difference. Two normalizations, both of values that are not answers: the
  project directory, and completion items' order and probe id. The order of
  a large completion list differed between two `tsgo` processes for the same
  request ("the same 1061 item(s) in another order"); the editor orders
  items by `sortText`, and the probe id is a process counter.

### Decision 3: The baseline shows the answer in the source's terms

- **Decision and rationale**: Each question gets a section with the marker's
  line and a caret, then each request's answer: a range as coordinates and
  the text it covers, a location as the unit name (or, outside the case, the
  path under the project, such as `node_modules/@tt/std/result.ts`), and
  completion entries sorted by `sortText` and label. Entries TypeScript
  ranks as globals or keywords (`sortText` 15 with no `source`) are counted,
  not listed: about 1,040 lib globals per question would bury the answer;
  switch-case snippets and auto-imports have a source and are listed. When a
  `references` or `rename` marker sits inside a `[|range|]`, the answer's
  locations must be exactly the case's ranges, as fourslash's
  `verify.rangesAreRenameLocations` asks.

### Decision 4: Parity with a TypeScript twin, through tsgo's own LSP

- **Context**: The user-facing promise is that a `.tt` file answers as its
  TypeScript equivalent does.
- **Alternatives considered**: Asking the engine about a `.ts` file (the
  engine's answer, not TypeScript's); comparing whole answers (file names,
  extensions and generated code differ by construction).
- **Decision and rationale**: A twin (`<name>.ts`/`.tsx`, same units with
  `.ts`/`.tsx`) is written to its own project and opened in a `tsgo --lsp
  -stdio` from the pinned package; the same questions go to it at the same
  marker names, and both answers are reduced by `parity_view`: locations to
  the unit stem and covered text, hover to signature and documentation (the
  engine's markdown split), completion to its label set, signature help to
  labels, parameters and the active pair, rename and references to the set
  of covered spans. Semantic tokens and diagnostics are compared only when
  the twin's text is the source's. A difference is shown in the case's
  baseline as the lines only one side has.

### Decision 5: The failing list is a baseline

- **Context**: typescript-go skips listed tests and relies on
  `updateFailing.mts` to keep the list current; the list itself is not
  checked by a test run.
- **Decision and rationale**: `tests/baselines/reference/editor/failingParity.txt`
  lists `<case> <verb> <target>` for every parity difference and goes
  through the baseline helper (`expect`, or `expect_absent` when empty), so
  a new difference and a fixed one both fail with a diff, `UPDATE_EXPECT=1`
  rewrites it, and TASK-635's tracking covers it. The seeds have no
  difference, so the file does not exist.

### Decision 6: Seeds and budget

- **Decision and rationale**: 19 cases from the editor tasks and the probe
  repros in `target/probe5-editor` and `target/probe6-editor`:
  `armBodyCompletion` (TASK-632), `armGuardWithoutBody` (TASK-605),
  `autoImportEntry` (TASK-612, 629), `generatedSwitchCases` (TASK-608),
  `importPathCompletion` (TASK-609), `matchHoverAndDefinition` and
  `semanticTokensOverTtConstructs` (TASK-606), `matchInJsxChild`,
  `patternCompletion` (TASK-607), `pipelineStep` (TASK-613),
  `renameBindings` (TASK-611), `signatureHelpInTry` (TASK-604),
  `signatureHelpStoredCallee` (TASK-603, 630), `standardLibraryTags`
  (TASK-610), `unfinishedIfLet` (TASK-602), `unusedPayloadList`
  (TASK-633), and three plain TypeScript files with identical twins
  (`plainTypeScript`, `plainTypeScriptDeclarations`, `plainTsx`), which
  hold contract 1 in the editor. 71 questions, 37 parity comparisons; the
  suite takes 20 to 35 seconds on four workers (three TypeScript processes
  per case).

## Work log

- 2026-09-30: Read typescript-go's fourslash parser, harness, baseline
  utilities, failing list, and its update script at `16c25522`.
- 2026-09-30: Added `tests/editor_cases.rs` with the engine mirror, the
  server client, and the LSP client; restricted the case runner to its two
  suites; registered the editor root in `scripts/check-baselines`.
- 2026-09-30: Wrote the seed cases and twins, generated the baselines with
  `UPDATE_EXPECT=1 cargo test --test editor_cases`, and read each one.
- 2026-09-30: Documented "Adding an editor case" in `CONTRIBUTING.md`.

## Issues and resolutions

### Issue 1: Completion order differs between two TypeScript processes

- **Symptom**: The first run failed six cases with "completion ... differs
  between the transports"; the difference was "the same 1061 item(s) in
  another order".
- **Cause**: The engine returns TypeScript's items in the order `tsgo`
  sends them, which is not stable between processes.
- **Resolution**: Decision 2's normalization and Decision 3's sorted
  rendering. Items are compared as a multiset; any other difference still
  fails.

### Issue 2: A twin written with `switch` differed for a reason of its own

- **Symptom**: `armBodyCompletion`'s first twin put the cursor inside a
  `switch`, and TypeScript offered `case "Point": ...` there.
- **Cause**: The twin was not an equivalent program.
- **Resolution**: The twin uses `if`/`else`; the parity is then exact.

### Issue 3: Findings the seed baselines record

- **Symptom**: Go to definition on a user variant's tag in a pattern answers
  no location (`matchHoverAndDefinition`: `definition /*tag*/` is empty,
  while `ttSymbol` names the declaration); typed completion inside a
  hand-written union's pattern payload (`Alpha(|)`) offers `kind` and `x`
  while `patternCompletions` offers only `x`; semantic tokens classify
  `match` and `result` as keywords but not `variant`, `val`, or `try`.
- **Cause**: Not investigated; each is the current behaviour.
- **Resolution**: Recorded in the baselines, so a change to any of them is
  a reviewed diff. Candidate follow-ups.

### Issue 4: A comment inside a JSX tag hides the element from the lexer facts

- **Symptom**: With `plainTsx` committed, the library test
  `lexer::facts::tests::the_machine_reads_the_corpus_as_swc_does`, which
  reads every file under `tests/`, failed for `plainTsx.ttx` and
  `plainTsx.tsx`: `jsx swc only: 439..474 "<Button /*attribute*/label=\"ok\" />;"`.
  The raw case file has a marker comment between the tag name and its
  attribute; swc reads the JSX element there and ttc's lexer facts do not.
- **Cause**: A defect in the lexer facts' JSX recognition when a comment
  sits inside an opening tag (`export const view = <Button /*c*/label="ok" />;`
  in a `.tsx` or `.ttx` file). Not fixed here.
- **Resolution**: The `attribute` marker was removed from the case (its
  hover is covered by `prop`), so the existing test passes; the defect is
  reported as a follow-up with that repro.

## Regression test (fails before the fix)

Not applicable: this task adds a test runner and seed cases and fixes no
compiler bug. The runner's own failures are shown under Verification.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases`, then two plain runs:
  both pass, 20 to 35 seconds, with no change to the baselines.
- [x] Every baseline read; no absolute path in any of them.
- [x] A twin with an extra local fails with "modified baseline ...
  armBodyCompletion.baseline" and `+ completions body: differs` /
  `+   ts only: extra`; the transport check reported the order difference
  of Issue 1; a range that the answer does not match fails with "the
  references at /*binding*/ are not the case's [|ranges|]".
- [x] `cargo clippy --test editor_cases --test case_baselines -- -D warnings`.
- [x] Full gate over TASK-637 to TASK-639, after merging
  `claude/ecstatic-dijkstra-qw5pf9` at `f656b15` (TASK-621 to TASK-628):
  `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_BASELINE_TRACKING_DIR=<dir> cargo test` (46 test binaries, 1762
  passed, 0 failed, no `SKIP`, 401 seconds); `node scripts/check-baselines
  --tracking <dir>` ("127 compared, none unused"); `cargo check
  --manifest-path fuzz/Cargo.toml --all-targets --locked`; `./scripts/ci
  agents` passed (warnings: rolldown not on PATH, doctor reports the
  checkout not ready, both environmental). The first run of this gate
  failed on Issue 4 and on the fuzz lock file (TASK-637 Issue 3). The
  extension suite was not run: no extension file changed.

## Result

Changed files: `tests/editor_cases.rs`, `tests/cases/editor/**`,
`tests/baselines/reference/editor/**`, `tests/case_baselines.rs`,
`scripts/check-baselines`, `CONTRIBUTING.md`, `AGENTS.md`,
`docs/tasks/INDEX.md`, and this record. Follow-ups: the lexer facts defect
of Issue 4 and the findings of Issue 3.
