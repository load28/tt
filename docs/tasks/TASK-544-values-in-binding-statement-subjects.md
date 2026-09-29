# TASK-544: Lower a tt value inside a let-else or `if let` subject once

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A `match` or a value-form `try` below the top level of a let-else or
`if let` subject produced broken TypeScript. `if let Some(value: w) =
opt(match (k) { 1 => 1, _ => 2 }) { return w; }` failed the output
self-check (`verify-failed`); with `--no-verify` the output contained
`$tt_t0 = if let Some(value: w) = opt($tt_v0);`. The same happened for
`const Some(value: v) = opt(match …) else {…};`, `= opt(try r()) else`,
and `= [match …][0] else`. Inside a `result { }` block, `= (try r()) else`
and `= opt(try r()) else` stopped the compiler with `internal compiler error:
unscheduled expression try reached inline emission` (exit 101). `docs/ai/tt.md`
says the let-else initializer is any expression and that a value-form `try`
works inside a larger expression; only a bare `= try expr else` is excluded.

## Scope

- Included: the source extent of a let-else and `if let` subject (HIR), the
  subject's TypeScript structure in the program projection, the planning of
  the values the subject contains (Evaluation IR), the value slot of a
  nested `try` that exits a `result` block, the design note, and compile and
  runtime regression tests.
- Excluded: a pipeline head `(try r()) |> f` inside a `result` block. It
  also stopped with the same internal error before this task; with the slot
  fix below it reports the `try-crosses-value-region` placement error that
  sema already raised for it, which is the documented rule and is left as
  is.

## Decisions

### Decision 1: The subject's sequence node spans the subject

- **Context**: HIR lowers a subject that contains a tt construct to
  `Expr::Seq { node, body }`. The operand emitter
  (`Emitter::emit_operand`, TASK-501) delivers the operand's source from the
  node's span with the values replaced by their slots. `lower_let_else` and
  `lower_if_let` gave that node the statement head (`const Some(value: v) =
  opt(…)`), so the delivered operand started at the declaration keyword.
- **Alternatives considered**: Trimming the head in the emitter would teach
  one consumer about a wrong span every other consumer still sees.
- **Decision and rationale**: The node spans the subject expression
  (`stmt.expr.span`), as a `match` subject's node spans its scrutinee.

### Decision 2: The projection shows the subject, and the statement head bounds its values

- **Context**: With the span fixed, the function-level shapes compiled, but
  the subject's values had no TypeScript host context: the projection wrote
  a statement decision as `{ $tt_syntax_stmt_N; if (true) { … } }` without
  its subject. A value there was planned with no evaluation protocol, so
  `opt(match …)` read `opt` after the match ran, where a `match` subject
  (`match (opt(match …))`) captures `opt` first, as ECMA-262 `EvaluateCall`
  evaluates the callee before the arguments. Projecting the subject alone
  made the value an ordinary value of the statement's host owner, planned
  with its call completion, and the owner rewrite and the decision's subject
  emission then both claimed `opt(`
  (`validate_source_preservation … SourceOmitted`).
- **Alternatives considered**: (a) Place every value under a statement
  decision as a nested region without a protocol. That keeps the wrong
  callee order. (b) Special-case the call completion for subjects in the
  emitter. That leaves conditional operations and placement capability of
  subject values unplanned.
- **Decision and rationale**: The projection writes each subject after the
  placeholder (`{ $tt_syntax_stmt_N; (subject); if (true) { … } }`), and a
  chained `else if let` is projected as its own statement decision, so its
  subject has a host context as well. In `EvaluationFile::lowering_plan`, the
  head of every let-else and `if let` of an owner is an outer bound next to
  the owner's statement-capable values: a value strictly inside it is an
  owned child, its schedule is the prefix inside the head, and the group's
  conditional operations are planned as for any enclosing value
  (`docs/design/program-lowering.md` §7.5). The decision's subject
  initialization already delivers the operand through `emit_operand`, which
  now receives the capture steps.

### Decision 3: A nested `try` that exits a `result` block has a value slot

- **Context**: Inside a `result` block, `(try r())` as a subject still
  stopped with `unscheduled expression try reached inline emission`. The
  planner skipped the value slot of a nested propagation whose exit is a
  `result` block when its protocol had no steps (TASK-311), so
  `emit_operand` found no slot and fell back to inline emission.
- **Alternatives considered**: Emitting such a propagation inline would need
  a statement position the operand does not have.
- **Decision and rationale**: The skip is removed; the propagation gets a
  slot like every other nested value region and is emitted by the operand
  path with the `result` block's failure edge. No test depended on the skip:
  the full suite passes without it.

## Work log

- 2026-09-29: Reproduced every reported shape on `c27ad13` with
  `ttc src -o out` and `--no-verify` in isolated directories.
- 2026-09-29: Changed `src/hir/lower.rs` (Decision 1); function-level shapes
  compiled, `result`-block shapes still stopped. Removed the slot skip in
  `src/evaluation_ir/evaluation.rs` (Decision 3).
- 2026-09-29: Projected statement-decision subjects and chained `else if
  let` statements in `src/program_syntax/projection.rs`; found the
  `SourceOmitted` failure for `opt(match …)`; added the statement-decision
  bounds to `lowering_plan` (Decision 2).
- 2026-09-29: Added `a_value_inside_a_let_else_or_if_let_subject_is_lowered_once`
  (`tests/compile/cases_14.rs`, 5 subjects × 3 statement forms × function
  and `result` hosts) and
  `runtime_a_value_inside_a_let_else_or_if_let_subject_keeps_evaluation_order`
  (`tests/integration/cases_05.rs`). Documented the subject in
  `docs/design/program-lowering.md` §7.5.

## Issues and resolutions

### Issue 1: A projected subject's value was emitted by two owners

- **Symptom**: `validate_source_preservation broke the contract … (SourceOmitted)`
  for `if let Some(value: w) = opt(match (k) {…})` after the subject was
  projected.
- **Cause**: The match became a host value of the statement's owner with a
  call completion, so the owner rewrite and the subject initialization both
  emitted the call.
- **Resolution**: Decision 2's outer bound makes the value an owned child of
  the statement decision.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change (verify-failed, the internal
  error, and the callee read after the match).

## Result

Changed `src/hir/lower.rs`, `src/program_syntax/projection.rs`,
`src/evaluation_ir/evaluation.rs`, `docs/design/program-lowering.md`,
`tests/compile/cases_14.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, and this record. Every reported shape compiles,
type-checks, and runs with the subject's callee read before its value.
