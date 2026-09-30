# TASK-593: Run earlier declarators before a later declarator's value

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-593`

> Follow-ups: TASK-600 closes Issue 2 (a closure in the first declarator's
> value that names a head binding is now a placement diagnostic), and
> TASK-601 lowers the value-form `try` in the first declarator of a
> multi-declarator head that the Scope section excludes.

## Purpose

A tt value in a later declarator of a declaration list was hoisted above
the whole declaration. For
`const a = t("a", 10), b = match (t("m", o)) { A(n) => a + n, B => 0 };`
the output was `let $tt_v0; {switch…} const a = t("a", 10), b = $tt_v0;`:
the match ran first (order `m a` instead of `a m`) and read `a` in its TDZ
(a `ReferenceError`). `var a = 10, b = match …` silently computed `NaN`, and
`for (let i = 5, j = match (o) { A(n) => i + n, B => 0 }; …)` silently read
an outer `i` (`5 101` instead of `5 6`). Value-form `try` and `result`
blocks behaved the same, and `ttc --check-types` reported TS2448/TS2454 on
valid source.

## Scope

- Included: a host owner per later declarator in the syntax layer
  (`src/program_syntax`), the loop-head placement fact and reason in the
  Evaluation IR, the declaration split in target lowering
  (`src/codegen/core`), the diagnostic messages and `ttc explain` texts,
  `docs/ai/tt.md`, `docs/design/program-lowering.md` §7.9, and regression
  tests.
- Excluded: the first declarator of a C-style `for` head keeps lowering
  before the loop. A closure written inside such a value that names a
  binding the head declares resolves to the enclosing scope there, which
  is a separate defect (see Issues). A value-form `try` in the first
  declarator of a multi-declarator `for` head still ends in
  `lowering-plan-failed`, as before this task.

## Decisions

### Decision 1: Every later declarator is its own host owner, and the declaration is split before it

- **Context**: ECMA-262 evaluates a declaration list element by element in
  the scope the declaration binds into (§14.3.1.2 `LexicalDeclaration`
  evaluation of a `BindingList`; §14.3.2.1 `VariableDeclarationList`). An
  earlier `let`/`const` binding is initialized before a later initializer
  runs; reading a later one throws while it is uninitialized (§9.1.1.1.6
  `GetBindingValue`, the TDZ). The lowering chose the whole
  statement as the host owner of every value inside it, so a prelude always
  ran before the first declarator.
- **Alternatives considered**: (a) Keep the statement owner and capture
  earlier initializers into temporaries before the prelude
  (`const $c = t("a"); …; const a = $c, b = $v`). This keeps order but not
  scope: the value's own references to `a` still precede `a`'s binding, so
  the TDZ error remains for `const`/`let`, and a closure in an earlier
  initializer would observe a different evaluation. (b) Special-case
  `match` in the emitter by rewriting the declaration text. That is a
  per-construct branch; `try`, `result`, and any composed value share the
  same placement machinery. (c) Model the declarator as a host owner in the
  syntax layer and split the declaration where its prelude is written.
- **Decision and rationale**: (c). `ParentCollector::visit_var_declarator`
  pushes a `HostOwnerKind::Declarator` owner for every declarator after the
  first of a `var`/`let`/`const`/`using`/`await using` declaration statement
  (exported or not), carrying a `DeclaratorSplit` (where the previous
  declarator ends, the declaration kind, `export`, `declare`). Every value
  inside that declarator gets it as its host owner, so the Evaluation IR
  plans it exactly as before, only anchored at the declarator. Target
  planning emits a `DeclaratorSplitRewrite` for each such owner that writes
  a prelude: the comma before the declarator becomes `;`, the prelude
  follows on its own line, and the remaining list continues under the
  repeated head. Separate declarations in one statement list bind into the
  same scope in the same order, so the split is not observable; `using`
  declarations still dispose in reverse order at the end of the block. An
  owner whose values write no prelude (an inline pipeline, a `result`
  block at an expression boundary) is not split, so such output is
  unchanged.

### Decision 2: What belongs to the statement stays keyed by the statement

- **Context**: Two facts were keyed by the owner's anchor but describe the
  statement: the block an unbraced body (`if (c) var a = 1, b = match …`) or
  a script's enclosed top-level `var` needs, and the global binding a
  script's generated names derive from.
- **Alternatives considered**: (a) Treat the declarator anchor as the block
  owner; the `{` would then open in the middle of the statement and leave
  the earlier declarators under the `if`. (b) Record the statement span on
  every owner.
- **Decision and rationale**: (b). `HostOwner::statement()` is the anchor of
  the nearest enclosing non-declarator owner (its own anchor otherwise);
  `requires_block`, `block_required_owners`, and the script globals use it.
  A split whose statement needs a block opens it at the statement's start
  (`open_declaration_blocks_at`) and the existing owner-block close ends it.
  The split also opens a layout scope at the statement's start so the
  prelude lines up with the statement even when the declarator sits on a
  continuation line.

### Decision 3: A later declarator of a C-style `for` head is rejected with a placement diagnostic

- **Context**: A `for` head's declarators run in the loop's own scope
  (ECMA-262 §14.7.4.2 `ForLoopEvaluation`), `let` bindings are copied into
  each iteration (§14.7.4.4 `CreatePerIterationEnvironment`), and the head has no
  statement position between declarators.
- **Alternatives considered**: (a) Split the head: move the earlier
  declarators before the loop. For `let`/`const` this drops them from the
  loop's scope and its per-iteration copies. (b) Declare the earlier
  declarators in a block before the loop and re-declare them in the head
  from generated temporaries (`let i = $c`). This renames authored
  bindings' initializers, loses type annotations and destructuring
  structure, and changes `using` disposal. (c) Split only `var` heads,
  where (a) is faithful; this adds a second relocation shape for a rare
  form. (d) Reject with the existing placement family.
- **Decision and rationale**: (d), for `var`, `let`, `const`, and `using`
  alike. `EvaluationContext::loop_head_declarator` records, from the AST
  path, that the value sits in a declarator after the first of a `for`
  head, and `target_capability` answers
  `ExpressionBoundary(ExpressionBoundaryReason::LoopHeadDeclarator)`. A
  `match` reports `match-placement` and a value-form `try` `try-placement`,
  each saying why and how to fix it; a `result` block, which has an
  expression form, runs in place through `$tt_expr` and keeps the head's
  scope. The generic `UnsupportedForInitializer` plan failure no longer
  pre-empts that diagnostic for such a `try`.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/repro/r1_declarators.tt`:
  order `m a` and a `ReferenceError`, `NaN` for `var`, `5 101` for the loop
  head, and TS2448/TS2454 from `ttc --check-types`.
- 2026-09-30: `src/program_syntax.rs`, `src/program_syntax/collector.rs`,
  `src/program_syntax/visit.rs`: `HostOwnerKind::Declarator`,
  `DeclaratorSplit`, `DeclarationKind`, `HostOwner::statement`,
  `EvaluationContext::loop_head_declarator`, and the declarator owner.
  `src/program_syntax/projection.rs`, `src/evaluation_ir/evaluation.rs`:
  globals and owner blocks keyed by the statement.
- 2026-09-30: `src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`:
  `ExpressionBoundaryReason::LoopHeadDeclarator`. `src/lib/compile.rs`:
  its `match-placement` and `try-placement` messages.
  `src/diagnostics.rs`: the `ttc explain` texts.
- 2026-09-30: `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`,
  `src/codegen/core/emitter/{mod,source,host}.rs`: `DeclaratorSplitRewrite`
  and its emission in the source walk.
- 2026-09-30: Tests
  `a_value_in_a_later_declarator_splits_the_declaration_before_its_prelude`
  and
  `a_statement_value_in_a_later_loop_head_declarator_is_a_placement_error`
  (`tests/compile/cases_14.rs`),
  `runtime_a_later_declarator_value_runs_after_earlier_declarators_in_their_scope`
  (`tests/integration/cases_05.rs`, tsc + node),
  `a_later_declarator_reading_an_earlier_one_checks_clean`
  (`tests/native/cases_10.rs`; the unfixed compiler reports TS2448 and
  TS2454 here), and the emit fixture
  `tests/fixtures/emit/declarator-list-values` (reviewed: the comment
  between declarators is kept verbatim, the prelude starts at the
  statement's indentation, and `export const` is repeated).
- 2026-09-30: Checked by hand through tsc + node: `export`, `var` in an
  unbraced `if` body, script-mode top-level `var` (enclosed) and `const`
  (globally named slot), `using` (disposal order `d2 d1`), a label, a
  `result` block body, a match arm block, a namespace, `await` between
  declarators, and a value in the first and a later declarator of one
  statement.

## Issues and resolutions

### Issue 1: A generated line break outside a layout scope

- **Symptom**: The first build raised the internal error "a generated line
  break has a layout scope (LayoutScopeMissing)".
- **Cause**: The split's line break was pushed straight into the source
  walk's rope, which has no layout scope of its own.
- **Resolution**: The split opens a scope at the statement's start and
  breaks inside it; when the walk did not start at the statement, the break
  gets a scope of its own.

### Issue 2: Remaining gap in the first declarator of a loop head

- **Symptom**: `for (let f = match (o) { A(n) => () => f, B => null }; …)`
  lowers the match before the loop, where a closure naming `f` resolves to
  the enclosing scope instead of the head's binding.
- **Cause**: The same scope rule as Decision 3, reached through a closure;
  a direct reference is a TDZ error in the source as well.
- **Resolution**: Not changed by this task (outside the reported defect);
  left for a follow-up that decides it from name resolution rather than
  from placement alone.

### Issue 3: `workflow_repairs` failed in the first gate run

- **Symptom**: `dependencies_of_a_configured_project_are_only_its_inputs`
  listed the TypeScript compiler outside the repository.
- **Cause**: The worktree's `node_modules` was a symbolic link to another
  checkout's, so the resolved TypeScript lived outside this repository. The
  test is right to reject that; the change did not cause it.
- **Resolution**: Replaced the link with `npm ci` and reran the test binary
  and the doctests, which the first run skipped after that failure; both
  pass.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/{collector,visit,projection,tests}.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,planning}.rs`,
`src/codegen/core/{mod,planning}.rs`,
`src/codegen/core/emitter/{mod,source,host}.rs`, `src/lib/compile.rs`,
`src/diagnostics.rs`, `docs/ai/tt.md`, `docs/design/program-lowering.md`,
`tests/compile/cases_14.rs`, `tests/integration/cases_05.rs`,
`tests/native/cases_10.rs`, `tests/fixtures/emit/declarator-list-values/`,
`docs/tasks/INDEX.md`, and this record. A declaration list now evaluates
as written; a later `for`-head declarator is a placement diagnostic.
