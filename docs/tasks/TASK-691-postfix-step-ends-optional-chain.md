# TASK-691: End the optional chain of a pipeline head before a postfix step

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-691`

## Purpose

`box?.v |> .toFixed(1)` emitted `box?.v.toFixed(1)`: the postfix step
joined the head's optional chain, so it was skipped where
`(box?.v).toFixed(1)` throws, and its type gained `| undefined`
(TASK-685 Issue 3). `docs/ai/tt.md` documents a postfix step as the
postfix chain on the piped value, the value the head evaluates to.

## Scope

- Included: the receiver question codegen asks before a postfix step
  (`src/lexer/queries.rs`, `src/codegen/core/emitter/helpers.rs`,
  `src/codegen/core/emitter/expression.rs`); the matrix's optional-chain
  companion, which now short-circuits (`tests/matrix/harness.ts`,
  `scripts/generate-cases`), and the `booleanLiterals` twin that this
  exposed (`tests/matrix/match.mjs`); a case file;
  `tests/editor-matrix-differences.txt`; `docs/ai/tt.md`;
  `docs/design/compiler-architecture.md`.
- Excluded: a call step's callee (`x |> o?.m` stays the optional call
  `o?.m(x)`, as documented); `flow` postfix steps, which already apply to
  an arrow parameter.

## Decisions

### Decision 1: A postfix receiver must be a primary expression that does not end in an optional chain

- **Context**: ECMA-262 §13.3.9 (`OptionalExpression :
  MemberExpression OptionalChain`, `OptionalChain : ?. IdentifierName`,
  `OptionalChain . IdentifierName`, ...) makes every member access,
  index, or call written after `?.` part of the same chain, and the whole
  chain evaluates to `undefined` when the `?.` base is nullish
  (§13.3.9.1). A `ParenthesizedExpression` is a `PrimaryExpression`
  (§13.2), so `(a?.b).c` reads `c` of the chain's value. TypeScript models
  the same thing (`a?.b!.c` continues the chain, `(a?.b).c` ends it;
  TypeScript 3.9 release notes, "Parsing Differences Around Optional
  Chaining and Non-Null Assertions"). `push_receiver` asked only
  `is_primary_expression`, which accepts `a?.b` because member access
  binds as tightly as the chain.
- **Alternatives considered**: (a) Always parenthesize a postfix
  receiver: correct, but writes `(s).trim()` for every primary receiver,
  noise the receiver question exists to avoid. (b) Look for `?.` in the head's
  source text: a `?.` inside brackets or a template does not continue the
  chain, so the question must be asked over tokens at the top level.
  (c) Decide from the HIR head expression: the receiver is emitted text
  (a lowered head can also be glue), and the existing receiver question is
  already asked of that text over the lexer's tokens.
- **Decision and rationale**: A new token query, `is_member_receiver`,
  shares `is_primary_expression`'s scan and also reports whether a
  top-level `?.` occurs; `push_receiver` parenthesizes unless the receiver
  is primary and closed. The callee of a call step keeps the old question
  under a new name (`push_callee`), because there joining the chain is the
  documented optional call. The same rule covers every structural shape of
  the problem: an optional-chain head, a head ending in `!`
  (`box?.v! |> .toFixed(1)`), and a preceding optional postfix step
  (`s |> ?.trim() |> .length`, now `(s?.trim()).length`, matching "an
  optional step short-circuits only its own tail").

### Decision 2: The optional-chain companion short-circuits on every other call

- **Context**: The matrix's `optionalChain` companion wrote
  `box(input)?.inner.value!`, but `box` always returned a box, so no case
  saw the chain short-circuit and the defect was invisible to the runtime
  oracle.
- **Alternatives considered**: (a) A new companion whose chain always
  short-circuits: the all-pairs selection in `scripts/generate-cases`
  would reassign companions across most of the matrix, and a chain that
  never continues tests only half of the companion. (b) Short-circuit on a
  particular input value: forms have different inputs and types, so no
  single value works for all of them.
- **Decision and rationale**: `box` alternates, like `flip` and `risky`
  already do, so each case sees the chain continue on one call and
  short-circuit on the next; the companion's title says so. Every case of
  the companion stays where it was and is regenerated; its twin is written
  with the same operand, so the oracle is unchanged in kind.

### Decision 3: The `booleanLiterals` twin throws for a value outside its patterns

- **Context**: With Decision 2, `match_booleanLiterals_*_optionalChain`
  receives `undefined` typed `boolean`. tt throws
  `tt match: unexpected literal undefined` (documented in `docs/ai/tt.md`,
  "Exhaustiveness": a `_`-less literal match gets a runtime `throw` guard),
  while the twin's `t === true ? ... : note("no", 0)` treated every
  non-`true` value as `false`.
- **Alternatives considered**: Listing the three cases in
  `tests/oracle-failures.txt` as defects: the tt behaviour is the
  documented one, so it would record the oracle's mistake as the
  compiler's.
- **Decision and rationale**: The twin tests `false` too and calls a new
  harness `unexpected(value)` that throws the documented error, so it is
  written from the documented semantics for every value.

## Work log

- 2026-09-30: Reset onto `claude/ecstatic-dijkstra-qw5pf9` (6b39c6b),
  `npm ci` in the root and `editors/vscode`, `npm run compile` there,
  `scripts/fetch-typescript-cases`.
- 2026-09-30: Reproduced with `ttc -p`: `box?.inner.value |> .toFixed(1)`
  gave `box?.inner.value.toFixed(1)`, `s |> ?.trim() |> .length` gave
  `s?.trim().length`, `box?.inner |> .value |> String` gave
  `$tt_ap(box?.inner.value, String)`.
- 2026-09-30: Added `is_member_receiver` and `push_callee`; the three
  forms now emit `(box?.inner.value).toFixed(1)`, `(s?.trim()).length`,
  and `$tt_ap((box?.inner).value, String)`.
- 2026-09-30: Added `tests/cases/compiler/postfixStepAfterOptionalChain.tt`
  (with a TypeScript twin) and the lexer unit test
  `a_member_receiver_ends_any_optional_chain`.
- 2026-09-30: Made `box` alternate, retitled the companion, regenerated
  (`node scripts/generate-cases`: 429 cases, 252 editor files); the
  full optional-chain matrix then showed the three `booleanLiterals`
  twins (Decision 3); fixed the twin and regenerated (42 cases, 48 editor
  files). Removed the three TASK-685 Issue 3 lines from
  `tests/editor-matrix-differences.txt`.
- 2026-09-30: `UPDATE_EXPECT=1 TT_MATRIX_CASES=all TT_CASES=_optionalChain`
  and `TT_CASES=match_booleanLiterals` for `case_baselines` and
  `editor_cases` (with `TT_REQUIRE_EXTENSION=1`); `cargo test --test
  compile --test snapshot --test case_baselines`.

## Issues and resolutions

### Issue 1: The `booleanLiterals` twin disagreed once the chain short-circuited

- **Symptom**: `match_booleanLiterals_{awaitOperand,forInitializer,spreadElement}_optionalChain`:
  `prints what its twin does not`, the twin `0 | no 0` against tt's
  `threw tt match: unexpected literal undefined`.
- **Cause**: the twin was not written from the documented semantics for a
  value outside the patterns (Decision 3).
- **Resolution**: Decision 3.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/postfixStepAfterOptionalChain.tt`
  (`cargo test --test case_baselines`); also the matrix cases
  `pipeline_postfixStep_*_optionalChain` (`TT_MATRIX_CASES=all`).
