# TASK-379: Complete structurally owned lowering and the remaining audit defects

> TASK-381 supersedes Decision 1 for guards: a complete guard owns its evaluation plan; selecting one nested child schedule does not cover sibling values.

- **Status**: Complete
- **Started**: 2026-09-15
- **Completed**: 2026-09-16
- **Commit**: —

## Purpose

TASK-377 left three repairs bounded and seven audit findings unrepaired. This task finishes them at the layer that owns each contract: the evaluation plan keeps the schedules of structurally owned children, the parser grows the stack in expression recursion, sibling values inside a captured operand ride their slots, ambient contexts get declaration-only emission, and the remaining parser and engine gaps are closed.

## Scope

- Included: evaluation-plan ownership of nested children (guards and arm bodies), operand positions through grouping parentheses, captured operands carrying earlier sibling values, `declare`/ambient `variant`, `val` rest parameters, statement-form `try` as an unbraced body, plan diagnostics on a recovered TypeScript AST, Windows verbatim path prefixes in the engine, and the parser stack growth that the hosted CI exposed.
- Excluded: the VS Code grammar, which TASK-380 covers.

## Decisions

### Decision 1: A structurally owned child keeps its evaluation schedule

- **Context**: the evaluation plan removed every tt value lexically nested inside a statement-capable outer value ("owned children") before planning conditional operations, so the outer construct's structural emitter evaluated the child at its position with no ordering or short-circuit facts. A `match` in a guard reached the emitter with no plan at all, and a `match` under a call argument or `&&` in an arm body evaluated before its earlier siblings or on a skipped branch.
- **Alternatives considered**: projecting guards as their own host-owned statements (they already are; the owner is then folded into the enclosing statement, which is where the child is dropped); classifying guards by source shape and rejecting conditional ones (TASK-377's bounded answer).
- **Decision and rationale**: `lowering_plan` records, for each owned child, the prefix of its resolved schedule whose parents lie inside the owning value, as a nested schedule. The emitter's continued-expression path already lowers nested schedules with captured inputs and `if`-guarded conditional regions, so guards (`emit_guard`) and arm bodies gain correct ordering and short-circuiting through one mechanism. A guard test is the guard source with the delivered operation replaced by its slot.

### Decision 2: An operand position is the value beneath grouping parentheses

- **Context**: `(try a()) + (try b())` captured `(try a())` as raw source because the input position was the parenthesized operand and the slot table is keyed by the value's own span.
- **Decision and rationale**: the projection visitor records operand positions through `operand_span`, which unwraps `Expr::Paren` only while the inner span is neither a placeholder the projection wrote nor a span with no exact source mapping (a pipeline projection spans several segments). A capture that still contains an earlier sibling value is allowed by `target_capability` and the order validator, and `captured_source` writes it with that sibling's slot in place of its text. Because a capture can cross the HIR statement segments the emitter walks separately, the walk applies a capture to host source inside it, leaves a captured sibling's own emission (its `try` operand) alone, and drops the sibling's inline occurrence the capture already carries.

### Decision 3: Ambient and unbraced contexts are evaluation-context facts

- **Context**: a `variant` inside `declare namespace` emitted an initializer; a statement-form `try` as the unbraced body of `if` emitted statements without braces.
- **Decision and rationale**: `EvaluationContext` carries `ambient` (an enclosing `TsModuleDecl` with `declare`, read from the visitor's AST path) and `requires_block` (the owning statement is the body of `if`, a loop, a label, or `with`), and the lowering plan exposes the roots they apply to. `emit_adt` renders an ambient constructor object as a declaration without an initializer; the parser also claims `declare variant` and `export declare variant`, whose `declare` modifier is kept on both emitted declarations.

### Decision 4: Plan diagnostics survive a recoverable TypeScript error

- **Context**: a `source-not-typescript` failure prevented the lowering plan, so `match-placement` and `try-placement` elsewhere in the file were not reported in that run.
- **Decision and rationale**: when the strict projection fails at copied source, a tolerant projection keeps the parser's recovered module. Its plan is used only for diagnostics, never for emission. A fatal parse error (no recovered module) still reports only the syntax error.

### Decision 5: Engine paths drop the Windows verbatim prefix

- **Context**: `std::fs::canonicalize` returns `\\?\C:\...` on Windows, which Node's `realpathSync` never produces, so the editor could not match typed diagnostics to its buffers and produced unopenable URIs.
- **Decision and rationale**: `engine::paths::canonical` simplifies the verbatim prefix when the plain form still names the same file (no trailing dot or space, no reserved device name, under the legacy length limit), the rule `dunce` implements; every engine canonicalization goes through it.

## Work log

- 2026-09-15: The hosted `fmt / clippy / test` and `coverage` jobs aborted in the 20,000-level nesting test; the vendored parser now grows the stack in `parse_assignment_expr` (recorded under TASK-377 Decision 2).
- 2026-09-15: Kept owned children's schedules in the plan, routed guards through the continued-expression path, made operand positions see through parentheses with placeholder awareness, allowed captures to carry earlier sibling values, and removed the guard shape classification.
- 2026-09-15: Added `requires_block` and `ambient` to the evaluation context and plan, the `declare` modifier to the variant grammar and IR, rest-parameter `val`, tolerant projection for recovered diagnostics, and `engine::paths`.
- 2026-09-15: Added regression tests (`tests/compile/cases_11.rs`), updated `docs/ai/tt.md`, and ran the gates.

## Issues and resolutions

### Issue 1: Owned children lost their schedules

- **Symptom**: `SourceOmitted` internal errors for a `match` in a guard or under a call argument in an arm body; hoisting past `&&`.
- **Cause**: `values.retain` dropped owned children before `plan_conditional_operations` and nothing carried their steps.
- **Resolution**: Decision 1.

### Issue 2: Unwrapping a placeholder's own parentheses

- **Symptom**: `the evaluation position N..M maps to no source` for `(try a())`, and a projected pipeline operand.
- **Cause**: `operand_span` unwrapped into the parentheses the projection wrote around a placeholder, and a pipeline projection spans several segments.
- **Resolution**: the unwrap stops at placeholder spans, and an input that has no exact evaluation mapping falls back to the structural mapping.

### Issue 4: A capture crossing statement segments was emitted twice or not at all

- **Symptom**: for `(try a()).toFixed(1).length + (try b())`, first the `try` operand `a()` vanished (`const $tt_t0 = ;`), then the member chain appeared twice.
- **Cause**: the capture replacement was consulted by every source walk that started inside it, including the earlier sibling's own operand emission, while the parent's value substitution still wrote the sibling's slot after the capture.
- **Resolution**: `inside_captured_value` exempts walks inside a captured sibling value, `carried_by_capture` suppresses the sibling's inline occurrence, and `source_range_with_value_slots` ignores values a capture already carries.

### Issue 3: The nested delivery wrote a second region exit

- **Symptom**: `break; break;` after a nested arm value.
- **Cause**: the nested-schedule delivery emitted the arm exit that the arm action also writes.
- **Resolution**: the nested path delivers without a region exit; the arm action owns it.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` (all stages: rust, native, agents, npm, website, extension)

## Result

Changed files: `src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,planning,validation}.rs`, `src/program_syntax.rs`, `src/program_syntax/{collector,projection,protocol,visit}.rs`, `src/codegen/{mod.rs,core/mod.rs,core/planning.rs}`, `src/codegen/core/emitter/{expression,helpers,host,mod,pattern,result,source}.rs`, `src/codegen/rope/builder.rs`, `src/core_ir/{mod,lower}.rs`, `src/hir/{mod,lower}.rs`, `src/ast.rs`, `src/parser/{parse,variants}.rs`, `src/val.rs`, `src/val/checker.rs`, `src/lib/compile.rs`, `src/engine/{mod,names,project}.rs`, `src/engine/paths.rs` (new), `src/engine/language/{project,service}.rs`, `vendor/swc_ecma_parser/src/parser/expr.rs`, `vendor/swc_ecma_parser/TT-PATCH.md`, `tests/compile.rs`, `tests/compile/cases_10.rs`, `tests/compile/cases_11.rs` (new), `docs/ai/tt.md`.

The Windows path change is verified by compilation and its `cfg(windows)` unit tests; no Windows runner exists in this repository's CI. A fatal TypeScript parse error (one the parser cannot recover from) still reports only the syntax error, since no recovered module exists to plan against.
