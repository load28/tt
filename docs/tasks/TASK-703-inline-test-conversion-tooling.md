# TASK-703: Convert inline Rust tests to case files by proving each conversion

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
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

What each inline test asserts on, over every `#[test]` function (795: 580
in the compile suite, 215 in the integration suite). The first rows are
what the recording step observed the converted tests compile; the others
are why a test stays in Rust.

| Class | Compile | Integration | Becomes |
| --- | --- | --- | --- |
| Emitted TypeScript and tt diagnostics of one or more programs (`ok`, `ok_tsx`, `err`, `advice`, `codes`), proven | 394 | — | case: `.ts`, `.errors.txt` (`ttc --out-dir` section) |
| What the emitted program prints (`run`, `run_with_std`) or whether `tsc` accepts it (`typecheck`, `typecheck_with_std`), proven | — | 153 | case: `@run` `.stdout`, `.errors.txt` (`tsc` section) |
| A library API directly (`analyze`, `compile_report`, `compile_mapped`, `emit_mapped`, `ExternVariant`, ...) | 122 | 6 | stays Rust |
| A library API through a helper (`token_extern`, `hole`, `generated_lines`, `run_with_tsc_flags`, `typecheck_recovery`, `options_with_runtime`) | 10 | 5 | stays Rust |
| Non-default `Options` or another `SourceKind` | 10 | 7 | stays Rust |
| Its own files, processes, or the CLI | — | 19 | stays Rust |
| A panic (`catch_unwind`) | 3 | — | stays Rust |
| A table of more than six programs over one rule | 29 | 1 | stays Rust |
| An observation a baseline does not hold (the library's default support specifiers, an error's end position; the `./tt/` std files the integration helper writes) | 10 | 24 | stays Rust |
| No program compiled through a case-observable helper | 2 | — | stays Rust |

TASK-704 and TASK-705 list every kept test with its reason.

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
  changed: 90 matrix `.stdout` baselines and five `.ts` baselines change.
- 2026-10-01: A helper copied beside the bodies could still call the
  library through a `ttc::` path; the textual check now applies to helpers
  as well (a test that only reached the library through such a helper had
  recorded nothing and was not deleted, so no conversion was affected).

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

## Time impact

Measured on the shared 4-core development container (load average 12 to
19 from other work, so the figures are an upper bound), each suite alone,
before (`b3a1c9e`'s tests, or the case directory without the converted
cases) and after TASK-704 and TASK-705:

| Suite | Before | After |
| --- | --- | --- |
| `tests/case_baselines.rs` (pull-request sample) | 64 s wall, 131 s CPU | 468 s wall, 671 s CPU |
| the same, converted cases without `.types` (measured, not adopted) | — | 355 s wall, 409 s CPU |
| `tests/compile.rs` | 54 s wall, 24 s CPU | 50 s wall, 21 s CPU |
| `tests/integration.rs` | 39 s wall, 75 s CPU | 23 s wall, 34 s CPU |

A case costs about 0.75 s of CPU (two `ttc` runs, `tsc` on the emitted
tree, the engine's hovers, and `node` for a run case) where an inline
`ok` test cost milliseconds, so a pull request's `cargo test` grows by
about 6 minutes of wall time on this machine. The nightly job grows by the
same amount: the converted cases are not part of the sampled matrix, and
`tests/incremental.rs` and `tests/fuzz_regressions.rs` draw a fixed-size
sample from the larger corpus, so their time does not change.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=... cargo test --no-fail-fast`: every suite passes
- [x] `node scripts/check-baselines --tracking ...`: 5,073 compared, none unused
- [x] `./scripts/ci agents`: passed
- [x] Baseline changes reviewed and committed with the change: the 95
  changed baselines differ from the committed ones only by a restored `\`
  or by the final-newline marker (checked by comparing each with its
  committed text after undoing exactly those two changes)

## Result

`scripts/convert-inline-tests`, `@origin`, and the two baseline
corrections. The conversions are TASK-704 (394 compile tests, 562 cases)
and TASK-705 (153 integration tests, 158 cases). Follow-up defect: the
semantic-token timeout of Issue 1.
