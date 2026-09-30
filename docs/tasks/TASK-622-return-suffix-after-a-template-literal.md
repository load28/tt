# TASK-622: Write a return's suffix after a template literal that ends its statement

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-622`

## Purpose

`result { const x = try r(); return \`v\`}` stopped the compiler with
`internal compiler error: Result return start has no matching end`
(`src/codegen/rope.rs`). A `return` whose value ends with a template literal
(or a tagged template) directly before the block's `}` is valid: ECMA-262
§12.10.1 (automatic semicolon insertion, rule 1) inserts the `;` before a
`}`. The same shape in a match arm's block
(`A(n) => { return \`${n}\`}`) failed the same way.

## Scope

- Included: where a body's source edits are written when a statement
  boundary falls after a statement that is not opaque
  (`src/codegen/core/emitter/source.rs`, `src/codegen/core/emitter/result.rs`),
  and output tests.
- Excluded: how the parser segments a template literal (a segment of its
  own, since its substitutions may hold tt constructs), which is unchanged.

## Decisions

### Decision 1: An empty edit is written after the statement that ends at it when no opaque statement holds it

- **Context**: A value exit is lowered by two source edits around its
  argument: the prefix replaces `return ` and the suffix replaces
  `argument.end..statement.end`. Without a `;` the suffix is empty, a point
  after the argument. Edits were written only while copying an opaque
  statement's source (`source_rope_with_edits`). A template literal is a
  statement of its own (`Statement::Expr` of `Expr::Template`), so for
  `return \`v\`}` the body is `return ` (opaque) followed by the template,
  and no opaque statement holds the point after it: the suffix, the
  assignment's end, and the `break` were never written, and the `Result`
  return's end mark with them, which the rope's pairing check then reported.
- **Alternatives considered**: (a) Make every template literal without a
  tt construct inside it opaque. The parser cannot know that before HIR,
  and a tagged template or `x + \`v\`` ends the same way. (b) Widen the
  suffix edit to the `}`. That edit belongs to the enclosing block, which
  the edit does not own. (c) Write the empty edits at the point after a
  statement that is not opaque, when no opaque statement of the body holds
  that point.
- **Decision and rationale**: (c). An edit is written by exactly one
  statement: the opaque statement whose source holds it, as before, or,
  when none does, the statement that ends at it. The rule is in one place
  (`Emitter::edits_after_statement`) and is used by every loop that writes
  a body with edits: `emit_statements_with_edits`, the value-exit body
  (`emit_body_with_exits`), and the `result` body
  (`emit_result_statements_with_exits`). The writing of one edit is shared
  (`push_source_edit`), so the `Result` return's marks sit on the value's
  side of the text wherever the edit is written.

## Work log

- 2026-09-30: Reproduced `target/probe6-compiler/clean/r.tt` and `m.tt`
  (exit 101, `src/codegen/rope.rs:832`). A trace of the result body showed
  the statements `return ` (opaque, 131..139) and the template (139..142)
  and a suffix edit at 142..142 that no statement wrote.
- 2026-09-30: `src/codegen/core/emitter/source.rs`: `push_source_edit`,
  `emit_statement_with_edits` (one statement, reports whether it was
  written), `edits_after_statement`, and the value-exit body loop.
  `src/codegen/core/emitter/result.rs`: the `result` body loop.
- 2026-09-30: Test
  `a_returned_template_literal_that_ends_its_statement_keeps_the_return_suffix`
  (`tests/compile/cases_14.rs`): a template, a tagged template, `x + \`v\``,
  a template followed by another statement (the suffix is written once),
  and both match-arm block forms. `cargo test --test compile --test
  snapshot --test emit_map --test passthrough` passes unchanged otherwise.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/codegen/core/emitter/source.rs`,
`src/codegen/core/emitter/result.rs`, `tests/compile/cases_14.rs`,
`docs/tasks/INDEX.md`, and this record. A `return` whose value ends with a
template literal before `}` compiles as the same `return` with a `;`.
