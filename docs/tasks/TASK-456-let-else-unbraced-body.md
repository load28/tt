# TASK-456: Lower a let-else in an unbraced body inside its own block

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-456`

## Purpose

A let-else written as the unbraced body of an `if` or loop was emitted with
only its subject temporary under the parent:
`if (true) var Some(value: hv) = o else { return 0; };` became
`if (true) const $tt_t0 = o;` followed by an unconditional check and binding,
which fails verification. `while (true) var Some(...) = o else {...};` had the
same defect. A `const`/`let` let-else in that position is not TypeScript at
all (ECMA-262 §14.6: the body is a `Statement`, which excludes a
`LexicalDeclaration`), but it produced the same invalid output instead of a
located diagnostic.

## Scope

- Included: the host owner of a statement decision in the TypeScript
  projection (`src/program_syntax/projection.rs`); the Evaluation IR's
  block-required statements and a new lexical-declaration-body fact
  (`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`); the let-else
  emitter (`src/codegen/core/emitter/`); the placement diagnostics
  (`src/lib/compile.rs`, `src/diagnostics.rs`); tests; `docs/ai/tt.md`.
- Excluded: `if let`, which already emitted one block of its own.

## Decisions

### Decision 1: The projected block of a statement decision is its host owner

- **Context**: TASK-447 made "this owner is the unbraced body of an `if`,
  loop, label, or `with`" a property of the host owner
  (`EvaluationContext.requires_block`, from `is_unbraced_body` at the owner's
  parent edge). A statement decision was projected as
  `{ {$tt_syntax_stmt_N;} if (true) {...} }` (TASK-448), and the inner
  placeholder block, whose parent is the outer block, was the host owner, so
  `requires_block` was always false for a decision.
- **Alternatives considered**: (a) reading the flag a fixed number of AST
  edges above the placeholder — depends on the projection's exact shape;
  (b) a let-else-specific parent check in codegen — codegen has no parent
  context, and contract 3 forbids shape special cases.
- **Decision and rationale**: The projection writes one block
  `{$tt_syntax_stmt_N; if (true) {...}}` and records that whole block as the
  decision's placeholder segment, so the existing owner search maps the block
  (and not the placeholder expression statement) to the decision source. The
  block requirement is then evaluated at the block's parent edge by the same
  `is_unbraced_body` as every other owner.

### Decision 2: One set of block-required statements for `try` and decisions

- **Context**: A statement-form `try` and a let-else both replace their
  statement with several; the Evaluation IR already exposed the `try` case as
  `block_required_propagations`.
- **Decision and rationale**: The set becomes `block_required_statements`
  and also holds every statement decision whose context requires a block. The
  let-else emitter wraps its lowering in a block (`Rope::braced`, shared with
  the `try` path) when its decision is in the set. `if let` already emits one
  block, so it is unchanged. A `var` binding inside the block keeps its
  function scope, so code after the parent still sees it.

### Decision 3: Reject a lexical binding statement in an unbraced body with the construct's placement code

- **Context**: `if (c) const Some(x) = o else {...};` and
  `if (c) const x = try r;` are lexical declarations in a `Statement`
  position. Plain TypeScript `if (c) const y = 1;` is already rejected
  (verify-failed); the tt forms were silently braced (`try`) or invalid
  (let-else).
- **Alternatives considered**: (a) bracing the lexical form too — accepts a
  program TypeScript rejects and binds a name nothing can read; (b) only
  rejecting let-else — leaves the structurally identical `try` statement
  accepted.
- **Decision and rationale**: The Evaluation IR reports every block-required
  let-else or statement `try` whose binding mode is `const`/`let`
  (`LexicalDeclarationBody`), and the public boundary renders it as
  `let-else-placement` or `try-placement` at the statement, with the help
  "wrap the statement in braces". `ttc explain` for both codes describes the
  rule.

## Work log

- 2026-09-28: Reproduced both shapes with `ttc -p` and `--no-verify`; found
  that `if (c) const x = try p(s);` compiled silently.
- 2026-09-28: `src/program_syntax/projection.rs`: `push_placeholder_name`
  split out of `push_placeholder`; `emit_statement_decision` writes one block
  and records it as the decision's placeholder segment.
- 2026-09-28: `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`:
  `block_required_statements` (renamed from `block_required_propagations`)
  includes decisions; `lexical_declaration_bodies`.
- 2026-09-28: `src/codegen/rope/builder.rs`: `Rope::braced`;
  `src/codegen/core/emitter/expression.rs`, `source.rs`: let-else and `try`
  statements open their block through it.
- 2026-09-28: `src/lib/compile.rs`, `src/diagnostics.rs`: the placement
  diagnostic and explanations; `docs/ai/tt.md`.
- 2026-09-28: Tests `a_var_let_else_as_an_unbraced_body_is_lowered_inside_one_block`
  and `a_lexical_binding_statement_as_an_unbraced_body_is_a_placement_error`
  (`tests/compile/cases_11.rs`), and
  `runtime_a_var_let_else_as_an_unbraced_body_stays_under_its_parent`
  (`tests/integration/cases_05.rs`, tsc + node).

## Issues and resolutions

### Issue 1: The runtime program failed strict `tsc` with TS2454

- **Symptom**: `return hv;` after `if (c) var Some(value: hv) = o else {...};`
  reported "Variable 'hv' is used before being assigned".
- **Cause**: This is TypeScript's own definite-assignment rule for a `var`
  assigned only on one path; plain TypeScript `if (c) var x = 1; return x;`
  reports the same error.
- **Resolution**: The test reads the hoisted binding through a closure
  declared before the statement, which still observes `var` hoisting at
  runtime.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Runtime: `5 0 undefined`, `2 -1`, `-3 4 0`, `2 -2`.

## Result

Changed `src/program_syntax/projection.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/rope/builder.rs`,
`src/codegen/core/emitter/expression.rs`, `src/codegen/core/emitter/source.rs`,
`src/codegen/core/emitter/mod.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/planning.rs`, `src/lib/compile.rs`, `src/diagnostics.rs`,
`docs/ai/tt.md`, `tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, and this record.
