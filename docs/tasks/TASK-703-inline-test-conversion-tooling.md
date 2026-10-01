# TASK-703: Convert inline Rust tests to case files by proving each conversion

- **Status**: In progress
- **Started**: 2026-10-01
- **Completed**: —
- **Commit**: see `git log --grep TASK-703`

## Purpose

Most of the repository's ~2,000 older tests are Rust functions that compile
a string and assert on part of the answer (`contains`, one position, one
line of stdout). TypeScript keeps its behaviour as case files whose whole
output is a reviewed baseline (TASK-634). This task decides which inline
tests can become case files without losing an assertion, and builds the
tool that converts them and proves every conversion mechanically.

## Scope

- Included: the classification of the inline tests of `tests/compile.rs`
  (with `tests/compile/*.rs`) and `tests/integration.rs` (with
  `tests/integration/*.rs`); `scripts/convert-inline-tests`; the
  `// @origin` directive in `tests/case_baselines.rs`; two corrections of
  the `.ts` baseline that a lossless conversion needs (a missing final
  newline is shown, and the text is no longer rewritten on Linux and
  macOS); the baselines those corrections change; `CONTRIBUTING.md`.
- Excluded: the conversions themselves (TASK-704 for the compile suite,
  TASK-705 for the integration suite); `tests/cli*.rs`, `tests/passthrough.rs`,
  and `tests/native/*` (see Decision 1); any compiler change.

## Sources modelled

- microsoft/TypeScript `release-6.0` at `050880c`, `CONTRIBUTING.md`,
  "Adding a Test" and "Managing the baselines": a behaviour is a file under
  `tests/cases/compiler` or `tests/cases/conformance/<area>`, and its
  outputs are baselines under `tests/baselines/reference` that a change
  must update and a reviewer reads.
- `src/harness/harnessIO.ts` (`Compiler.doJsEmitBaseline`, `Baseline.runBaseline`):
  the `.js` baseline lists the inputs (`//// [name]`) and the outputs, and a
  baseline holds the text it compares. `src/testRunner/compilerRunner.ts`,
  `CompilerTest`: one case yields every artifact kind.
- `src/testRunner/unittests/evaluation/*.ts`: runtime behaviour is pinned by
  what an executed program observed, which the case runner's `@run`
  `.stdout` baseline models (TASK-652).

## Classification

What an inline test asserts on, counted over every `#[test]` function (795
in all: 580 in the compile suite, 215 in the integration suite):

| Class | Compile | Integration | Converts to |
| --- | --- | --- | --- |
| Emitted TypeScript of one program (`ok`, `ok_tsx`) | most | — | `.ts` baseline |
| tt diagnostics: message, position, rule, advice (`err`, `advice`, `codes`) | many | — | `.errors.txt` (`ttc --out-dir` section) |
| TypeScript's verdict on the emitted tree (`typecheck`, `typecheck_with_std`) | — | 63 programs | `.errors.txt` (`tsc` section) |
| What the emitted program prints (`run`, `run_with_std`) | — | 135 programs | `@run` `.stdout` |
| A library API other than the emission: `analyze`, `compile_report`, `compile_mapped`, `emit_mapped`, `Options` other than the defaults, `ExternVariant`, `SourceKind`, `std_imports` | 125 + 19 | 6 + 9 | stays Rust |
| Its own files, processes, or the CLI (`fs`, `Command`, `ttc` binary, `tsc` with extra flags) | — | 27 | stays Rust |
| A panic or unwinding (`catch_unwind`) | 3 | — | stays Rust |

The counts in the first four rows overlap (a test can observe several
things) and are measured per program by the recording step; the exact
figures per test are in TASK-704 and TASK-705.

## Decisions

### Decision 1: Convert only what a case's baselines observe, and keep the rest in Rust

- **Context**: A case drives the real CLI and engine (TASK-634 Decision 1):
  `ttc --out-dir`, `ttc --check-types`, `tsc` on the emitted tree, the
  engine's hovers, the editor projection's mappings, and `node` on the
  emitted program. An inline test that asserts on anything else would lose
  its assertion as a case.
- **Alternatives considered**: (a) Convert every test that compiles a
  string, by its source text alone: tests of `compile_mapped`'s mappings,
  `analyze`'s full diagnostic list, or a non-default `Options` would
  become cases that never look at what they asserted. (b) Add baseline
  kinds for each library API: a second runner inside the case runner.
  (c) Convert what the existing artifacts observe and keep the rest.
- **Decision and rationale**: (c). The kept classes are the behaviour of a
  library API (TypeScript keeps such tests as unit tests under
  `src/testRunner/unittests/`, not as compiler cases), the CLI and
  processes (`tests/cli*.rs`, `tests/native/*`), panics, and
  `tests/passthrough.rs`, whose whole-corpus byte-identity check over
  TypeScript's own cases is already a data-driven corpus test and not a
  per-program assertion.

### Decision 2: Prove a conversion by running the test's own assertions against the baselines

- **Context**: A test that asserts `contains`, `!contains`, a position, or
  a line of stdout becomes an exact baseline. The baseline must still say
  what the assertion said, and checking that by reading ~1,000 baselines
  by hand would be neither complete nor repeatable.
- **Alternatives considered**: (a) Parse the assertions (`assert!(out.contains(..))`)
  and evaluate them on the baseline in the script: only the shapes the
  parser knows, and a second interpreter of Rust. (b) Run each test body
  itself twice, once against the library and once against the baselines.
