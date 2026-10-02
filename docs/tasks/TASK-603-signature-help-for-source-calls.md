# TASK-603: Answer signature help for the source call, never for a generated one

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-603: Answer signature help for the source call, never for a generated one`

## Purpose

Signature help showed the compiler's helpers: in `console.log(m |> ha|lf)`
it showed `$tt_ap(v: number, f: (v: number) => number): number`, in a
`flow` `$tt_fl(f, g)`, and for `4 |> obj.twice` the generated arrow
`($tt_v: number, $tt_r: {...})`, on every keystroke of a step name. The
equivalent `.ts` code `console.log(half(m))` shows `log(...data: any[])`,
and code with no call around the cursor shows nothing.

## Scope

- Included: Where the engine asks TypeScript for signature help
  (`Project::signature_help`, `signature_position` in
  `src/engine/language/service.rs`), for the served projection and for a
  completion probe alike.
- Excluded: Which calls the emission writes, hover and completion while a
  step is typed (their answers are unchanged), and TASK-604's recovered
  `try` (verified there).

## Decisions

### Decision 1: Ask TypeScript outside every generated argument list

- **Context**: `signature_help` passed TypeScript's answer through. The
  LSP 3.17 `SignatureHelp` result carries the signatures, the active
  signature, and the active parameter, but not the invocation it is about,
  so the answer cannot be mapped back or checked against the source after
  the fact. TypeScript chooses that invocation itself
  (`services/signatureHelp.ts`, `getContainingArgumentInfo`): from the token
  left of the position it walks up to the innermost call, `new`, or tagged
  template whose argument list holds the position. In the served text
  `console.log($tt_ap(m, half))`, that is the generated `$tt_ap(...)`.
- **Alternatives considered**: (a) Drop an answer whose label names a
  generated name, as completion drops generated entries (TASK-556): the
  label is display text (the generated arrow has no name, only generated
  parameters), and dropping answers nothing where TypeScript would answer
  for `console.log`. (b) Ask twice and compare the answer with the one
  asked just inside the generated list: equality of display text is not a
  statement about which invocation was chosen. (c) Record every invocation
  the emitter writes: the emission already records which bytes are copied
  (`mappings`), and an invocation is generated exactly when its `(` is not
  copied, so a second model would restate the first.
- **Decision and rationale**: The engine chooses the position before
  asking. It lexes the served text with ttc's lexer, whose grammar facts
  already say whether a token completes an operand, and finds the innermost
  argument list open before the position: a `(` (or a type-argument `<`)
  directly after a token that completes an operand or after `?.` — the
  ECMAScript `Arguments` production (§13.3), which is what TypeScript's
  walk stops at. When the emission copied that `(` from the source, the
  invocation is the user's and the position stands. When it did not, the
  position moves to that `(`: the token left of it is the callee's, so
  TypeScript's walk leaves the generated invocation and reaches the one
  around it, with the same argument index, since the generated invocation
  is inside one argument. The step repeats until the innermost open list is
  the user's or there is none, and TypeScript then answers as it answers
  for the equivalent source: `console.log(m |> half)` shows `log`,
  `Math.max(1, m |> half |> String)` shows `max`, `half(m |> half)` shows
  `half`, and a pipeline with no source call around it shows nothing.

## Work log

- 2026-09-30: Reproduced with the probe harness (`sh.tt`): all four
  markers answered `$tt_ap(...)`; the served text is
  `console.log($tt_ap(m, half), m)`.
- 2026-09-30: Checked the lexer's facts before `(` over declarations,
  control statements, arrows, methods, optional calls, and type arguments:
  only a call's `(` follows an operand-completing token.
- 2026-09-30: Added `signature_position` (`src/engine/language/service.rs`)
  and used it in `signature_help` (`src/engine/language/project.rs`).
- 2026-09-30: Re-ran the harness and the typing simulation over
  `console.log(m |> half |> String)`: every keystroke shows `log`; `flow`,
  `4 |> obj.twice`, and `m |> (x => x + 1)` show nothing.
- 2026-09-30: Tests:
  `signature_help_is_asked_outside_every_generated_argument_list`
  (`src/engine/language/tests.rs`, including a template interpolation) and
  `signature_help_answers_for_the_source_call_around_generated_calls`
  (`tests/native/editor_service.rs`), which failed before the change with
  `$tt_ap(v: number, f: (v: number) => number): number`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

## Result

Changed `src/engine/language/service.rs`, `src/engine/language/project.rs`,
`src/engine/language/tests.rs`, `tests/native/editor_service.rs`,
`docs/design/lsp-architecture.md`, and the task index. Signature help
answers for the call the user wrote around the cursor, as TypeScript does
for the equivalent source.
