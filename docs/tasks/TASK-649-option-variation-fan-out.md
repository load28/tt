# TASK-649: Run a case once per value of a comma-separated option

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-649`

## Purpose

A behaviour that depends on a ttc option (`--rewrite-imports`,
`--no-verify`) needs one case file per option value today, each a copy of
the others. TypeScript's compiler runner reads a comma-separated option
value as a request to run the case once per value, with the configuration in
each baseline's name. This task adds that to the case runner.

## Scope

- Included: `tests/case_baselines.rs` (configurations and their names),
  `tests/common/cases.rs` (`Unit` is `Clone`), the seed case
  `tests/cases/conformance/modules/importSpecifierRewrite.tt` with its
  baselines, and `CONTRIBUTING.md` ("Adding a test case").
- Excluded: editor cases (`tests/editor_cases.rs` has no ttc options), and
  TypeScript compiler options, which a case sets in a `tsconfig.json` unit
  (TASK-634, Decision 5).

## Sources modelled

- microsoft/TypeScript `release-6.0` at
  `050880ce59e30b356b686bd3144efe24f875ebc8`:
  `src/harness/harnessIO.ts`, `splitVaryBySettingValue` (lines 1069 to
  1128: comma-split, trimmed and lowercased entries; `*` for every value;
  `-value` or `!value` to exclude; no variation when there is at most one
  entry and no `*` or exclusion; an empty result is an error),
  `computeFileBasedTestConfigurationVariations` (lines 1130 to 1143: the
  cartesian product), and `getFileBasedTestConfigurations` (lines 1165 to
  1185: "Provided test options exceeded the maximum number of variations"
  past 25); `src/testRunner/compilerRunner.ts`, `CompilerTest.varyBy`
  (lines 128 to 153) and the configured name (lines 173 to 189:
  `basename(key=value,key=value)extname`, keys sorted and lowercased).

## Decisions

### Decision 1: TypeScript's syntax and naming, over ttc's two options

- **Context**: TypeScript varies every boolean and enumerated compiler
  option that affects a program; ttc's case runner accepts only
  `@rewriteImports` and `@noVerify` (TASK-634, Decision 5).
- **Alternatives considered**: A separate `// @varyBy` directive (not
  TypeScript's shape); only a plain list without `*` and exclusions (a
  partial copy that reads the same but means less).
- **Decision and rationale**: The whole `splitVaryBySettingValue` behaviour
  over the two options' value sets (`noVerify`: `true`, `false`;
  `rewriteImports`: `js`, `ts`, `off`). Every value is now checked, so
  `@noVerify: yes` fails instead of meaning `false`. A configured case is
  named `<name>(key=value,...)` with the varied options only, keys
  lowercased and sorted, so each configuration has its own four baselines,
  TASK-635's tracking sees each file, and a filter (`TT_CASES`) matches the
  case's own name.

### Decision 2: Keep the cap though it cannot be reached yet

- **Context**: The two options make at most six configurations.
- **Decision and rationale**: The check that fails past 25 configurations
  stays, with TypeScript's limit, so a third option cannot make a case
  quietly expensive. Recorded here that it is currently unreachable.

### Decision 3: The seed shows what each import mode does to a program

- **Decision and rationale**: `importSpecifierRewrite.tt`
  (`@rewriteImports: js, ts, off`) imports a variant from another tt unit.
  Its baselines show `./token.js`, `./token.ts`, and `./token.tt` in the
  three emissions; with `off`, `tsc` on the emitted TypeScript reports
  TS2307 for `./token.tt`, while `ttc --out-dir` and `--check-types`
  succeed, which is what the option documents.

## Work log

- 2026-09-30: Read `harnessIO.ts` and `compilerRunner.ts` at `050880c`.
- 2026-09-30: Replaced the case runner's single `Settings` with
  `configurations`, added the seed case, generated its baselines
  (`UPDATE_EXPECT=1 TTC_REQUIRE_TSGO=1 TT_CASES=importSpecifierRewrite
  cargo test --test case_baselines`), and read them.
- 2026-09-30: Checked `*`, exclusions, the two-option product, and an
  invalid value with a temporary case (below), then removed it.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this task extends the test runner and fixes no bug.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test case_baselines`: every case,
  including the three configurations of the seed, passes; no existing
  baseline changed.
- [x] A temporary case with `@rewriteImports: *, -off` and `@noVerify: *`
  produced exactly four configurations, `(noverify=false,rewriteimports=js)`
  to `(noverify=true,rewriteimports=ts)`; `@rewriteImports: js, cjs` failed
  with "@rewriteimports takes js, ts, off, not `cjs`". Both files removed.
- [x] `cargo clippy --test case_baselines --test incremental -- -D
  warnings`.
- [x] The full gate ran over the final tree of TASK-647 to TASK-651; its
  results are in TASK-651.

## Result

Changed files: `tests/case_baselines.rs`, `tests/common/cases.rs`,
`tests/cases/conformance/modules/importSpecifierRewrite.tt`,
`tests/baselines/reference/importSpecifierRewrite(*)`, `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and this record.
