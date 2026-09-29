# TASK-496: Keep the source's automatic semicolons when generated code starts a statement

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A member pipeline step at a statement start after a line with no semicolon compiled to code that continues the previous statement: `const v = 1⏎v |> o.m` emitted `const v = 1⏎(($tt_v, $tt_r) => $tt_r.m($tt_v))(v, (o))`, which parses as a call of `1` and throws a `TypeError` at run time. The self-check did not notice, because the output parses.

## Scope

- Included: One rule, applied while the target file is finalized, that keeps every statement boundary the source makes by automatic semicolon insertion (ASI) a boundary of the output, for every emitter path; the list of those boundaries, owned by the lexer; a self-check that fails when output continues a statement the source ended; the program-syntax projection's boundary semicolon, which missed statements inside template interpolations for the same reason.
- Excluded: Changing any emitted form (the member step's IIFE, the parenthesized callee of TASK-454, receivers); they stay as they are and are separated where they start a statement. SWC's handling of a block-bodied arrow function followed by a line starting with an operator is TASK-497.

## Decisions

### Decision 1: Separate statements in the target, not in each emitter path

- **Context**: The member step (`emit_member_step`) is one of several lowerings whose text starts with a token that continues an expression: a parenthesized receiver (`(await p).then(g)`, `(u as number).toFixed(1)`), the bound-member form of `flow`, the recovery IIFE of a statement `match`, TASK-454's parenthesized callee. Each can land where the source relied on ASI, at any depth (a function, class method, arrow body, `if let` body, `match` arm, `result` block, template interpolation) and after a restricted production (`return`, `yield`).
- **Alternatives considered**: (a) Emit a leading `;` in `emit_member_step` and every similar path. It is a per-path patch and misses the next path. (b) Decide it in the emitter's statement loops (`emit_statements_with_edits`, `emit_body_with_exits`, `emit_sequence_continued`) and in the replacement splice of `source_range_rope`. Four places, and a statement nested in pass-through text reaches only the last one. (c) Decide it once where the target is finalized (`TargetFile`), which sees every piece every path wrote, with its provenance.
- **Decision and rationale**: (c). `TargetFile::separate_statements` runs before the target is validated and printed. The one fact it needs about the source — where ASI separates two statements — comes from the lexer's token facts (TASK-491), and the one fact about the output — whether a token would continue the statement before it — comes from the same facts machine. The emitter is unchanged.

### Decision 2: What counts as "the target continues a source boundary with other text"

- **Context**: A source piece that merely ends where a statement ends is not enough: a lowering copies source into its own text, and the end of `o.m` in `$tt_r.m($tt_v)` is also the end of a source statement. The first implementation matched both the end of the ended statement and the start of the next one, and wrote `$tt_r.m;($tt_v)`.
- **Decision and rationale**: A boundary is recognized on a source piece that ends exactly at the next statement's start and carries a line terminator, i.e. that copies the source's separation of the two statements. That text exists only where source statements are copied as statements. When the next text-bearing piece is that statement's own source (a piece starting at the boundary), nothing changes. Otherwise the text that follows (up to the end of its first significant line, layout breaks read as line breaks) is asked `lexer::continues_statement`, and a `;` is inserted when it continues. The `;` is a generated piece with a new synthetic origin, `SyntheticReason::StatementSeparator`, parented at the boundary.

### Decision 3: "Would this continue the statement" is a token-facts question

- **Context**: ECMA-262 §12.10.2 lists `(`, `[`, a template, `+`, `-`, and `/` as the tokens that continue a statement across a line break; TypeScript adds `<` (type assertions and type arguments). A byte list would be one more local predicate of the kind TASK-491 removed, and would get `++`/`--` (restricted productions), `//` and `/*` (comments), and `;` wrong without special cases.
- **Decision and rationale**: `continues_statement` lexes the text on the line after an operand statement (`x⏎…`) and asks the facts machine about the first token: it continues when no automatic semicolon precedes it, it starts no statement, and it is not `;` or a closer (`statement_continues_after`). The unit table covers the ECMA list, `<T>`, comments, `++`/`--`, `let`, `{`, and `;`.

### Decision 4: A self-check for the class, independent of the fix

- **Context**: The bug was silent because the output parses. SWC's parse of the output cannot tell `1⏎(f)(v)` from the intended code.
- **Alternatives considered**: Compare SWC's statement spans of the output with the source's. Needs a mapping of every statement, including the ones lowerings rewrite, and SWC spans give no more than the token facts do (TASK-491 holds the machine to SWC's statement spans on TypeScript).
- **Decision and rationale**: `verify::verify_statement_boundaries`, run by `verify_emit` after the SWC parse wherever the self-check runs (`compile`, `compile_report`, and the contextual re-verification). For each source ASI boundary whose ending token the output copies together with the separation after it, the output's own token facts must show a boundary after that token (`lexer::statement_continues_after` over the lexed output). Output that copies the source across the boundary unchanged is skipped without lexing, and the output is lexed at most once. A violation is `verify-failed` with its own message ("generated TypeScript changed the meaning of this code …"), located at the construct whose glue follows. The check reads the output, not the rope, so it does not restate the fix; its unit test feeds it the output this task's reproduction produced before the fix.

### Decision 5: The projection reads the same boundary list

- **Context**: The projection (`program_syntax/projection.rs`, TASK-391) wrote a boundary `;` before a tt statement whose first token had `asi_before`, looking the token up in the top-level stream. Tokens inside a template interpolation are nested in the template token, so a pipeline statement inside `${(() => { const w = 3⏎ w |> o.m … })()}` was projected as a call of `3` and failed with `lowering-plan-failed: the evaluation position … maps to no source`.
- **Decision and rationale**: The projection keeps `lexer::automatic_semicolons` (which walks interpolations) instead of the raw token stream and looks the statement start up in it, so both sides of lowering read one list.

## Work log

- 2026-09-28: Reproduced on `d35a90e` with `ttc -p`: the member step after `const v = 1`, `const a = f`, `if (c) f`, `type T = number`, `let d: number`, and `f // c`, for `o.m`, `o[k]`, `o?.m`, `this.m`, and `String |> o.m`, at the top level and in a function body, emitted text that runs into the previous line; node threw `TypeError: 1 is not a function`.
- 2026-09-28: Added `AutomaticSemicolon`, `automatic_semicolons`, `continues_statement`, and `statement_continues_after` to `src/lexer/queries.rs`; `TargetFile::separate_statements` and `leading_text` and `SyntheticReason::StatementSeparator` to `src/codegen/rope.rs`; threaded the boundaries through `Rope::flatten` and a new `codegen::EmitSource` (text, kind, boundaries) taken by `emit_with_map` in place of the source and kind arguments (`src/lib/compile.rs`, `src/lib/mapped.rs` — which now keeps the tokens `lex_and_parse_with_kind` returns). First run inserted a `;` inside the member-step body (Issue 1).
- 2026-09-28: Added `verify::verify_emit`, `verify_statement_boundaries`, and `FailureKind`; `compile_mapped` and `verified_emit` call `verify_emit`. First run rejected the fixed output (Issue 2).
- 2026-09-28: Checked every form with node (`--experimental-strip-types`, the runtime compiled next to it): member steps after each line above, `this.m` in a method, in an arrow body, in a template interpolation, in an `if let` body, in a `match` arm block, in a `result` block, after `return` and `yield`, after `const z = {}` and `const g = () => {}`, a label, an `if` body without braces (no separator, as required), `(o.m)`, `u as number |> .toFixed(1) |> …`, `a + 1 |> .toString() |> …`, and `flow |> o.m`. The template-interpolation case failed before this task's projection change (Decision 5).
- 2026-09-28: Tests: `tests/compile/cases_13.rs` (every line × every continuing step; a name-first lowering, an explicit `;`, and a block get no separator; `return`/`yield`; a method, an arrow body, and a template interpolation), `a_member_step_after_a_semicolon_free_line_calls_the_member` in `tests/integration.rs` (tsc and node), unit tests in `src/lexer/queries.rs`, `src/codegen/rope/tests.rs`, and `src/verify.rs`. Updated `docs/design/compiler-architecture.md`.

## Issues and resolutions

### Issue 1: A separator inside a lowering

- **Symptom**: `v |> o.m` followed by `function g() {` emitted `$tt_r.m;($tt_v)` and failed the self-check.
- **Cause**: The first rule also matched source pieces ending at the end of an ASI-ended statement. The member-step body copies `.m`, which ends the source statement, and glue follows it.
- **Resolution**: Only a piece that ends at the next statement's start and carries the separating line break marks a boundary (Decision 2).

### Issue 2: The self-check flagged the same copy

- **Symptom**: The fixed output failed `verify-failed` at `o.m`.
- **Cause**: Same shape: the check looked at every output copy of a statement's last token.
- **Resolution**: The check applies only where the output copies the ending token together with the source text after it (the separation), which a lowering's copy of `.m` never does.

### Issue 3: `;(a)` read as a continuation

- **Symptom**: The unit table said text starting with `;` continues a statement.
- **Cause**: `continues_statement` first asked only for "no `asi_before`", which an explicit `;` does not carry.
- **Resolution**: It asks `statement_continues_after`, which also excludes a statement start, `;`, and closers.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0, no failures; no snapshot changed.
- [x] `./scripts/ci extension`: exit 0.
- [x] `scripts/check-task-index`: the index and the records agree.
- [x] Every reproduction in the work log runs under node with the expected output.

## Result

Changed `src/lexer.rs`, `src/lexer/queries.rs`, `src/codegen/mod.rs`, `src/codegen/core/mod.rs`, `src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `src/codegen/rope/tests.rs`, `src/program_syntax.rs`, `src/program_syntax/projection.rs`, `src/verify.rs`, `src/lib/compile.rs`, `src/lib/mapped.rs`, `tests/compile.rs`, `tests/integration.rs`, `docs/design/compiler-architecture.md`, `docs/tasks/INDEX.md`, and this record; added `tests/compile/cases_13.rs`.

A statement the source ends by ASI now stays ended in the output whatever lowering follows it, the projection agrees inside template interpolations, and the self-check rejects output that joins two such statements.
