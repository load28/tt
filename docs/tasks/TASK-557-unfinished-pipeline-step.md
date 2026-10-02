# TASK-557: Keep a pipeline whose last step is not written yet

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-557: Keep a pipeline whose last step is not written yet`

## Purpose

While a step was being typed (`const n = xs |> .length |> ` above
`return n;`), both `|>` were reported as `stray-pipe`, the service reported
TS2355 and TS6133 for `xs` and `n` (TS2304 `m` with `const m = n + 1;`
next), and hover on `xs` answered `(property) undefined: undefined`.
TypeScript on `const n = xs.length + ` above `return n;` reports only TS1109
at `return`.

## Scope

- Included: How the parser reads an empty pipeline step, its AST/HIR/Core IR
  node, its emission, its diagnostic (`missing-pipeline-step`), and where
  the recovery of a `|>` that stays unclaimed ends
  (`recovery_expression_span`).
- Excluded: Other unclaimed `|>` shapes (an unparenthesized ternary or
  arrow), which remain `stray-pipe` and keep recovering their whole
  expression.

## Decisions

### Decision 1: An empty step is a missing step of the pipeline written

- **Context**: `parse_pipeline` gave up on the whole pipeline when a step
  had no text, so every `|>` of it was stray, and the recovery node started
  at the head: the projection replaced `xs |> .length |>` with
  `undefined as any`, dropping the use of `xs`.
- **Alternatives considered**: (a) A textual recovery node for the dangling
  `|>`: replacements are byte-length preserving, and no two-byte text makes
  `acc |>` an expression of TypeScript's error type. Erasing the `|>`
  types the value as the head's, a checker consequence of the stand-in
  (`const s: string = xs |> .length |>` would report TS2322). (b) Keep
  `stray-pipe` for the empty step: that code blocks projection because a
  stray `|>` is left as written, which is not true of a claimed pipeline.
- **Decision and rationale**: As TypeScript keeps the left operand of `a +`
  with a missing right operand, the parser keeps the head and the written
  steps and records a `PipeStepKind::Missing` step (zero-width at the end
  of its `|>`) when the step would start at `;`, `,`, a closer, another
  `|>`, the region's end, or a statement keyword other than `try` (which tt
  reads as a value and which stays an aborted claim as before). The node
  lowers through HIR and Core IR (`ApplyMode::Missing`), and codegen
  applies TypeScript's error type to the accumulator:
  `(undefined as any)(xs.length)`, `$tt_fl(acc, (undefined as any))` in a
  `flow`, the same in a structured apply. Sema reports the new
  `missing-pipeline-step` (tt51) at the `|>`. It does not block projection,
  so the file keeps a projection and TypeScript checks the rest of it.

### Decision 2: The recovery of an unclaimed operator ends at a statement boundary

- **Context**: `recovery_expression_span` ran to the next `;` at depth 0,
  so a stray `|>` on a line without a `;` took the following statements
  into its placeholder.
- **Decision and rationale**: As TypeScript ends an expression there, the
  span also stops at a token with a statement boundary before it
  (`boundary_before`: an automatic semicolon or a statement start) and at a
  statement-only keyword other than `try`.

## Work log

- 2026-09-29: Reproduced through `ttc --server` (`tsDiagnostics`, `check`,
  `hover`) with both reported bodies and the `.ts` twin.
- 2026-09-29: Added `PipeStepKind::Missing` (`src/ast.rs`,
  `src/hir/mod.rs`, `src/hir/lower.rs`), `ApplyMode::Missing`
  (`src/core_ir/mod.rs`, `src/core_ir/lower.rs`), its emission
  (`src/codegen/core/emitter/expression.rs`, `source.rs`), the parser rule
  (`src/parser/pipes.rs`), `DiagnosticCode::MissingPipelineStep` and its
  explanation (`src/diagnostics.rs`), the sema report
  (`src/sema/checker.rs`), and the boundary rule
  (`src/parser/parse.rs`). Removed "A step may not be empty" from the
  `stray-pipe` explanation.
- 2026-09-29: Served the reproductions again: `tsDiagnostics` answers `[]`
  for both, `check` answers only `missing-pipeline-step` at 2:27, hover on
  `xs` answers `(parameter) xs: number[]`.
- 2026-09-29: Tests that used `1 |> ;` as their example of an unclaimed
  `|>` (`tests/compile/cases_09.rs`, `tests/engine_cache.rs`,
  `src/engine/projection.rs`, `src/engine/language/tests.rs`,
  `src/parser/tests.rs`, `engine.test.ts`, `sidecar.test.ts`) now use an
  unparenthesized ternary, which stays unclaimed; the empty-step tests
  (`tests/compile/cases_05.rs`, `cases_06.rs`) assert the new code and
  position. Added `a_missing_step_keeps_the_pipeline_written_before_it`,
  `a_stray_pipe_recovers_only_to_the_end_of_its_statement`
  (`tests/compile/cases_05.rs`), the `missing-pipeline-step` diagnostic
  fixture, and
  `an_unfinished_pipeline_step_leaves_the_rest_of_the_function_checked`
  (`tests/native/cases_10.rs`). The native test fails with the parser's old
  `return None` for an empty step, the recovery test without the boundary
  rule.
- 2026-09-29: Updated `docs/ai/tt.md` and
  `docs/design/pipeline-operator.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed.
- [x] `editors/vscode`: `npm run compile`, then `engine.test.js` and
  `sidecar.test.js`: 45 passed.

## Result

A `|>` whose step is not written yet is reported once, at the `|>`, and the
rest of the file keeps its type information in the editor. Changed files
are listed in the work log, plus `src/diagnostics/tests.rs` and
`tests/fixtures/diagnostic/missing-pipeline-step/`.
