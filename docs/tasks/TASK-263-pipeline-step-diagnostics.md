# TASK-263: Pipeline diagnostics name the rejecting step

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

A type mismatch at any pipeline boundary after the first collapses to one
diagnostic underlining the entire pipeline (CLI and editor alike), so the user
cannot tell which step rejected the value. Make the diagnostic land on the
step that rejected the piped value and say so in pipeline vocabulary.

## Scope

- Included: per-step `Pipe` anchors in codegen (`$tt_ap`, `$tt_fl`, and the
  slotted apply form), pipeline-specific wording for checker mismatches in the
  shared CLI/editor translation layer, regression tests.
- Excluded: any type inference in ttc (the error-layer contract stands — the
  checker still decides, ttc only re-homes and re-words), changes to other
  anchor kinds, changes to the emitted TypeScript itself.

## Decisions

### Decision 1: Anchor the piped-value argument per step, pointing at the step that consumes it

- **Context**: A pipeline emits as nested helper calls
  (`$tt_ap($tt_ap(h, s1), s2)`). TypeScript infers the helper's `A` from the
  step function and reports the mismatch on the *value* argument. For every
  boundary after the first that argument is itself a nested helper call —
  compiler glue — and the only `Pipe` anchor covered the whole pipeline, so
  `diagnostic_origin` had nothing narrower to hand back.
- **Alternatives considered**: (a) narrowest-anchor selection changes in the
  mapper — unnecessary, anchors are already recorded innermost-first and the
  consumer takes the first match; (b) teaching the reporting layer pipeline
  step tables from `PatternAnalyses` — more machinery, and the emitter
  already knows both the boundary and the step span at the moment it writes
  the glue; (c) anchoring each nested call to the *prefix* source span — the
  underline would still not name the culprit, because a mismatch on the
  accumulated value means the *next* step rejected it.
- **Decision and rationale**: wrap the accumulated value (the first helper
  argument, receiver of a postfix step, or the re-piped accumulator slot in
  the slotted form) in a `Pipe` anchor whose display span is the **consuming
  step**. Verbatim spans keep winning (mappings are consulted before
  anchors), so precise cases stay precise; only diagnostics that cross glue
  re-home, and they re-home onto the step that rejected the value. Checked
  against Rust (primary span on the rejecting position), ReScript ("this has
  type X but expected Y" at the exact expression), and TypeScript (argument
  node spans): all three put the primary span on the consumer of the value.

### Decision 2: One pipeline sentence shared by CLI and editor

- **Context**: On glue the CLI printed the generic
  `type mismatch: expected …, found …` and the editor printed the raw
  TypeScript sentence plus "(in code ttc generated for this construct)" —
  which reads as a compiler defect, not the user's type error. The editor
  also had no deduplication class for `Pipe`, so one failed boundary could
  surface several stacked full-span diagnostics.
- **Alternatives considered**: keeping the generic wording (already better
  once the span narrows, but says nothing about piping); editor-only wording
  (the CLI and editor tables are deliberately one — drift is the failure
  mode TASK-213 removed).
- **Decision and rationale**: add `(Pipe, 2345)` to the shared
  `translation_class`/`translate` table with the sentence "this step cannot
  accept the value the pipeline feeds it — it expects \`E\`, but receives
  \`F\`", where E/F are the minimal incompatible pair. The CLI structured
  branch renders the same sentence from the checker's structured mismatch;
  the editor derives the pair from the message text and falls back to the
  raw sentence when it cannot. The class also gives the editor per-step
  deduplication.

## Work log

- 2026-08-28: Reproduced the collapse with release ttc on 7 probe files:
  every mismatch at a boundary ≥ 2nd of a `|>` chain (and ≥ 2nd composition
  of `flow`) rendered over the whole pipeline; the editor (`tsDiagnostics`)
  additionally wore the "(in code ttc generated for this construct)" suffix
  and duplicated cause/consequence. 2-step chains, non-callable steps, and
  postfix property errors were already precise (verbatim spans).
- 2026-08-28: Traced the mechanism: single whole-pipeline anchor
  (`src/codegen/core.rs` `emit_apply`/`emit_flow`/`emit_apply_continued`),
  glue-crossing spans resolved by `diagnostic_origin`
  (`src/typescript/mapper.rs`) to that anchor, rendered at `src..src_end` by
  `src/engine/semantics.rs` (CLI) and `src/engine/language.rs` (editor).
- 2026-08-28: Implemented per-step anchors in `emit_apply`, `emit_flow`, and
  `emit_apply_continued`; added the shared pipeline mismatch sentence
  (structured branch + `translate`/`translation_class`); added emit-level
  anchor tests, translation unit tests, and toolchain-backed CLI/server
  tests including the curried-combinator chain from the report.
- 2026-08-28: Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` (toolchain-backed native suite included).

## Issues and resolutions

### Issue 1: TypeScript blames the value argument, not the step

- **Symptom**: `1 |> inc |> shout` reported "expected string, found number"
  over the whole pipeline; nothing pointed at `shout`.
- **Cause**: `$tt_ap<A, B>(v, f)` infers `A` from `f`, so the mismatch lands
  on `v` — for boundaries after the first, `v` is glue, and the only anchor
  was the whole pipeline.
- **Resolution**: per-step anchors on the value position naming the
  consuming step (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

A pipeline type mismatch now underlines exactly the step that rejected the
value, in both surfaces, and says
`this pipeline step expects \`E\`, but receives \`F\`` with the minimal
incompatible pair (`required type:` keeps the complete obligation). The
editor loses the misleading "(in code ttc generated for this construct)"
suffix for this case and deduplicates per step.

Changed files:

- `src/codegen/core.rs` — per-step `Pipe` anchors in `emit_apply`,
  `emit_flow`, and `emit_apply_continued`.
- `src/engine/semantics.rs` — `mismatch_pair` extraction,
  `anchored_diagnostic_message`, `assignability_pair`, and the
  `(Pipe, 2345)` entries in `translate`/`translation_class`.
- `src/typescript/host.mjs` — `incompatibleLeaf` descends single-signature
  function returns and identity-matched instantiations' properties
  (both optional-API guarded).
- `docs/design/pipeline-operator.md` §5.1.1 — the mechanism, recorded.
- Tests: `tests/emit_map.rs` (per-step anchors, innermost-first),
  `tests/native.rs` (CLI and server, value pipes, `flow`, the curried
  combinator chain from the report), `src/engine/semantics.rs` unit tests
  (translation, message parsing).

### Decision 3: Leaf reduction descends call signatures and matched instantiations only

- **Context**: a `flow` boundary mismatches as two whole function types, and
  a variant-payload mismatch as two lowered object types — neither names
  the value types that actually differ. A first attempt that descended any
  object's properties regressed primitives (`string` vs `number` reduced to
  their apparent `valueOf` members).
- **Alternatives considered**: parsing the checker's elaboration text in
  Rust (string-shape dependence on message wording); leaving the complete
  pair (precise span, but the user still diffs two long types by eye).
- **Decision and rationale**: descend structurally in the host where the
  checker itself is available — into the return type when both sides have
  exactly one call signature, and into the single differing property only
  through an identity-matched counterpart (two instantiations of one
  declaration), which excludes primitives' apparent members by
  construction. Both descents are guarded so a bridge without those APIs
  keeps the complete pair.
