# TASK-794: Fix the round-nine compiler findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-794: Fix the round-nine compiler findings`

## Purpose

The ninth audit found nine compiler findings (K1–K9) on `c48ad20e`. A
nested-pipeline measurement found one more: compile work grew
quadratically with the nesting depth of parenthesized pipeline heads.
This task fixes the findings that contradict the documented behaviour.
Three findings (K4, K8, K9) are left for a decision, because each needs a
change to a documented design.

## Scope

- Included:
  - K1: pipeline comments and directives.
  - K2: a statement `try` with a comment before its `;`.
  - K3: the JSDoc of an exported let-else.
  - K5: `try` operands that start with `new`, `function` or `class`.
  - K6: comments after tt keywords and after a let-else block.
  - K7: the false exhaustiveness error after a missing arm comma.
  - Nested pipeline heads.
- Excluded: K4, K8 and K9 (Decisions 8–10).

## Decisions

### Decision 1: Every pipeline form keeps the comments between its operands (K1)

- **Context**: docs/ai/tt.md says a pipeline step keeps the comments
  written between its operands. Only the `$tt_ap` and reference forms
  did; the direct call (`f(x)`), the member step, the postfix step and
  every `flow` form dropped them.
- **Decision and rationale**: Each form writes the source gap between
  the previous operand and the step (the `|>` excluded) between the
  operands it writes, with its line breaks:
  - The direct call and the member step write it before the piped value
    (`needsString(\n  // @ts-expect-error\n  1)`). TypeScript reports an
    argument error at the argument, so a directive above a step governs
    the line that holds the value.
  - A postfix step writes it between the value and the member access.
  - `flow` writes it before the step's function. The first step's gap
    starts after the `flow` keyword.

### Decision 2: A statement `try`'s operand ends at its last token (K2)

- **Context**: `parse_try_tail` required the operand's last token to be
  directly before the `;`, with no trivia between them, so a comment
  there made the statement not a `try`.
- **Decision and rationale**: The tail check compares token indices, so
  trivia between the operand and the `;` belongs to neither. The comment
  is copied with the operand.

### Decision 3: A let-else's documentation is keyed by its whole statement (K3)

- **Context**: The JSDoc relocation looked for documentation above the
  statement's head span, which starts at `const`. For an exported
  let-else, `export` sits between the JSDoc and `const`, so nothing was
  found.
- **Decision and rationale**: Both the relocation table and the emitter
  use the statement's owner extent, which starts at `export`.

### Decision 4: A `try` operand is any member expression (K5)

- **Context**: ECMAScript's MemberExpression includes `new C(args)`,
  `new.target`, function expressions and class expressions, each with a
  postfix chain. The operand scanner accepted only primary expressions
  that start with an identifier, literal or bracket.
- **Decision and rationale**: The lexer gains `is_operand_expression`.
  It accepts a primary expression, or one of those keyword heads,
  followed by the same postfix chain. `new X` without arguments is
  accepted only at the end of the operand, as in the grammar.

### Decision 5: Comments after tt keywords are copied (K6)

- **Context**: The lowering rebuilt the text between a tt keyword and the
  next operand, and dropped the comments in it.
- **Decision and rationale**:
  - `try`: the gap between `try` and its operand is written after
    `const $tt_tN =`.
  - `match`: the gap between `match` and the first subject is written
    after `const $tt_m =`.
  - `result`: the gap is written where the lowered block opens, after
    the label of a labeled block. The expression-boundary form writes
    each comment on its own line inside the arrow body.
  - `variant`: HIR records the header span (after `variant` to the `{`),
    and the type alias keeps the gaps before and after the name.
  - let-else: the site's `trailing` span covers the source between the
    else block and the `;`. Its comments are written after the bindings,
    as a match's trailing comments are.

### Decision 6: A parse error inside a match hides that match's coverage (K7)

- **Context**: When TypeScript does not parse inside a match's body, the
  arms ttc read may not be the arms written. The coverage error and its
  edit then name a case the user wrote.
- **Decision and rationale**: When a `source-not-typescript` error lies
  inside a match's body, the match's `match-not-exhaustive` error is not
  reported. Other constructs keep their diagnostics, as docs/ai/tt.md
  line 11 says.

### Decision 7: A parenthesized pipeline head is written once (nested pipelines)

- **Context**: A head that is a parenthesized source group was guarded
  for line comments and re-scanned at every level, so each nesting level
  re-read the levels inside it.
