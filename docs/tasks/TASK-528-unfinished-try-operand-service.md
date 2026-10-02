# TASK-528: Answer completion and signature help inside an unfinished tt value

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-528: Answer completion and signature help inside an unfinished tt value`

## Purpose

Signature help and completion answered nothing while the user typed the
operand of a value `try`, a `try` inside a `result` block, or the body of a
match's last arm (`const n = try parse("1", |` → no signature, no items),
although `const n = parse("1", |` answers both.

## Scope

- Included: How a value `try` is emitted when there is no owner model, how
  the parser bounds an operand whose call or index is still open, how the
  service reaches a cursor the projection did not copy, and the completion
  resolve edits of such a probe.
- Excluded: The typed batch path and the lowering of files whose TypeScript
  parses; placement-refused values in a file with an owner model, which keep
  their `(undefined as any)` stand-in (TASK-527).

## Decisions

### Decision 1: Emit an ownerless value `try` around its operand, not instead of it

- **Context**: Mid-edit the file's TypeScript does not parse, so the plan
  has no owner model and TASK-416 emitted every value `try` as a
  placeholder, dropping the operand: `const n = undefined`. The call being
  typed never reached TypeScript.
- **Alternatives considered**: (a) Emit the operand alone, dropping `try`:
  the binding would have the Result's type, not its payload's, and every
  use of it would report a false type error. (b) `((operand) as any)`: keeps
  the text, but every binding from a `try` in a file with a syntax error
  anywhere turns `any` and hides real errors. (c) Put the operand in a
  closure: TASK-416 rejected closures around user code, which change
  `this`, `arguments`, `await`, and parameter scope.
- **Decision and rationale**: The operand is emitted where it stands, mapped,
  as the argument of glue that reads its success payload under the same
  Result ABI a statement `try` tests (`result_failure_test`,
  `layout.payload_field`):
  `(($tt_result) => { if (!("value" in $tt_result)) throw $tt_result; return $tt_result.value; })(operand)`.
  The operand is evaluated outside the arrow, so its scope is the
  enclosing function's; TypeScript types the arrow's parameter from its
  argument, so the value has the payload's type, and a non-Result operand is
  reported on the glue through the `try` anchor as the ordinary lowering's
  is. Only the `Err` edge is lost, as it is in every recovery.

### Decision 2: Read an unterminated operand the way TypeScript reads it

- **Context**: `scan_primary_operand` counted brackets by depth whatever
  their kind. In a function body `try parse("1", ` took the function's `}`
  as the call's closer, so the operand swallowed the brace; in a `result`
  block, whose body is its own region, the call was cut off and the operand
  was just `parse`, leaving `("1", ` to call the `try`'s value.
- **Alternatives considered**: Leave the extent and ask through probes only:
  the result-block operand would still be `parse`, and signature help would
  ask about the glue's call.
- **Decision and rationale**: The scan keeps a stack of the closers it
  waits for. A closer of another bracket, or a statement keyword directly
  inside a parenthesis or an index — except `try`, which tt reads as a
  value there — ends the operand, as it ends TypeScript's unterminated list;
  a bracket still open then extends the operand to where the enclosing
  syntax resumes (the next token, or the region's end), trailing whitespace
  included, so the cursor after `, ` is inside it. Balanced text never
  reaches either rule; the value-region nesting matrix pinned the `try`
  exception when `try (try load())` stopped parsing without it.

### Decision 3: A cursor the served text has no place for is asked through a probe

- **Context**: A match arm's body is copied without the whitespace after
  it, so the cursor in `A(n) => parse("x", |` had no position in the served
  text. `Project::completion` used the probe only for member completions,
  `signature_help` not at all, and `build_probe` used only a verified
  emission, which a buffer whose TypeScript does not parse never has.
- **Alternatives considered**: Map the cursor to the nearest copied byte (a
  guess that lands after the glue's closing parenthesis, outside the call).
- **Decision and rationale**: When `to_service` has no position for the
  cursor, completion and signature help splice `$tt_probe` there and ask at
  its mapped position; completion keeps its member rule. `build_probe`
  compiles the spliced text into the projection the service would serve
  (`emit`, or TASK-527's `withheld`). The probe's auto-import edits already
  map back through it (TASK-526); a regression now pins that.

## Work log

- 2026-09-29: Reproduced with the five reported cursors: `emit_mapped`
  showed `const n = undefined`, `const q = undefined("1",` in a `result`
  block, and `$tt_recovery = (parse("x",);` for the arm.
- 2026-09-29: Emitted the operand around its glue
  (`src/codegen/core/emitter/source.rs`); the function case then showed
  `(parse("1", \n})`, the operand holding the function's `}`. Replaced the
  depth count in `scan_primary_operand` (`src/parser/tries.rs`) with a
  closer stack and the unterminated-list rule.
- 2026-09-29: Probes for unmapped cursors in `Project::completion` and
  `Project::signature_help`; `build_probe` over `withheld`.
- 2026-09-29: Tests: `an_unfinished_tt_value_answers_signature_help_and_completion`
  and `an_auto_import_resolves_through_a_probe` (`tests/native/cases_08.rs`),
  `a_value_try_without_an_owner_keeps_its_operand_mapped`
  (`tests/emit_map.rs`, including the function's brace and a following
  statement staying outside), and "the operand of an unfinished try gets
  signature help and completion" (`engine.test.ts`). The TASK-416 placeholder
  test now covers the placement-refused values it still applies to.
- 2026-09-29: The full Rust run failed the value-region matrix
  (`try (try load())`, Issue 1); fixed and rerun. One extension run timed
  out in "filesystem and config changes refresh ttx -> tt" (60 s, with the
  machine loaded; it passed alone in 12 s and in the next full run).
- 2026-09-29: Corrected TASK-526's forward reference to this task.

## Issues and resolutions

### Issue 1: A nested value `try` stopped the operand

- **Symptom**: `try (try load())` reported `source-not-typescript`.
- **Cause**: `try` is on TypeScript's statement-keyword list, and the new
  stop rule ended the operand at it inside the parenthesis.
- **Resolution**: The stop rule excepts the keyword tt reads as a value in
  an expression (Decision 2).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed.
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`: 224 passed.

## Result

Signature help and completion answer inside the operand of a value `try`,
inside a `result` block, and in a match's last arm while the buffer does not
parse, with the binding typed as the Result's payload. Changed
`src/codegen/core/emitter/source.rs`, `src/parser/tries.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`tests/native/cases_08.rs`, `tests/emit_map.rs`,
`editors/vscode/server/src/test/engine.test.ts`,
`docs/design/lsp-architecture.md`, and the TASK-526 record.
