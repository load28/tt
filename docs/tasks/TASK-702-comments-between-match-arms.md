# TASK-702: Keep the comments between match arms, and let a directive there govern its arm

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-702`

## Purpose

TASK-697 (Issue 2) found that a comment written on a line of its own
between two arms of a `match` is missing from the output, and with it a
`// @ts-expect-error` or `// @ts-ignore` placed there. That breaks the
first contract (source the compiler does not interpret is preserved) and
changes what the program means to TypeScript: the directive no longer
suppresses the error on the arm it was written above, so `--check-types`
reports an error the author silenced (and, for `@ts-expect-error`, the
same text written as TypeScript reports nothing).

## Scope

- Included: the source between arms in HIR (`hir::SiteArm::gap`,
  `hir::PatternSite::trailing`) and Core (`DecisionArm::gap`,
  `Decision::trailing`); writing the comments there in every match
  lowering (`src/codegen/core/emitter/pattern.rs`: the `switch`, the
  conditional chain, the inline conditional expression, and the selected
  arm values); the layout of an arm on a governed line
  (`MarkKind::SourcePoint`, `governed_statements`); the source-preservation
  check for those comments; two compiler cases, a case-matrix construct,
  `docs/ai/tt.md`, and the public API baseline.
- Excluded: comments inside an arm's own text but outside its body (between
  a pattern and its guard or `=>`) and inside a pattern, which are dropped
  for the same reason and are recorded under Issue 1 rather than changed
  here.

## Sources

- TypeScript `src/compiler/program.ts`, `markPrecedingCommentDirectiveLine`:
  for a diagnostic on line N, the line N-1 is asked for a directive first,
  whatever else it holds, and then lines further up while they are blank or
  `//` comments; the first directive found suppresses the diagnostic, and
  `getDiagnosticsWithPrecedingDirectives` reports an `@ts-expect-error`
  that suppressed nothing as TS2578. The pinned TypeScript 7
  (`tsc/internal/scanner/scanner.go`, `processCommentDirective`, as cited by
  TASK-697) applies a directive to the same line. So a directive governs one
  line: the next one that is neither blank nor a `//` comment, and a
  directive at the end of an arm's line governs the next arm's line too.
- TASK-665 (Decision 1): a statement that starts on a governed line keeps
  its generated glue on that one output line.

## Decisions

### Decision 1: The gaps between arms are part of the match's syntax tree

- **Context**: The emitter writes each arm from its pattern, guard, and
  body, so text between two arms has no owner and was never written.
- **Alternatives considered**: (a) Find the comments in the emitter from
  the arms' pattern spans alone: an arm's body has no span in HIR, so a
  comment between a pattern and its body could not be told apart from one
  between two arms. (b) Attach the comments to the following arm in the
  parser's AST only: the emitter reads HIR and Core, not the AST.
