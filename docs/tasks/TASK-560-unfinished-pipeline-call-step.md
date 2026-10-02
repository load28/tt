# TASK-560: Read a pipeline step with an open list as TypeScript reads the list

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-560: Read a pipeline step with an open list as TypeScript reads the list`

## Purpose

Signature help in an unfinished call step at the end of a file answered the
runtime helper or nothing: `const v = 1 |> String |> cat("x", ` answered
`$tt_ap(v: string, f: …)`, and `const v = 1 |> add(2, ` answered `null`.
Inside a function body, or with the `)` written, `cat(a, b)` and
`add(a, b)` with parameter 1 active were answered.

## Scope

- Included: Where a pipeline step whose call or index is still open ends
  (`parse_pipeline`).
- Excluded: The emission of steps and the service's probe, which already
  answer a cursor the projection has no place for (TASK-528).

## Decisions

### Decision 1: A step that leaves a bracket open follows TASK-528's operand rule

- **Context**: A step ended at its last token. The whitespace after
  `cat("x",` was copied after the pipeline's glue, so the cursor after
  `, ` (read on its preceding side, TASK-538) landed after the glue's `)`,
  in the `$tt_ap(` argument list or outside every call. The step's scan
  counted only depth, so with a `(` open it also took the next statement
  (`const w = 1;`) and the enclosing function's `}` in.
- **Alternatives considered**: Map a cursor after unmapped whitespace to
  the end of the chunk before it: the whitespace after a complete step is
  outside the step, and a cursor there asks about the pipeline, not about
  the step's call.
- **Decision and rationale**: The scan keeps the closers it waits for, as
  `scan_primary_operand` does for a `try` operand (TASK-528, Decision 2): a
  closer of another bracket, or a statement keyword other than `try`
  directly in a parenthesis or index, ends the step, as it ends
  TypeScript's unterminated list; and a step with a bracket still open runs
  to where the enclosing syntax resumes (the next token, or the region's
  end), whitespace included. The cursor after `, ` is then inside the step;
  the step's emission trims that whitespace, so the service answers
  through a probe at the cursor, inside the call.

## Work log

- 2026-09-29: Reproduced through `ttc --server` with the two reported
  buffers, the `)`-closed form, the function-body form, and a following
  `const w = 1;`: the served text was `$tt_ap(String(1), (cat("x",)) ` with
  the cursor after the last space, and `(add(2, \nconst w = 1;)(1)`.
- 2026-09-29: Changed the step scan (`src/parser/pipes.rs`) and passed the
  region end to it (`src/parser/parse.rs`). All five buffers answer the
  called function with parameter 1 active; `const w = 1;` and the
  function's `}` stay outside the step.
- 2026-09-29: Tests: `an_unfinished_pipeline_call_step_answers_signature_help`
  (`tests/native/cases_10.rs`) and
  `a_step_with_an_open_list_ends_where_typescript_ends_the_list`
  (`tests/compile/cases_05.rs`); both fail without the change (`$tt_ap`
  answered; the statement inside the step).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed.

## Result

Signature help and completion in a call step being typed answer the called
function, at the end of a file as inside a function. Changed
`src/parser/pipes.rs`, `src/parser/parse.rs`, `tests/native/cases_10.rs`,
and `tests/compile/cases_05.rs`.
