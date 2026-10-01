# TASK-688: Offer a function body's keywords in the arms and blocks of a module-level construct

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-688`

## Purpose

TASK-686 Issue 2: completion inside a match arm or a `result` block that
initializes a module-level declaration (`const top = match (input) { 1 =>
f(/*c*/input), _ => 0 };`) listed 18 keywords (`abstract`, `any`,
`asserts`, `bigint`, `boolean`, `declare`, `infer`, `keyof`, `module`,
`namespace`, `never`, `number`, `object`, `readonly`, `string`, `symbol`,
`unique`, `unknown`) that TypeScript does not offer there; 35 questions of
the editor matrix recorded it.

## Scope

- Included: the function-like-body fact of each construct's
  `CompletionScope` (`src/program_syntax/completion.rs`), the keyword
  restatement in `src/engine/language/scope.rs`, a hand-written editor case
  with a TypeScript twin, the matrix list lines and baselines, and the
  exported-declaration correction to TASK-687 (Issue 1).
- Excluded: any other keyword list; TypeScript's answer stands wherever
  its walk stays in the user's text.

## Decisions

### Decision 1: Cause

- **Context**: `getGlobalCompletions` (typescript-go
  `internal/ls/completions.go`) chooses the keywords of a global
  completion by `tryGetFunctionLikeBodyCompletionContainer`, which walks up
  from the context token to a function-like declaration whose body holds
  it, stopping at a class: inside one, `KeywordCompletionFiltersFunctionLikeBodyKeywords`;
  otherwise `KeywordCompletionFiltersAll`, which
  `getTypescriptKeywordCompletions` extends with `declare`, `module`,
  `type`, `namespace`, `abstract`, and every type keyword but `undefined`.
- **Cause**: a match arm and a `result` block are region functions in the
  construct's documented TypeScript form and in the compiler's syntax
  model (TASK-687 Decision 2; `docs/ai/tt.md`: a match arm and a `result`
  block are isolated value regions whose `return` delivers the
  construct's value), and the editor twins write them as arrow bodies; but
  lowering runs a module-level construct's arms in a block at module level,
  where TypeScript's walk finds no function body. The 18 keywords are
  exactly those the `All` filter adds to the function-body filter:
  `isFunctionLikeBodyKeyword` admits a contextual keyword only when it is
  `async`, `await`, `using`, `as`, `satisfies`, or `type`, and these 18 are
  contextual. In a function, the served arms are in that function's body
  and TypeScript already answers with the function-body filter, which is
  why the same construct inside a function agreed with its twin.

### Decision 2: Restate the answer with the function-body filter where the walk leaves the user's text

- **Alternatives considered**: (a) Lower module-level arms into a function
  in the served text: it changes what the checker sees (narrowing of the
  storage, top-level `await` in an arm), and the service must check what
  the build checks. (b) Remove the 18 keywords whenever the cursor is in
  an arm: wrong in a type position (`_ => input as /*c*/number`, where
  TypeScript's `KeywordCompletionFiltersTypeKeywords` offers `number`)
  and in a class body inside an arm. (c) Re-derive TypeScript's
  type-position decision (`isTypeOnlyLocation`: `isPossiblyTypeArgumentPosition`,
  `IsPartOfTypeNode`, `isContextTokenTypeLocation`) on the tt source: a
  reimplementation of TypeScript's parser-level heuristics that would
  drift from them.
- **Decision and rationale**: (d) the compiler's syntax walk of TASK-687
  also records whether each construct's place is inside a function-like
  body, with a class ending it as TypeScript's walk does, and a region
  statement that is not a scrutinee is one. In the engine, where
  TypeScript's container walk over the served text stops in the user's
  text (a function-like declaration whose body holds the position, or a
  class, copied from the source) its answer stands; where it leaves the
  user's text and the construct says "function-like body", the answer is
  restated as the function-body filter would have produced it, and only
  when TypeScript applied `KeywordCompletionFiltersAll`: that filter is the
  only one that offers `namespace` (`getTypescriptKeywordCompletions`), so
  its presence is TypeScript's own classification of the position as a
  value position outside every function body, and the 18 keywords removed
  are exactly `All` minus `FunctionLikeBodyKeywords`. A type position, a
  class body, or a member completion carries another filter and is left as
  TypeScript answered it.

## Work log

- 2026-09-30: Confirmed from the matrix baselines that the operand of the
  same module-level construct agrees with its twin while its arms do not,
  and read `getGlobalCompletions`,
  `tryGetFunctionLikeBodyCompletionContainer`,
  `getTypescriptKeywordCompletions`, and `isFunctionLikeBodyKeyword` in
  typescript-go.
- 2026-09-30: `CompletionScope` gains `function_body`; the walk keeps a
  stack of scopes so a class resets it and a declaration inherits it.
  `scope::restate_completions` records which of the two walks stop in the
  user's text and applies the construct's facts to the rest.
- 2026-09-30: Added `tests/cases/editor/completionModuleLevelArm.tt` with
  a `.ts` twin (`@parityIgnores: t, run, unwrap`): an expression arm, a
  block arm, a type position in an arm, a scrutinee, and a `try` in a
  `result` block, all at module level.
- 2026-09-30: The twin exposed Issue 1. After the fix, all five questions
  agree with the twin.
- 2026-09-30: Removed the five `TASK-686 Issue 2` lines of
  `tests/editor-matrix-differences.txt`; `TT_MATRIX_CASES=all
  TT_CASES=topLevel cargo test --test editor_cases` then failed only on
  baselines: 17 stale and one modified, accepted with
  `scripts/baseline-accept`. The diff deletes 804 lines and adds none: 35
  `completions` sections, each exactly the 18 keywords as `tt only`.

## Issues and resolutions

### Issue 1: TASK-687 left an exported declaration out, which TypeScript keeps

- **Symptom**: at `export const scrutinee = match (/*scrutinee*/input) {
  _ => 0 };` the twin `export const scrutinee = ((t: number) =>
  0)(/*scrutinee*/input);` offered `scrutinee` and tt did not. A plain
  TypeScript probe (`export const a1 = g(/*p*/input);`) offers `a1` too.
- **Cause**: the binder declares an exported module member twice
  (`declareModuleMember` in typescript-go `internal/binder/binder.go`): a
  local symbol flagged `ExportValue` alone, and the export symbol. Only
  the export symbol gets the value declaration; `getSymbolsInScope` copies
  the source file's locals first, so the entry is the local symbol, whose
  value declaration is not the `VariableDeclaration`, and
  `shouldIncludeSymbol` keeps it.
- **Resolution**: the syntax walk names no declaration for a declarator of
  an exported variable statement (`visit_export_decl`), so the fact is
  TypeScript's. `completionOwnInitializer`'s module-level `export const
  top` now keeps `top`, as TypeScript does; the TASK-687 record says so at
  its top. No generated case exports the declaration it asks in, so the
  matrix is unchanged by it.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/completionModuleLevelArm.tt` (with its
  twin), and the five removed `TASK-686 Issue 2` lines of
  `tests/editor-matrix-differences.txt`.
