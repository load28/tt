# TASK-715: Point every `val-mutation` report at the binding's declaration

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-715`

## Purpose

TASK-701 Issue 2: for `val const cfg = {...}; cfg.n = 2;`, `ttc
--check-types` draws "the read-only binding is declared here" under the
`val` and offers "remove `val` if this binding is intended to be mutable",
and the editor publishes the label as related information; `ttc --check`
(and the server's `check`) report the same diagnostic with neither. The
untyped pipeline's diagnostics had no way to carry a label at all.

## Scope

- Included: labels on the untyped pipeline's diagnostics
  (`TtError::labels`, `Diagnostic::labels`, `DiagnosticLabel`), drawn by
  `render::diagnostic`, sent by the server's `check` (`labels`, the shape
  `typedCheck` already sends), relayed by the VS Code adapter's text
  layer, and carried into the typed report's tt layer; one author of the
  `val-mutation` report (`val::mutation_error`) used by both paths;
  `tests/editor-diagnostic-differences.txt`.
- Excluded: the TypeScript content mapper's wire diagnostic, which has a
  message and a range only (`src/content_mapper.rs`); mutating method
  calls, which only the typed path can judge.

## Sources

- LSP 3.17, `Diagnostic.relatedInformation` (`DiagnosticRelatedInformation`:
  a location and a message), what the adapter publishes a label as.
- rustc's diagnostic structure (`rustc_errors::Diagnostic`, a primary span
  with secondary labeled spans), the model `src/render.rs` draws.

## Decisions

### Decision 1: Labels are part of the untyped diagnostic, in byte offsets

- **Context**: The typed path's `engine::Diagnostic` has labels; the
  untyped `Diagnostic` (`#[non_exhaustive]`, byte offsets) had none, so no
  untyped rule could point at a second place.
- **Alternatives considered**: (a) Fold the declaration into the message
  text: every surface then shows different words from the typed path's.
  (b) Give the untyped diagnostic labels in its own coordinates, as it has
  suggestions.
- **Decision and rationale**: (b): `DiagnosticLabel { start, end,
  message }`, exported at the crate root as `Suggestion` is (a public API
  addition, `tests/baselines/reference/api/ttc.api.txt`), added with
  `TtError::label`, drawn by the renderer as the typed labels are, and
  sent by the server's `check` as `labels` only when there are any (the
  convention `typedCheck` set for consumers of the older shape). The typed
  report keeps the labels of the tt diagnostics it relays from a
  projection or from sema (`labels_in`), which it dropped before.

### Decision 2: One author for the `val-mutation` report

- **Context**: The message, the suggestion, and the label were written
  twice, once in `src/val/checker.rs` and once in
  `src/engine/semantics/report.rs`, and only the second had all three.
- **Decision and rationale**: `val::mutation_error(start, end, name,
  method, binding)` writes the message (with or without a mutating
  method), and, when the binding's declaration is known, the suggestion
  and the label. The untyped checker knows the `val` offset from its scope
  (`lookup`) and the modifier's end from the parser's modifiers; the typed
  path knows both from the probe and converts the label against the file
  that declares the binding, so a binding declared in another file is
  still labeled there.

## Work log

- 2026-10-01: Compared `ttc --check`, `ttc --check-types`, and the
  adapter on the two sampled editor cases.
- 2026-10-01: Added `Label`, the builder, the renderer and server fields,
  the adapter relay, and `val::mutation_error`; removed the two listed
  differences; regenerated the baselines and read their diff.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/diagnostics/val-mutation_propertyWrite_functionBody.tt`
  and `..._componentBody.ttx` (`TT_REQUIRE_EXTENSION=1 cargo test --test
  editor_cases`), with their lines removed from
  `tests/editor-diagnostic-differences.txt`.
- **Observed failure**: Without the change in `src/`:
  "val-mutation_propertyWrite_componentBody: the editor publishes
  `val-mutation` with 1 related place(s), and `ttc --check` shows 0 other
  one(s)" (and the same for `_functionBody`).

## Verification

The gate for TASK-713 to TASK-716, run on this tree with
`CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0`:

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast`: 30 test binaries, all passed.
- [x] `node scripts/check-baselines --tracking <dir>`: 5 327 compared, none
  unused.
- [x] `TT_MATRIX_CASES=all` `UPDATE_EXPECT=1` runs of `case_baselines` and
  `editor_cases` (the baselines regenerated and their diff read: the
  label and suggestion under every untyped `val-mutation`, the variant
  named in every typed `match-not-exhaustive`), then a
  `TT_MATRIX_CASES=all` run of `case_baselines` without it: passed.
- [x] `npm test` in `editors/vscode`: 238 passed.
- [x] `./scripts/ci agents`: passed (doctor reports this checkout not set
  up: no release `ttc`, which `./scripts/setup` builds and this task did
  not run).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed `src/diagnostics.rs`, `src/diagnostics/tests.rs`, `src/error.rs`,
`src/lib.rs`, `src/render.rs`, `src/server.rs`, `src/val.rs`,
`src/val/checker.rs`, `src/engine/semantics/report.rs`,
`editors/vscode/server/src/ttc.ts`,
`tests/editor-diagnostic-differences.txt`, the public API baseline, and
the baselines of the untyped `val-mutation` reports. Every surface shows
the same label and suggestion.
