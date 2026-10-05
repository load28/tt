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
- 2026-10-05: Ran the mutation pass with 100,000 of 3,054,478 mutants (the
  default gate samples 1,000) and found Issues 15-17.
- 2026-10-05: A review of the third round's diff found Issues 19-21 and a
  stale comment on `val` candidates in regions the host cannot parse.
- 2026-10-05: The LSP typing simulation (1,020 published diagnostic sets
  over 10 tt/ttx files, 22 statements retyped character by character and 35
  closing delimiters deleted, TypeScript twins through `tsc --lsp`) found
  the diagnostic spreads of Issues 10-14. Remaining differences from the
  twins are TypeScript's own reading of the text inside the edited
  construct; they are listed in the parity inventory.

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

### Issue 10: Any syntax error disabled `val` parameters in the whole file

- **Symptom**: `function b() { c = }` made every `val p` parameter in the file
  reach TypeScript as written (`Parameter 'val' implicitly has an 'any' type`).
- **Cause**: The parser decides that a `val` names a formal parameter from a
  host parse of its region, and that parse was strict: one error anywhere
  left no parameter known.
- **Resolution**: When the strict parse fails, the region is read with the
  host parser's editor recovery behind the lexical protections, as
  TypeScript reads the complete productions of an erroneous file.

### Issue 11: A generated name fused with the next statement's keyword

- **Symptom**: `const h = try f(` followed by `if (...)` published `Cannot find
  name '$tt_v0if'` and a cascade to the end of the function; a pipeline step
  left open put a later match's prelude inside the following `return (`.
- **Cause**: An operand or pipeline step that leaves a list open ran to the
  next token, so the line break that separated the following statement was
  replaced together with the operand, in the host projection and in codegen.
- **Resolution**: Such an operand keeps the rest of its last line, where the
  next argument is typed, and leaves the line break to the syntax resuming
  after it (`lexer::line_trivia_end`; a block comment crossing the line stays
  whole). A cursor at the end of that operand, after blanks the output does
  not copy, is a cursor in the open list, so completion and signature help
  ask a probe there as before (`mapper::typed_cursor_to_output`). Codegen
  also keeps a generated word and a copied word apart, and restores the line
  break a construct's span absorbed (`TargetFile::separate_tokens`); no
  complete program's emission changes. The first attempt ended the operand at
  its last token, which broke signature help after `f(2, `; the native suite
  caught it.

### Issue 12: A pipeline in a JSX child with a missing `}` spread to EOF

- **Symptom**: `<td>{a |> f</td>` reported unclosed tags through the end of
  the file.
- **Cause**: The lexer looked for the container's `}` past the parent's
  closing tag, and the whole element stopped being JSX.
- **Resolution**: In a JSX file `</` is TypeScript's `LessThanSlashToken`
  (`scanner.ts`), never an operator, so a child container ends there. The
  published diagnostic now equals the twin's `'}' expected`.

### Issue 13: An unfinished arm reported false missing arms

- **Symptom**: An arm with an unclosed `(` or an unterminated string reported
  `match-not-exhaustive` naming arms that are written.
- **Cause**: The open bracket or string runs over the following arms, which
  coverage then did not see.
- **Resolution**: The parser records that the arm list is left open, and
  neither the text nor the typed coverage question is asked for that match.

### Issue 14: A later statement's prelude was read inside a skipped statement

- **Symptom**: `const` or `const Some(value:` above a statement with a match
  published `Cannot find name '$tt_m'` and other generated names.
- **Cause**: A skipped statement keeps its text, and TypeScript's list
  recovery reads the following generated prelude as part of it (`;` is not
  a statement start during its error recovery).
- **Resolution**: Before a tt value, a skipped statement is closed with the
  terminators of the lists it left open and `;`; its own syntax error stays
  TypeScript's. The bare `const` case now equals its twin.

### Issue 15: A JSX child container at end of file indexed past the text

- **Symptom**: The mutation pass found `return<>{` panicking in the lexer.
- **Cause**: Issue 12's change read the closing byte inside `then_some`,
  which evaluates its argument even when the container reached the end.
- **Resolution**: The byte is read only for a container that ends inside the
  text.

### Issue 16: A parenthesized pipeline step after a match head was an ICE

- **Symptom**: `match (g) { E => 1, F => 2 } |> (p |> add)` failed with
  `structured apply step was not emitted`, in the CLI too and before PR #140;
  the editor projection now reached it while a match was being typed.
- **Cause**: Planning gives the inner pipeline the outer value's slot, but the
  inner pipeline has no statement form through which a value is written.
- **Resolution**: A step is written through a slot only when it has a
  statement form; otherwise it is the ordinary expression it is.

### Issue 17: Repairs of a skipped statement were applied

