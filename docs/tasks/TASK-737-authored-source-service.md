# TASK-737: Serve authored buffers with their contextual projections

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Implement the approved TASK-735 serving contract and resolve TASK-733's installed-mapper coordinate collision.

## Scope

- Included: revision exchange, mapper integration, coordinate boundaries, probes, regression tests, full local gates, and the requested new PR.
- Excluded: upstream TypeScript modifications and unrelated TASK-732 findings.

## Decisions

### Decision 1: Execute through repository task records

- **Context**: User approved the design and plan and explicitly requested the repository task workflow.
- **Alternatives considered**: Maintain parallel skill documents or consolidate them.
- **Decision and rationale**: TASK-735 contains the approved design; this record contains its implementation steps. No further skill approval gates are added.

### Decision 2: Invalidate the TypeScript mapper cache at its owning session

- **Context**: Closing and reopening an equal source buffer does not invalidate TypeScript's content-keyed mapper transform. The new contextual-refresh test changed the emitted storage type but still observed the old function return type.
- **Alternatives considered**: Alter source text to perturb the key, rewrite user configuration, or replace the private service host when its projection changes independently of source.
- **Decision and rationale**: Publish the complete graph atomically before document notifications. For equal source/new projection, replace the host once for that graph and replay overlays. Clear completion handles owned by the previous host. Unchanged graphs retain the process. This replaces the plan's insufficient close/reopen step.

### Decision 3: Keep generated probes under a distinct virtual identity

- **Context**: Signature and completion probes intentionally ask about generated syntax, while ordinary installed-mapper requests address original source. Sending generated text under the source URI recreates the original ambiguity.
- **Decision and rationale**: `open_document` publishes an authored document and its projection; temporary generated requests use the existing virtual document identity. Restore the prior document on success and error, including completion resolve. The probe's existing mapping remains local to its request; no source buffer is overwritten with lowered text.

### Decision 4: Carry named-declaration and shared-binding mappings

- **Context**: Byte-copy maps alone omit names the compiler declares structurally. The existing configured-mapper parity test caught missing variant definitions/references.
- **Decision and rationale**: Add emitter-owned exact name spans to mapper mappings without exposing surrounding glue. A shared declaration has one canonical source representative; normalize queries through the binding metadata and expand returned targets/edits to all authored occurrences. Do not create overlapping virtual spans. The pinned TypeScript `spanmap.go` requires disjoint virtual segments and exact text for editable mappings.

### Decision 5: Read diagnostic provenance before LSP mapping

- **Context**: The pinned TypeScript LSP converts and sometimes aggregates generated diagnostic spans before returning them. Authored ranges cannot recover the lowering anchor or payload destructuring list reliably.
- **Alternatives considered**: Guess provenance from authored ranges; temporarily duplicate modules under virtual paths; or use the existing native API's per-file diagnostics on the same contextual graph.
- **Decision and rationale**: Add a separate editor diagnostic request to the native backend. It preserves generated UTF-16 spans, category, tags and related places for syntactic, semantic, suggestion and enabled declaration diagnostics. Normalize native and virtual LSP responses into one projected diagnostic contract and reuse the existing anchor/list mapping. Only installed-mapper sessions use native diagnostics; unconfigured and foreign-mapper sessions keep their existing LSP project/module-resolution arrangement. Restrict the diagnostic request to its target file; preserve the complete graph, standard packages and host overlays. CLI diagnostic collection is unchanged.

## Work log

- 2026-10-03: Started from the existing task branch, preserving TASK-733/734 changes. Doctor passed. The referenced executing-plans skill was absent from installed skill roots; execution follows the approved plan and the user's repository-task instruction.

- 2026-10-03: Implemented exchange isolation, exact-source matching, version acknowledgment, shared span serialization, authored serving, virtual probe restoration, and coordinate handling across editor operations. The six installed-mapper regressions passed, including contextual output changes, `.ttx`/spaced roots, and isolated unsaved sessions. Added auto-import and incompatible-mapper tests; the first native suite passed 170 tests before review.

- 2026-10-03: Independent review reproduced diagnostic provenance and completion-scope regressions. The corrections pass all 10 installed-mapper tests. A first full-native rerun exposed intentional differences in the unconfigured LSP project arrangement; retained that arrangement and normalized both diagnostic transports into one projected-span contract. Restarted the complete gate after this correction; the interrupted earlier run is not a passing gate.
- 2026-10-03: Fetched origin again; main remains at the branch base. Baselines for TASK-733/734 were reread; no unrelated baseline updates were accepted.

