# TASK-662: Make a `flow` optional-chain step the optional call a pipeline makes

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-662`

## Purpose

`flow |> inc |> o?.m` emitted `$tt_fl(inc, o?.m)`: the method was read
off its receiver unbound, and nothing short-circuited, so the composed
function lost `this` when `o` was present and threw `g is not a function`
when it was absent; `--check-types` reported TS2345 on the output. The same
held for `o?.inner.m` and `o?.[key]`. The guide says an optional-chain step
is the optional call `obj?.m(x)` and that `flow` uses the same step rules,
and the pipeline form already made that call.

## Scope

- Included: `flow` step emission (`src/codegen/core/emitter/expression.rs`),
  the first-step rule (`src/sema/checker.rs`, `src/sema.rs`,
  `src/lib/compile.rs`, the `flow-first-step-method` explanation in
  `src/diagnostics.rs`), `docs/ai/tt.md`, and two cases under
  `tests/cases/compiler/`.
- Excluded: non-optional member steps, whose bound emission is unchanged,
  and the pipeline's own emission, which is the model reused here.

## Sources

- `docs/ai/tt.md`, "|>": "An optional-chain step (`x |> obj?.m`) is the
  optional call `obj?.m(x)`: it short-circuits to `undefined` when the chain
  does", and `flow`: "Same step rules".
- ECMAScript 2025, 13.3.9 "Optional Chains": `a?.b(c)` evaluates `a`, and
  when it is `undefined` or `null` the whole chain evaluates to `undefined`
  without evaluating the rest; otherwise the call is a method call with
  `a.b`'s base as `this` (EvaluateCall with the reference's base value).
- TypeScript `src/compiler/checker.ts`, `getContextuallyTypedParameterType`
  and `getTypeOfParameter` for an immediately invoked function expression
  (`getImmediatelyInvokedFunctionExpression`): an IIFE's parameters are typed
  by its arguments. The bound member step already relies on this.
- TypeScript `src/compiler/checker.ts`, `inferTypeArguments` /
  `checkFunctionExpressionOrObjectLiteralMethod`: a context-sensitive
  argument (an arrow with an unannotated parameter) is typed after the
  context-free arguments have fixed the type parameters, so in
  `$tt_fl(f, (v) => ...)` the arrow's `v` gets `f`'s return type.

## Decisions

### Decision 1: Reuse the pipeline's member call inside a capture that also takes the composition

- **Context**: The pipeline makes the call `$tt_r?.m($tt_v)` over captured
  operands (`emit_member_step`). A `flow` step must be a function whose
  input TypeScript can type without an annotation ttc would have to write.
- **Alternatives considered**: (a) `$tt_fl(acc, ($tt_v) => o?.m($tt_v))`,
  the receiver written inside the arrow. The receiver would run on every
  call instead of once, unlike a bound member step, and an `await` or
  `yield` in it would land inside an arrow. (b) Capture only the receiver,
  `$tt_fl(acc, (($tt_r) => ($tt_v) => $tt_r?.m($tt_v))(o))`. The inner arrow
  is a return value, not an argument of `$tt_fl`, so nothing types `$tt_v`
  (implicit `any`). (c) Bind at composition, `$tt_r?.m.bind($tt_r)`. It
  reads the method early, binds the wrong object for `o?.inner.m`, and a
  short-circuited chain is still not a function.
- **Decision and rationale**: The capture takes the composition so far and
  the step's operands, `(($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) =>
  $tt_r?.m($tt_v)))(acc, (o))`. The composition and the receiver are
  evaluated once, in the order a bound member step evaluates them; the
  arrow is a direct argument of `$tt_fl`, so TypeScript types `$tt_v` from
  the previous step; and the call body is the pipeline's own
  (`member_call`, now shared by `emit_member_step`).

### Decision 2: An optional-chain first step is `flow-first-step-method`

- **Context**: The first step is the composed function itself, and its
  parameter list is the composed function's input type. An optional call
  whose function may be absent has neither.
- **Alternatives considered**: (a) Leave it: the output keeps an unbound
  possibly-`undefined` function, a runtime `TypeError` and TS2345. (b) Emit
  a wrapper arrow; its parameter would be implicitly `any` without an
  annotation ttc cannot write without type tricks. (c) A new diagnostic
  code. The existing rule already states the reason (the first step fixes
  the input type) and its fix (write a function first).
- **Decision and rationale**: Extend the existing rule rather than add a
  code: sema classifies the first step with the same
  `source_member_callee` the emitter uses and reports
  `flow-first-step-method` with a help that writes the annotated function.
  `ttc explain flow-first-step-method` names both shapes.

## Work log

- 2026-09-30: Reproduced with the probe case: `$tt_fl(inc, present?.by)`
  and five TS2345 errors under `--check-types`.
- 2026-09-30: Added `member_call` and `emit_optional_flow_step`; later
  optional steps now emit the capture.
- 2026-09-30: Found the first-step form (`flow |> o?.m |> String` emitted
  `$tt_fl(o?.m, String)`); added the sema rule, passing the source kind to
  `sema::check_all` so the step parses as the emitter parses it.
- 2026-09-30: Added `flowOptionalMemberStep.tt` (with `@run`) and
  `flowFirstStepOptionalChain.tt`; updated `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The first step had the same defect and no emitted form could fix it

- **Symptom**: `flow |> scale?.by |> String` emitted `$tt_fl(scale?.by,
  String)`, TS2345.
- **Cause**: The composed function's input type comes from the first step's
  own parameters, which an optional chain cannot guarantee.
- **Resolution**: Decision 2.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/flowOptionalMemberStep.tt` and
  `tests/cases/compiler/flowFirstStepOptionalChain.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: Six baselines differed. `flowOptionalMemberStep`
  reported `error[ts2345]: type mismatch: expected \`(b: number) =>
  number\`, found \`undefined\`` at every optional step, so the program was
  not run and no `.stdout` was produced; `flowFirstStepOptionalChain`
  compiled with exit 0 and TS2345 instead of `flow-first-step-method`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the case runner and the `flow` tests of
  `tests/compile.rs`, `tests/snapshot.rs`, and `tests/cli.rs` here; the full
  gate over TASK-661 to TASK-666 is recorded in TASK-666
- [x] Baseline changes reviewed and committed with the change

## Result

A later optional-chain `flow` step calls the method on its receiver and
gives `undefined` when the chain short-circuits (`.stdout`: `member 20 30`,
`absentMember undefined undefined`, `receiver` evaluated once); an
optional-chain first step is a located tt error. Changed:
`src/codegen/core/emitter/expression.rs`, `src/sema/checker.rs`,
`src/sema.rs`, `src/lib/compile.rs`, `src/diagnostics.rs`, `docs/ai/tt.md`,
the two cases, and their baselines.