- **Decision and rationale**: The emitter receives the file's tokens. A
  head whose emitted text is exactly one source parenthesis pair is a
  closed member receiver and needs no guard. The pair is checked with the
  tokens' matching-close distances, in constant time.

### Decision 8: K4 waits for a decision

- **Context**: A computed property key or a template substitution
  captured before a lowered match is converted (`ToPropertyKey`,
  `ToString`) after the match runs. ECMA-262 converts it first.
- **Alternatives considered**:
  - Capturing the converted value changes the static type of the key or
    substitution (`string` instead of the written type). That changes
    the object's type and a template's contextual typing, which contract
    2 forbids.
  - A helper that converts and keeps the type would need a type
    assertion, which contract 2 also forbids.
  - TypeScript's own generator transform spills such operands without
    converting them, so it has the same order.
- **Decision and rationale**: No change preserves both the type and the
  conversion order, so the choice is left to the maintainer.

### Decision 9: K8 waits for a decision

- **Context**: `variant O { Alpha, Beta }` and a hand-written union with
  `Alpha` and `Betta` make `Betta` a near miss of `O`. The partial
  coverage rule is documented.
- **Decision and rationale**: Changing the evidence rule changes which
  typos plain `ttc` reports, so the choice is left to the maintainer.

### Decision 10: K9 waits for a decision

- **Context**: `Project::scan` projects every `.tt` file under the root
  before TypeScript decides membership, so a file outside `include`
  costs time on every compile.
- **Decision and rationale**: Projecting only members needs the
  configuration's file list, or projecting on TypeScript's first read.
  Either changes the snapshot model, so the choice is left to the
  maintainer.

## Work log

- 2026-10-08: Reproduced K1–K9 with the audit binary (`c48ad20e`).
- 2026-10-08: Changed `src/parser/tries.rs`, `src/lexer/queries.rs`,
  `src/lexer.rs` (K2, K5); `src/lib/compile.rs` (K7);
  `src/codegen/core/emitter/{mod,result,expression,pattern,helpers}.rs`,
  `src/hir/{mod,lower}.rs`, `src/core_ir/{mod,lower}.rs` (K6, K1);
  `src/codegen/core/mod.rs` (K3); `src/codegen/core/mod.rs`,
  `src/codegen/rope/builder.rs`, `src/lib/{compile,mapped}.rs` (nested
  pipelines). Added case files and a scaling test.
- 2026-10-08: Regenerated the case baselines. The new cases are the only
  output changes; the public API baseline gains `VariantItem::header`.

## Issues and resolutions

### Issue 1: Comment gaps doubled the space after `=`

- **Symptom**: `const $tt_t1 =  r();` for a `try` with no comment.
- **Cause**: `push_gap` writes both its `before` and `after` text when
  the gap has no comment, and both carried a space.
- **Resolution**: The text before the gap ends at `=`, and the gap or the
  `after` text supplies the space.

## Regression test (fails before the fix)

Each case was run with the audit binary built from `c48ad20e`.

- **Path**: `tests/cases/compiler/pipelineStepCommentsStayBetweenTheOperands.tt`
- **Observed failure**: only `// keep me 6` was in the output; the
  `@ts-expect-error` line was dropped.
- **Path**: `tests/cases/compiler/aStatementTryKeepsTheCommentBeforeItsSemicolon.tt`
- **Observed failure**: `source-not-typescript`, "Expected '{', got
  'ident'" at 4:7.
- **Path**: `tests/cases/compiler/anExportedLetElseKeepsItsDocumentationOnTheBinding.tt`
- **Observed failure**: `export const { n: z }` had no JSDoc above it.
- **Path**: `tests/cases/compiler/aTryOperandMayStartWithNewFunctionOrClass.tt`
- **Observed failure**: `source-not-typescript`, "Expression expected".
- **Path**: `tests/cases/compiler/commentsAfterTtKeywordsAreKept.tt`
- **Observed failure**: none of the keyword comments was in the output.
- **Path**: `tests/cases/compiler/aMissingArmCommaIsOnlyAParseError.tt`
- **Observed failure**: `match-not-exhaustive: missing "B"` before the
  parse error.
- **Path**: `src/lib/scaling_tests.rs::compiling_does_linear_work_in_nested_pipeline_heads`
- **Observed failure**: top-level query bytes went from 1288 at depth 8 to
  5392 at depth 16.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change

## Result

K1, K2, K3, K5, K6, K7 and the nested-pipeline cost are fixed and pinned
by case files and a scaling test. K4, K8 and K9 wait for a decision
(Decisions 8–10).
