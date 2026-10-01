# TASK-722: Judge a pipeline step as the call it makes in the `val` call check

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-722`

## Purpose

`touch(cfg)` with `val cfg` and a mutable parameter of `touch` reports
`val-pass`, but `cfg |> touch`, which is the same call (`docs/ai/tt.md`,
"`x |> f` = `f(x)`"), reported nothing, while a postfix step
(`items |> .push(1)`) was already judged as the member call it is. The
round-8 probe found it; the `val` matrix had no pipeline form, so nothing
caught it.

## Scope

- Included: the `val` call check (`src/val.rs`, `src/val/checker.rs`) in
  both halves (the untyped report and the probes `ttc --check-types` and
  the editor pair by symbol identity), a parser side table of pipeline
  shapes (`src/parser/parse.rs`), the `val-pass` explanation
  (`src/diagnostics.rs`), `docs/ai/tt.md`, a case, and three matrix forms
  in `tests/matrix/val.mjs`.
- Excluded: a composition stored before it is called
  (`const run = flow |> touch; run(cfg)`), see Decision 2.

## Decisions

### Decision 1: One call rule, fed by the pipeline's structure from the parse

- **Context**: The call check is a token walk (the bindings it reasons
  about live in passthrough TypeScript, which the AST keeps opaque); it
  found calls as an identifier followed by `(`. A pipeline has no such
  token shape, and where its head ends is the parser's decision
  (`parse_pipeline`), not something a token walk should re-derive.
- **Alternatives considered**: (a) Recognize `ident |> ident` in the token
  walk: wrong for any head longer than one token and a second reading of
  the pipeline grammar. (b) Judge the emitted `$tt_ap(cfg, touch)`: that
  is lowering output, which TASK-721 changes, and it is not the call the
  author wrote. (c) Read the parse's pipelines (`parser::pipeline_shapes`:
  head span, extent, first function step) as a side table, the way the
  walk already reads the parser's `val` modifiers, and hand each step the
  argument it receives to the same judgement a written call gets.
- **Decision and rationale**: (c). `check_call` and `probe_call` take the
  argument ranges and the callee identifier; a written call supplies its
  argument list, a pipeline's first function step its head (`x |> f` is
  `f(x)`, `docs/ai/tt.md` "|>"; later steps receive the previous step's
  result, which is never a binding).
  The typed half records the same `ValPass` with the step identifier as
  `callee_at`, so the checker pairs it with the declaration by symbol, as
  for a written call.

### Decision 2: A `flow` composition hands its argument to its first step

- **Context**: `docs/ai/tt.md` says flow's first step fixes the input
  type: the composed function calls its first step with its argument.
  Where the composition is applied in place, which function receives a
  binding is syntax: `(flow |> touch |> String)(cfg)` and
  `cfg |> (flow |> touch)` pass `cfg` to `touch`.
- **Alternatives considered**: (a) Leave flows out: the same mutation
  through a parameter escapes by adding `flow`. (b) Treat a `const` bound
  to a composition as a named function with its first step's parameters:
  the check's documented scope is named functions with written parameter
  lists, and following a stored function value is the data-flow analysis
  `src/val.rs` says it does not do.
- **Decision and rationale**: The receiver of a step or a call's callee is
  resolved through a parenthesized `flow` composition to its first step
  (recursively, `(flow |> (flow |> touch) |> String)(cfg)`); a stored
  composition is not judged, as a stored arrow is not, and the guide says
  so.

## Work log

- 2026-10-01: Reproduced with the probe's case: no diagnostic for
  `cfg |> touch`, `val-pass` for `touch(cfg)`.
- 2026-10-01: Added `PipelineShape` and `pipeline_shapes`
  (`src/parser/parse.rs`, exported from `src/parser/mod.rs`); passed them
  to `val::check_all` and `val::probes` (`src/lib/compile.rs`,
  `src/lib/mapped.rs`); added `Applications`, `receiver`, and
  `token_range` (`src/val.rs`) and the shared `call` in
  `src/val/checker.rs`.
- 2026-10-01: Added `tests/cases/compiler/valBindingPipedIntoMutableParameter.tt`
  (from the probe, with the flow forms and passing counterparts), the
  matrix forms `pipedToMutableParameter`, `flowStepToMutableParameter`
  (both rejected with `val-pass` in every diagnostic host), and
  `pipedToValParameter` (a run against its twin `show(cfg)`), appended
  last so the all-pairs companions of the existing forms do not move;
  `node scripts/generate-cases`; `UPDATE_EXPECT=1 TT_CASES=val_
  TT_MATRIX_CASES=all cargo test --test case_baselines` and
  `--test editor_cases`.
- 2026-10-01: Updated `docs/ai/tt.md` (val, "Call check") and the
  `val-pass` explanation.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/valBindingPipedIntoMutableParameter.tt`;
  the matrix cases `val_pipedToMutableParameter_*` and
  `val_flowStepToMutableParameter_*`
- **Observed failure**: with the non-test changes reverted,
  `TT_CASES=PipedIntoMutableParameter cargo test --test case_baselines`
  reported `valBindingPipedIntoMutableParameter: compiles cleanly` (its
  `@expectErrors: val-pass` oracle), and
  `val_pipedToMutableParameter_topLevel_using: compiles cleanly`.

## Verification

- [x] `TT_CASES=val_ TT_MATRIX_CASES=all cargo test --test case_baselines`
  and `--test editor_cases`: pass
- [x] `cargo test --lib val`, `--test cli`, `--test compile`: pass
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

A val binding piped into a same-file function's mutable parameter, or
handed to a `flow` composition whose first step has one, reports
`val-pass` at the binding on every path, as the written call does.