- **Decision and rationale**: HIR records, for each arm of a match or a
  tuple match, the source between the end of the previous arm (past a
  block body's `}`) or the body's `{` and the arm's pattern, and the source
  after the last arm; Core carries it on `DecisionArm` and `Decision`. The
  emitter writes the comments in a gap, each on its own line, before the
  arm's lowering in every match lowering, and the trailing ones after the
  last arm. The comments are added to the source-preservation check's
  pass-through ranges as relocated ranges, so a lowering path that forgets
  them fails the build (exactly once, in any order) instead of dropping
  them silently.

### Decision 2: An arm on a governed line keeps its glue on that line

- **Context**: In a `switch` or a conditional chain an arm becomes several
  output lines (`case "Square": {`, the destructuring, the delivery), and a
  directive governs only the first.
- **Alternatives considered**: (a) Move the directive down to the line that
  delivers the arm's value: an error in the guard or in a binding would
  escape it. (b) Repeat it per line: `@ts-expect-error` would report TS2578
  for each line without an error.
- **Decision and rationale**: TASK-665's printer rule, applied to the arm:
  an arm whose pattern starts on a governed line is a governed statement
  from its pattern to the next arm's gap, and a `SourcePoint` mark at the
  start of its lowering tells the printer that the glue after it is written
  for that pattern, so the case or `if` head, the destructuring, the guard,
  and the value stay on the line after the directive. The source's own line
  breaks remain, as for a statement. The inline and selected-value
  expression lowerings end a commented arm's line after its value, so a
  directive above it does not reach the next arm.

## Work log

- 2026-10-01: Reproduced with `tests/cases/compiler/commentsBetweenMatchArms.tt`
  and `directiveBetweenMatchArmsGovernsOneArm.tt` before the change (Issue
  2 of TASK-697's repro, extended to guards, block arms, tuple arms,
  literal switches, and comments at the end of an arm's line).
- 2026-10-01: Added the gaps, the emission, the mark, and the preservation
  ranges. The first run of the expression lowerings failed with
  `LayoutScopeMissing` (Issue 2).
- 2026-10-01: Added `tests/matrix/matchComments.mjs`, a construct whose
  form writes line and block comments before, between, and after the arms,
  run in every host position against its twin (367 cases).
- 2026-10-01: Merged `claude/ecstatic-dijkstra-qw5pf9` (TASK-703 to
  TASK-705). One converted case, `runtimeNestedPatternFallsThroughOnInnerMismatch`,
  has two comment lines between arms; its `.ts` and `.map.txt` baselines
  now hold them, the only existing baselines this change moved.

## Issues and resolutions

### Issue 1: Comments inside an arm's pattern are dropped too

- **Symptom**: `match (s) { Circle(/* the radius */ r) /* after */ => r, Point => 0 }`
  emits neither comment. (Comments between `variant` cases are kept.)
- **Cause**: The same as this task's: the pattern and the text up to `=>`
  are construct text the lowering rewrites, and no node owns the comments
  in it.
- **Resolution**: Out of scope here (a pattern's comments have no place in
  the generated tests that would keep their meaning); suggested as a
  follow-up.

### Issue 2: A line break in an expression lowering needs a layout scope

- **Symptom**: `validate_origin broke the contract that a generated line
  break has a layout scope (LayoutScopeMissing)` for a match inside a call
  argument.
- **Cause**: The inline and selected-value lowerings had no breaks before,
  so their ropes opened no layout scope.
- **Resolution**: They are wrapped in one when an arm gap holds a comment
  (`scoped_if_commented`); output without comments is unchanged.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/commentsBetweenMatchArms.tt` (with
  `@run`) and `tests/cases/compiler/directiveBetweenMatchArmsGovernsOneArm.tt`
  (`cargo test --test case_baselines`).
- **Observed failure**: Without the change in `src/`, the first case's
  `.errors.txt` has four errors its directives suppress (`Property 'side'
  does not exist on type 'number'` at 11:20, the TS2367 comparison at
  22:18, `Property 'missing'` at 24:31, and `Property 'edge'` at 47:43),
  so it is not run and `.stderr` says so, and its `.ts` has none of the
  six comments; the second case reports four TS2339 errors instead of the
  two on arms no directive governs.

## Verification

The full gate, on the merged tree, for TASK-700 to TASK-702:

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast`: every suite passed except `case_baselines`, whose one
  failure was the comment lines kept in the merged case above (reviewed and
  accepted with `scripts/baseline-accept`); `node scripts/check-baselines
  --tracking <dir>`: 5,327 compared, none unused.
- [x] `TTC_REQUIRE_TSGO=1 TT_MATRIX_CASES=all cargo test --test
  case_baselines`: passed in 2,192 s (the case matrix with `matchComments`,
  3,653 cases; the diagnostics matrix, 2,830; every other case), every
  listed defect observed as listed.
- [x] `TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 TT_MATRIX_CASES=all cargo
  test --test editor_cases`: passed in 834 s.
- [x] `./scripts/ci extension` and `./scripts/ci agents`: passed (rolldown
  absent, as the warning says).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/hir/{mod,lower}.rs`, `src/core_ir/{mod,lower}.rs`,
`src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,pattern}.rs`,
`src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `docs/ai/tt.md`,
`tests/matrix/matchComments.mjs` and its 367 generated cases with their
baselines, the two compiler cases with their baselines, the public API
baseline, and the two baselines of the merged case. A comment between,
before, or after match arms reaches the output in place, and a directive
there governs the arm on the line after it; comments inside a pattern
remain dropped (Issue 1).
