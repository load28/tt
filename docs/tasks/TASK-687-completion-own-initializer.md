# TASK-687: Leave the declaration being initialized out of completions inside a construct

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-687`

**Update (TASK-688)**: an exported declaration is not left out. TypeScript
keeps it in its own initializer, because the symbol in scope is the local
symbol `declareModuleMember` creates with `ExportValue` alone, which has no
value declaration (TASK-688 Issue 1).

## Purpose

TASK-686 Issue 1: completion inside a tt construct in a declaration's
initializer offered the name being declared (`const value = match
(/*c*/input) { _ => 1 }` offered `value`; `const n = try
read(/*c*/input);` offered `n`), which TypeScript leaves out of its own
initializer. 193 questions of the editor matrix recorded it.

## Scope

- Included: a completion fact per construct computed from the compiler's
  whole-program syntax (`src/program_syntax/completion.rs`, marks recorded
  by `src/program_syntax/projection.rs`), carried through the lowering plan
  and `MappedEmit` to the engine's service projection, and the engine's
  restatement of TypeScript's answer (`src/engine/language/scope.rs`,
  called from `Project::service_completion`); a hand-written editor case;
  the matrix list lines and baselines.
- Excluded: the keywords offered in a construct at module level (TASK-688),
  which the same fact carries in that task.

## Decisions

### Decision 1: Cause

- **Context**: TypeScript leaves a variable out of the completions of its
  own initializer by syntax: `getClosestSymbolDeclaration`
  (typescript-go `internal/ls/completions.go`) walks up from the
  position to a `VariableDeclaration`, returning nothing at a function
  block, an arrow function's body, or a binding pattern, and
  `shouldIncludeSymbol` drops the symbol whose value declaration is that
  declaration ("Filter out variables from their own initializers").
- **Cause**: lowering evaluates a construct before the declaration it
  initializes: `const value = match (input) { ... }` becomes `let $tt_v0;
  { const $tt_m = input; switch ... }` followed by `const value = $tt_v0;`,
  and `const n = try read(input);` becomes `const $tt_t0 = read(input);`,
  its test, and `const n = $tt_t0.value;`. The walk from the served
  position reaches `$tt_m`'s or `$tt_t0`'s declaration, or nothing, never
  `value`'s; and `value` is in scope there (a block-scoped declaration later
  in the same block), so TypeScript offers it.

### Decision 2: The construct's place decides where the walk leaves the user's text

- **Alternatives considered**: (a) Lower the construct inside the
  initializer (an IIFE): `docs/ai/tt.md` excludes it ("match never emits an
  IIFE, callback"), and it changes `this`, `await`, and evaluation.
  (b) Find the enclosing declaration by scanning the `.tt` tokens: a
  second, approximate TypeScript parser. (c) Treat every part of the
  construct, match arms and `result` bodies included, as part of the
  initializer: in TypeScript statements can stand inside an initializer
  only in a function body, where the walk stops; the compiler's own
  syntax model and the editor twins (TASK-686 Decision 1) write a value
  `match` and a `result` block as a region function; and dropping the name
  inside an arm would need the symbol's identity, since an arm's binding
  can shadow it (`const same = match (b) { Full(same) => same }`). (d) Ask
  TypeScript again with the name spliced in, to learn which symbol it
  resolves to: a round trip per completion for a fact the syntax states.
- **Decision and rationale**: (e) the compiler's whole-program syntax
  (`ProgramSyntax`, the SWC parse of the source with each construct as a
  placeholder where it is written, a value `match` and a `result` block as
  the region function `(() => { ... })()` whose scrutinee statements are
  evaluated at the construct's place) is walked with TypeScript's rule:
  a variable declarator's initializer names its identifier, a function or
  arrow body, a parameter, a catch parameter, a binding pattern, or a region
  statement that is not a scrutinee ends it, and a declaration-form
  propagation (`const n = try ...;`, projected as one expression statement)
  names its binding when it is an identifier. Each construct's
  `CompletionScope` records that name, whether the construct is a region
  function, and its scrutinees' source spans. In the engine, TypeScript's
  answer stands where the walk from the served position stops in text the
  user wrote (the innermost declarator, function, arrow, constructor, or
  catch parameter around it is copied source, found with SWC over the
  served text); where it reaches compiler-written text, the innermost
  construct around the cursor decides, and its declaration's entry (the
  label with no `source`; an auto-import entry is another module's export)
  is left out. Inside a scrutinee or a `try` operand no tt binding can
  shadow the name, so the label is the symbol.

## Work log

- 2026-09-30: Reproduced on `ttc -p` output (`let $tt_v0; { ... }` then
  `const value = $tt_v0;`) and read `getClosestSymbolDeclaration` and
  `shouldIncludeSymbol` in typescript-go.
- 2026-09-30: `ProjectionBuilder` records each region function's start and
  scrutinee statements, and each declaration-form propagation's statement
  with its identifier binding (`CompletionMarks`); `completion_scopes`
  walks the parsed projection; `lowering_plan_with` stores the scopes in
  the plan, `MappedEmit` and `ServiceDoc` carry them;
  `scope::restate_completions` runs on the plain and the probed answer.
- 2026-09-30: Added `tests/cases/editor/completionOwnInitializer.tt`:
  module-level and function-level scrutinees, an arm binding that shadows
  the declared name, an arrow inside a scrutinee, a value `try`, a
  statement `try`, a `try` and a `return` inside a `result` block.
- 2026-09-30: Removed the 10 `TASK-686 Issue 1` lines of
  `tests/editor-matrix-differences.txt` and ran every generated case
  (`TT_MATRIX_CASES=all cargo test --test editor_cases`); the difference
  baselines were updated through `scripts/baseline-accept` after reading
  the diff (below).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/completionOwnInitializer.tt` (and the 10
  removed `TASK-686 Issue 1` lines of
  `tests/editor-matrix-differences.txt`)
- **Observed failure**: with the two `restate_completions` calls in
  `Project::service_completion` removed, `TT_CASES=completionOwnInitializer
  cargo test --test editor_cases` failed with a modified baseline: the
  answers at `topScrutinee`, `scrutinee`, `valueTry`, `statementTry`, and
  `inResult` each gained the declared name (`top`, `value`, `t`, `n`, `k`
  as `Variable`), in the engine's list and in the adapter's; the other
  three markers were unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Every generated editor case and the hand-written cases
  (`TT_MATRIX_CASES=all TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 cargo test
  --test editor_cases`, 1,103 s): the only failures were the baselines of
  the fixed questions; `scripts/baseline-accept` then deleted 160
  difference baselines and changed 17, and the diff (1,291 deleted lines,
  none added) removes exactly the 193 listed `completions` sections, whose differences
  were `tt only: n`, `a`, `b`, `value`, `checked`, or `top0` (199 lines); no
  other question changed and no list line became unused.
- [x] Baseline changes reviewed and committed with the change.
- The full gate for TASK-687 to TASK-690 is recorded in TASK-688's record,
  which ran it once after the last of them.

## Result

Changed files: `src/program_syntax.rs`, `src/program_syntax/completion.rs`,
`src/program_syntax/projection.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/core/mod.rs`,
`src/lib/compile.rs`, `src/lib/mapped.rs`, `src/engine/language.rs`,
`src/engine/language/scope.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`,
`tests/cases/editor/completionOwnInitializer.tt` and its baseline, the
matrix difference baselines, `tests/editor-matrix-differences.txt`,
`docs/design/lsp-architecture.md`, `docs/tasks/INDEX.md`, and this record.
