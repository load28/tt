# TASK-697: Keep a directive on the JSDoc line it governs when the JSDoc moves to its declaration

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-697`

## Purpose

TASK-665 left open the case where a JSDoc comment and a `// @ts-expect-error`
or `// @ts-ignore` directive both precede a statement whose lowering writes
generated statements. Two orders exist, and each was handled wrongly or not
at all:

- Directive above the JSDoc: the JSDoc moved down to the declaration and
  the directive stayed above the generated statements, so it governed the
  first generated line instead of the JSDoc line it governed in the source.
  `// @ts-ignore` then hid an error TypeScript reports on the same text
  written as `.ts`, and `// @ts-expect-error` hid an error and a TS2578.
- JSDoc above a directive that governs the statement: the JSDoc stayed
  above the generated storage, so declaration emit and hover lost it.

## Scope

- Included: `leading_documentation` and `governed_line` in
  `src/lexer/comments.rs` with unit tests, the case
  `tests/cases/compiler/jsdocAndDirectiveBeforeLoweredStatement.tt`,
  `docs/ai/tt.md`, and the TASK-665 note.
- Excluded: comments between match arms, which the emitter drops (found
  here, Issue 2, suggested as a separate task).

## Sources

- microsoft/TypeScript at `5739027c` (the pinned `typescript` package),
  `tsc/internal/scanner/scanner.go`: `processCommentDirective` records a
  `//` or block comment directive, and the program applies it to the next
  line that is neither blank nor a `//` comment (TASK-665 Sources,
  `markPrecedingCommentDirectiveLine`). A `/** */` line therefore stops the
  search: a directive above a JSDoc line governs that line.
- Same file, `iterateCommentRanges`: for leading comments `collecting`
  starts false (unless at position 0) and becomes true only at a line break
  outside a comment, so a comment on the same line after a previous token
  is not a leading comment of the next node.
- `tsc/internal/parser/utilities.go`, `GetJSDocCommentRanges`: a
  statement's JSDoc is the `/**` comments among
  `GetLeadingCommentRanges(text, node.Pos())`.

## Decisions

### Decision 1: A directive that governs a line of the relocated range moves with it

- **Context**: The relocated range runs from the first JSDoc of the run of
  comments directly above the statement to the statement. A directive in
  that run but above the JSDoc governs a line inside the range.
- **Alternatives considered**: leave the JSDoc in place when a directive
  governs its line (the declaration loses its documentation, which the
  source gives it); move only the JSDoc (the directive changes the line it
  governs).
- **Decision and rationale**: The range starts at the earliest directive of
  the run whose governed line lies in the range, so the directive and the
  line it governs keep their relation and the JSDoc still leads the
  declaration. `governed_line` is factored out of
  `directive_governed_lines` so both use one definition of the governed
  line. Declaration emit of the output keeps `/** The documented value. */`
  for the case's `ungoverned`, and the errors equal those of a hand-written
  `.ts` twin (`// @ts-ignore` / `/** d */` / an erroneous declaration
  reports TS2322; a `// @ts-expect-error` there reports TS2578).

### Decision 2: A JSDoc above a directive that governs a lowered statement stays where it is

- **Context**: The directive governs the statement's line, so TASK-665
  keeps the statement's generated statements and its declaration on that
  one output line. The JSDoc would have to lead the declaration.
- **Alternatives considered**:
  - Write the JSDoc on the governed line, directly before the declaration
    (`...glue... /** doc */ export const governed = $tt_v0;`). Tried: the
    output's declaration emit drops the comment, because a comment after
    another token on the same line is not a leading comment
    (`iterateCommentRanges`).
  - Put a line break before the JSDoc: the declaration then starts on a line
    the directive does not govern, and an error TypeScript reports at the
    declaration (the name of `const z: number = try r;`) escapes it.
  - Move the directive down with the JSDoc: the generated statements before
    it, where most of the statement's errors land, escape it (TASK-665
    Decision 1, alternative (a)).
  - Repeat the directive: a second `@ts-expect-error` with no error on its
    line reports TS2578.
- **Decision and rationale**: Under TypeScript's two rules a declaration
  preceded by generated statements on one governed line cannot also be led
  by a JSDoc, so the directive keeps priority (it decides which errors are
  reported) and the JSDoc stays above the generated statements, as before.
  The case pins this (`governed`), and `docs/ai/tt.md` says why.

## Work log

- 2026-10-01: Read TASK-665, `leading_documentation`, the emitter's
  relocation and the printer's governed-line rule.
- 2026-10-01: Implemented the inline JSDoc of Decision 2's first
  alternative (a `Documentation { span, inline }` result, an attached span
  on `GovernedStatement`); `tsc --declaration` on the output dropped the
  comment, and `iterateCommentRanges` explains why. Reverted it.
- 2026-10-01: Implemented Decision 1, added unit tests and the case; the
  first version of the case put errors only on later generated lines and
  passed without the fix (Issue 1).

## Issues and resolutions

### Issue 1: The first regression case passed without the fix

- **Symptom**: With the fix reverted, the case's `.errors.txt` was
  unchanged; only `.ts` and `.map.txt` differed.
- **Cause**: The misplaced directive governed `let $tt_v0: number;`, a line
  with no error, so the misplacement was not observable through errors.
- **Resolution**: The case adds statement-form `try` lowerings, whose first
  generated line (`const $tt_t0 = r("x");`) carries the error.

### Issue 2: A comment between match arms is dropped

- **Symptom**: A `// @ts-expect-error` on its own line between two arms of
  an expression `match` is missing from the output.
- **Cause**: Not investigated; independent of this task.
- **Resolution**: Suggested as a separate task.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/jsdocAndDirectiveBeforeLoweredStatement.tt`;
  `lexer::comments::tests::a_directive_governing_the_jsdoc_line_moves_with_it`.
- **Observed failure**: Without the change in `src/lexer/comments.rs` the
  case's `--check-types` section has only the `ungoverned` TS2322: the
  `@ts-ignore` hides `Argument of type 'string' ...` at 16:19, and the
  `@ts-expect-error` hides the error at 22:19 and its TS2578; `.ts` and
  `.map.txt` show the directives above the generated statements.

## Verification

- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] `cargo test --test case_baselines --test compile --test snapshot
  --test emit_map --test cli_outputs --test passthrough --lib`: passed, no
  other baseline changed. The full gate is recorded in TASK-699.
- [x] `tsc --declaration` on the emitted case keeps the JSDoc of the moved
  declarations.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/lexer/comments.rs`, the case and its baselines,
`docs/ai/tt.md`, the TASK-665 note, `docs/tasks/INDEX.md`, and this record.
