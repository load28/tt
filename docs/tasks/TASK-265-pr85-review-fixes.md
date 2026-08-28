# TASK-265: PR #85 review fixes — step-anchor gating and typed-path labels

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

The PR #85 review found two real defects in TASK-263/264: the whole-pipeline
anchor was translated with the step-boundary wording, misdiagnosing a
pipeline whose *result* misfits its surrounding position; and the VS Code
default path (typed check replacing the language-service layer) dropped the
new secondary labels because the extension's typed-check types never carried
them.

## Scope

- Included: gate the pipeline step vocabulary on per-step anchors
  (`EmitAnchor::context` present) in both report surfaces; carry `labels`
  through the extension's typed-check path into LSP related information;
  regression tests at the CLI, engine-server, and LSP-publish layers.
- Excluded: the review's third item ("repository forbids AI attribution in
  commits") — no such rule exists anywhere in the repository (`AGENTS.md`
  opens by naming Claude Code and Codex as parties to the working
  contract), so no history rewrite was performed; the finding was answered
  instead of acted on.

## Decisions

### Decision 1: The discriminator is the anchor's producer context

- **Context**: the whole-pipeline anchor and the per-step anchors share
  `AnchorKind::Pipe`, but only a step anchor means "a boundary rejected the
  value". `takesString(1 |> inc)` resolves to the whole-pipeline anchor and
  was worded as a step failure.
- **Alternatives considered**: a new `AnchorKind::PipeStep` (wider surface:
  every consumer matching on kinds needs the new case, and the content
  mapper treats kinds uniformly); span-width heuristics (string-shape
  reasoning the repository forbids).
- **Decision and rationale**: a per-step anchor always records where its
  consumed value was produced (`context: Some(..)`, TASK-264) and the
  whole-pipeline anchor never does — the existing field is already the
  exact semantic discriminator. `pipe_step_anchor` gates the wording, the
  translation class, and the dedup key on both surfaces; a contextless
  `Pipe` anchor renders exactly as `main` rendered it.

## Work log

- 2026-08-28: Reproduced both findings: `takesString(1 |> inc)` rendered
  "this pipeline step expects `string`, but receives `number`"; traced the
  extension's `runTypedCheck` mapping and `mergeTyped` replacement to
  confirm labels were dropped on the default path.
- 2026-08-28: Verified the third finding against the repository: no
  attribution rule exists (`grep` over AGENTS.md, CONTRIBUTING.md, docs/,
  .github/) — reported as unfounded rather than rewriting history.
- 2026-08-28: Implemented the gate (`src/engine/semantics.rs`
  `pipe_step_anchor` + `anchored_diagnostic_message` on the anchor;
  `src/engine/language.rs` same gate on the service path) and the extension
  label pass-through (`ttc.ts` `TtcLabel`, `runTypedCheck`; `server.ts`
  `toDiagnostic` → LSP `relatedInformation`).
- 2026-08-28: Regression tests: `tests/native.rs`
  (`a_whole_pipeline_mismatch_keeps_the_generic_wording`),
  `typedcheck.test.ts` (labels survive `runTypedCheck`), `server.test.ts`
  (the *published* diagnostic keeps `relatedInformation` after the typed
  layer replaces the service layer — the merge the review asked to pin).

## Issues and resolutions

None beyond the review findings themselves.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci --skip rust`

## Result

`takesString(1 |> inc)` reports `type mismatch: expected `string`, found
`number`` over the pipeline (as `main` did); boundary failures keep the
step wording and producer label; the default VS Code path now publishes
diagnostics whose labels survive the typed replacement.
