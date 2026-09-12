# TASK-371: Audit and repair editor type services

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Exercise inference, completion, and diagnostics in real VS Code extension hosts and repair failures at their owning compiler boundaries.

## Scope

- Included: tt and ttx editor projections, pattern services, incomplete buffers, and cross-file updates.
- Excluded: Release installation and publication; modifications to the external TypeScript extension.

## Decisions

### Decision 1: Preserve source kind through editor analysis and emission

- **Context**: Editor-only queries used TypeScript-default entry points for ttx documents.
- **Alternatives considered**: Masking JSX in the adapter duplicates compiler syntax ownership; filtering suggestions or diagnostics leaves an incorrect underlying projection.
- **Decision and rationale**: Pass the file-derived source kind to the existing lexer, parser, and mapped emitter. Preserve the public TypeScript-default pattern API, sharing its implementation with a source-kind-aware internal entry point. Both standalone and cached project analyses use that implementation.

### Decision 2: Validate actual providers in isolated VS Code profiles

- **Context**: Compiler tests alone cannot prove editor activation, document synchronization, and provider integration.
- **Alternatives considered**: Only exercising the JSON-lines or LSP server would omit the extension host.
- **Decision and rationale**: Run the installed VS Code executable with extension development/test paths and isolated workspaces. Test both the tt extension alone and integration with the installed TypeScript native extension. Keep failures visible rather than suppressing diagnostics or changing timeout behavior.

## Work log

- 2026-09-12: Ran `./scripts/doctor` successfully and fast-forwarded main from 67173eb to f2b46b5. Created `task-371-editor-structural-audit`; the existing `codex` ref prevents a `codex/` branch.
- 2026-09-12: Ran the original 32 real-editor cross-file tests successfully. Added match payload completion, inferred result hover, diagnostic range/lifecycle, JSX text completion, and incomplete pipeline coverage.
- 2026-09-12: Reproduced JSX source corruption in the completion probe. Audited equivalent boundaries and corrected pattern completion, isolated alternative hover, standalone analysis, and the shared project semantic cache.
- 2026-09-12: Added four compiler regression tests covering source preservation, UTF-16 cursor mapping, JSX declaration ownership, the project cache, and pattern completion context.
- 2026-09-12: Corrected the new hover assertion to read `MarkdownString.value`; JSON serialization omits that API property. Diagnostic lifecycle assertions now retain diagnostic details on failure.
- 2026-09-12: Ran the native-extension matrix: 69 of 71 cases passed, including every tt/ttx case and all seven added cases. Two TS-only dependency-discard cases retained TypeScript diagnostics. A separate native-only control without the tt extension passed eight repetitions; the intermittent cause remains unconfirmed.

## Issues and resolutions

### Issue 1: Editor queries interpreted JSX text as tt code

- **Symptom**: Completion probes rewrote JSX text containing tt-looking syntax. Pattern completion could offer cases inside JSX text, and pattern hover/analysis used TypeScript projections for TSX documents.
- **Cause**: Five editor paths discarded `SourceKind` before lexing, analysis, or emission, including the shared semantic cache.
- **Resolution**: Preserve source kind across all five paths using the existing compiler stages. Regression tests pin source preservation, cursor mapping, declaration ownership, and completion context.

### Issue 2: Native-extension TS-only discard diagnostics remain intermittent

- **Symptom**: In the native matrix, `ts -> tsx` and `tsx -> tsx` retained TS2322/TS2339 after discarding a dependency edit. An earlier run also failed a discard assertion on another edge.
- **Cause**: Not established. A native-only control passed eight repetitions; the final native matrix passed every tt/ttx case.
- **Resolution**: No workaround or diagnostic suppression introduced. Preserve the failing results under `target/editor-tests/run-6iHUP2/results.json` and the control under `target/editor-tests/native-control-8qCzSh/results.json`.

### Issue 3: Website verification could not reach its prerender server

- **Symptom**: `./scripts/ci website` failed with ETIMEDOUT/ECONNREFUSED for its local preview port, including a retry outside the sandbox.
- **Cause**: Not established; this is outside the changed compiler/editor paths.
- **Resolution**: No website changes retained. The generated highlighted file was restored after verification. The npm gate, initially blocked by network access, passed on retry.

## Verification

- [x] `./scripts/doctor`
- [x] `./scripts/ci rust` on the final compiler changes, including fmt, clippy, all Rust tests, and Rust auxiliary checks (`/tmp/tt-task371-rust-complete.log`).
- [x] All 279 compiler library tests, including four new regression tests.
- [x] `./scripts/ci npm` with network access.
- [x] Final `./scripts/ci extension`: 171 passed, zero skipped (`/tmp/tt-task371-extension-final.log`).
- [x] Final standalone VS Code matrix: 39/39 (`target/editor-tests/run-nuQYK5/results.json`).
- Native VS Code matrix: 69/71; two TS-only failures described above.
- Native-only discard control: 8/8.
- Full `./scripts/ci` is not wholly green: website prerender verification remains unsuccessful.

## Result

Completed the source-kind correction across five editor/compiler paths. The final standalone editor matrix, extension server suite, and Rust gates pass. External/native-control and website verification limitations remain explicitly recorded above.

Changed files: `src/analysis/mod.rs`, `src/engine/project.rs`, `src/engine/completions.rs`, `src/engine/language/{project,service,tests}.rs`, `editors/vscode/test/editor.cjs`, `editors/vscode/scripts/test-editor.mjs`, and this task/index record.