- **Observed failure**: without the changes under `src/`, the case does
  not compile cleanly: `error[ts2322]: type mismatch: expected string,
  found undefined` at `box(present)?.inner.value! |> .toFixed(1)` (tsc:
  `Type 'string | undefined' is not assignable to type 'string'`), and its
  `.stdout` and `.types` baselines differ (`value: number | undefined`).
  With the regenerated matrix, 10 `pipeline_postfixStep_*_optionalChain`
  cases fail with `prints what its twin does not`, for instance
  `callArgument(7) => [1,null,3]` against the twin's
  `threw Cannot read properties of undefined (reading 'toFixed')`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (targeted: `compile`, `snapshot`, `case_baselines`,
  `lexer::queries`); the full gate is recorded in TASK-692, which ends
  this batch.
- [x] `TT_MATRIX_CASES=all` `case_baselines` and `editor_cases` for the
  optional-chain companion and the `booleanLiterals` form.
- [x] Baseline changes reviewed and committed with the change: the
  optional-chain matrix `.stdout` baselines show the second input
  short-circuiting identically in tt and its twin; the three
  `pipeline_postfixStep_*_optionalChain` editor baselines are gone (they
  agree with their twins now).

## Result

Changed `src/lexer/queries.rs`, `src/lexer.rs`,
`src/codegen/core/emitter/helpers.rs`,
`src/codegen/core/emitter/expression.rs`, `tests/matrix/harness.ts`,
`tests/matrix/match.mjs`, `scripts/generate-cases`, the regenerated
matrix cases and baselines, `tests/editor-matrix-differences.txt`,
`docs/ai/tt.md`, `docs/design/compiler-architecture.md`,
`docs/tasks/TASK-685-editor-matrix-infrastructure.md` (Issue 3 points
here), `docs/tasks/INDEX.md`; added
`tests/cases/compiler/postfixStepAfterOptionalChain.tt` and its baselines.
