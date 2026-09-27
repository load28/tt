# TASK-433: Map recovered syntax as atoms in the content mapper

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A `.tt` file with a recoverable tt error (a malformed `match` or `variant`, a top-level `try`, a bad variant field type) made `tsc --runExternalCode` reject the whole file with `TS100029: The content mapper '@openload28/tt-lang' produced a verbatim mapping that does not match the original content`, so the tt diagnostic that names the actual mistake never reached the user.

## Scope

- Included: The span map `ttc --content-mapper` returns from `transform` (`src/content_mapper.rs`), a protocol-level regression, and an end-to-end regression through the pinned TypeScript.
- Excluded: The recovery placeholders themselves (`compile_projection_report`), which the typed engine already consumes correctly through `ProjectionReport::recovered`, and TypeScript's own CLI policy of reporting semantic diagnostics only when there are no syntactic ones (see Issue 2).

## Decisions

### Decision 1: Split verbatim chunks at recovered ranges in the mapper, and map the recovered stretch as an `Atom`

- **Context**: `compile_projection_report` (`src/lib/compile.rs`) overwrites each recovered node's bytes with a length-preserving placeholder (`undefined as any`, `;`, `any`, `class Name {}`) and compiles that altered copy, reporting the node ranges in `ProjectionReport::recovered`. Its mappings are therefore true byte copies of the *altered* source. The mapper published every one of them as `SpanMapKind.Verbatim` against the *original* `content` and ignored `recovered`. The pinned TypeScript (`typescript@7.1.0-dev.20260826.1`) defines `Verbatim` as a length-preserving, edit-safe copy (`dist/ast/spanMap.d.ts`: `isExact` — "a precise, edit-safe projection through one verbatim segment"; `dist/ast/spanMap.js` `mapVerbatimPosition` — "a length-preserving verbatim segment") and its Go host validates it against the original text, reporting diagnostic 100029 "The content mapper '{0}' produced a verbatim mapping that does not match the original content (virtual offset {1}, original offset {2})" (string table of `@typescript/typescript-linux-x64/lib/tsc`). `SpanMapKind` offers `Atom` for a correspondence with different text (`dist/enums/spanMapKind.enum.d.ts`).
- **Alternatives considered**: (a) Rewrite `ProjectionReport` mappings in the library so they never cover recovered text — changes what the typed engine and language service consume, which already skip recovered ranges by position (`src/engine/projection.rs`, `src/engine/language/service.rs`), for no gain there. (b) Drop the emission when anything was recovered — loses every sibling declaration the recovery exists to keep. (c) Drop only the mappings that overlap recovery — leaves unmapped virtual text whose diagnostics TypeScript maps to insertion points with `None` fidelity instead of the construct.
- **Decision and rationale**: The mapper is the layer that asserts `Verbatim` to TypeScript, so it applies the recovery information it is given. Each mapping is split at the `recovered` ranges: the parts outside stay `Verbatim`; each part inside becomes an `Atom` whose original range is the whole recovered node and whose features are `SpanMapFeature.None`, the same treatment compiler-written glue already gets. Virtual spans stay sorted and non-overlapping, and anchors still claim only unclaimed stretches.

## Work log

- 2026-09-27: Reproduced in a scratch mapper project (TypeScript and `@typescript` linked to the repository copies, the mapper package exec pointing at the debug `ttc`, the documented top-level `contentMappers` entry). All three inputs (malformed `match`, malformed `variant`, top-level `try`) printed only `src/b.tt(1,1): error TS100029 ... (virtual offset 0, original offset 0)`: the first verbatim chunk spans the whole file including the placeholder.
- 2026-09-27: Added the `recovered` parameter to `span_mappings` and the split. After the change the same runs print `src/b.tt(2,11): error tt7: tt \`match\` could not be parsed`, `src/b.tt(1,8): error tt6: tt \`variant\` could not be parsed`, and `src/b.tt(2,11): error tt11: \`try\` must be inside a function ...`, with no TS100029.
- 2026-09-27: Added `recovered_syntax_never_travels_as_verbatim_text` (`src/content_mapper/tests.rs`), which applies TypeScript's verbatim rule to the `transform` answer for four recovery kinds, and `recovered_syntax_reports_its_tt_diagnostic_instead_of_a_rejected_mapping` (`tests/content_mapper.rs`), which runs the pinned `tsc --runExternalCode`. With the fix reverted both fail (`verbatim span [0,88,0,88,0] differs from the original`; TS100029 in the tsc output); with it both pass.

## Issues and resolutions

### Issue 1: Every recovered file was rejected by TypeScript

- **Symptom**: `src/b.tt(1,1): error TS100029: The content mapper '@openload28/tt-lang' produced a verbatim mapping that does not match the original content (virtual offset 0, original offset 0).` and nothing else.
- **Cause**: `transform` emitted the recovered compilation's mappings as `Verbatim` without consulting `ProjectionReport::recovered`.
- **Resolution**: Decision 1.

### Issue 2: `main.ts`'s TS2322 is still not printed next to the tt diagnostic

- **Symptom**: After the fix, `tsc -p . --runExternalCode` prints the tt diagnostic but not `main.ts(2,7): error TS2322`.
- **Cause**: Not a mapper defect. The `tsc` CLI reports semantic diagnostics only when a program has no syntactic ones (TypeScript's `emitFilesAndReportErrors`), and mapper diagnostics are reported in the syntactic phase. Measured with the pinned TypeScript: a `.tt` file with only a non-exhaustive `match` (no recovery, unaffected by this task) also suppresses `main.ts`'s TS2322, and so does an ordinary `src/c.ts` containing `const q = (;` (only `TS1109` is printed). Removing the tt error restores `main.ts(2,7): error TS2322`.
- **Resolution**: None needed in ttc; the mapping is now accepted, so once the tt error is fixed (or in the editor, which checks per file) TypeScript checks the file and its importers normally.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --bin ttc content_mapper`: 23 passed.
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test content_mapper`: 14 passed.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/content_mapper.rs`, `src/content_mapper/tests.rs`, and `tests/content_mapper.rs`. A file with a recoverable tt error now reaches TypeScript as an accepted virtual file, and its tt diagnostics report at their `.tt` positions under the `tt` source.
