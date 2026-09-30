# TASK-574: Report a postfix pipeline's result mismatch on the whole pipeline

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: (see the work log)

## Purpose

A result, annotation, or argument mismatch of a pipeline whose last step
is a postfix tail was blamed on a step.
`export function g(): number { return "a" |> .trim(); }` reported
``error[ts2322]: this pipeline step expects `number`, but receives `string` ``
on `.trim()`; `const x: number = "a" |> .trim() |> .toUpperCase();` blamed
`.trim()`, and `f("a" |> .toUpperCase())` did the same with ts2345. With a
function step the same mismatch is reported on the whole pipeline, which is
what `docs/design/pipeline-operator.md` §5.1.1 and the `pipe_step_anchor`
comment (`src/engine/semantics/translate.rs`) promise.

## Scope

- Included: which lowering anchor owns a checker diagnostic
  (`diagnostic_origin`, `src/typescript/mapper.rs`), the design note, and
  regression tests.
- Excluded: the step and pipeline anchors themselves, which already record
  the right ranges.

## Decisions

### Decision 1: A diagnostic belongs to the anchor whose output holds its whole span

- **Context**: A postfix step emits its tail after the piped value
  (`"a".trim()`), so the step's input anchor (the piped value `"a"`) starts
  where the checker's span on the whole call starts. `diagnostic_origin`
  took the first anchor containing the span's start, which was that step
  anchor, although the span ran past it. A function step's call
  (`inc("a")`) starts before its input anchor, so the same mismatch fell
  through to the whole-pipeline anchor.
- **Alternatives considered**: (a) Give postfix steps no input anchor, or
  special-case `Pipe` anchors in the report layer. That would lose the
  step diagnostics a postfix boundary does need, and it would treat one
  construct's symptom instead of the ownership rule. (b) Let an anchor own a
  diagnostic only when its output holds the whole span, falling back to the
  previous start-based choice for spans no anchor holds.
- **Decision and rationale**: (b), in the mapper that decides ownership
  for every construct: "a lowering owns what lies in its output". A span
  inside a step's input still resolves to the step, and a span that crosses
  glue no single anchor holds keeps the previous answer. The CLI, editor,
  and server all read this one function.

## Work log

- 2026-09-30: Reproduced `target/probe4-compiler/min/b5.tt` and the
  annotation and argument shapes with `ttc --check-types`; a function step
  (`"a" |> .trim() |> inc`) and `1 |> inc` checked against `string` were
  compared as controls.
- 2026-09-30: Changed `diagnostic_origin` (`src/typescript/mapper.rs`);
  added `a_diagnostic_belongs_to_the_anchor_that_holds_its_whole_span`
  (unit test there) and
  `a_postfix_pipeline_result_mismatch_is_reported_on_the_pipeline`
  (`tests/native/cases_10.rs`); documented the rule in
  `docs/design/pipeline-operator.md` §5.1.1.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change.

## Result

Changed `src/typescript/mapper.rs`, `docs/design/pipeline-operator.md`,
`tests/native/cases_10.rs`, `docs/tasks/INDEX.md`, and this record.
