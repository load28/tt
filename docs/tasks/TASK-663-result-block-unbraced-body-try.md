# TASK-663: Open a block for a `result` block's statements in an unbraced body

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-663`

## Purpose

Inside a `result` block, a `try` in the unbraced body of an `if`, `else`,
loop, or label produced output that does not parse:
`if (c) const $tt_t0 = g(); if (!("value" in $tt_t0)) {...}`. It affected
`if (c) return try g();`, `if (c) return (try g());`, `if (c) try g();`,
`else`/`else if`/nested `if` bodies, `while`/`for` bodies, and labels. A
returned `match` there kept its block but left the region's `break`
outside it, so the block ended unconditionally. The guide promises that
such a statement opens its own block, and it does in an ordinary function.

## Scope

- Included: the nested region's placement (`src/evaluation_ir.rs`,
  `src/evaluation_ir/builder.rs`), the block-required statement set
  (`src/evaluation_ir/evaluation.rs`), the Result region's statement
  emission (`src/codegen/core/emitter/result.rs`), `docs/ai/tt.md`, a unit
  test update in `src/evaluation_ir/tests.rs`, and a runtime case.
- Excluded: the storage annotations TypeScript's refinement writes for the
  Result slot (unchanged layout), and the function-body path, which was
  already correct.

## Sources

- `docs/ai/tt.md`, "try": "A statement-form `try` written as the unbraced
  body of an `if`, loop, or label opens its own block, and so does any
  statement there whose `match`, value-form `try`, or `result` is hoisted".
- ECMAScript 2025, 14.6 "The if Statement", 14.7 "Iteration Statements",
  14.13 "Labelled Statements": each takes one `Statement` as its body, and
  14.3.1 "Let and Const Declarations" is a `Declaration`, not a
  `Statement`, so `if (c) const x = ...;` is a syntax error; a `Block`
  (14.2) is the statement that holds several.

## Decisions

### Decision 1: A nested region keeps the host projection's evaluation context

- **Context**: Whether a statement is an unbraced body is decided once, by
  the host projection (`requires_block` in `EvaluationContext`). A `try`
  in a `result` block is placed as a `Nested` region, and the nested
  placement dropped the binding's context, so the block-required set never
  saw it.
- **Alternatives considered**: (a) Re-derive "unbraced body" in the Result
  emitter from the source text. A second model of the same fact. (b) Place
  such a `try` as a host region. It would no longer be emitted by the
  Result region that owns its failure edge. (c) Keep the context on the
  nested placement and include nested propagations and decisions in the
  block-required set with the same conditions as host ones.
- **Decision and rationale**: (c). `RegionPlacement::Nested` carries
  `context: Option<EvaluationContext>`, and the Result emitter braces a
  statement-form propagation in that set exactly as the function-body
  emitter does. A lexical `const`/`let` `try` there now gets the same
  `try-placement` error a function gives.

### Decision 2: A rebuilt `return` braces its whole replacement

- **Context**: The Result region rebuilds `return try v;` and a returned
  `match` into several statements plus a `break` to its label, and the
  host exit already records whether the `return` needs a block
  (`HostExit::requires_block`), which the match-arm path already honours.
- **Alternatives considered**: (a) Brace only the value's own statements.
  The trailing `break` would stay outside, which is the returned-`match`
  bug. (b) Brace the replacement, value and `break` together, when the
  exit requires it.
- **Decision and rationale**: (b), through a `ResultReturn` record that
  keeps the exit's `requires_block` beside the statement, argument, value,
  and propagation, replacing two tuple shapes.

## Work log

- 2026-09-30: Reproduced with the probe case (`verify-failed`, "Expression
  expected"), and found the returned-`match` variant while testing
  neighbouring shapes.
- 2026-09-30: Added the nested context, the nested block-required roots,
  and the `ResultReturn` braces; updated the one unit test that spelled
  the nested placement out.
- 2026-09-30: Added `tests/cases/compiler/resultBlockUnbracedBodyTry.tt`
  with `@run` over `Ok` and `Err` inputs; updated `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: A returned `match` left the region's `break` outside its block

- **Symptom**: `if (c) return match (s) {...};` in a `result` block emitted
  `if (c) { ...switch... } break $tt_v1;`, ending the block on every path.
- **Cause**: The structured-return path appended the `break` after the
  match's own block and ignored `requires_block`.
- **Resolution**: Decision 2.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/resultBlockUnbracedBodyTry.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: `error[verify-failed]: generated TypeScript failed
  to parse: Expression expected` at `if (c) return try r;` in both the
  `ttc` and `ttc --check-types` sections; nothing was emitted or run.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the whole case runner, `cargo test --lib evaluation_ir`,
  and `tests/compile.rs`, `tests/snapshot.rs`, `tests/integration.rs` here;
  the full gate over TASK-661 to TASK-666 is recorded in TASK-666
- [x] Baseline changes reviewed and committed with the change

## Result

Every statement a `result` block rebuilds in an unbraced body is braced,
and the runtime baseline pins each shape over `Ok` and `Err`. Changed:
`src/evaluation_ir.rs`, `src/evaluation_ir/builder.rs`,
`src/evaluation_ir/evaluation.rs`, `src/evaluation_ir/tests.rs`,
`src/codegen/core/emitter/result.rs`, `docs/ai/tt.md`, the case and its
baselines.