- **Symptom**: `match (x) { 1 => { return(return } }` raised
  `SourceEmittedTwice` in the editor projection.
- **Cause**: When a statement fails and is skipped whole, the records its
  productions made stayed, so a missing operand inside it was materialized
  in text that no node owns.
- **Resolution**: A skipped statement discards the records of its parts, as
  its node does.

### Issue 18: A statement pipeline ending the file left its block open

- **Symptom**: `match (g) { E => 1, F => 2 } |> (show)` as the last bytes of a
  file, with no semicolon or line break, raised `LayoutScopeMissing`, in the
  CLI too and before PR #140; `d9043f3fd9631d2c.tt` reached it.
- **Cause**: The owner's block is closed by the source walk that writes what
  follows the value inside the owner. Every walk that reached the owner's end
  ran inside the owner's own prelude, so none could close it, and when
  nothing follows the value no later walk did.
- **Resolution**: The two structured-value paths that consume an owner's
  compose rewrite close the owner blocks ending where the value ends, after
  writing the value. Any later walk finds the block closed.

### Issue 19: A statement terminator was written inside a trailing comment

- **Symptom**: `const // note` before a statement with a `match` projected
  `const // note;`, and TypeScript reported `',' expected` on generated code.
- **Cause**: The insertion point was the next token's start minus blanks, so
  a comment after the skipped text stayed before it.
- **Resolution**: The insertion follows the skipped text's last token.

### Issue 20: Only some statement keywords ended a skipped expression

- **Symptom**: `const a = g(1, 2 +` followed by `if (flag) {}` and a `match`
  moved the match's prelude to module level, with three parse errors where
  the twin has one.
- **Cause**: The parser's recovery boundary and the projection's check were
  two copies of a six-keyword list, narrower than TypeScript's, and the
  projection compared words of the source text.
- **Resolution**: The parser has one boundary: a token TypeScript's
  `isStartOfStatement` accepts and `isStartOfExpression` does not
  (`abortParsingListOrMoveToNextToken`). Each recovery record states whether
  the parser resumed there, and the projection reads that. A word after `.`
  or `?.` in skipped text stays a member name (`parseRightSideOfDot`).

### Issue 21: A generated word could join a following non-ASCII identifier

- **Symptom**: Reasoned, not observed: at a recovery seam on one line,
  generated text ending in a word byte followed by source starting with a
  non-ASCII identifier character would print as one identifier.
- **Cause**: The seam rule treated only ASCII bytes as word bytes.
- **Resolution**: Source that follows a generated word with a non-ASCII byte
  is kept apart from it, as a word is. The first attempt applied the rule in
  both directions and changed the valid output of
  `aByteOrderMarkPassesThroughAheadOfLoweredConstructs.tt`: a byte order mark
  ends the source before the generated prelude, and gained a space. Source
  before generated text is left as written.

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

- **Path**: `tests/cases/editor/recoveryValParameters.tt`,
  `recoveryTryOperandBeforeStatement.tt`, `recoveryJsxChildPipe.ttx`,
  `recoveryArmOpenBracket.tt`, `recoveryArmUnterminatedString.tt`,
  `recoveryBareConstBeforePrelude.tt` and
  `recoverySkippedStatementBeforePrelude.tt`.
- **Observed failure**: Without the respective changes the baselines showed
  `ts7006` on `val`, `$tt_v0if`, `TS17008` through the end of the file,
  `match-not-exhaustive` for written arms, and generated names in diagnostics.

- **Path**: `fuzz/regressions/compile_any_bytes/2dbd7de9c414347a.ttx`,
  `d9043f3fd9631d2c.tt`, `b62ddb59e7abb322.tt`;
  `tests/cases/compiler/aParenthesizedPipelineStepAfterAMatchHead.tt` (runs,
  printing `11 12`); `tests/swc_editor_recovery.rs`,
  `a_skipped_statement_keeps_no_recovery_of_its_parts`.
- **Observed failure**: The mutation pass (100,000 mutants) reported the three
  crashes; the compiler case failed with the ICE; the parser test found a
  replacement record inside the skipped statement.

- **Path**: `tests/cases/compiler/aStatementPipelineEndingTheFileWithoutASemicolon.tt`
  (runs, printing `2`).
- **Observed failure**: Without Issue 18's change the case failed with
  `LayoutScopeMissing`.

- **Path**: `tests/cases/editor/recoveryCommentAfterSkippedStatement.tt` and
  `recoveryStatementKeywordAfterUnfinishedCall.tt`.
- **Observed failure**: Without Issues 19 and 20's changes the first
  baseline had four diagnostics on generated code (`',' expected`, a
  constant `$tt_v0`) instead of the twin's one, and the second had three
  parse errors and a `expected )` on the next line.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change
- [ ] `./scripts/ci`

## Result

Pending.
