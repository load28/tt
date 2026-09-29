# TASK-554: Keep a member step's simple key where the member is read

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A computed member step passed its key to the generated function as a
parameter, which widened a literal key.
`export const c = (flow |> obj["m"])(3);` emitted
`(($tt_r, $tt_k) => ($tt_r[$tt_k]).bind($tt_r))((obj), ("m"))`, where
`$tt_k` is `string`, so `tsc` reported TS7053; `h(3) |> (() => obj)()["m"]`
did the same, and `h(3) |> getFns()[0]` on a tuple widened `0` to `number`
(TS2349). The same program without pipelines type-checks.

## Scope

- Included: which computed keys a member step captures
  (`source_member_callee`, `src/program_syntax.rs`) and a regression test.
- Excluded: a key that must be captured and has a literal type only by
  inference (`obj[cond ? "m" : "n"]`); an arrow parameter widens it too.
  Keeping its type needs a typed capture the expression-form step does not
  have.

## Decisions

### Decision 1: A simple-copiable key is read where the member is read

- **Context**: The step `x |> receiver[key]` evaluates the piped value,
  then the receiver, then the key, and calls the member with the piped
  value (`docs/ai/tt.md`). The emitter passes each captured part as an
  argument of an arrow, and TypeScript types an immediately invoked arrow's
  parameter from its argument with the literal widened.
- **Alternatives considered**: (a) Annotate the parameter with the key's
  type. That needs the checker's type in codegen, or a type assertion.
  (b) A generic parameter (`<K extends PropertyKey>`), a type trick.
- **Decision and rationale**: As TypeScript's own down-level transforms
  copy a simple-copiable operand (`isSimpleCopiableExpression`: a string,
  template, or numeric literal, a keyword, an identifier) instead of
  capturing it, and as TASK-522 reads such a part of an assignment target
  again, a member step keeps such a key in its authored place. The body
  reads it right after the receiver argument, which is the order
  `receiver[key]` reads it in, and it has the type TypeScript gives it
  there: `$tt_r["m"]`, `$tt_r[0]`.

## Work log

- 2026-09-29: Reproduced with `ttc` and `tsc --strict` (TS7053, TS2349).
- 2026-09-29: Added `simple_copiable` and used it for member and `super`
  keys in `source_member_callee` (`src/program_syntax.rs`).
- 2026-09-29: Added `runtime_a_member_step_s_simple_key_names_its_member`
  (`tests/integration/cases_05.rs`), which also checks that an identifier
  key is read after its receiver runs.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] The new test fails without the change (TS7053, TS2349).

## Result

Changed `src/program_syntax.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, and this record.
