# TASK-721: Call a pipeline step that names its function directly, so TypeScript resolves its overloads

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-721`

## Purpose

`s |> conv` with an overloaded `conv` (`(x: string): number`,
`(x: number): string`) and a head that is not a literal lowered to
`$tt_ap(s, conv)`. TypeScript infers `$tt_ap`'s `(v: A) => B` from the
last signature of an overloaded argument only, so the program reported a
false TS2345/TS2322 that the hand-written `conv(s)` does not have
(contract 2). Only an inert head (a literal) was inlined as `conv("lit")`
(`direct_apply_inputs`). The round-8 probe found it; the pipeline matrix
had no overloaded or generic step.

## Scope

- Included: the emission of a pipeline step that names its function
  (`src/codegen/core/emitter/expression.rs`), its planning
  (`reference_apply_steps`, `src/codegen/core/planning.rs`), the syntactic
  test (`source_reference_callee`, `src/program_syntax.rs`), a `flow`'s
  later steps, `docs/ai/tt.md` ("|>"), the matrix forms
  `pipeline_overloadedStep`, `pipeline_genericStep`, `flow_overloadedLater`,
  `flow_genericLater` with two harness functions, their editor differences,
  and a case.
- Excluded: a step that computes its function (a call, `await`, `??`) and
  a `flow`'s first step, which keep the runtime helpers (Decision 2).

## Decisions

### Decision 1: A step that names its function is called directly, from an arrow's parameters

- **Context**: The step must be evaluated after the head (`docs/ai/tt.md`:
  "the piped value is evaluated before the receiver"; `$tt_ap(v, f)`
  evaluates `v`, then `f`) and each once. TypeScript resolves overloads
  only in a call whose callee is the overloaded function ("Function
  Overloads", TypeScript Handbook, More on Functions); when an overloaded
  function is an argument to a generic parameter, "inferences are made from
  the last signature" (Handbook, Conditional Types, "Inferring Within
  Conditional Types").
- **Alternatives considered**: (a) Inline `conv(s)` whenever the head is
  not inert: reorders the evaluation of `s` and `conv`, which ttc proves
  unobservable only for an inert head. (b) `(($v) => conv($v))(s)`: reads
  `conv` inside a closure, where TypeScript drops the narrowing of a `let`
  binding reassigned later in the function. (c) Hoist the head into a
  statement-level slot (`const $h = s; conv($h)`): needs a statement
  position, which an expression-only owner (a parameter default, a class
  field) does not have. (d) `(($tt_v, $tt_f) => $tt_f($tt_v))(s, conv)`:
  the arguments evaluate `s` then `conv`, as `$tt_ap` does, each once; an
  immediately invoked arrow's parameters take the types of its arguments
  (checked with the pinned `tsc` 7.1.0-dev.20260826.1), so `$tt_f` is
  `conv`'s overloaded type, as narrowed where the step is written, and
  `$tt_f($tt_v)` is a call TypeScript resolves against `$tt_v`. The member
  step already uses this shape (`(($tt_v, $tt_r) => $tt_r.m($tt_v))(x, o)`).
- **Decision and rationale**: (d) for a step that is an identifier,
  possibly with type arguments and inside parentheses, `!`, `as`,
  `satisfies`, or `<T>`: such an expression's type does not depend on a
  contextual type. A literal head's argument widens (`"a"` to `string`)
  as `$tt_ap`'s inference already did (checked: `$tt_ap(q, id)` with
  `q: "a"` is `string` too), and an inert head keeps its inlined call. The
  `$tt_v` in the call is anchored to the step with the piped value as
  context, so a mismatch is reported as "this pipeline step expects ..."
  at the step, where the old position under the head reported TypeScript's
  raw message.

### Decision 2: A computed step keeps `$tt_ap`; a `flow`'s later step gets the same call

- **Context**: `x |> Option.mapP(n => n + 1)` relies on `$tt_ap`'s
  argument position giving the step a contextual type (`docs/design/pipeline-operator.md`
  §3, "argument position gives each step contextual typing"); a `flow`'s
  first step receives the composed function's arguments, which nothing
  types where the step is written.
- **Decision and rationale**: Only a step that names its function changes.
  A `flow`'s later step is written
  `(($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))(composed, step)`:
  the composition so far and the step are still evaluated at composition
  and in order, as an immediately invoked function's arguments; TypeScript
  types those parameters from the arguments, and the arrow passed to
  `$tt_fl` is context sensitive, so `B` is inferred from `$tt_g` first and
  `$tt_v` gets that type; the call then resolves the overload (checked with
  the pinned `tsc`). The first form tried,
  `(($tt_f) => ($tt_v) => $tt_f($tt_v))(step)` as `$tt_fl`'s argument,
  resolved the overload too but typed `$tt_v` from `$tt_fl`'s not yet
  inferred `B`, so a mismatch said "receives `unknown`" (Issue 2). The first step and a computed step keep `$tt_fl`/`$tt_ap`; the
  guide says an overloaded function there is inferred from its last
  signature, as TypeScript does for a generic call.

### Decision 3: Hover on a generic or overloaded step answers for the function it names (by design)

- **Context**: The editor matrix compares hover on the step with the
  twin's `keep(x)`, which shows the instantiated `keep<number>`.
- **Decision and rationale**: The step is the function value in both the
  old and the new lowering, and `tests/editor-matrix-differences.txt`
  already lists that class as by design (`docs/ai/tt.md`, "|>": a step is
  an expression that evaluates to the function to call); the new forms are
  listed under the same reason. The generic form's second step is
  unmarked, since the non-generic `double` agrees with its twin.

## Work log

- 2026-10-01: Reproduced the probe's case: `$tt_ap(s, conv)` and TS2345 /
  TS2322 under `ttc --check-types`.
- 2026-10-01: Checked candidate shapes with the pinned `tsc` (overloads,
  generics, literal widening, narrowing, the `flow` IIFE).
- 2026-10-01: Implemented Decisions 1 and 2; anchored the generated `$tt_v`
  after the first full case run showed TS2345 moved to the whole pipeline
  (Issue 1).
- 2026-10-01: Added `shape` and `keep` to `tests/matrix/harness.ts`, the
  four matrix forms, `node scripts/generate-cases` (the `flow_twoSteps`
  companions moved under all-pairs; their old baselines were removed),
  `UPDATE_EXPECT=1 TT_MATRIX_CASES=all` for the new forms in
  `case_baselines` and for `flow_`/`pipeline_` in `editor_cases`, and the
  editor differences.
- 2026-10-01: Added
  `tests/cases/compiler/pipelineIntoOverloadedFunctionResolvesItsOverload.tt`
  (from the probe, with a generic step, a parenthesized step, and a `flow`
  later step); `UPDATE_EXPECT=1 cargo test --test case_baselines` and read
  the changed baselines: emitted pipelines, mappings, the runtime
  import gone from files that no longer use `$tt_ap`, positions in `tsc`'s
  section, `unknown` that became `any` where the step names an undeclared
  function (as in the twin's call), and the cascading TS18046/TS2339 on
  such a step's result gone.

- 2026-10-01: Tests that pinned `$tt_ap` to test something else (the
  runtime import, a script's helper declarations, generated-name
  allocation, a piped value's anchors, the support modules an emission
  reports) now pipe into a computed step (`pick()`), which still uses the
  helper (`tests/compile/cases_05.rs`, `cases_10.rs`, `cases_11.rs`,
  `tests/emit_map.rs`, `tests/integration/cases_03.rs`); tests that pinned
  the shape of a reference step's emission take the new shape
  (`cases_06.rs`, `cases_13.rs`, `cases_14.rs`); `UPDATE_EXPECT=1 cargo
  test --test snapshot` rewrote `tests/fixtures/emit/pipeline-and-flow/expected.ts`.

## Issues and resolutions

### Issue 1: A step's type mismatch moved to the whole pipeline

- **Symptom**: `partialUnionMismatch` reported "type mismatch: expected
  `number`, found `boolean`" under the whole pipeline instead of "this
  pipeline step expects `number`, but receives `boolean`" at the step.
- **Cause**: TypeScript reports the mismatch at `$tt_v` in `$tt_f($tt_v)`,
  generated text with no anchor.
- **Resolution**: That `$tt_v` is anchored to the step with the piped
  value's span as context, as the old argument was.

### Issue 2: A `flow` mismatch at a named later step said "receives `unknown`"

- **Symptom**: The final gate's
  `a_flow_mismatch_names_the_composed_step_and_the_boundary_types`
  (`tests/native/cases_01.rs`) failed: `flow |> inc |> inc |> shout`
  reported "this pipeline step expects `string`, but receives `unknown`".
- **Cause**: In `$tt_fl(prev, (($tt_f) => ($tt_v) => $tt_f($tt_v))(shout))`
  the IIFE is not context sensitive, so TypeScript checks it while `B` is
  still uninferred, and `$tt_v` takes `B`'s constraint, `unknown`.
- **Resolution**: The IIFE takes the composition so far as its first
  argument and composes inside (Decision 2), so the arrow is the context
  sensitive argument of a call whose `B` is inferred first; the mismatch
  says "receives `number`". The test now pins both forms: a computed step
  keeps the function obligation as context, and a named step reports the
  value obligation of its call. A follow-up commit carries the change, the
  regenerated `flow` baselines, and three `tests/cli.rs` tests and one
  `tests/native/cases_06.rs` test that relied on `|> String` needing the
  runtime import; they now use a computed step, which still does. One
  type baseline moved: in `flowFirstStepOptionalChain`, whose first step
  `scale?.by` is already a `flow-first-step-method` error, the hover of
  `direct` reads `(n: number) => unknown` where it read
  `(n: number) => string`, because a `$tt_fl` call whose first argument
  fails gives the arrow no inferred `B`; a composition that type checks is
  typed as before.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/pipelineIntoOverloadedFunctionResolvesItsOverload.tt`;
  the matrix cases `pipeline_overloadedStep_*` and `flow_overloadedLater_*`
- **Observed failure**: with the non-test changes reverted, the case
  produced an `.errors.txt` (TS2322 and TS2345 under `ttc --check-types`)
  and was not run; `pipeline_overloadedStep_declarationInitializer_plain`
  and `flow_overloadedLater_declarationInitializer_yield` reported "does
  not compile cleanly".

## Verification

- [x] `TT_MATRIX_CASES=all` `case_baselines` for the four new forms and
  `editor_cases` for `flow_` and `pipeline_`: pass
- [x] `UPDATE_EXPECT=1 cargo test --test case_baselines`: baselines read
- [x] `cargo test --lib`, `--test compile`, `--test snapshot`, `--test
  integration`, `--test emit_map`, `--test content_mapper`, `--test
  passthrough`, `--test editor_cases`: pass
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

A pipeline step, or a `flow`'s later step, that names its function is
called by TypeScript as `f(x)` is, so an overloaded or generic function
resolves against the piped value; evaluation order is unchanged.
