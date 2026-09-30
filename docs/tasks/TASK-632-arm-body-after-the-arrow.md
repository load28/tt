# TASK-632: Complete an expression right after an arm's `=>`

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-632: Complete an expression right after an arm's =>`

## Purpose

Completion right after the `=>` of an arm whose body is not written yet
(`Rect(width) => |`) offered the patterns `_ Point Circle Rect`; with one
letter typed it offered the arm's bindings, the locals, and the globals.
TypeScript's twin, `case "Rect": return |`, offers expressions.

## Scope

- Included: The pattern-position classification tt's completion reads
  (`context` in `src/engine/completions.rs`), which the adapter's
  `ttCompletions` `pattern` flag and pattern items come from.
- Excluded: How the parser claims an arm with a missing body (TASK-605,
  unchanged), and the expression completions themselves (the service's,
  asked through the completion probe as before).

## Decisions

### Decision 1: The token before the cursor decides that a body begins, as TypeScript's context token does

- **Context**: TASK-605 claims `Rect(width) =>` with a missing body whose
  span is empty at the end of `=>`, as TypeScript's parser gives a missing
  node an empty span at the end of the previous token (`parser.ts`,
  `createMissingNode`). `arms_at` places a position in an arm only up to its
  body's end, so a position after the whitespace following `=>` fell past
  the arm and was read as a slot between arms: a pattern position. With a
  letter typed, the body is that identifier and the position is inside it.
- **Alternatives considered**: (a) Stretch the missing body's span over the
  trivia after `=>` in the parser: the span is shared by the arm's
  diagnostic span (`hir::lower`), the pattern analysis, and the control-flow
  checks, and an empty span at the missing node is TypeScript's own model.
  (b) Give `arms_at` the source so it can see that only trivia follows the
  missing body: the same fact, read in a place that has no token stream.
- **Decision and rationale**: TypeScript classifies a completion position
  by the token before it (`services/completions.ts`, `getCompletionData`:
  the `contextToken` found by `findPrecedingToken`), not by node
  containment. tt's classifier reads the same token stream: when the
  significant token before the position (past the word being typed) is
  `=>`, a body begins there, and no pattern position ever follows `=>` in
  tt's grammar (an arm starts after `{`, `,`, or a finished arm). The
  position is then not a pattern position, and the adapter asks for
  TypeScript's expression completions, which the completion probe answers
  with the arm's bindings in scope.

## Work log

- 2026-09-30: Reproduced with the probe harness (`cmp.cjs w/mab.tt - c`):
  `Rect(width) => |` answered `_ Point Circle Rect`; `width.|` and a guard
  answered TypeScript's lists.
- 2026-09-30: Added the rule to `context`. Re-ran the harness: the marker
  answers `width`, `s`, `extra`, the locals, and the globals.
- 2026-09-30: Tests: five cases in `arm_bodies_and_guards_are_expressions`
  (`src/engine/completions.rs`), including a line break after `=>` and a
  guarded arm; `the_body_of_an_arm_written_up_to_its_arrow_completes_expressions`
  (`tests/native/editor_service.rs`); and the extension test "the body of
  an arm written up to its arrow completes expressions" (`server.test.ts`).
  All three fail without the rule (the position is a pattern position, and
  the editor lists `Circle`, `Point`, `_`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib completions`, `TTC_REQUIRE_TSGO=1 cargo test --test native the_body_of_an_arm`
- [x] `node --test --test-name-pattern="arm written up to its arrow" server/out/test/server.test.js`
- [x] Full gate (recorded in TASK-633, run once for TASK-629 to TASK-633)

## Result

Changed `src/engine/completions.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/test/server.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. An
arm's body is completed as an expression from its first keystroke.
