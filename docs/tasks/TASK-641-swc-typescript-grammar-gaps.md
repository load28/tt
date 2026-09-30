# TASK-641: Read five TypeScript forms the vendored swc parser rejected

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-641`

## Purpose

After TASK-640, 13 units of TypeScript's own test cases that the pinned
TypeScript accepts were still rejected (contract 1): swc cannot parse them,
so both the output self-check and, in a file with tt syntax, the projection
that models the file's TypeScript fail. Each is fixed in the vendored parser
when the fix is small and follows TypeScript's grammar, or triaged with the
reason it is not.

## Scope

- Included: `vendor/swc_ecma_parser` (`src/parser/stmt.rs`,
  `src/parser/expr.rs`, `src/lexer/state.rs`, `src/parser/module_item.rs`,
  `TT-PATCH.md`), `tests/swc_typescript_grammar_gaps.rs`, a case file, and
  `tests/passthrough-triaged.txt`.
- Excluded: AST changes to the vendored `swc_ecma_ast` and `swc_ecma_visit`,
  and parser changes that need a reparse or a change of swc's module
  detection (the four triaged units).

## Sources

- TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`,
  `tsc/internal/parser/parser.go` and `tsc/internal/scanner/scanner.go`:
  `parseForOrForInOrForOfStatement` (line 1292; "this one is meant to allow
  of" for `await using`), `isParenthesizedArrowFunctionExpressionWorker`
  (line 4303, microsoft/TypeScript#44466), `ScanJsxAttributeValue`
  (scanner line 1348, skips whitespace before the string),
  `tryParseImportAttributes` (line 2546, the line-break restriction only on
  `assert`), `parseTypeQuery` (line 3160, `parseEntityName` with
  `allowPrivateName`), `parseConditionalExpressionRest` (line 4601),
  `parseSourceFileWorker` and `reparseTopLevelAwait` (lines 430-560).
- ECMA-262: `ForInOfStatement` with `[lookahead ≠ using of]` for a plain
  `using` only; `WithClause` has no `[no LineTerminator here]`.
- swc upstream: swc-project/swc#12354 ("allow of bindings in await using
  loops") and swc-project/swc#12356 ("allow line breaks before import
  attributes"), both merged after the vendored 45.0.0, make the same two
  changes; the other three have no upstream fix that was found.

## Decisions

### Decision 1: Patch the five forms that are local parser decisions

- **Context**: The verify-layer header (`src/verify.rs`) makes an swc parse
  failure on valid TypeScript a compatibility bug to fix at the swc boundary.
- **Alternatives considered**: Triaging all 13 (leaves contract 1 broken for
  forms whose fix is a few lines); upgrading swc to a release with #12354 and
  #12356 (the vendored copy carries ttc's own patches, and three of the
  forms are not fixed upstream).
- **Decision and rationale**: Five patches, each the check TypeScript's
  parser makes, listed in `TT-PATCH.md`: `of` as the binding of an
  `await using` declaration in a `for` head; a modifier followed by `as` as
  the first element of a parenthesized list opens an expression; whitespace
  before a JSX attribute string; `export { type "x" as "y" } from`; and
  `export @dec abstract class`; plus `with` import attributes after a line
  break. Nine units now pass.

### Decision 2: Triage the four that are not local

- **Decision and rationale**, each listed in `tests/passthrough-triaged.txt`
  under TASK-641 with its reason:
  - `typeof this.#a`: swc's `TsQualifiedName.right` is an `IdentName`; a
    private name there is an AST change in the vendored `swc_ecma_ast` and
    `swc_ecma_visit`.
  - `throw await` followed by a line break in a module: TypeScript reparses a
    module's top-level statements in an await context; swc decides in one
    pass from the context seen so far.
  - `import await = foo.await`: swc counts an import-equals declaration of an
    entity name as module syntax, TypeScript only `import x = require()`.
  - `a ? b ? c : (d) : e => f`: TypeScript parses a conditional's true branch
    without return types on arrow functions, which swc's conditional context
    does not carry into a nested conditional.

## Work log

- 2026-09-30: Read each unit's failure and the TypeScript parser function
  for it; found swc-project/swc#12354 and #12356.
- 2026-09-30: Patched the parser; a first version of the `readonly as` check
  peeked the token after every first element and broke
  `compiler/commaOperatorLeftSideUnused.ts` (Issue 1).
- 2026-09-30: Added `tests/swc_typescript_grammar_gaps.rs` and
  `tests/cases/compiler/hostGrammarBesideTt.tt`, removed nine triaged
  units, and rewrote the reasons of the four that remain.

## Issues and resolutions

### Issue 1: Peeking past a template literal

- **Symptom**: `xx = (\`wat\`, 'ok')` failed with "Expression expected".
- **Cause**: The check peeked the next token whatever the current one was,
  and a peek past a template start changes swc's lexer state.
- **Resolution**: Peek only when the current token is one of the four
  modifiers `eat_any_ts_modifier` reads.

## Regression test (fails before the fix)

- **Path**: `tests/swc_typescript_grammar_gaps.rs` (six tests) and
  `tests/cases/compiler/hostGrammarBesideTt.tt`.
- **Observed failure**: Without the vendored changes all six tests fail, for
  example `"export { type \"x\" as \"c d\" } from \"./m\";\n": Error { error:
  (15..18, Expected(",", "string literal")) }` and `"declare const dec:
  any;\nexport @dec abstract class C11 {}\n": Error { error: (37..45,
  Expected("{", "abstract")) }`; the case reports `modified baseline:
  .../hostGrammarBesideTt.map.txt is out of date` (the projection rejects the
  file, so no lowering exists).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` (final gate, TASK-643)
- [x] `cargo test` (targeted: `--lib`, `compile`, `passthrough`,
  `case_baselines`, `snapshot`, `corpus`, and the five swc test files; full
  gate in TASK-643)
- [x] `TTC_TYPESCRIPT_CASES=all cargo test --test corpus typescript_test_cases`:
  "12779 parse, 45 differ (45 listed)".
- [x] Baseline changes reviewed: the new case emits the host forms unchanged
  and reports nothing.

## Result

Changed files: the four vendored source files and `TT-PATCH.md`,
`tests/swc_typescript_grammar_gaps.rs`, `tests/passthrough-triaged.txt`
(13 to 4 units), `tests/cases/compiler/hostGrammarBesideTt.tt` and its
baselines, `docs/tasks/INDEX.md`, and this record.
