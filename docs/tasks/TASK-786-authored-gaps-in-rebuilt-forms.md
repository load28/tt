# TASK-786: Keep authored comments inside the forms a lowering rebuilds

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-786: Keep authored comments inside the forms a lowering rebuilds`

## Purpose

TASK-781 decision K2 left one finding of the sixth audit open as a
structural proposal, and the user approved it: a comment or a TypeScript
directive written between the operands of a form the lowering rebuilds
(a completed call, an optional call, `?:`, `&&`, `||`, `??`, a logical
assignment, a pipeline step) was dropped, so
`g(\n// @ts-expect-error\n"x",\nmatch ...)` reported TS2345, and of
`/*C1*/`…`/*C13*/` around such operands only six survived.

## Scope

- Included: every form the lowering rebuilds from slot names, and a
  comment written after a match arm's `=>`.
- Excluded: an operand moved into a capture keeps only the comments
  written inside it (decision 3).

## Decisions

### Decision 1: A rebuilt form writes the trivia authored between its operands

- **Context**: The rebuilt forms are assembled from generated separators
  (`, `, ` = `, ` && `, `(`, `)`), not copied from the source, so the text
  between the operands is lost. TypeScript applies a directive to the next
  line only (`Program.getDiagnosticsWithPrecedingDirectives` in tsgo's
  `compiler/program.go`), so a directive keeps its meaning only when the
  operand it was written above stays on the line after it.
- **Alternatives**: (a) Copy each form's whole authored text with its
  operands replaced. That rewrites the separator of every rebuilt form in
  the baselines (`g(a,b)` would keep `,` without a space) for no gain.
  (b) Keep the generated separators and, where the source between two
  operands holds a comment, write the trivia runs of that source (its
  comments and the line breaks around them, the operator tokens left out)
  in place of the separator's spacing.
- **Decision and rationale**: (b). The program syntax records the source
  between a call's callee (or type arguments), each argument and the
  call's end, and between a conditional operation's pieces
  (`CallCompletionFacts::gaps`, `ConditionalFacts::gaps`); target planning
  and the emitter write a gap through one helper (`AuthoredText`,
  `push_gap` in `src/codegen/core/authored.rs`). A form without comments is
  written exactly as before, so no existing baseline changes.

### Decision 2: A comment after an arm's `=>` precedes the arm's value

- **Context**: A comment between `=>` and an arm's value was written after
  the lowered arm with the comments of its pattern, so a directive there
  governed the next case label (TS2578 and the arm's error both reported).
- **Decision and rationale**: The part of an arm's head after its `=>` is
  written before the statement that delivers the value (inside a guard's
  `if`, and before the value in the expression forms), each comment on its
  own line; comments in the pattern keep their place after the arm.

### Decision 3: A captured operand keeps only its own comments

- **Context**: An operand that must run before a later lowered value is
  captured (`const $tt_v2 = (h("s"));`) and the rebuilt form reads the
  capture. A directive above that operand governs one source line, whose
  text now lands on two output lines: an error inside the operand is
  reported at the capture, and an error about its place (an argument's
  assignability) at the rebuilt form. A directive copied to both lines
  reports TS2578 on the one without an error.
- **Decision and rationale**: The directive stays with the rebuilt form,
  where the operand's place is, and `docs/ai/tt.md` says that it does not
  reach an error inside a captured operand. Applying directives to source
  lines inside ttc would make the typed check disagree with `tsc` on the
  emitted file (TASK-773 decision 17).

## Work log

- 2026-10-07: Added `gaps` to `CallCompletionFacts` and `ConditionalFacts`
  (`src/program_syntax{.rs,/protocol.rs}`) and to
  `PlannedConditionalOperation` (`src/evaluation_ir{.rs,/planning.rs}`).
- 2026-10-07: Wrote the gaps in the completed call's invoke text
  (`src/codegen/core/planning{.rs,/rewrites.rs}`), the optional call,
  ternary, logical and logical-assignment forms
  (`src/codegen/core/emitter/host.rs`), and both pipeline forms
  (`src/codegen/core/emitter/expression.rs`); the invoke text and its
  closing are `AuthoredText`, so they keep their source mapping.
- 2026-10-07: Moved the comments after an arm's `=>` before its value
  (`src/codegen/core/emitter/{mod,pattern}.rs`). `gap_comments` now finds a
  gap's comments by binary search instead of scanning every comment of
  the file for each gap.
- 2026-10-07: Added the two cases below and documented the behaviour in
  `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: A directive above a pipeline step did not reach an annotated input

- **Symptom**: in the first version of the case, `// @ts-expect-error`
  above `|> h` (with `h(n: number)` after `|> String`) was reported unused
  while the mismatch stayed.
- **Cause**: a statement-form pipeline annotates the value flowing into a
  step with that step's parameter type, so the mismatch is reported on the
  previous step's line; the directive is correctly written above the step.
- **Resolution**: none needed in the compiler; the case uses an error the
  step's own call reports (an arity error).

### Issue 2: One native test lost its TypeScript backend under load

- **Symptom**: the first full run failed
  `a_configured_installed_mapper_changes_no_answer_where_extension_probing_already_resolved`
  with "Unexpected EOF while reading from child process", and `cargo test`
  stopped at that binary.
- **Cause**: a release build ran beside the suite on a nearly full disk;
  the test does not touch the code this task changes and passes alone.
- **Resolution**: reran the whole suite with `--no-fail-fast` and nothing
  else running; every test passed.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aDirectiveInsideARebuiltFormGovernsTheLineItWasWrittenAbove.tt`
- **Observed failure**: on `b510ea88`, eight errors: TS2345 at lines 12,
  16 and 21, TS2362 at 25 and 28, TS2554 at 32, and TS2578 with TS2339 at
  35–36; no error with the fix.
- **Path**: `tests/cases/compiler/commentsInsideARebuiltFormStayWhereTheyWereWritten.tt`
- **Observed failure**: on `b510ea88` the output lost `/*C1*/`, `/*C3*/`,
  `/*C4*/`, `/*C7*/`, `/*C8*/`, `/*C10*/` and `/*C13*/`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change (only the two new cases)
- [x] Callgrind on the benchmark module: 12.92M instructions, 12.90M on `b510ea88`

## Result

Complete. Comments and directives between the operands of every rebuilt form,
and after an arm's `=>`, stay where they were written; both cases fail on
`b510ea88`, and the full gate passes.