- 2026-10-03: Final complete local gate passed all six stages. Baseline ownership audit found none unused; no unrelated baselines changed. Task and index statuses were finalized together.

## Issues and resolutions

- Declaration-map/direct coordinate collision: fixed by authored buffers and explicit virtual probes.
- Equal-source contextual cache reuse: fixed by complete graph publication and host replacement.
- Variant/shared-binding targets lost through basic byte maps: fixed by emitter-owned name and binding metadata.
- Independent review found two regressions: completion scope analysis parsed authored TT instead of emitted TypeScript, and authored LSP diagnostics lost generated-span provenance (pipeline translations and payload-list hints). Both are being pinned before correction.
- Review fixes and full local gate: verified.

## Regression test (fails before the fix)

- **Path**: `tests/native/editor_service.rs::installed_mapper_keeps_direct_and_declaration_map_targets_in_source_coordinates`.
- **Observed failure**: Before production changes, `installed_mapper_keeps_direct_and_declaration_map_targets_in_source_coordinates` failed with `work(1): []` (zero locations, expected one). Direct navigation passed first. Log: `/private/tmp/tt-737-before.log`.
- Reviewer regression `tests/native/editor_service.rs::installed_mapper_preserves_completion_scope_after_lowering` failed with `unexpected answer` before the scope correction (`/private/tmp/tt-737-scope-before.log`). `installed_mapper_preserves_diagnostic_provenance` failed with raw TS2345 messages and missing pipeline related places before the native diagnostic path (`/private/tmp/tt-737-diagnostics-before.log`).
- The direct mapper protocol test failed before integration: actual `export const n = 1;` instead of the published contextual projection `export const n: number = 1;`. Log: `/private/tmp/tt-737-mapper-before.log`. Both regressions pass after integration.

## Verification

- [x] Failing installed-mapper regression automated
- [x] Focused tests and reviewed baselines: 172 native tests pass, including 10 installed-mapper tests; TASK-733/734 baselines reread
- [x] Full local gate
- [x] Independent branch review: two regressions fixed and independently rechecked
- [x] PR description prepared for the verified branch

## Final verification (2026-10-03)

- `./scripts/ci` passed all six stages: agents, rust, npm, website, native, and extension.
- Default macOS temporary root; `RUST_TEST_THREADS=2 GOMAXPROCS=2`. Disposable test commits used process-local `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false`.
- 419 library tests, 172 native tests, and 238 extension tests passed. Baseline audit: 5,468 compared, none unused; 5,793 unsampled matrix baselines remain outside this standard gate.
- Log: `/private/tmp/tt-737-final-ci-2.log`. Full nightly matrices and complete upstream sweeps remain separate TASK-732 work.

## Result

Authored-source serving, exact contextual projections, probe restoration, shared symbols, completion scope and diagnostic provenance are verified. Independent review findings are resolved and all required local gates passed.

## Approved implementation steps

1. Publish exact-source contextual projections in an isolated, atomic, versioned exchange and reject incompatible mapper sessions.
2. Serve authored documents, keep generated probes under virtual identities, restore documents on success and error, and refresh the owning host for equal-source projection changes.
3. Pin definitions, shared symbols, completion, diagnostics, contextual refresh and session isolation with genuine installed-mapper regressions. Retain the existing virtual arrangement when no trusted installed mapper is configured.
4. Run the complete repository gate, address independent review, finish the task records and create the requested PR against main.

## Changed files

- `src/typescript/content_projection.rs`, `service.rs`, `backend.rs`, `native.rs`, `host.mjs`, and `mod.rs`; `src/lib.rs`.
- `src/content_mapper.rs` and `src/content_mapper/tests.rs`.
- `src/engine/language.rs`, `language/project.rs`, `language/service.rs`, `language/tests.rs`, `project.rs`, and `projection.rs`.
- `tests/common/installed_mapper.rs`, `tests/common/mod.rs`, `tests/content_mapper.rs`, and `tests/native/editor_service.rs`.
- `docs/design/lsp-architecture.md`, this record, and `docs/tasks/INDEX.md`.

TASK-733/734 own their case baselines and canonical-file fixes; TASK-736 owns portability-only tests.
