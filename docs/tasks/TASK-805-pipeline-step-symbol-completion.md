# TASK-805: Record the pipeline step completion TypeScript's `Symbol` entry differs in

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-805: Record the pipeline step completion TypeScript's Symbol entry differs in`

## Purpose

The nightly editor matrix failed on 72 cases
(`pipeline_assertionThenStep_*`, `pipeline_chain_*`,
`pipeline_overloadedStep_*`): at `|> ./*member*/length` the twin's
completion offers `Symbol` and tt's does not. The same cases fail on
0776bbb8.

## Scope

- Included: `docs/ai/tt.md`, `tests/editor-matrix-differences.txt`, and the
  72 matrix baselines.
- Excluded: the completion code.

## Decisions

### Decision 1: The difference is by design

- **Context**: The pinned tsgo answers member completion on a string with
  an entry `Symbol` (kind Variable) whose text edit replaces the `.` with
  `[Symbol]` (`filterText` `.Symbol`), checked with `tsgo --lsp` on
  `(s as string).length`. `ts_completions` drops an entry whose replaced
  range starts at a pipeline step's `.` and whose text does not start with
  `.` or `?.` (`pipe_step_dot`): a step starting `.` is a postfix chain, and
  `x |> [Symbol]` is not one.
- **Alternatives considered**: Offering the entry would edit the step into
  invalid syntax.
- **Decision and rationale**: Documented under "Install" in
  `docs/ai/tt.md` and listed as `by-design` for the three case families;
  the other pipeline families with a `member` question agree with their
  twins, so the lines do not use a wider pattern.

## Work log

- 2026-10-14: Grouped the 144 modified baselines of the full matrix run:
  72 were TASK-803's; the other 72 the `Symbol` entry. Wrote the doc and
  list lines; `UPDATE_EXPECT=1 TT_MATRIX_CASES=all TT_CASES=<family>` for
  the three families; the baseline diffs add only the `ts only: Symbol
  (Variable)` question.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: no behaviour changes; the matrix baselines pin it.

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=<family> cargo test --release --test editor_cases` for the three families
- [x] Baseline changes reviewed and committed with the change

## Result

`docs/ai/tt.md`, `tests/editor-matrix-differences.txt`, 72 baselines under
`tests/baselines/reference/editor/matrix/pipeline/`.