- **Decision and rationale**: (b). `scripts/convert-inline-tests harness`
  copies every test body verbatim into a generated test target, around
  helpers with the original names. `record` runs it against the library
  (the original helpers, copied into a private module) and logs each
  program a helper compiled, with the test that compiled it; `generate`
  writes one case per distinct program; `prove` runs the same bodies with
  the helpers answering from the committed baselines. Whatever the
  assertion's shape (`assert_eq!`, a loop over lines, `starts_with`, a
  computed range), it is the test's own code that judges the baseline. A
  body that reaches anything but those helpers does not compile in the
  harness, because the library is not in scope there: each round the
  script drops the items rustc names and rebuilds, and a textual check
  drops a body that names `ttc::` (other than the `CompileError` and
  `DiagnosticCode` data types), `std::fs`, `std::process`, `std::env`,
  `include_str!`, or `catch_unwind`. A test is deleted only when it
  recorded at least one program, every program it compiled has a case
  naming it in `@origin`, and its proof passed.

### Decision 3: One case per distinct program, named after the test

- **Context**: A test may compile several programs; two tests may compile
  the same one.
- **Alternatives considered**: (a) One case per test with an `@filename`
  unit per program: the units share one project, so scripts share a
  global scope, `tsc` reports redeclarations, and contextual typing sees
  the other units' declarations, which changes what is emitted. (b) One
  case per program.
- **Decision and rationale**: (b), named in camelCase after the test
  (`a_crlf_source_gets_crlf_glue` → `aCrlfSourceGetsCrlfGlue`), with a
  number when the test compiles several. A program several tests compile
  is one case with one `@origin` line per test. A test that compiles more
  than six programs is a generated table over one rule, and its assertion
  is the rule across the rows; it stays in Rust (the case matrix of
  TASK-678 is where such a family belongs as cases).

### Decision 4: Show a missing final newline, and stop rewriting the text of a baseline

- **Context**: The proof failed on 31 tests that compared a whole emission
  with `assert_eq!`, and on the CRLF tests. The `.ts` baseline appended a
  newline to an output that had none, so the baseline could not say
  whether the output ended in one; and `normalize` replaced every `\r\n`
  with `\n` and every `\` with `/` in every baseline, meant for Windows
  paths. The second also corrupted existing baselines: six matrix
  `.stdout` baselines read `"<p>/"negative -2/"</p>"` where the program
  printed `"<p>\"negative -2\"</p>"`, and any source line with an escape
  was shown with `/`.
- **Alternatives considered**: (a) Keep the rewriting and let such tests
  stay in Rust: the case runner would keep baselines that are not what
  the program wrote. (b) Show the missing newline as `git diff` does (`\ No
  newline at end of file`) and replace only the case directory by `$DIR`,
  rewriting separators and line endings only on Windows, where they come
  from the platform rather than from the program (the suites run on
  Linux in CI).
- **Decision and rationale**: (b). Five existing `.ts` baselines gain the
  marker and the matrix baselines whose program printed a backslash are
  corrected; their diffs are the review of this decision.

### Decision 5: `@origin` records provenance and nothing else

- **Context**: A converted case should say which Rust test it replaces, so
  a reader of the case (and of `git log`) can find the history and the
  intent the test's name carried.
- **Alternatives considered**: a comment line: the case runner would not
  check its form, and the proof needs a machine-readable key.
- **Decision and rationale**: `// @origin: tests/<file>.rs::<test>`, a
  per-file directive the runner checks for form and otherwise ignores.

## Work log

- 2026-10-01: Reset onto `claude/ecstatic-dijkstra-qw5pf9` (b3a1c9e); `npm
  ci` at the root and in `editors/vscode`, `npm run compile` there,
  `scripts/fetch-typescript-cases`.
- 2026-10-01: Wrote `scripts/convert-inline-tests` (scan, harness, record,
  generate, prove, apply, clean) and `@origin` in `tests/case_baselines.rs`.
- 2026-10-01: First proof of the compile suite: 364 of 436 candidate tests;
  the failures showed the final-newline and rewriting faults (Decision 4).
  After the correction: 397.
- 2026-10-01: `UPDATE_EXPECT=1 TT_MATRIX_CASES=all cargo test --test
  case_baselines` to correct every matrix baseline the rewriting had
  changed.

## Issues and resolutions

### Issue 1: The engine's semantic tokens time out on one converted program

- **Symptom**: `verifyPreflightsUnbalancedDelimitersBeforeSwc` (converted
  from `tests/compile/cases_04.rs::verify_preflights_unbalanced_delimiters_before_swc`,
  a fuzz-shaped line of unbalanced `K<[({[(`) fails its `.types` baseline
  with "TypeScript language service request `textDocument/semanticTokens/full`
  timed out after 8s", every run.
- **Cause**: The type baseline asks the engine for semantic tokens, which
  asks the TypeScript language service; on this input that request does
  not answer within the engine's 8-second limit. The original test only
  asserts that `ttc` reports a tt error rather than crashing in swc.
- **Resolution**: The case keeps `@baselines: ts, errors.txt, map.txt` (the
  observation the test made); the timeout is reported as a defect for a
  separate task rather than accepted into a baseline.

## Regression test (fails before the fix)

Not applicable: this task fixes no compiler bug. The corrected baselines
of Decision 4 are their own evidence.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
