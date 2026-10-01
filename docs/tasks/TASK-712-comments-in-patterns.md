# TASK-712: Keep the comments written in patterns and in a construct's head

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-712`

## Purpose

TASK-702 (Issue 1) left comments inside a pattern, and between a pattern
and its guard or `=>`, out of the output:
`match (s) { Circle(/* the radius */ r) /* after */ => r, Point => 0 }`
emits neither comment. Source the compiler does not interpret must reach
the output (AGENTS.md, contract 1), and the source-preservation check must
hold it there.

## Scope

- Included: the arm's own source in HIR (`hir::SiteArm::head`) and Core
  (`DecisionArm::head`) for match arms, tuple arms, `if let`, and
  let-else; the first arm's gap now starting at the scrutinee's `)`
  (`arm_gaps`); writing the comments in every lowering
  (`src/codegen/core/emitter/pattern.rs`: the `switch`, the conditional
  chain, the inline conditional expression, the selected arm values;
  `expression.rs`: `if let` and let-else); the source-preservation ranges
  (`src/codegen/core/mod.rs`); two compiler cases; `docs/ai/tt.md`; the
  public API baseline.
- Excluded: comments between an `if let`'s then-block and its `else`
  (Issue 2).

## Sources

- AGENTS.md, contract 1, and TASK-702 (Decision 1): the text between arms
  is part of the match's syntax tree, and the comments in it are relocated
  ranges of the source-preservation check, so a lowering path that forgets
  them fails the build.
- TypeScript `src/compiler/program.ts`, `markPrecedingCommentDirectiveLine`
  (as cited by TASK-702): a directive governs the next line that is
  neither blank nor a `//` comment, so text written between a directive and
  the arm it governs must not be a line of its own other than a `//`
  comment.

## Decisions

### Decision 1: An arm's head is part of the syntax tree

- **Context**: A pattern, its guard's `if`, and the `=>` are construct text
  the lowering rewrites into tests and a destructuring; no node owned the
  comments in that text, so no lowering wrote them.
- **Alternatives considered**: (a) Find the comments from the pattern's
  span in the emitter: the guard's condition is copied with its own
  comments and must be left out, and an `if let` or a let-else has text
  around its pattern (`if`, `let`, `=`, and the text before its block)
  that a pattern span does not cover. (b) Extend the gap of the next arm
  to cover this arm's head: the head is the arm's own text and an `if let`
  or a let-else has no next arm.
- **Decision and rationale**: HIR records, for each arm, `head`: the
  ranges of its own source outside its guard's condition and its body
  (from the pattern to the body; for an `if let` and a let-else, from the
  keyword to the scrutinee and from the scrutinee to the block). Core
  carries it on `DecisionArm`. The first arm's gap now starts at the
  scrutinee's `)`, so a comment between `match (s)` and `{` is written
  with the comments before the first arm. Every comment in those ranges is
  a relocated range of the source-preservation check.

### Decision 2: The comments follow the arm's lowering

- **Context**: The pattern has no counterpart in the output that would
  keep a comment's position, so the comments have to be relocated, and the
  place must not change what a `// @ts-expect-error` or `// @ts-ignore`
  above the arm governs (TASK-702 Decision 2).
- **Alternatives considered**: (a) Before the arm, after the gap's
  comments: a block comment there becomes a line between a directive in
  the gap and the arm, which ends TypeScript's directive search, so the
  directive would no longer govern the arm. (b) Inside the arm, before its
  bindings: an arm on a governed line keeps its glue on one output line,
  where a `//` comment would turn the rest of the arm into a comment.
  (c) After the arm's lowering.
- **Decision and rationale**: (c). Each comment is written, in source
  order, on a line of its own after the arm: after a `case` block's or an
  `if` block's `}`, after the arm's value in a conditional expression,
  after an `if let`'s then-block (before its `else`), and after a
  let-else's bindings. An arm on a governed line keeps its glue on the
  directive's line and the comments follow on that line, which a `//`
  comment can end. The one effect left is recorded as Issue 1.

## Work log

- 2026-10-01: Reproduced with every lowering: a `switch`, a guarded
  conditional chain, a nested pattern, a literal or-pattern, a tuple arm,
  `if let` (chained, with a `//` comment in the pattern), a let-else, and
  comments around `if`, `let`, `=`, and between `match (s)` and `{`.
- 2026-10-01: Added `head` (HIR, Core), `push_head_comments` and
  `head_comments`, the calls in the six lowerings, and the preservation
  ranges. Checked that dropping one call fails the build
  (`validate_source_preservation ... SourceOmitted`).
- 2026-10-01: Added `tests/cases/compiler/commentsInPatternsAreKept.tt`
  (`@run`) and `directiveAboveArmWithPatternComments.tt`; updated
  `docs/ai/tt.md` and the public API baseline.

## Issues and resolutions

### Issue 1: A directive written inside a pattern governs what follows the arm

- **Symptom**: A `// @ts-expect-error` written on a line of its own inside
  a pattern is now kept, after the arm, where it governs the next line.
- **Cause**: Decision 2: the place that keeps a directive above the arm
  governing it cannot also place one from inside the pattern above the
  arm's value.
- **Resolution**: Recorded in `docs/ai/tt.md` (write the directive above
  the arm). Before this task such a directive was dropped.

### Issue 2: Comments between an `if let`'s then-block and its `else`

- **Symptom**: `if let A(x) = s { ... } /* c */ else { ... }` still drops
  `/* c */`.
- **Cause**: That text belongs to the `else` continuation, which is
  outside the pattern site's arm.
- **Resolution**: Out of scope; noted for a follow-up.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/commentsInPatternsAreKept.tt` and
  `directiveAboveArmWithPatternComments.tt` (`cargo test --test
  case_baselines`).
- **Observed failure**: Without the change in `src/`, both cases failed
  their baselines: the first case's `.ts` and `.map.txt` lacked all 22 of
  the comments in its patterns and construct heads (`/* the radius */`,
  `// no area`, `/* before the guard */`, `/* deep */`, `/* by kind */`,
  `/* a */`, `/* k */`, ...), and the second case's `.ts` lacked
  `/* the radius */ /* read */` and `/* the side */`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib` (412 passed); `cargo test --test public_api`
  (the new `SiteArm::head` field accepted);
  `TTC_REQUIRE_TSGO=1 TT_CASES=omment cargo test --test case_baselines`
  (the `matchComments` matrix, TASK-702's cases, and these two): passed.
- [x] The full gate for TASK-706 to TASK-712 is recorded in the batch's
  final report.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/hir/{mod,lower}.rs`, `src/core_ir/{mod,lower}.rs`,
`src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,pattern,expression}.rs`,
`docs/ai/tt.md`, the public API baseline, and the two compiler cases with
their baselines. Every comment in a pattern, between a pattern and its
`=>`, between `match (...)` and its `{`, and in an `if let` or a let-else
before its block reaches the output and is held there by the
source-preservation check.
