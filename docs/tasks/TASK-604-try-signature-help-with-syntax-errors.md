# TASK-604: Keep a `try`'s signature help the same while the file has a syntax error

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-604: Keep a try's signature help the same while the file has a syntax error`

## Purpose

While any line of the file had a syntax error, signature help after a
`try` operand (`const q = try getUser(id)|`, and the same inside a
`result { ... }` block) showed the glue's arrow,
`($tt_result: TResult<{ name: string; }, string>): { name: string; }`.
Without the syntax error it showed nothing, as TypeScript shows nothing
after `const q = getUser(id)`.

## Scope

- Included: Signature help around a `try` lowered without an owner model
  (the faithful projection of a file whose TypeScript does not parse,
  TASK-527 and TASK-528), in a statement and in a `result` block, and its
  regression test.
- Excluded: The lowering itself, and completion and hover at the same
  positions (they answer the same with and without the syntax error).

## Decisions

### Decision 1: The fix is TASK-603's; this task pins the case

- **Context**: Without an owner model, a value `try` is emitted as
  `(($tt_result) => { if (!("value" in $tt_result)) throw $tt_result;
  return $tt_result.value; })(operand)` (TASK-528,
  `src/codegen/core/emitter/source.rs`). A cursor right after the operand
  sits in that invocation's argument list, whose `(` and `)` are glue, so
  TypeScript answered for the arrow. With an owner model the operand is
  bound in a statement before the declaration, and no invocation surrounds
  the cursor.
- **Alternatives considered**: (a) Lower the recovered `try` without an
  invocation (a conditional over a temporary): it needs a statement
  position the faithful projection does not have, which is why TASK-528
  chose the invocation, and the operand must still be evaluated where it
  stands. (b) Drop answers whose parameters are generated names: the
  display-text rule TASK-603 Decision 1 rejected.
- **Decision and rationale**: The glue invocation is an argument list the
  emission did not copy, which TASK-603 asks outside of. Its `(` is
  generated, so the question moves to it, and TypeScript answers for the
  code around the `try`: nothing after `const q = try getUser(id)`, and
  `getUser`'s signature inside `getUser(|id)`, with or without a syntax
  error elsewhere. No change is needed beyond TASK-603; the test fixes
  both lowerings to the same answers.

## Work log

- 2026-09-30: Reproduced with the probe harness (`t21.tt`) at the commit
  before TASK-603: `($tt_result: TResult<{ name: string; }, string>):
  { name: string; }`.
- 2026-09-30: After TASK-603, compared the file with and without
  `const r3 = (;` at four positions (after and inside the operand, in a
  statement and in a `result` block): signature help, completion, hover,
  and definition answer the same.
- 2026-09-30: Test:
  `signature_help_in_a_try_is_the_same_while_the_file_has_a_syntax_error`
  (`tests/native/editor_service.rs`). It fails with TASK-603's change
  reverted, with the `$tt_result` signature.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

## Result

Changed `tests/native/editor_service.rs` and the task index. Signature
help around a `try` answers the same with and without a syntax error in
the file.
