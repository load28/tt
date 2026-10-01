# TASK-699: Read TypeScript's keyword filter from the served syntax, not from its answer

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-699`

## Purpose

TASK-688 Decision 2 restates a completion answer inside a module-level
construct's arm with a function body's keywords only when TypeScript's
answer contains the keyword `namespace`, taking that as proof that
TypeScript applied `KeywordCompletionFiltersAll`. That is a decision on the
shape of an answer (AGENTS.md contract 3): an answer without `namespace`
for any other reason (a narrowed list, a client that filters by prefix, a
future TypeScript that ranks it away) would keep the 18 module-level
keywords. The decision has to come from the syntax TypeScript reads.

## Scope

- Included: `src/engine/language/keyword_filter.rs` (new), its use in
  `restate_completions` (`src/engine/language/scope.rs`), unit tests in
  `src/engine/language/tests.rs`, `docs/design/lsp-architecture.md`, the
  TASK-688 note, and the full gate over TASK-695 to TASK-699 on the tree
  merged with `claude/ecstatic-dijkstra-qw5pf9` (TASK-691 to TASK-694).
- Excluded: the container walk and the keyword list, which TASK-688
  already derives from the syntax and from `getTypescriptKeywordCompletions`.

## Sources

microsoft/TypeScript at `5739027c` (the pinned `typescript`
7.1.0-dev.20260826.1), `tsc/internal/ls/completions.go`:

- `getGlobalCompletions` sets `KeywordCompletionFiltersFunctionLikeBodyKeywords`
  when `tryGetFunctionLikeBodyCompletionContainer(contextToken)` finds a
  function-like body (stopping at a class) and `KeywordCompletionFiltersAll`
  otherwise, then replaces either with `KeywordCompletionFiltersTypeAssertionKeywords`
  or `KeywordCompletionFiltersTypeKeywords` when `isTypeOnlyLocation`.
- `isTypeOnlyLocation` (line 790): `!isContextTokenValueLocation(contextToken)
  && (isPossiblyTypeArgumentPosition(...) || ast.IsPartOfTypeNode(location)
  || isContextTokenTypeLocation(contextToken))`, besides JSDoc type
  expressions and type-only imports.
- `isContextTokenValueLocation`, `isContextTokenTypeLocation`, and
  `isPossiblyTypeArgumentPosition` (lines 3331-3367), and
  `getPossibleTypeArgumentsInfo` (`tsc/internal/ls/utilities.go`, line 109),
  which confirms a candidate with the checker's generic signatures.
- `tryGetClassLikeCompletionSymbols` runs before `getGlobalCompletions` and
  answers a class, interface, or type literal member position with its own
  filter.
- `ast.IsPartOfTypeNode` (`tsc/internal/ast/utilities.go`, line 2028):
  type nodes, minus the expression of a `typeof` query, plus interface
  heritage and `implements` clauses.

## Decisions

### Decision 1: Decide "TypeScript chose `All`" from the served syntax at the served position

- **Context**: The restatement turns `All` into `FunctionLikeBodyKeywords`,
  so it applies exactly when TypeScript reached `getGlobalCompletions`,
  found no function-like container (already decided by the TASK-688 walk:
  `!contained`), and did not take a type filter. Any other path offers no
  keywords (`KeywordCompletionFiltersNone`: member, JSX, object literal
  completions), where removing keywords removes nothing, or a filter that
  shares keywords with the 18 (type keywords, class and interface element
  keywords), where the answer must stand.
- **Alternatives considered**:
  - Keep reading the answer (`namespace` present): contract 3, as above.
  - Lower module-level arms into a function in the served text so
    TypeScript's own walk finds a body: TASK-688 rejected it because the
    checker's view of the program would change.
  - Ask TypeScript a second question in a probe text: a second service
    round trip for every completion inside a construct, to learn one fact.
  - Port the conditions above onto the served text's syntax (swc's AST of
    the served text, which `decided_in_source` already parses, and ttc's
    lexer for the context token).
- **Decision and rationale**: The last. `offers_all_keywords` finds
  TypeScript's `contextToken` (the token before the position, or before the
  word being typed) and `location` (the word touching the position, else
  the next token) and answers false in a comment or literal, at an
  interface or type literal member position (inside an interface body or a
  type literal), and at a type-only location: `typeof`/`asserts` context
  tokens are value locations; `IsPartOfTypeNode(location)` is the innermost
  of the type spans (every `TsType`, interface heritage, `implements`) and
  value spans (a `typeof` query's expression); `isContextTokenTypeLocation`
  is a `:` that starts a type annotation, `=` of a type alias or type
  parameter default, `as` of an `as` expression or `as const`,
  `satisfies`, `<` of a type reference's arguments or a type assertion,
  and `extends` of a type parameter. `restate_completions` uses this fact
  instead of the answer's contents.

### Decision 2: A possible type argument position is type-only

- **Context**: `isPossiblyTypeArgumentPosition` confirms the token scan of
  `getPossibleTypeArgumentsInfo` with the checker (the called expression
  has generic signatures), which the syntax cannot answer.
- **Decision and rationale**: The scan is ported; where it finds a
  candidate (`x < |y` after an identifier), the position counts as
  type-only and the answer stands. The cost is that TypeScript's 18
  module-level keywords stay in an answer at `a < |b` inside a module-level
  arm when `a` is not generic; the alternative, assuming a value position,
  would remove `number` and the other type keywords from a real type
  argument list.

## Work log

- 2026-10-01: Read `getGlobalCompletions`, `isTypeOnlyLocation` and its
  helpers, `getPossibleTypeArgumentsInfo`, `IsPartOfTypeNode`, and the
  scanner's keyword table at the pinned commit.
- 2026-10-01: Added `keyword_filter.rs`; `decided_in_source` returns the
  fact with its two walks; removed `ALL_FILTER_KEYWORD`.
- 2026-10-01: Added the unit tests; with TASK-688's check the restatement
  test fails (below). `completionModuleLevelArm` and
  `TT_MATRIX_CASES=all TT_CASES=topLevel` editor cases pass unchanged.
- 2026-10-01: Merged `claude/ecstatic-dijkstra-qw5pf9` (7e8e350); the only
  conflict was the index rows, ordered by number. Ran the full gate on the
  merged tree.

## Issues and resolutions

### Issue 1: The first unit test asked at the wrong offset

- **Symptom**: The restatement test kept every keyword; the construct
  scope was not found.
- **Cause**: The marker `f(` first matched `declare function f(`.
- **Resolution**: The marker is `1 => f(`.

### Issue 2: The full case matrix found baselines TASK-698 changed

- **Symptom**: `TT_MATRIX_CASES=all` failed 15 `variant_requiredAfterOptional_*`
  cases: each `.errors.txt` lost `error[ts1016]: A required parameter
  cannot follow an optional parameter. (in code ttc generated for this
  construct)` at the variant name, beside the case's
  `variant-required-after-optional` error.
- **Cause**: TASK-698 gives each generated constructor parameter an anchor
  at its field. TS1016 on the parameter `b` now lands on the field `b:
  number`, which the tt diagnostic covers, so the reporter treats it as a
  consequence of that tt cause (`origin_intersects_tt_error`), as
  `docs/ai/tt.md` describes. The sampled matrix of TASK-698's runs did not
  include these cases.
- **Resolution**: Accepted; the tt error remains the one report of the
  mistake. Recorded in TASK-698 as well; committed here because TASK-698
  was already committed.

## Regression test (fails before the fix)

- **Path**: `src/engine/language/tests.rs`,
  `a_module_level_arm_takes_a_function_body_keywords_whatever_the_answer_holds`
  (with `typescript_offers_every_keyword_only_outside_type_and_member_positions`
  for the decision itself).
- **Observed failure**: With TASK-688's `namespace` check restored, the test
  failed with `left: ["abstract", "declare", "if", "return"]` and
  `right: ["if", "return"]`: an answer without `namespace` at a value
  position in a module-level arm kept `abstract` and `declare`.

## Verification

- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  `node scripts/generate-cases --check` (3,286 generated cases and 3,706
  editor case files match).
- [x] Full gate for TASK-695 to TASK-699 on the tree merged with
  `claude/ecstatic-dijkstra-qw5pf9` (7e8e350): `RUST_TEST_THREADS=2
  TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1
  TT_BASELINE_TRACKING_DIR=<dir> cargo test --no-fail-fast`: 1,804 passed,
  0 failed, no skip; `node scripts/check-baselines --tracking <dir>`: "756
  compared, 3331 of unsampled matrix cases left unjudged, none unused";
  `TT_MATRIX_CASES=all cargo test --test case_baselines`: 15 modified
  baselines (Issue 2), accepted with `scripts/baseline-accept` and the
  family rerun clean; `TT_MATRIX_CASES=all cargo test --test editor_cases`:
  passed, 763 s; the extension suite (`node --test` over the server and
  client tests, the built `ttc` on `PATH`): 238 passed, 0 failed, 0
  skipped; `./scripts/ci agents`: passed. The comment-end refinement of
  `offers_all_keywords` landed after the unfiltered run and was checked
  with `cargo test --lib` (412 passed) and the matrix runs. No `tsgo` or
  `ttc --server` process was left running.
- [x] Baseline changes reviewed and committed with the change (Issue 2).

## Result

Changed files: `src/engine/language/keyword_filter.rs`,
`src/engine/language.rs`, `src/engine/language/scope.rs`,
`src/engine/language/tests.rs`, `docs/design/lsp-architecture.md`, the
TASK-688 note, the 15 `variant_requiredAfterOptional_*` matrix baselines
and the TASK-698 note (Issue 2), `docs/tasks/INDEX.md`, and this record.
