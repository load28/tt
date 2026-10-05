# TASK-761: Preserve template boundaries during editor recovery

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: `TASK-761: fix(editor): preserve skipped template boundaries`

## Purpose

Review PR #140 repeatedly and correct reproducible recovery defects.

## Scope

- Included: parser recovery boundaries, regression coverage, final review and gates.
- Excluded: language features and toolchain changes.

## Decisions

### Decision 1: Track template interpolation boundaries in the recovery scanner

- **Context**: Skipping a malformed statement treats interpolation braces as statement boundaries.
- **Alternatives considered**: Reparse skipped expressions, or track their lexical delimiters.
- **Decision and rationale**: Track template delimiters and rescan template continuations using the existing lexer API. Skipped text does not need a new semantic interpretation.

## Work log

- 2026-10-05: Doctor passed. Updated the existing PR branch to remote head 940527f8. Reviewed the diff and previous findings. A 1,201-input mutation probe found a skipped template consuming a later independent tt expression.

- 2026-10-05: Added failing parser and projection tests, fixed template scanning, and confirmed the type-recovery path independently fails without its change. Added two editor cases with TypeScript twins; diagnostics, hover, and definition agree. Checked malformed interpolation separately: TypeScript itself loses following declarations for that input, so no speculative interpolation change is retained.

- 2026-10-05: Rebuilt the projection probe against the final production code. All 1,201 character-deletion and truncation inputs completed without panic, timeout (four seconds per input), or invalid mapping extent. Reviewed the final recovery loop and PR boundaries again; no additional actionable defect was found.
- 2026-10-05: The sandboxed full gate could not run the idle-host CPU measurement because `ps` is restricted. Restarted the same gate outside the sandbox; the idle-host test passed.

- 2026-10-05: Final `./scripts/ci` passed all six stages (agents, rust, npm, website, native, extension), including 420 library tests, 193 compile API tests, 25 content-mapper tests, 16 parser recovery tests, all case baselines, incremental tests, and 241 extension tests. Log: `/tmp/tt-761-ci-final.log`.

## Issues and resolutions

### Issue 1: Skipped templates consume following declarations

- **Symptom**: A malformed declaration preceding a template makes a later match become an error placeholder.
- **Cause**: Recovery skips tokens without tracking template interpolation or rescanning its closing brace.
- **Resolution**: Track interpolation frames in the recovery delimiter stack and rescan matching braces as template continuations. Share this scanner with type recovery, retaining its initializer boundary.

## Regression test (fails before the fix)

- **Path**: `tests/swc_editor_recovery.rs::skipped_templates_preserve_following_declarations` and `tests/compile.rs::editor_recovery_does_not_replace_matches_after_skipped_templates`.
- **Observed failure**: On unchanged production code, the parser returned no `later` declaration instead of `["later"]`; the projection replaced its complete match with `undefined as any`.
- **Path**: `tests/swc_editor_recovery.rs::skipped_type_templates_preserve_following_declarations`.
- **Observed failure**: Restoring the original type-recovery loop after the statement fix returned only `["broken"]`, losing `later`. Reinstating the shared scanner passes.
- Editor cases `recoverySkippedTemplate` and `recoveryTypeTemplate` pin diagnostics, hover, definition, and TypeScript twin parity.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed
- [x] `./scripts/ci` (all six stages)
- [x] Final 1,201-input mutation pass

## Result

Fixed template-boundary loss in statement/expression and type recovery. Final review found no further actionable defect in the inspected changes and tested inputs.

Changed files:

- `vendor/swc_ecma_parser/src/parser/recovery.rs`: shared template-aware recovery scanner.
- `tests/swc_editor_recovery.rs` and `tests/compile.rs`: parser ownership and projection API regressions.
- `tests/cases/editor/recoverySkippedTemplate.{tt,ts}` and `recoveryTypeTemplate.{tt,ts}`, with their two editor baselines: TypeScript diagnostic/query parity.
- This record and `docs/tasks/INDEX.md`.

