# TASK-624: Accept a `using` declaration in a `for` initializer and a function as an `if` clause

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-624`

## Purpose

Valid TypeScript was rejected. `for (using q = res(); ; )` and
`for (await using q = res(); ;)` failed with `verify-failed: generated
TypeScript failed to parse: Expected ';'` (or `source-not-typescript` when
the file had a tt construct), and `if (Math.random()) function f() {}` with
`verify-failed: Declaration is not allowed`. The pinned `tsc` accepts all
three.

## Scope

- Included: the swc parse ttc uses both to model a `.tt` file's TypeScript
  and to verify its output: the vendored `swc_ecma_parser`, and vendored
  copies of `swc_ecma_ast` and `swc_ecma_visit`; the source model's reading
  of the new initializer (`src/program_syntax/scopes.rs`,
  `src/program_syntax/visit.rs`); tests.
- Excluded: declarations TypeScript's parser accepts in statement position
  that no ECMAScript grammar has (`if (c) class C {}`,
  `while (c) function f() {}`, `if (c) function* g() {}`), which stay
  rejected; whether a function clause is allowed in strict code, which is
  TypeScript's checker's rule, not the parse's.

## Decisions

### Decision 1: Represent the initializer in the AST; no swc version has it

- **Context**: ES2026's explicit resource management makes a
  `UsingDeclaration` or `AwaitUsingDeclaration` a `LexicalDeclaration`, so
  `for (using x = e; c; u) S` is a `ForStatement` (ECMA-262,
  "Explicit Resource Management", §14.7.4 `ForStatement`; TypeScript 5.2
  release notes, "`using` Declarations and Explicit Resource Management").
  `swc_ecma_parser` 45.0.0 reads only `for (using x of xs)`, and
  `swc_ecma_ast`'s `VarDeclOrExpr` (a `for` initializer) has only `VarDecl`
  and `Expr`, also on upstream `main`
  (`crates/swc_ecma_ast/src/stmt.rs`), whose TypeScript conformance run
  still excludes `awaitUsingDeclarationsInFor.ts`. No parser configuration
  flag enables it (`explicit_resource_management` is always on for
  TypeScript and covers only the statement and `for-of` forms).
- **Alternatives considered**: (a) Upgrade swc: no version represents it.
  (b) Parse it as a `const` `VarDecl`: the AST would claim a declaration
  that needs no disposal, and the source model's cleanup analysis
  (`statement_is_cleanup_free`, whose match over the initializer already
  waited for a using variant) would take the loop for one that needs none.
  (c) Parse it as `{ using x = e; for (; c; u) S }`: the same runtime
  meaning, but a block the source does not have, whose span is the loop's,
  where the lowering places generated statements. (d) Skip the output
  verification for such files: a fallback, and the source model would still
  refuse a file with a tt construct. (e) Add `VarDeclOrExpr::UsingDecl`,
  vendoring `swc_ecma_ast` and `swc_ecma_visit` next to the already
  vendored parser.
- **Decision and rationale**: (e). It is the one representation that says
  what the source says, as `ForHead::UsingDecl` already does for `for-of`.
  The AST gains the variant, the visitor's generated code gains the arms
  the generator writes for `ForHead::UsingDecl`, the parser produces it
  when `using` (or `await using`) is followed on the same line by a binding
  identifier other than `of`/`in`, and the source model reads it: its names
  are scoped to the loop like a `const`'s, it is not cleanup-free, and a
  second declarator already counted as a loop-head declarator. The
  vendored crates depend on each other by path so one AST is in the build.
  Each change is recorded in the crate's `TT-PATCH.md`.

### Decision 2: A plain function declaration is an `if` clause in every file

- **Context**: ECMA-262 Annex B.3.4 ("FunctionDeclarations in IfStatement
  Statement Clauses") adds `if (e) FunctionDeclaration` in non-strict code.
  ttc parses every file as a module, which is strict, and swc rejected the
  declaration in any clause. TypeScript's parser reads it in every file
  (`parseStatement` parses a declaration in statement position), and the
  pinned `tsc` reports nothing for it in a script or a module.
- **Alternatives considered**: (a) Accept it only in non-strict code: ttc
  learns whether a file is a script only from its parse, so this needs a
  second, script-goal parse of every file. (b) Accept every declaration
  TypeScript's parser accepts there: `while (c) function f() {}` and
  `if (c) class C {}` are in no ECMAScript grammar. (c) Accept the Annex B
  production in the parse and leave strictness to the TypeScript checker.
- **Decision and rationale**: (c). The parse ttc runs is a question about
  syntax TypeScript reads; whether the file's mode allows the clause is a
  diagnostic TypeScript owns (the error-layer contract). A generator stays
  rejected, as the production excludes it.

## Work log

- 2026-09-30: Reproduced both with `ttc -p` (`verify-failed`).
- 2026-09-30: Checked swc: 45.0.0's `parse_for_head` and upstream `main`'s
  `VarDeclOrExpr`; the upstream conformance exclusion.
- 2026-09-30: Vendored `swc_ecma_ast` and `swc_ecma_visit` 29.0.0 (path
  dependencies in `Cargo.toml` and the vendored crates' manifests), added
  `VarDeclOrExpr::UsingDecl` and its visitor arms (six `generated.rs` match
  arms, the field kind, the node iterator), the parser's initializer and
  `parse_if_clause`, and the source model's cases.
- 2026-09-30: Tests `tests/swc_for_using_and_if_function.rs` (the parser
  directly), `a_using_declaration_in_a_for_statement_and_a_function_as_an_if_clause`
  (`tests/passthrough.rs`), and
  `a_using_for_statement_and_an_if_function_clause_host_tt_values`
  (`tests/compile/cases_14.rs`: a match in the loop body, a `try` in the
  using initializer lowered before the loop, a match in the function
  clause's body).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `Cargo.toml`, `Cargo.lock`, `vendor/swc_ecma_parser/` (`Cargo.toml`,
`src/parser/stmt.rs`, `TT-PATCH.md`), new `vendor/swc_ecma_ast/` and
`vendor/swc_ecma_visit/` (with `TT-PATCH.md`), `src/program_syntax/scopes.rs`,
`src/program_syntax/visit.rs`, `tests/swc_for_using_and_if_function.rs`,
`tests/passthrough.rs`, `tests/compile/cases_14.rs`, `docs/tasks/INDEX.md`,
and this record. Both forms compile as the TypeScript they are.
