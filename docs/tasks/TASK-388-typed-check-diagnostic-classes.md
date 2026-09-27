# TASK-388: Report project, configuration, and syntax diagnostics from the typed check

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttc --check-types` asked TypeScript only for semantic diagnostics and dropped every diagnostic without a file. A malformed `tsconfig.json`, a missing `extends` target, an unknown `types` entry, an invalid option value, or a syntax error in a hand-written `.ts` file therefore exited 0, while `tsc -p` with the pinned TypeScript fails on each.

## Scope

- Included: The TypeScript host's diagnostic collection (`src/typescript/host.mjs`), a backend answer for diagnostics without a source position, and their report in the engine.
- Excluded: How TT-level diagnostics and positioned semantic diagnostics are mapped, and `tsc`'s rule of withholding semantic diagnostics when syntax errors exist.

## Decisions

### Decision 1: Collect the diagnostic classes the TypeScript compiler reports

- **Context**: The pinned client (`typescript@7.1.0-dev.20260826.1`, `dist/api/sync/api.d.ts`) exposes `getConfigFileParsingDiagnostics`, `getProgramDiagnostics` ("including compiler options diagnostics"), `getGlobalDiagnostics`, `getSyntacticDiagnostics`, and `getSemanticDiagnostics`. typescript-go's `GetDiagnosticsOfAnyProgram` (`internal/compiler/program.go`) reports configuration parsing, syntactic, program, global, and semantic diagnostics, asking for global diagnostics again after checking.
- **Alternatives considered**: Reproducing `tsc`'s gating (no semantic diagnostics while syntax errors exist) would hide type errors in every other file whenever one hand-written file is mid-edit, which the editor and the existing typed report do not do.
- **Decision and rationale**: Collect all five classes in that order, keep semantic diagnostics unconditionally as before, and merge identical reports (`--extends` failures appear both as configuration and global diagnostics in the probe below).

### Decision 2: Carry positionless diagnostics as project diagnostics

- **Context**: A diagnostic without a file (TS5083, TS2688), or one inside a configuration this host serves with rewritten `.tt` patterns, has no position in text the user wrote.
- **Alternatives considered**: Giving them offset 0 would draw a caret at the start of an unrelated line.
- **Decision and rationale**: The host answers them in `projectDiagnostics`; the backend attributes a fileless one to the project's `tsconfig.json`, and the report emits them without a position.

### Decision 3: Report project-level diagnostics only for a configured project

- **Context**: Without a `tsconfig.json` the host opens an inferred project. typescript-go's `NewInferredProject` (`internal/project/project.go`) supplies default options, including `AllowImportingTsExtensions` without `noEmit`, so the program answers TS5096 about options the user never wrote.
- **Alternatives considered**: Filtering TS5096 by code would be a special case that hides one symptom.
- **Decision and rationale**: Positionless diagnostics describe a configuration; when the user has none, they are not reported. File diagnostics are reported in both cases.

## Work log

- 2026-09-27: Reproduced with a probe script against the pinned client: `getSemanticDiagnostics` returned nothing for all five cases; the other classes returned TS1328/TS5024, TS5083, TS6046, TS2688, and TS1109. `tsc -p . --pretty false` reported the same codes and exited 2.
- 2026-09-27: Changed `src/typescript/host.mjs`, `src/typescript/backend.rs`, `src/typescript/native.rs`, and `src/engine/semantics/report.rs`. Added five CLI regression cases; they failed before the change and pass after it.
- 2026-09-27: The full suite failed `relative_declaration_roots_preserve_duplicate_basenames` with TS5096 from the inferred project. Applied Decision 3; the full suite passed.

## Issues and resolutions

### Issue 1: Configuration, program, and syntax errors exited 0

- **Symptom**: `ttc --check-types src` printed nothing and exited 0 for each case in Purpose.
- **Cause**: The host iterated only `getSemanticDiagnostics()` and skipped diagnostics without `fileName`.
- **Resolution**: Decisions 1 and 2.

### Issue 2: The inferred project's own defaults were reported

- **Symptom**: `ttc --types src -o types` without a `tsconfig.json` failed with TS5096.
- **Cause**: The inferred project's default options, not user input.
- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] The five new `types_reports_*` cases in `tests/cli.rs` failed against the previous host and pass now.

## Result

Changed `src/typescript/host.mjs`, `src/typescript/backend.rs`, `src/typescript/native.rs`, `src/engine/semantics/report.rs`, and `tests/cli.rs`. The typed check now fails on the configuration, program, and syntax errors `tsc` reports.
