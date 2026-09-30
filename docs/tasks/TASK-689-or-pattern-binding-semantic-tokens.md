# TASK-689: Classify every alternative's binding of an or-pattern as a declaration in semantic tokens

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-689`

## Purpose

TASK-686 Issue 3: in `Num(value) | Neg(value)`, the binding written in the
first alternative was classified `variable []`, where TypeScript classifies
the binding its twin declares (`const { value } = t0`) as `variable
[declaration, local, readonly]`. Every alternative's binding is a
declaration of the one name the arm binds, so each must carry the
modifiers TypeScript gives that declaration.

## Scope

- Included: the engine's translation of TypeScript's semantic tokens onto
  the source (`src/engine/language/service.rs`), a hand-written editor case,
  and the 68 generated cases and six list lines that recorded the defect.
- Excluded: definition and rename at an or-pattern binding, which answer
  every alternative by design (`docs/ai/tt.md`, "match") and stay listed.

## Decisions

### Decision 1: A token on a shared binding is each alternative's token

- **Context**: The alternatives of an or-pattern bind one name; the
  emission declares it once (`const { value } = $tt_m;` in the arm's
  `case "Num": case "Neg":` block) and records that declaration and the
  source occurrences it stands for as a `SharedBinding`. The engine
  translated a service token to the source through the byte mappings only,
  and a mapping reaches at most one occurrence, so TypeScript's
  classification of the declaration landed on one alternative at most (none
  in the hand-written case below, the second in the generated `match`
  cases) and the rest kept tt's own `variable` token with no modifiers.
- **Alternatives considered**: (a) have tt's own token classifier
  (`src/engine/tokens.rs`) add `declaration`, `readonly`, and `local` to a
  pattern binding. It would restate TypeScript's classification rule
  (`readonly` for a `const`, `local` for a non-module scope) outside the
  service, and it would be wrong wherever the emitted declaration differs
  (a let-else binding is declared with the statement's own `let`, which is
  not `readonly`). (b) Emit one declaration per alternative. The
  alternatives are one binding at run time and in the checker; two
  declarations would be two symbols, and hover, references, and rename of
  one name would split.
- **Decision and rationale**: (c) the translation of a service token asks
  the emission's record first: a token whose served span is a shared
  binding's declaration is placed at every occurrence the binding stands
  for, with TypeScript's type and modifiers (LSP 3.17
  `SemanticTokenModifiers`: `declaration` "for declarations of symbols",
  `readonly`, and TypeScript's `local`). The layer that already maps
  hover, definition, and rename of a shared binding to its occurrences
  (`to_service_name`, `map_shared_target`) now maps its classification the
  same way; TypeScript still decides every modifier.

## Work log

- 2026-09-30: Confirmed the cause on `match (t) { Num(value) | Neg(value)
  => value, _ => 0 }`: the emitted `const { value } = $tt_m;` is one
  declaration, and the source token list had `variable` without modifiers
  at the alternatives it did not map to.
- 2026-09-30: `source_tokens` places a token through `token_sources`,
  which returns every occurrence of the shared binding whose served span
  the token covers, or the mapped source span otherwise.
- 2026-09-30: Added `tests/cases/editor/orPatternSemanticTokens.tt`
  (shorthand and aliased bindings in a module-level `match`, a `match` in a
  function, an `if let`, and a `let` let-else); removed the six
  `TASK-686 Issue 3` lines from `tests/editor-matrix-differences.txt`;
  regenerated the difference baselines with
  `UPDATE_EXPECT=1 TT_MATRIX_CASES=all TT_CASES=orPattern cargo test --test
  editor_cases` and read the diff: 68 baselines each lose exactly the
  semantic-token section (67 `[declaration, local, readonly]`, one
  module-level `[declaration, readonly]`), nothing else changes, and no
  baseline was added or deleted.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/orPatternSemanticTokens.tt` (and the six
  removed `TASK-686 Issue 3` lines of `tests/editor-matrix-differences.txt`,
  whose 68 cases then agree with their twins)
- **Observed failure**: with `src/engine/language/service.rs` reverted,
  `TT_CASES=orPatternSemanticTokens cargo test --test editor_cases` failed
  with a modified baseline: every or-pattern binding in the case (`3:36
  "value"`, `3:49 "value"`, `5:42 "n"`, `5:58 "n"`, `6:14 "value"`, `6:27
  "value"`, `9:18 "v"`, `9:34 "v"`) was `variable` with no modifiers instead
  of `variable [declaration, readonly, local]` (`[declaration, local]` for
  the `let` let-else).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TT_MATRIX_CASES=all TT_CASES=orPattern TT_REQUIRE_EXTENSION=1
  TTC_REQUIRE_TSGO=1 cargo test --test editor_cases`: passes (every
  `ifLet_orPattern_*`, `letElse_orPattern_*`, `match_orPattern_*` case).
- [x] The default editor suite (`cargo test --test editor_cases`, the
  hand-written cases and the sampled matrix) passes.
- [x] Baseline changes reviewed and committed with the change.
- The full gate is recorded in TASK-690's record, which ran it once for
  TASK-687 to TASK-690.

## Result

Changed files: `src/engine/language/service.rs`,
`tests/cases/editor/orPatternSemanticTokens.tt` and its baseline, 68
difference baselines under `tests/baselines/reference/editor/matrix/`,
`tests/editor-matrix-differences.txt`, `docs/tasks/INDEX.md`, and this
record.
