# TASK-390: Keep block returns of a value region that `try` propagates

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A `return` written in a `result` block, or in a block arm of a `match`, delivers that construct's value (docs/ai/tt.md, `result block` and `match`). When the construct was the operand of `try`, the generated code kept the `return` as the enclosing function's return. `function f() { const q = try result { ...; return w + 1; }; return Ok(q * 2); }` returned the raw number `11` instead of `Ok(22)`; `(try match (b) { true => { return Ok(10); }, ... }) * 2` returned `Ok(10)` from the function. Both compiled without a diagnostic.

## Scope

- Included: The program-syntax projection of a propagated value region, source-span mapping of projected spans, and the lowering plan's record of block exits.
- Excluded: The placement rules for `try` and `match` themselves.

## Decisions

### Decision 1: Project every propagated value region beside its propagation

- **Context**: A value region's block returns are learned from the projected TypeScript AST. A statement-form `try` whose operand contained a `match` was projected as `(operand, $tt_syntax_expr_N);`, so the arm returns were visible. A `result` operand, and any operand of the value form of `try`, were hidden inside one placeholder.
- **Alternatives considered**: Rewriting returns from source text would bypass the TypeScript AST that owns statement structure.
- **Decision and rationale**: The predicate that decides the shadow projection now answers for both value regions (`match` and `result`), and the value form of `try` uses the same shadow projection as the statement form, without the statement terminator.

### Decision 2: An exactly matching projection segment names its source

- **Context**: In the shadow projection the operand comes first, so the start of the whole group coincides with the start of the operand's own segment. `source_span_for_projection` took the first segment starting there, so a conditional branch such as the right side of `ready && (try match ...)` mapped to the `match` instead of the `try`, and the planner rejected the operation.
- **Alternatives considered**: Reordering the shadow would change the evaluation order the typed overlays describe.
- **Decision and rationale**: A segment whose projected span equals the queried span is the authoritative mapping; boundary scanning remains the fallback.

### Decision 3: Structurally owned children keep their exits

- **Context**: When an outer statement-capable value (here the propagation) lexically owns an inner value, the inner one is removed from the owner's planned values. Its exits were dropped with it, so the emitter saw no exits for the `match`.
- **Alternatives considered**: Keeping the child as a sibling value would plan it twice.
- **Decision and rationale**: The exits of removed children are carried in the plan's nested exits, which the emitter already consults by expression.

## Work log

- 2026-09-27: Reproduced both forms with `ttc -p` and Node. Applied Decision 1; the `result` cases were correct, while `(try match ...) * 2` still returned from the function.
- 2026-09-27: An attempt to exclude enclosing frames from the shadowed operand's protocol broke `ready && (try match ...)`, which previously compiled, and a projection unit test that pins shared protocols for pipeline heads. Reverted it.
- 2026-09-27: Traced the remaining failures to the span mapping (Decision 2) and to the owned-children filter (Decision 3). Added an integration test that type-checks with `tsc` and runs the output with Node; it fails on the previous compiler and passes now.

## Issues and resolutions

### Issue 1: Block returns became function returns

- **Symptom**: See Purpose.
- **Cause**: Decisions 1 and 3.
- **Resolution**: Decisions 1 and 3.

### Issue 2: The value form inside a conditional was rejected after the shadow projection

- **Symptom**: `ready && (try match ...)` reported `try-placement` and `match-placement`.
- **Cause**: Decision 2.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `a_propagated_value_region_keeps_its_block_returns` fails with the previous `src/` and passes now.

## Result

Changed `src/program_syntax/projection.rs`, `src/program_syntax/protocol.rs`, `src/evaluation_ir/evaluation.rs`, and `tests/integration.rs`.
