# TASK-665: Keep a comment directive's line and a statement's JSDoc where TypeScript reads them

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-665`

## Purpose

A `// @ts-expect-error` or `// @ts-ignore` above a statement whose lowering
writes generated statements stopped reaching the errors of that statement:
the output put the directive above `let $tt_v0: number;` and the rest of the
lowering on later lines, so `--check-types` reported TS2578 ("Unused
'@ts-expect-error' directive") and the error the directive was written for.
A JSDoc comment above such a statement documented the generated storage
instead of the declaration, so declaration emit and hover lost it.

## Scope

- Included: reading comments and the lines directives govern
  (`src/lexer/comments.rs`, `src/lexer.rs`), their plumbing into emission
  (`src/lib/compile.rs`, `src/lib/mapped.rs`, `src/codegen/core/mod.rs`),
  the printer's layout of a governed statement (`src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`, `src/codegen/rope/tests.rs`), the storage
  annotation written on such a line (`src/codegen/contextual.rs`,
  `src/codegen/mod.rs`), JSDoc relocation in the emitter
  (`src/codegen/core/emitter/{mod,source,result,expression}.rs`),
  `docs/ai/tt.md`, and a runtime case.
- Excluded: `export const Pat(...) = x else {...};`, which already fails
  with `lowering-plan-failed` before this change (Issue 1).

## Sources

- TypeScript `src/compiler/scanner.ts`: `commentDirectiveRegExSingleLine =
  /^\/\/\/?\s*@(ts-expect-error|ts-ignore)/` for a `//` comment and
  `commentDirectiveRegExMultiLine = /^(?:\/|\*)*\s*@(ts-expect-error|ts-ignore)/`
  for the last line of a `/* */` comment.
- TypeScript `src/compiler/program.ts`, `markPrecedingCommentDirectiveLine`:
  for a diagnostic on line N, lines N-1, N-2, ... are searched while they
  are blank or `//` comments; the first one holding a directive suppresses
  the diagnostic. A directive therefore governs one line: the first after
  it that is neither blank nor a `//` comment. `getDiagnosticsWithPrecedingDirectives`
  reports an `@ts-expect-error` that suppressed nothing as TS2578.
- TypeScript `src/compiler/utilities.ts`, `getJSDocCommentRanges`: a node's
  JSDoc is the `/**` comments among its leading comment ranges, which is
  what declaration emit copies and quick info shows.

## Decisions

### Decision 1: Generated glue adds no line break to a statement that starts on a governed line

- **Context**: A directive governs one source line, but a lowering spreads
  that line over many output lines (storage, the dispatch block, the
  owner), and the errors land on those lines. The directive is TypeScript's
  and is applied by TypeScript to the output, including under the content
  mapper, so the output layout has to put the governed code where the
  directive reaches it.
- **Alternatives considered**: (a) Move the directive down to the owner
  statement, after the generated statements. Errors reported inside the
  lowering (an arm value against annotated storage, a `try` operand) stay
  out of reach, and a directive that reached `const $tt_t0 = f(bad)` before
  would stop reaching it. (b) Filter diagnostics on the tt side by source
  line. That is diagnostic suppression outside the layer responsible, and
  `tsc --runExternalCode` would still apply the directive to the output.
  (c) Repeat the directive before each output line. `@ts-expect-error`
  would then report TS2578 for every line without an error. (d) Keep the
  line structure the source has: for a lowered statement starting on a
  governed line, a generated break or a generated newline between two
  pieces of that statement's source is printed as a space while source
  from the governed line is still to come; the source's own line breaks
  are kept.
- **Decision and rationale**: (d). It is a layout rule of the printer
  (`TargetFile::print`) fed with the governed statements, so every lowering
  path (owner preludes, statement-form `try`, let-else, result blocks,
  match statements, variants) follows it, source mappings are computed on
  the printed text as before, and a statement on an ungoverned line is
  printed exactly as before (no existing baseline changed). A storage
  annotation the typed refinement writes later is printed on one line when
  its declaration is on such a line.

### Decision 2: A JSDoc comment is written after the generated statements, directly above the declaration

- **Context**: TypeScript attaches a JSDoc comment to the node it leads.
- **Alternatives considered**: (a) Move every leading comment. Plain
  comments have no attachment and would churn every output for no reader.
  (b) Move the comments from the first JSDoc comment among those on the
  lines directly above the statement, unless a directive governs the
  statement (Decision 1 keeps those comments where they are).
- **Decision and rationale**: (b). The emitter skips that range where it
  was and writes it right before the statement's own text: after the
  owner's preludes, before a `try` statement's binding, before a let-else's
  destructuring. The range is recorded as relocated for the target's
  source-preservation check, which still requires it exactly once.

## Work log

- 2026-09-30: Reproduced with the probe case: five TS2578 and seven TS2322
  under `--check-types`, and `/** The documented value. */` above
  `let $tt_v3: number;`.
- 2026-09-30: Added `lexer::comments`, `directive_governed_lines`, and
  `leading_documentation` with unit tests.
- 2026-09-30: Flattened glue by source line first; a multi-line `match`
  under a directive then still put the owner statement after the arms'
  lines, so the rule became per statement (start on a governed line, glue
  flattened while source from that line is still to come).
- 2026-09-30: Added the JSDoc relocation and the single-line annotation in
  the typed refinement; added
  `tests/cases/compiler/tsDirectiveBeforeLoweredStatement.tt` with `@run`;
  updated `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: An exported let-else cannot be lowered

- **Symptom**: `export const A(n: v) = S.A(3) as S else { throw ...; };`
  reports `lowering-plan-failed` ("Expected ',', got ';'") on this change's
  base as well.
- **Cause**: Not investigated here; it is independent of comments.
- **Resolution**: The case uses a module-level `const` let-else for the
  JSDoc check; the exported form is left for a follow-up task.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/tsDirectiveBeforeLoweredStatement.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: An unexpected `.errors.txt` with five
  `error[ts2578]: Unused '@ts-expect-error' directive.` and seven
  `error[ts2322]` diagnostics under `--check-types`, so the case was not
  run, and the `.ts` baseline differed (the JSDoc above the generated
  storage).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the whole case runner (only this case's baselines are
  new), `tests/compile.rs`, `tests/snapshot.rs`, `tests/emit_map.rs`,
  `tests/cli_outputs.rs`, and `cargo test --lib lexer::comments` here; the
  full gate over TASK-661 to TASK-666 is recorded in TASK-666
- [x] Baseline changes reviewed and committed with the change

## Result

A directive above a lowered statement reaches its errors and is used, and
a JSDoc comment stays on its declaration; statements on other lines print
as before. Changed: `src/lexer/comments.rs`, `src/lexer.rs`,
`src/lib/compile.rs`, `src/lib/mapped.rs`, `src/codegen/mod.rs`,
`src/codegen/contextual.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/codegen/rope/tests.rs`,
`src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`,
`src/codegen/core/emitter/source.rs`, `src/codegen/core/emitter/result.rs`,
`src/codegen/core/emitter/expression.rs`, `docs/ai/tt.md`, the case and its
baselines.
