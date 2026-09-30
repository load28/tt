# TASK-681: Reject a statement `try` in class code of a class written inside a function

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-681`

## Purpose

`function f() { class H { static { const v = try read(1); } } }` compiled
and emitted `if (!("value" in $tt_t0)) { return $tt_t0; }` inside the
static block, which TypeScript rejects (TS18041), while the same statement
in a class at a module's top level and the value form in the same block
were `try-placement` as documented (`docs/design/try-result-scopes.md`
§4.2 and §4.5, "class static block: Reject"). TASK-679 Issue 1 found it
through six matrix cases (`tryStatement_*_staticBlock_*`).

## Scope

- Included: the lexer's token facts (`src/lexer/facts.rs`,
  `src/lexer/facts/statements.rs`: a class body `{` and a static block `{`),
  the function-boundary model built on them (`src/flow/syntax.rs`:
  `FunctionTarget::StaticBlock` and `FunctionTarget::ClassElement`), the
  statement `try` placement check (`src/sema/checker.rs`), unit tests for
  both layers, the case `tests/cases/compiler/tryInClassCodeInsideFunction.tt`,
  the six matrix baselines, and `tests/oracle-failures.txt`.
- Excluded: the value form, whose owner is already the SWC
  `EvaluationOwner::StaticBlock`/`ClassInitializer` and was correct; a
  function-targeted `try` in a match block arm (TASK-683).

## Decisions

### Decision 1: Class code outside a method is a function-like boundary of the token model

- **Context**: The statement form's target is the token model's innermost
  function body (`flow::function_target_at`), which only knew function,
  constructor, and generator bodies; a static block was found by a
  separate scan (`in_static_block`) that was only consulted when no
  function enclosed the `try`. Inside a function the scan was never
  consulted, so the outer function became the target.
- **Alternatives considered**:
  - Consult `in_static_block` whether or not a function encloses the
    `try`: it answers "some enclosing brace is a static block", so a
    function written inside the static block would be rejected too, and
    it does not cover a field initializer.
  - Route the statement form through the SWC evaluation owner: the value
    form's owners are right, but the statement form is judged by the
    semantic pass before any lowering plan exists, and moving it is a
    larger change than this defect.
- **Decision and rationale**: The lexer, which already records which `{`
  opens a function, constructor, or generator body, also records the `{`
  of a class body and of a class static block, and the boundary model
  treats both as the innermost target: `StaticBlock` and `ClassElement`
  (what a class body holds outside its methods: field initializers,
  computed names). Every token-model consumer then sees the same
  boundaries. ECMA-262 is the source: a static block's statements are
  `ClassStaticBlockStatementList : StatementList[~Yield, +Await, ~Return]`
  (§15.7, so `return` is a syntax error there), and a field initializer is
  evaluated as its own method-like function (§15.7.10
  ClassFieldDefinitionEvaluation) whose `Initializer` is an expression, so
  neither can return from the function the class is written in. Methods,
  accessors, and arrows keep their own function target because their
  bodies are inner boundaries.

## Work log

- 2026-09-30: Reproduced with `ttc --check`: the repro compiled cleanly.
- 2026-09-30: Added the `CLASS_BODY` and `STATIC_BLOCK` facts, the two
  `FunctionTarget` variants, and replaced the `in_static_block` check in
  `check_try`; removed `in_static_block`.
- 2026-09-30: Added the unit tests
  `lexer::facts::tests::a_brace_records_the_function_body_it_opens` (class
  and static braces) and
  `flow::tests::class_code_outside_methods_is_its_own_function_target`,
  and the case `tryInClassCodeInsideFunction` (errors only); removed the six
  `TASK-679 Issue 1` lines from `tests/oracle-failures.txt`; ran
  `TT_MATRIX_CASES=all TT_CASES=staticBlock cargo test --test
  case_baselines` and accepted the six changed baselines with
  `scripts/baseline-accept` after reading them (each now reports
  `try-placement` at the `try` instead of TS18041).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tryInClassCodeInsideFunction.tt`, and the
  matrix cases `tryStatement_declaration_staticBlock_using` and
  `tryStatement_propagate_staticBlock_{exception,finally,optionalChain,plain,spread}`.
- **Observed failure**: with the non-test changes reverted, `TT_CASES=tryInClassCodeInsideFunction
  cargo test --test case_baselines` failed with
  `tryInClassCodeInsideFunction: does not report try-placement`; the
  matrix cases were listed in `tests/oracle-failures.txt` as `does not
  report try-placement` (TASK-679 Issue 1).

## Verification

- [x] `cargo test --lib flow::` and `cargo test --lib lexer::facts`: pass.
- [x] `TT_MATRIX_CASES=all TT_CASES=staticBlock cargo test --test case_baselines`: pass.
- [x] `cargo test --lib`, `--test compile`, `--test case_baselines`,
  `--test snapshot`: pass.
- [x] Baseline changes reviewed and committed with the change.
- The full gate is recorded in TASK-684.

## Result

Changed files: `src/lexer/facts.rs`, `src/lexer/facts/statements.rs`,
`src/lexer/facts/tests.rs`, `src/flow/syntax.rs`, `src/flow/mod.rs`,
`src/flow/tests.rs`, `src/sema/checker.rs`,
`tests/cases/compiler/tryInClassCodeInsideFunction.tt` and its
`.errors.txt` baseline, six `tryStatement_*_staticBlock_*` baselines,
`tests/oracle-failures.txt`, `docs/tasks/INDEX.md`, and this record.
