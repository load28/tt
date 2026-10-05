# TASK-762: Audit structural editor recovery for regressions

- **Status**: In progress
- **Started**: 2026-10-05
- **Completed**: —
- **Commit**: —

## Purpose

Review PR #140 (TASK-759 to TASK-761) from several independent perspectives,
fix every confirmed regression at its responsible layer, and repeat the review
until no further defect is found.

## Scope

- Included: parser recovery, editor projection and emission separation,
  diagnostic publication across engine, server and LSP layers, strict-path
  differential testing against the pre-PR compiler.
- Excluded: editor validation performance (TASK-763), language features and
  toolchain changes.

## Decisions

### Decision 1: The service answer states the causes its projection hides

- **Context**: An editor repair replaces malformed host text, so TypeScript
  never reports that cause. `retains` only kept a text-layer diagnostic at the
  same position, and the strict text layer reports one cause per file.
- **Alternatives considered**: Make the server `check` method report editor
  recovery diagnostics (breaks its equality with the one-shot `ttc --check`);
  rely on the typed layer (absent when `tt.typedChecks` is off).
- **Decision and rationale**: The layer whose projection hid the cause states
  it, as the content mapper already does. `retains` entries carry the range
  and message, and the adapter publishes a retained cause that no text
  diagnostic states; the typed layer still replaces it by rule and position.

## Work log

- 2026-10-05: Ran `./scripts/doctor` (TypeScript missing; ran `npm ci`).
  Built the pre-PR compiler (`e114ef3b^1`) in a scratch worktree.
- 2026-10-05: Differential test of `check`, `emitMap` and `semanticTokens`
  over all 10,214 `.tt`/`.ttx` files under `tests/cases`: zero differences
  between the pre-PR and current compilers. The strict path is unchanged.
- 2026-10-05: Diagnostic-publication review found Issue 1. Parser review
  found Issues 2-5; projection review found Issue 6. Fixed each at its
  owning layer and committed the first round.
- 2026-10-05: First full `cargo test`: only the public API baseline (new
  `RetainedSyntax`) and the editor parity inventory (three new twin cases)
  changed; both reviewed and accepted. `cargo clippy --all-targets -- -D
  warnings` passed; the extension suite passed 243 tests with no skips.
- 2026-10-05: Second review round (diff review, mutation fuzzing of
  59,799 editor projections, LSP typing simulation). Found Issues 7-9.

## Issues and resolutions

### Issue 1: A repaired syntax cause disappears when typed tt checks are off

- **Symptom**: With `tt.typedChecks: false`, two unclosed calls published one
  diagnostic; a missing operand after an earlier raw error published none.
- **Cause**: The editor projection repairs the text, so TypeScript reports
  nothing there. The repaired cause existed only in the projection report,
  which reaches the editor through the typed layer, whose tt diagnostics the
  adapter filters out when typed checks are off.
- **Resolution**: `Project::service_retained_syntax` returns `RetainedSyntax`
  (code, range, message); the server and adapter carry it, and
  `publishedDiagnostics` states a retained cause no text diagnostic states.

### Issue 2: Valid keyword type members looped forever in editor parsing

- **Symptom**: `interface I {\n const\n x: number\n}` never returned in
  release builds and failed a span assertion in debug builds.
- **Cause**: The member-name recovery for `obj.` followed by a declaration
  also ran for type-member keys and returned an empty name without consuming
  input, so the type-member list parsed the same token again.
- **Resolution**: Member-name recovery applies only after `.`/`?.`, using
  TypeScript's `parseRightSideOfDot` rule (a line break, then a name followed
  by another name on the same line). The type-member list stops when an
  editor element consumes nothing, as TypeScript's
  `abortParsingListOrMoveToNextToken` returns to the enclosing context.

### Issue 3: Incomplete superclass type arguments panicked

- **Symptom**: `class B extends A<` followed by a declaration asserted in
  debug builds and swallowed the declaration in release builds.
- **Cause**: TASK-759 let `parse_ts_type_args` leave a missing `>` to its
  callers but two callers still used `assert_and_bump`.
- **Resolution**: Superclass and decorator type arguments use the checked
  `expect!`, like every other caller.

### Issue 4: Valid keyword member names on the next line were errors

