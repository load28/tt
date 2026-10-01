# TASK-728: Report declaration diagnostics as `tsc` does

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-728`

## Purpose

TASK-717 D3: with `declaration` (or `composite`) on, `tsc` reports the
declaration transformer's diagnostics (TS9xxx under `isolatedDeclarations`,
TS2883, TS4023, TS4025, TS4094, TS4118, TS5088, TS7056, TS2527, ...);
`ttc --check-types` never asked for them. Repro: `{"compilerOptions":
{"declaration":true,"isolatedDeclarations":true,"noEmit":true}}` and
`export const x = Math.random() ? 0 : 1;`: `tsc` prints
`a.ts(1,14): error TS9010`, ttc printed nothing.

## Scope

- Included: the declaration stage in `src/typescript/host.mjs`, mapped back
  to source like every other TypeScript diagnostic; regression cases;
  `docs/ai/tt.md`; `tests/typed-parity-differences.txt`.
- Excluded: the declarations `ttc --types` writes (unchanged).

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/compiler/program.go`, `GetDiagnosticsOfAnyProgram` (lines
  1986-1990): when `skipNoEmitCheckForDtsDiagnostics` or `noEmit`, and
  `GetEmitDeclarations()` (`declaration` or `composite`), and no diagnostic
  was added after the configuration's, the declaration diagnostics of
  every file.
- `program.go`, `HandleNoEmitOptions` (lines 1906-1930): under
  `noEmitOnError` the emit first gathers the same stages with
  `skipNoEmitCheckForDtsDiagnostics`, and returns them as the emit's
  diagnostics when there are any.
- `program.go`, `Program.Emit` (line 1799) and
  `tsc/internal/compiler/emitter.go`, `emitDeclarationFile` (lines
  221-243): otherwise `tsc` emits, and the declaration transformer's
  diagnostics of each emitted file are emit diagnostics, whatever else was
  reported. `getDeclarationDiagnostics` (`emitter.go` line 566) runs the
  same transformer over the same files to emit, so the two are one set.
- `tsc/internal/execute/tsc/emit.go`, `EmitFilesAndReportErrors` (lines
  74-130): the CLI reports the staged diagnostics followed by the emit's.

## Decisions

### Decision 1: `ttc --check-types` reports the declaration diagnostics the project's own `tsc -p` reports

- **Context**: `ttc --check-types` writes nothing, but the project's
  options decide whether its `tsc -p` writes.
- **Alternatives considered**: (a) Treat every check as `noEmit` (only when
  nothing else was reported): a project that emits gets its declaration
  errors from `tsc` beside its type errors, and ttc would hide them until
  the type errors are fixed. (b) Report them always: a `noEmit` project's
  `tsc` does not.
- **Decision and rationale**: The host asks `getDeclarationDiagnostics`
  for the program's files when `declaration` or `composite` is on and
  `listFilesOnly` is not; under `noEmit` or `noEmitOnError` only when no
  syntactic, option, semantic, or global diagnostic (nor the TS2307 that
  stands for a lowered module's served name) was reported, otherwise
  always. They join the semantic diagnostics, so the report places them
  like any TypeScript diagnostic in a `.tt` file (the related "Add a type
  annotation to the variable x." becomes a label).

## Work log

- 2026-10-01: Reproduced D3; read `GetDiagnosticsOfAnyProgram`,
  `HandleNoEmitOptions`, `Program.Emit`, `emitDeclarationFile`, and
  `getDeclarationDiagnostics`.
- 2026-10-01: Added the stage to the host's `answer`; the 31 D3 corpus
  cases agree with `tsc`; added three cases (one per branch), removed the
  D3 lines from the list, and documented the stage in `docs/ai/tt.md`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aDeclarationDiagnosticIsReportedUnderNoEmitWhenNothingElseIs.tt`
  and `tests/cases/compiler/aDeclarationDiagnosticIsReportedBesideTypeErrorsWhenTheProjectEmits.tt`
  (`cargo test --test case_baselines`); the third,
  `aDeclarationDiagnosticWaitsForTheTypeErrorsUnderNoEmit.tt`, pins the
  `noEmit` short-circuit.
- **Observed failure**: with the previous host both baselines lost
  `error[ts9010]: Variable must have an explicit type annotation with
  --isolatedDeclarations.` at `a.tt:1:14`, and the first's
  `ttc --check-types` exited 0 instead of 1.

## Verification

- [x] The 31 D3 corpus cases (`TTC_TYPED_FILTER`, `TTC_TYPED_CASES=all`):
  31 compared, 0 differ; their 31 lines removed.
- [x] `RUST_TEST_THREADS=4 TTC_REQUIRE_TSGO=1 cargo test --test native
  --test cli --test sidecar --test case_baselines --test integration
  --test workflow_repairs`: all pass.
- [x] The full gate is recorded in TASK-731.

## Result

Changed files: `src/typescript/host.mjs`, `docs/ai/tt.md`, three cases and
their baselines, `tests/typed-parity-differences.txt`,
`docs/tasks/INDEX.md`, and this record. Every D3 difference is gone.
