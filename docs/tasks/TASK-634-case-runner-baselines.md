# TASK-634: Run case files against multi-artifact baselines

> **Superseded in part by TASK-650**: Decision 4 (one mechanism, reference
> files written in place) is reversed. A failing comparison now writes to
> `tests/baselines/local/` and `scripts/baseline-accept` accepts it;
> `UPDATE_EXPECT=1` remains as a shortcut.

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-634`

## Purpose

A bug fix here is pinned by hand-written Rust tests that assert a property
of the answer (`contains`, one diagnostic code). TypeScript pins a fix with
one case file whose whole output is compared with committed baselines, so a
later change anywhere in the output shows up as a reviewable diff. This task
adopts that model so that adding one case file is the default regression
test.

## Scope

- Included: the case runner `tests/case_baselines.rs`, case files under
  `tests/cases/{compiler,conformance/<feature>}/`, baselines under
  `tests/baselines/reference/`, the shared baseline comparison
  `tests/common/baseline.rs` (moved out of `tests/snapshot.rs` and reused by
  `tests/practical_diagnostics.rs`), eight seed cases, and the contribution
  documentation (`CONTRIBUTING.md`, `AGENTS.md`).
- Excluded: tracking unused baselines and the CI regeneration check
  (TASK-635), the contribution rules and the pull request template
  (TASK-636), TypeScript's `varyBy` option fan-out, and a local/reference
  baseline split with an accept command (see Decision 4). Existing tests are
  kept; the seeds duplicate their inputs rather than replacing them.

## Sources modelled

- microsoft/TypeScript `release-6.0`, `CONTRIBUTING.md`, "Adding a Test":
  a `.ts` file in `tests\cases\compiler` (or `tests\cases\conformance\<area>`),
  `// @metaDataName: value` tags, and `// @filename` sections as separate
  compilation units; test names must be distinct.
- `src/harness/harnessIO.ts`, `TestCaseParser.makeUnitsFromTest` and
  `extractCompilerSettings`: the option regex `^\/{2}\s*@(\w+)\s*:\s*([^\r\n]*)`,
  option lines removed from unit content, only comments before the first
  `@filename`, and a single unit named after the file when none is given.
- `src/harness/harnessIO.ts`, the `Baseline` namespace (`runBaseline`,
  `writeComparison`): a baseline per artifact under
  `tests/baselines/reference`, and "no content" meaning the file must not
  exist.
- `src/testRunner/compilerRunner.ts`, `CompilerBaselineRunner.runSuite`:
  one case yields the errors, JS output, source map, and type/symbol
  baselines. The `.js` baseline lists the inputs (`//// [name]`) and then the
  outputs, and appends `DtsFileErrors` when the emitted declarations do not
  compile (`harnessIO.ts`, `doJsEmitBaseline`).

## Decisions

### Decision 1: Drive the real CLI and engine instead of the library

- **Context**: A case needs its emission, its tt diagnostics, TypeScript's
  diagnostics, a mapping table, and types. The library (`compile_report`)
  compiles one buffer at a time; multi-file cases need the imported variants
  the CLI collects (`src/main/loading.rs`, `collect_extern_variants`), which
  is private to the binary.
- **Alternatives considered**: (a) Call `compile_report` per unit and
  reimplement import collection in the test: a second implementation of
  CLI behaviour that could drift. (b) Run `ttc --out-dir` and
  `ttc --check-types` on a project directory, and ask the engine
  (`Engine::open_project`, `Project::semantic_tokens`, `Project::hover`)
  for types: what users and editors actually get.
- **Decision and rationale**: (b). The units are written to a project under
  `target/tt-tests/cases-*` (so `node_modules/typescript` resolves as in
  every typed suite), built with `ttc --no-banner --jobs 1 --out-dir`,
  checked with `ttc --check-types`, and the emitted tree is checked by the
  pinned `tsc -p` with the case's own `tsconfig.json`. The output manifests
  (`.<file>.ttc-output.json`) decide which outputs are compiled sources and
  which are support modules, so standard-library files are listed by name
  rather than copied into every baseline.

