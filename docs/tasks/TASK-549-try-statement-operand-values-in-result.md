# TASK-549: Lower the values of a `try` statement's operand inside a `result` block

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

Inside a `result` block, a statement-form `try` whose operand holds a
`match` or a `result` block dropped that value's lowering.
`return result { const x = try r(match (v) { 1 => 10, _ => 20 }); return x; };`
emitted `const $tt_t0 = $tt_v2(($tt_v1 === 0 ? 10 : 20));` with neither the
match's `switch` nor the `$tt_v2 = r` callee capture, so `tsc` reported
TS2552 and the program threw a `ReferenceError`; the output self-check
passed because the output still parses. The same held for `let x = try …`,
a propagate-only `try r(match …);`, `try (match … |> id)`,
`try id(match …)`, and `try r(result {…} |> g)`. The same statements in an
ordinary function compiled correctly.

## Scope

- Included: the result-body emission of a `try` statement, the host owner
  of a region nested for its control flow (Evaluation IR builder), an
  emission check that every planned host prelude is written, and the
  placement of a value inside a larger argument of a return that leaves a
  `result` block or a match block arm (found while testing).
- Excluded: value-form `try` storage (TASK-550).

## Decisions

### Decision 1: A `try` statement in a `result` body writes its owner's prelude

- **Context**: The `match` in `r(match …)` is a host value of the `try`
  statement (the owner the statement's operand is evaluated in), planned as
  a compose rewrite on that owner: capture `r`, run the match into its slot.
  `emit_statements` writes that prelude before `emit_propagate`;
  `emit_result_statements_with_exits`, which emits a `result` body, went
  straight to `emit_region_propagate`, so the planned prelude was never
  written while its slots were still referenced.
- **Alternatives considered**: Emitting the operand's values from
  `emit_propagate_input` would give the propagation a second way to own
  values the plan already assigned to the statement owner.
- **Decision and rationale**: Both statement emitters call one
  `emit_propagate_owner_prelude`, which writes and claims the statement
  owner's compose rewrite.

### Decision 2: A region nested for its control flow keeps its source's host owner

- **Context**: With Decision 1, `result { const q = try result { … }; … }`
  declared the inner block's slot twice. A `try` statement that exits a
  `result` block is placed `Nested` in that block's region
  (`add_propagate`), so `region_host_owner` answered the owner of the
  outer `result` value instead of the statement. The inner `result`
  operand then did not qualify as a direct child of the statement's region
  and was planned as a host value of the statement as well as emitted
  structurally by `emit_propagate_input`. In a function, the statement's
  region is a host region and the same operand is nested.
- **Alternatives considered**: Suppressing the second emission in
  `emit_propagate_input` keeps two plans for the same statement depending
  on where it stands.
- **Decision and rationale**: The builder records the host owner of a
  region that is nested for its control flow but has a host binding
  (`nested_owners`), and `region_host_owner` answers it. A `try` statement
  in a `result` body is now planned as it is in a function.

### Decision 3: Emission checks that every planned prelude was written

- **Context**: The dropped prelude produced parseable output, so neither
  `verify_output` nor any validator noticed.
- **Decision and rationale**: `Emitter::emit_file` fails with an internal
  compiler error when a compose rewrite was not claimed by the end of
  emission (`docs/design/program-lowering.md` §11). Every compose emission
  path now claims its rewrite; the full suite passes with the check.

### Decision 4: Only a value the return delivers as it is transfers to the region's exit

- **Context**: While testing Decision 1, `return s(match (v) {…}) + q;` in a
  `result` block produced `s($tt_v7) + q` with the match never lowered
  (TS2552, `ReferenceError`), and the same return in a match block arm
  stopped with `validate_source_preservation … (SourceOmitted)`; both on
  the commit before this task too. `placement` made every value inside the
  argument of a return that leaves a value region `Nested` in that region
  (`nested_in_owned_exit`), but the region's exit emission only lowers a
  value that is the whole argument under wrappers
  (`returned_structured_expr`) or a returned template, and no planning pass
  gave the other values a schedule, so the callee `s` could not even be
  captured before the match.
- **Alternatives considered**: (a) Emit the argument as an operand in the
  exit emitters and plan schedules for the exit-nested values. That adds a
  third planning path for values an owner already plans. (b) Place such a
  value like any other value of the return statement's owner.
- **Decision and rationale**: (b). A `match` transfers to the exit only
  when no step inside the argument consumes it except a template
  interpolation (`delivered_by_exit`, `src/evaluation_ir/builder.rs`);
  otherwise it is a host value of the return statement, planned with its
  protocol (callee capture first, ECMA-262 `EvaluateCall`). The exit
  emitters rewrite the return's head through a local edit, so
  `source_rope_with_edits` writes the prelude of an owner that starts at an
  edit before the edit's text; an unbraced owner opens its block as every
  prelude does. Decision 3's check found the missing prelude while this
  was built.

### Decision 5: A returned template delivers its value without its own exit

- **Context**: `return \`${match (v) {…}}-${q}\`;` in a `result` block
  emitted `$tt_v0 = { … }; break;` followed by the region's
  `break $tt_v0;`, and the bare `break` is not valid in a labeled block
  (TS1107); in a match arm it was a redundant second `break`.
- **Decision and rationale**: `emit_continued_expr` delivers a template
  with `emit_value_delivery_without_region_exit`, as it delivers a
  sequence; the consumer of the continuation writes the exit.

## Work log

- 2026-09-29: Reproduced every reported shape with `ttc` and
  `tsc --strict` under `target/probe4-compiler/`.
- 2026-09-29: Added `emit_propagate_owner_prelude`
  (`src/codegen/core/emitter/source.rs`) and called it from
  `src/codegen/core/emitter/result.rs`; added `emit_file` and used it in
  `src/codegen/core/mod.rs`.
- 2026-09-29: The full suite failed `a_propagated_value_region_keeps_its_block_returns`
  (TS2451); added `nested_owners` (`src/evaluation_ir/builder.rs`,
  `src/evaluation_ir/evaluation.rs`).
- 2026-09-29: Added
  `runtime_a_try_statement_in_a_result_block_lowers_the_values_of_its_operand`
  (`tests/integration/cases_05.rs`), which also checks that the callee is
  read before the match runs.
- 2026-09-29: Found the return-argument failures (Decisions 4 and 5) with
  isolated probes; changed `placement` (`src/evaluation_ir/builder.rs`),
  `source_rope_with_edits` (`src/codegen/core/emitter/source.rs`), and the
  template delivery (`src/codegen/core/emitter/expression.rs`); added
  `runtime_a_value_inside_a_region_return_argument_runs_in_the_return_s_prelude`
  (`tests/integration/cases_05.rs`), which fails without the change.

## Issues and resolutions

### Issue 1: A nested `result` operand was declared twice

- **Symptom**: TS2451 `Cannot redeclare block-scoped variable '$tt_v16'`.
- **Cause**: Decision 2's context.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Both new tests fail without the change (TS2552 for the missing
  slots; the source-preservation internal error).

## Result

Changed `src/codegen/core/emitter/source.rs`,
`src/codegen/core/emitter/result.rs`,
`src/codegen/core/emitter/expression.rs`, `src/codegen/core/mod.rs`,
`src/evaluation_ir/builder.rs`, `src/evaluation_ir/evaluation.rs`,
`docs/design/program-lowering.md`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, and this record.
