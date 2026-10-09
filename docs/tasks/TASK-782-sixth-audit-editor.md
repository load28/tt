# TASK-782: Fix the sixth audit's editor findings

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-782: Fix the sixth audit's editor findings`

## Purpose

The sixth audit of the editor and `--server` session (a read-only agent
against a release build of `d8da7233`) found four defects: an auto-import
completion without its import edit, a pipeline-step member completion whose
edit is not tt, position parameters read without their types, and check time
quadratic in the length of a pipeline. Fix each in the layer that owns it.

## Scope

- Included: the four findings (E1–E4 of this round).
- Excluded: the command-line and compiler findings of the same round
  (TASK-780, TASK-781).

## Decisions

### Decision E1: An insertion at the start of the served text is an insertion at the start of the source

- **Context**: With a `variant` or a top-level `match` as the file's first
  statement, accepting an auto-import completion wrote the name and no
  import (the typed check then reported TS2304). TypeScript inserts the
  import at 0:0 of the served projection, which is the glue the lowering
  wrote for that statement, so no mapping reached it and the edit was
  dropped. A file that starts with a comment, a blank line or an import
  worked, because offset 0 is copied source there.
- **Decision and rationale**: Offset 0 of the projection and offset 0 of the
  source are both the start of the module, so a zero-width edit there maps
  to source offset 0. Any other edit that falls in glue is still refused.

### Decision E2: A pipeline step's member completion offers only edits a step can hold

- **Context**: In `o |> .`, TypeScript offers `"a-b"` with the edit
  `["a-b"]` over the `.`, which turns the step into `o |> ["a-b"]` (a call
  of an array). A step that starts with `.` must start with a name
  (`docs/ai/tt.md`), and `.["a-b"]` is not a step, so the property cannot
  be written in that step at all; a `?.` step can (`?.["a-b"]`).
- **Decision and rationale**: An item whose edit replaces the `.` that
  starts a pipeline step (the lexed `.` right after `|>`) with text that
  does not start with `.` or `?.` is left out.

### Decision E3: The server refuses a position that is not two non-negative integers

- **Context**: `position` was read with `as_u64().unwrap_or(0) as u32`, so
  a string, a negative or fractional number, or a value of 2^32 or more
  answered as 0:0 or as a wrapped line. TASK-769 decision 5 and TASK-770
  decision 8 already refuse a wrongly typed `text`, `filename`, `scope` or
  `member`.
- **Decision and rationale**: An absent `position` (or an absent `line` or
  `character`) still means 0, as the requests without one rely on; any
  other value that is not an integer from 0 to 2^32 − 1 is refused with the
  method and the field named.

### Decision E4: A pipeline's steps read the shape of the value flowing in, not its text

- **Context**: `check` of `x |> f |> ... f` took 4.7 s at 3200 steps. Each
  step asked whether the accumulated value needs parentheses
  (`needs_grouping`, `push_receiver`), which resolved and lexed the whole
  text built so far. The nested-match half of the finding is output size:
  each level of block structure indents its lines, as `docs/ai/tt.md` says
  the lowering does, so the emitted bytes grow quadratically with depth
  while the work per byte stays constant (800 levels: 9,605 lines,
  23 MB).
- **Decision and rationale**: Every step writes the value flowing in inside
  brackets or at the head of a member chain, so the result's top-level
  structure is known from the step alone (`f(x)`, `$tt_ap(x)`, `(x)(x)`,
  `x.body`). Each step records that shape with the value replaced by a
  placeholder, and the next step asks the same questions of the shape;
  they cannot see inside the brackets, so the answers are the same. 3200
  steps now take 0.06 s.

## Work log

- 2026-10-07: Ran the sixth audit's editor agent and reproduced the four
  findings with its harness (`h.js`, `p18.js`, `p20.js`, `p3.js`).
- 2026-10-07: Fixed E1 (`src/engine/language/service/targets.rs`), with the
  editor case `autoImportBeforeALoweredFirstStatement`.
- 2026-10-07: Fixed E2 (`src/engine/language/service{.rs,/targets.rs}`),
  with the editor case `pipeStepMemberCompletionOffersOnlyExpressibleEdits`.
- 2026-10-07: Fixed E3 (`src/server.rs`), with
  `server_refuses_a_position_that_is_not_two_non_negative_integers` in
  `tests/cli/server_print.rs`.
- 2026-10-07: Fixed E4 (`src/codegen/core/emitter/{expression,helpers}.rs`),
  measured with callgrind, with a byte counter on the top-level queries
  (`src/lexer/queries.rs`) and
  `compiling_does_linear_work_in_the_steps_of_a_pipeline`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/autoImportBeforeALoweredFirstStatement.tt` (E1)
- **Observed failure**: on `d8da7233`, `resolve kkValue from "./shapes.tt": 0 edit(s)`.
- **Path**: `tests/cases/editor/pipeStepMemberCompletionOffersOnlyExpressibleEdits.tt` (E2)
- **Observed failure**: on `d8da7233`, the `/*dot*/` editor completion listed
  `a-b` as well as `cc`.
- **Path**: `tests/cli/server_print.rs::server_refuses_a_position_that_is_not_two_non_negative_integers` (E3)
- **Observed failure**: on `d8da7233`, `ttSymbol` with `"line": "0"` answered
  `{"result": null}` and `ttCompletions` with `"line": -1` answered the
  0:0 completions instead of an error.
- **Path**: `src/lib/scaling_tests.rs::compiling_does_linear_work_in_the_steps_of_a_pipeline` (E4)
- **Observed failure**: on `d8da7233` with only the byte counter added,
  "top-level query bytes: 1425034 units for n matches but 5730034 for 2n".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Complete. The four editor findings are fixed with regression tests (E4's nested-match half is output size, recorded in decision E4), and the full gate passes.