### Decision 2: Four artifacts, with TypeScript's names

- **Context**: The requested artifacts map onto TypeScript's `.js`,
  `.errors.txt`, `.sourcemap.txt`, and `.types` baselines.
- **Alternatives considered**: One combined file per case (smaller tree, but
  a type change and an emit change would be one diff); per-unit files
  (TypeScript's `runMultifileBaseline`), which multiply files without adding
  information for cases this size.
- **Decision and rationale**: `<name>.ts` (inputs, then outputs, as the
  `.js` baseline does), `<name>.errors.txt` (the build's report, the
  `--check-types` report, then `tsc` on the emitted TypeScript, which is the
  `DtsFileErrors` analog: it is how contract 2 of `AGENTS.md` becomes
  visible), `<name>.map.txt` (the editor projection's verbatim mappings from
  `ttc::emit_mapped_with_kind`, one row per mapping with both positions, the
  length, and the text), and `<name>.types` (each semantic token's hover
  under its source line, in the `>name : type` form). `.errors.txt` is
  absent when all three commands succeed, as in TypeScript; a stale one
  fails, and `UPDATE_EXPECT=1` deletes it.

### Decision 3: Skip every toolchain-dependent artifact together

- **Context**: Without the pinned TypeScript, the emission loses its
  contextual annotations, `--check-types` and `tsc` cannot run, and there
  is no hover. Only the tsgo section of `.errors.txt` was asked to skip.
- **Alternatives considered**: Compare only the `ttc` sections of
  `.errors.txt` without a toolchain: the `--check-types` section is itself
  the typed pass, and the `--out-dir` build section lists outputs whose
  annotation differs, so the prefix would not be the same artifact either.
- **Decision and rationale**: Follow the existing emit-fixture policy
  (`tests/snapshot.rs`): without a toolchain, `.map.txt` (which needs none)
  is still compared, the other three are skipped with a `SKIP` line,
  `TTC_REQUIRE_TSGO=1` makes the skip a failure, and `UPDATE_EXPECT=1`
  refuses to run.

### Decision 4: Reuse `UPDATE_EXPECT` and write the reference files directly

- **Context**: TypeScript writes `tests/baselines/local` and accepts with
  `hereby baseline-accept`; this repository's convention is
  `UPDATE_EXPECT=1` writing the expectation in place.
- **Alternatives considered**: A local/reference split with an accept script
  (a second, parallel mechanism beside `UPDATE_EXPECT`).
- **Decision and rationale**: Keep one mechanism. The comparison moved from
  `tests/snapshot.rs` into `tests/common/baseline.rs` (`expect`,
  `expect_absent`, `diff`), and its failure message names the calling test
  binary (`CARGO_CRATE_NAME`), so every suite prints
  `UPDATE_EXPECT=1 cargo test --test <suite>`. `git diff` is the local
  versus reference comparison.

### Decision 5: Only ttc's own options are directives

- **Context**: TypeScript accepts every compiler option as `// @option`.
- **Decision and rationale**: A case accepts `@filename`, `@rewriteImports`
  (`--rewrite-imports`), and `@noVerify` (`--no-verify`), and an unknown
  directive fails. TypeScript options belong in a `tsconfig.json` unit,
  which is what a tt user writes.

### Decision 6: Seed cases from recent fixes

| Case | Fix | What its baselines show |
| --- | --- | --- |
| `compiler/declaratorListLaterValue.tt` | TASK-593 | the split declaration, clean `--check-types` and `tsc` |
| `compiler/enumMemberStatementValue.tt` | TASK-594 | both placement errors, nothing emitted |
| `compiler/conditionalOperationNarrowing.tt` | TASK-595 | the narrowed lowering, clean checks |
| `compiler/assertedValueStorage.tt` | TASK-596 | storage keeps the value's own type |
| `compiler/armWithoutBody.tt` | TASK-605 | `missing-arm-body`, hover on the kept guard binding |
| `compiler/impossibleCaseAtPattern.tt` | TASK-620 | TS2678 at each impossible pattern |
| `conformance/modules/importedVariantExhaustiveness.tt` | multi-file, `@rewriteImports: ts` | an imported variant's missing case in both reports |
| `conformance/jsx/matchInJsxChild.ttx` | `.ttx` | a match as a JSX child |

## Work log

- 2026-09-30: Read TypeScript's `CONTRIBUTING.md`, `harnessIO.ts`, and
  `compilerRunner.ts` (release-6.0) and typescript-go's `baseline.go`.
- 2026-09-30: Moved `expect` and `diff` from `tests/snapshot.rs` to
  `tests/common/baseline.rs`, added `expect_absent`, and switched
  `tests/practical_diagnostics.rs` to it.
- 2026-09-30: Added `tests/case_baselines.rs`, the seed cases, and their
  baselines (`UPDATE_EXPECT=1 cargo test --test case_baselines`), and read
  every baseline.
- 2026-09-30: Documented "Adding a test case" in `CONTRIBUTING.md` and the
  default in `AGENTS.md`.

## Issues and resolutions

### Issue 1: The `@rewriteImports: ts` case failed `tsc` on its emission

- **Symptom**: `tsc` on the emitted tree reported TS5097 for `./area.ts`.
- **Cause**: `.ts` specifiers need `allowImportingTsExtensions`, a
  TypeScript option the default configuration does not set.
- **Resolution**: The case carries its own `tsconfig.json` unit, which is
  how a project using `--rewrite-imports ts` is configured.

### Issue 2: Findings the seed baselines record

- **Symptom**: `impossibleCaseAtPattern.errors.txt` shows `ttc --out-dir`
  exiting 0 and emitting TypeScript that `tsc` rejects with TS2678 three
  times; `importedVariantExhaustiveness.errors.txt` shows the build and
  `--check-types` suggesting different missing arms (`, Rect => undefined,`
  against `, Rect(width, height) => undefined,`) for the same match; and
  `armWithoutBody.types` shows `const a: any` for the match whose arm has
  no body.
- **Cause**: The first is TASK-620's Decision 1 (the untyped check leaves a
  name that is not a near miss to the checker). The second is the untyped
  and typed passes rendering the help from different case information. The
  third is the recovered arm contributing the error type to the match's
  value.
