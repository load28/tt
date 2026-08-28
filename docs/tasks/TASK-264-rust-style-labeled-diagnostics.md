# TASK-264: Rust-style labeled diagnostics (secondary spans everywhere)

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

TASK-263 put every pipeline mismatch on the rejecting step; a rustc-grade
diagnostic also shows *why* — a secondary labeled span ("the piped value
comes from this step", "the expected type comes from this declaration").
Give every diagnostic, not just pipelines, the ability to carry labeled
secondary spans, rendered Rust-style in the CLI and as LSP related
information in the editor.

## Scope

- Included: a `labels` field through the whole diagnostic pipeline (tt-level
  `Diagnostic`, engine `Diagnostic`, renderer, `--server` JSON, VS Code
  extension); a synthesized producer-step label on pipeline mismatches
  (`EmitAnchor` gains a context span); forwarding the checker's own
  `relatedInformation` as labels on both backends (native host and LSP
  service), mapped through the existing origin machinery; regression tests.
- Excluded: restructuring tt-level analyses to compute new secondary spans
  (labels are attached where the data already exists — the checker's
  related information and the pipeline anchors); multi-file sub-snippets in
  the CLI (cross-file labels render as notes, like rustc's `note:` form).

## Decisions

### Decision 1: One label mechanism, two producers — no per-construct special cases

- **Context**: "make errors look like Rust" must not become one bespoke
  renderer per construct.
- **Alternatives considered**: per-construct label synthesis in the report
  layer (drifts, and duplicates what the checker already knows); parsing
  the checker's prose for provenance (string-shape heuristics).
- **Decision and rationale**: labels are plain data on the diagnostic. They
  are produced in exactly two general ways: (a) the checker's own
  `relatedInformation` spans, mapped back to `.tt` coordinates by the same
  `diagnostic_origin` machinery every span already travels through, and
  (b) a construct anchor's optional context span (`EmitAnchor::context`),
  which the emitter records where only it knows the relationship — the
  pipeline's producing step. Renderers and protocols see one field.

### Decision 2: CLI drawing rules

- **Context**: rustc draws primary (`^^^`) and secondary (`---`) underlines
  in one snippet with elision between annotated lines.
- **Decision and rationale**: same-file, single-line labels join the primary
  snippet: annotated lines in order, `-` underlines with the label text,
  `...` gutter when lines are far apart, secondary rows stacked under the
  source line when they share the primary's line. A multi-line primary or a
  cross-file label degrades to a `= note:` line naming the place — the
  honest fallback rustc also uses. The unlabeled rendering stays
  byte-identical, which keeps every existing fixture green.

## Work log

- 2026-08-28: Surveyed the surfaces: `render.rs` (single-span renderer),
  `diagnostics.rs`/`engine::Diagnostic` (no label field), `server.rs`
  (JSON has no related array), `language.rs` (drops LSP
  `relatedInformation`), `host.mjs` (drops checker related info),
  `EmitAnchor` (no context span; literal construction only in two tests).
- 2026-08-28: Implemented labels end-to-end; pipeline producer labels;
  checker related-information forwarding on both backends; extension
  mapping to LSP related information; tests at every layer.

## Issues and resolutions

### Issue 1: the tsgo LSP omits related information from pull diagnostics

- **Symptom**: the editor path (`tsDiagnostics`) received no
  `relatedInformation` for a diagnostic the native host reports one for
  (`{ name: 1 }` against `{ name: string }`).
- **Cause**: verified by dumping the raw `textDocument/diagnostic` items —
  the tsgo preview does not send the field today.
- **Resolution**: the mapping code is in place and guarded; the editor's
  labels currently come from the anchor context (pipelines), and the
  checker's related places light up without further work once tsgo sends
  them. The CLI path gets both today via the native host.

### Issue 2: TASK-263's negative assertion collided with the new label

- **Symptom**: `a_curried_combinator_chain_blames_the_step_with_the_wrong_argument`
  failed once labels landed — it asserted the healthy `mapP` line never
  appears in the diagnostic block, and the producer label now quotes it.
- **Cause**: the assertion over-specified its intent ("the primary span
  does not swallow healthy steps") as "the line is absent".
- **Resolution**: the test now asserts one primary caret row and the
  labeled producer line.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci --skip rust` (native, extension, npm, agents)

## Result

Every diagnostic can now carry labeled secondary spans, drawn rustc-style:

```
error[ts2345]: this pipeline step expects `number`, but receives `string`
  required type: `TOption<number>`
 --> src/labels.tt:7:8
  |
6 |     |> Option.mapP((x: number) => String(x))
  |        ------------------------------------- the piped value is produced here
7 |     |> Option.unwrapOrP(0)
  |        ^^^^^^^^^^^^^^^^^^^
```

and, from the checker's own related information:

```
error[ts2322]: type mismatch: expected `string`, found `number`
 --> src/opts.tt:2:26
  |
1 | type Opts = { name: string };
  |               ---- The expected type comes from property 'name' which is declared here on type 'Opts'
2 | export const o: Opts = { name: 1 };
  |                          ^^^^
```

Changed files:

- `src/render.rs` — `Label`, the annotated multi-span snippet writer, the
  `= note:` fallback; unlabeled output stays byte-identical.
- `src/engine/semantics.rs` — `DiagnosticLabel`, `checker_labels` (anchor
  context + checker related places).
- `src/engine/language.rs` — `ServiceRelated` on service diagnostics, from
  the anchor context and (when served) LSP related information.
- `src/lib.rs`, `src/codegen/rope.rs`, `src/codegen/core.rs` —
  `EmitAnchor::context`; pipeline emission records each consumed value's
  producer.
- `src/typescript/{backend.rs,native.rs,host.mjs}` — `related` forwarded
  from the checker.
- `src/server.rs` — `labels` (typedCheck) and `related` (tsDiagnostics)
  fields, present only when non-empty.
- `editors/vscode/server/src/{engine.ts,server.ts}` — mapped to LSP
  `relatedInformation`.
- Tests: renderer drawing rules, anchor context (emit_map), CLI and server
  labels (native), extension related info (emitmap.test.ts).