- **Symptom**: `x = obj.\nconst\ny = 1;` reported a missing identifier.
- **Cause**: The recovery treated `const`/`let`/`var` followed by `{` or `[`
  or a line break as a declaration start. TypeScript does not.
- **Resolution**: The `parseRightSideOfDot` rule of Issue 2.

### Issue 5: Recovery cost grew quadratically

- **Symptom**: 16,000 recoveries took over 20 seconds to parse.
- **Cause**: Each speculation checkpoint cloned every recovery record.
- **Resolution**: Records are only appended during a speculation, so a
  checkpoint stores the record count and a rollback truncates to it.

### Issue 6: A missing `)` inside a tt operand fused the next statement

- **Symptom**: `const x = try h(1;` and `1 |> f(2;` emitted glue running into
  the following statement (`$tt_v0return`, `(1)export`).
- **Cause**: The tt operand scanners end an operand at a statement keyword
  in an open list but not at `;`, which TypeScript's `isListTerminator`
  treats as the end of an argument list.
- **Resolution**: `;` directly in an open `(`/`[` ends the operand. The
  remaining differences from the TypeScript twins are inside the broken
  construct itself and are listed in the parity inventory.

### Issue 7: Valid `for (;;)` inside a block inside an operand was rejected

- **Symptom**: `try h(() => { for (;;) {} })` failed to compile after Issue 6.
  The same held before PR #140 for `for (let ...)` inside such a block.
- **Cause**: Both list-end rules looked only at the innermost bracket, not at
  an enclosing block, where `;` and statement keywords belong to statements.
- **Resolution**: Neither rule applies while a `{` is open in the operand.

### Issue 8: Two editor projections raised internal compiler errors

- **Symptom**: `[match(x){A=>}` and `function o(){(l)?.((try r),try ` panicked
  with `LayoutScopeMissing` and `SourceEmittedTwice`.
- **Cause**: An unfinished host production at end of file is not repaired,
  so a tt value inside it was lowered against an incomplete owner tree.
- **Resolution**: An end-of-file repair is materialized when a tt value lies
  inside the unfinished production; host-only text keeps TypeScript's own
  verdict there.

### Issue 9: A strict diagnostic moved for one invalid-input class

- **Symptom**: `try f([1 +;])` now reports the stray `]` rather than the
  missing operand.
- **Cause**: Issue 6's rule ends the operand at `;`, as the existing
  statement-keyword rule does; the strict host check then sees `]`.
- **Resolution**: Kept. TypeScript also reports an error at that `]`
  (TS1128), the file still fails, and valid input is unaffected.

## Regression test (fails before the fix)

- **Path**: `editors/vscode/server/src/test/server.test.ts`, "a repaired
  syntax cause is published when typed tt checks are off"; unit test in
  `diagnostics.test.ts`, "a repaired cause no other layer states is published
  from the service answer".
- **Observed failure**: Against the unfixed compiler the LSP test published
  only `3:11`; `assert.ok(starts.has("3:0") && starts.size >= 2)` failed with
  `actual: false`.

- **Path**: `tests/swc_editor_recovery.rs`,
  `keyword_type_members_on_their_own_line_are_members`,
  `a_keyword_member_name_on_the_next_line_is_the_member`,
  `incomplete_superclass_type_arguments_leave_following_statements`,
  `speculation_rollback_is_independent_of_earlier_recoveries`.
- **Observed failure**: Without the parser changes the first and third timed
  out after panicking at `parser/mod.rs:699` and `:708`, the second failed its
  error count assertion, and the fourth exceeded its 20-second limit.
- **Path**: `tests/cases/editor/recovery{Try,Pipe,Arm}OperandDelimiter`.
- **Observed failure**: Before Issue 6's change the try and pipe projections
  contained `$tt_v0return` and `(1)export`.
- **Path**: `tests/cases/compiler/operandBlockStatementSemicolons.tt`.
- **Observed failure**: Without Issue 7's change `ttc` reported
  `source-not-typescript` (unbalanced delimiter) for valid code.
- **Path**: `fuzz/regressions/compile_any_bytes/editor-eof-list-match-arm.tt`
  and `editor-eof-optional-call-try.tt`.
- **Observed failure**: `tests/fuzz_regressions.rs` reported both crashes
  (`LayoutScopeMissing`, `SourceEmittedTwice`) without Issue 8's change.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change
- [ ] `./scripts/ci`

## Result

Pending.