- **Resolution**: Recorded as they are; the baselines make both visible and
  any change to them a reviewed diff. The help difference and the `any`
  are candidate follow-up tasks.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --test case_baselines --test snapshot --test
  practical_diagnostics -- -D warnings`
- [x] The full gate (`cargo clippy --all-targets -- -D warnings`,
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`, `./scripts/ci agents`)
  ran once over the final tree of TASK-634 to TASK-636; its results are in
  TASK-636.
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test case_baselines`: 8 cases in
  about 16 to 18 seconds on four workers; two consecutive runs compare
  clean.
- [x] A hand-edited `.map.txt` fails with "modified baseline", the line
  diff, and `Run UPDATE_EXPECT=1 cargo test --test case_baselines and review
  the diff.`
- [x] `cargo test --test snapshot --test practical_diagnostics` after the
  move.

## Result

Changed files: `tests/case_baselines.rs`, `tests/common/baseline.rs`,
`tests/common/mod.rs`, `tests/snapshot.rs`, `tests/practical_diagnostics.rs`,
`tests/cases/**`, `tests/baselines/reference/**`, `CONTRIBUTING.md`,
`AGENTS.md`, `docs/tasks/INDEX.md`, and this record. Follow-ups: TASK-635
(baseline tracking and CI enforcement) and TASK-636 (contribution rules).