- **Observed failure**: with the keyword restatement disabled in
  `src/engine/language/scope.rs`, `TT_CASES=completionModuleLevelArm cargo
  test --test editor_cases` failed with a modified baseline: the `arm`,
  `block`, and `inResult` answers had 1,063 entries instead of 1,045 and
  their parity sections read `differs` with the 18 keywords as `tt only`;
  `typed` (2,007 entries, a type position) and `scrutinee` were unchanged.
  With TASK-687's walk before Issue 1's fix, the `scrutinee` parity was
  `differs` with `ts only: scrutinee (Variable)`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Full gate for TASK-687 to TASK-690, run once on this task's tree
  (TASK-687, TASK-689, and TASK-690 committed before it):
  `node scripts/generate-cases --check` (3,286 generated cases and 3,706
  editor case files match the spec); `cargo fmt --check`; `cargo clippy
  --all-targets -- -D warnings`; `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1
  TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1
  TT_BASELINE_TRACKING_DIR=<dir> cargo test --no-fail-fast`: 1,799 passed,
  0 failed, no `SKIP`, 483 s; `node scripts/check-baselines --tracking
  <dir>`: "701 compared, 3333 of unsampled matrix cases left unjudged, none
  unused"; `TT_MATRIX_CASES=all TTC_REQUIRE_TSGO=1 cargo test --test
  case_baselines`: passes, 1,084 s; `TT_MATRIX_CASES=all TTC_REQUIRE_TSGO=1
  TT_REQUIRE_EXTENSION=1 cargo test --test editor_cases`: passes, 1,119 s;
  the extension suite (`npm test` in `editors/vscode`): 238 passed, 0
  failed, 0 skipped; `./scripts/ci agents`: passed, with the environmental
  warnings TASK-639 recorded (rolldown not on PATH, doctor reports the
  checkout not ready). No `tsgo` or `ttc --server` process was left
  running.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/program_syntax/completion.rs`,
`src/engine/language/scope.rs`,
`tests/cases/editor/completionModuleLevelArm.tt` and `.ts` and its
baseline, `tests/baselines/reference/editor/completionOwnInitializer.baseline`,
18 matrix difference baselines, `tests/editor-matrix-differences.txt`,
`docs/design/lsp-architecture.md`, the TASK-687 record, `docs/tasks/INDEX.md`,
and this record.
