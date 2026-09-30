# TASK-600: Reject a for-head value whose initializer reads a head binding

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-600`

## Purpose

TASK-593 left one gap open (its Issue 2): a value in the first declarator
of a C-style `for` head lowers before the loop, and a closure written
inside it that names a binding the head declares then resolves outside the
head. `for (let g = match (o) { A(n) => () => g, B => null }; ...)` emitted
the closure `() => g` before the loop, where `g` names an outer binding or
nothing.

## Scope

- Included: resolution of the identifiers a `let`/`const` for head's first
  initializer reads (`src/program_syntax/scopes.rs`), the tt bindings the
  projection records for it (`TtBindings` in
  `src/program_syntax/projection.rs`), the collector fact and
  `EvaluationContext::loop_head_binding` (`src/program_syntax.rs`,
  `src/program_syntax/{collector,visit}.rs`),
  `ExpressionBoundaryReason::LoopHeadBinding` (`src/evaluation_ir.rs`,
  `src/evaluation_ir/planning.rs`; the `for` initializer plan failure in
  `src/evaluation_ir/evaluation.rs` no longer pre-empts it), the
  `match-placement`/`try-placement`
  messages (`src/lib/compile.rs`) and `ttc explain` texts
  (`src/diagnostics.rs`), `docs/ai/tt.md`,
  `docs/design/program-lowering.md` §7.9, and tests.
- Excluded: `for...of`/`for...in` heads, whose right-hand side is evaluated
  in a TDZ scope of the head's names (ECMA-262 §14.7.5.6
  `ForIn/OfHeadEvaluation`), keep lowering before the loop; a closure there
  that names the head binding would throw in the source when called before
  the binding exists, and the lowering does not model that throw. Recorded
  under Issues.

## Decisions

### Decision 1: Reject by name resolution, not by placement alone

- **Context**: `ForLoopEvaluation` (ECMA-262 §14.7.4.2) creates a scope for
  a `let`/`const` head and evaluates the first initializer in it; a direct
  read of a head binding there is a TDZ error (§9.1.1.1.6), but a closure
  created there captures the scope and reads the binding later. A `var`
  head declares in the function's scope, which the code before the loop
  shares. There is no statement position inside the head, and a function
  boundary cannot carry a `match` (never an IIFE, `docs/ai/tt.md`) or a
  `try` (its `return` would leave the wrong function).
- **Alternatives considered**: (a) Reject every value in a lexical head's
  first declarator. That rejects the common, faithful form
  `for (let i = match (o) { … }; …)`. (b) Reject when a head name appears
  as a token anywhere in the initializer. That rejects
  `for (let n = match (o) { A(n) => n, B => 0 }; …)`, whose `n` is the arm's
  own binding, which is the kind of string heuristic the project rules out.
  (c) Resolve the initializer's identifiers with TypeScript's lexical
  scoping plus the bindings tt constructs declare, and reject only when a
  reference resolves to a head binding. (d) Move the loop into a block
  that declares the bindings first; that changes the per-iteration copies
  of `let` bindings (§14.7.4.4 `CreatePerIterationEnvironment`).
- **Decision and rationale**: (c). `scopes::reads_outer` walks the SWC
  projection of the initializer with a scope stack (parameters, hoisted
  `var`s, block and function-body declarations, `catch` parameters, named
  function and class expressions, nested loop heads, and assignment
  targets as references). The projection does not declare tt bindings, so
  the builder records them (`TtBindings`): `match` and `if let` pattern
  bindings over the projected guard and body, and let-else and declaration
  `try` bindings as declarations of their block. A reference that resolves
  to a head name sets `loop_head_binding` for every value in that
  initializer, and `target_capability` answers
  `ExpressionBoundary(LoopHeadBinding)`, so `match` and `try` report
  placement diagnostics and a `result` block runs in place. Identifiers in
  type positions count as references; that can only reject, never accept a
  wrong lowering.

## Work log

- 2026-09-30: Reproduced the closure form and checked that `var` heads,
  arm bindings, and parameters of the same name are faithful.
- 2026-09-30: `src/program_syntax/scopes.rs` (new), `projection.rs`
  (`TtBindings`, `emit_decision_arm`, `pattern_names`,
  `binding_text_names`), `collector.rs`, `visit.rs`
  (`ParentCollector::loop_head_reads`), `src/program_syntax.rs`,
  `src/evaluation_ir.rs`, `src/evaluation_ir/{planning,evaluation}.rs`,
  `src/lib/compile.rs`, `src/diagnostics.rs`.
- 2026-09-30: Test
  `a_for_head_initializer_that_reads_a_head_binding_is_a_placement_error`
  (`tests/compile/cases_14.rs`): closures, a destructured head, and a
  closure in an earlier operand are rejected; an arm binding, a parameter,
  an inner `const` of the same name, and a `var` head are not.

## Issues and resolutions

### Issue 1: `for...of` and `for...in` right-hand sides

- **Symptom**: `for (const f of match (o) { A => [() => f], B => [] })`
  lowers before the loop, where `f` names an outer binding; in the source
  the closure reads the TDZ scope's `f`, which is never initialized, so
  calling it throws a `ReferenceError`.
- **Cause**: `ForIn/OfHeadEvaluation` evaluates the right-hand side in a
  scope where the head's names exist uninitialized.
- **Resolution**: Left open; the same resolution applies once that
  placement is modeled.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/{scopes,projection,collector,visit}.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{planning,evaluation}.rs`,
`src/lib/compile.rs`, `src/diagnostics.rs`, `docs/ai/tt.md`,
`docs/design/program-lowering.md`, `tests/compile/cases_14.rs`,
`docs/tasks/INDEX.md`, this record, and a note on TASK-593.
