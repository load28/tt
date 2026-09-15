# TASK-377: Repair compiler boundary and lowering defects

- **Status**: Complete
- **Started**: 2026-09-14
- **Completed**: 2026-09-14
- **Commit**: —

## Purpose

An audit of the compiler surfaced defects at the input boundary (byte-order mark, deep nesting, non-UTF-8 stdin), in position reporting, and in lowering (nested match subjects, statement-form `try`, result tails, guards). Each one is repaired in the layer that owns the contract rather than patched at the symptom.

## Scope

- Included: the swc input seam, the compiler stack policy, the server request loop, verify positions, emit-map source kind, the protocol end-position sentinel, output line endings, concise-arrow glue layout, nested decision subject naming, guard value ownership, statement-form `try` grammar, result-return edits, lowering error display.
- Excluded: editor (TypeScript) changes, which TASK-378 covers; the limitations listed under Result.

## Decisions

### Decision 1: One seam hands text to swc and reads positions back

- **Context**: `swc_common::SourceMap::new_source_file` removes a leading byte-order mark before assigning positions (its documentation says the input "should not have UTF8 BOM"), so every span it reported was three bytes short of the compiler's byte ruler. Four call sites each did their own `pos - start_pos` arithmetic.
- **Alternatives considered**: strip the mark at every public entry point (many entry points, and the mark is a pass-through byte by the source-preserving contract); patch the vendored parser (the vendor note limits the patch to the JSX entity fix).
- **Decision and rationale**: `src/host_input.rs` is the only place swc receives text; `HostOrigin::byte` converts a `BytePos` to a byte of the text given, accounting for the trimmed mark. `verify.rs`, `program_syntax` and `parser/host.rs` all read positions through it, and `verify::Failure` now carries a byte offset instead of a line/column pair that was re-derived with a different column ruler (`col_display`, which counts wide characters twice).

### Decision 2: The compiler runs on a thread with a declared stack

- **Context**: the vendored swc parser grows the stack only around statement bodies; expression nesting (`(((...)))`, about 3,400 levels) overflowed the 8 MiB main thread and aborted the process, which `ice::catching` cannot intercept and which ended a `--server` session.
- **Alternatives considered**: a larger thread stack alone (a bound, not a fix: the hosted CI's debug build overflowed a 256 MiB stack at 20,000 levels because debug frames are an order of magnitude larger); a nesting-depth limit (invents a language limit TypeScript does not have).
- **Decision and rationale**: two layers, each owning one recursion. The vendored parser's `parse_assignment_expr` grows the stack through the `maybe_grow` hook the parser already uses for statement bodies (`vendor/swc_ecma_parser/TT-PATCH.md` records the patch), so expression nesting is bounded by memory. `ttc::stack::on_compiler_stack` runs the CLI, server, and content mapper on a 256 MiB stack, and `par_map` workers reserve the same size, for the compiler's own AST walks (the program-syntax visitor recurses once per nesting level). The reservation is virtual; only touched pages are committed. This is the approach rustc and rust-analyzer take for the same problem.

### Decision 3: `try` binds to the following primary expression in every form

- **Context**: the statement forms (`try e;`, `const x = try e;`) claimed the whole expression up to `;`, so `const x = try total() * 1.1;` propagated `total() * 1.1` and the emitted test `"value" in 11` threw at runtime, while `docs/ai/tt.md` documents `try total() * 1.1` as `(try total()) * 1.1`.
- **Alternatives considered**: keep the wide statement form and document the difference (two precedences for one operator); reject a trailing operator in the statement form (a diagnostic for code that has one documented meaning).
- **Decision and rationale**: `scan_primary_operand` is shared by both forms. The operand is a primary expression, `await <primary>`, a `match`, or a `result` block; a pipeline is the one operator `try` binds looser than (`try readCfg() |> normalize` stays `try (readCfg() |> normalize)`, the form the mixed-source fixture and the language context both rely on), because `|>` is the lowest-precedence operator in the language. A statement form claims only when the operand is the entire tail; otherwise the declaration is ordinary TypeScript containing a value-form `try`, which the parser now also attempts after a declaration's `=` and at statement start when the statement form declined. A `const n = try g()` line without a semicolon is now the value form under ASI rather than an unclaimed rollback fact (the earlier `try_without_semicolon_is_not_recognized` test encoded the old reading and was replaced).

### Decision 4: A guard owns a tt value only when nothing else in the guard is evaluated

- **Context**: a `match` in a guard reached the emitter with no plan (guards are inside the construct the host projection replaces) and the emitter omitted the inner scrutinee; a `match` as the subject of another `match` reused the fixed `$tt_m` name and assigned to a shadowing `const`.
- **Alternatives considered**: projecting guard expressions as host-owned TypeScript so conditional operations inside guards get the full ownership machinery (a larger program_syntax change); file-unique subject names (changes every output).
- **Decision and rationale**: `CoreFile::guard_shape` classifies a guard as plain, a single tt value under grouping parentheses only, or conditional. Grouped values are lowered as a prelude before the `if` and the test is the guard source with the value replaced by its slot; conditional guards report `match-placement` (the value would otherwise be evaluated on a skipped branch), which is the documented answer for any host operation the lowering cannot own. Subject temporaries carry a nesting depth (`$tt_m`, `$tt_m_1`, `$tt_m0_1`), so only nested subjects change name.

### Decision 5: Generated text joins the file with the file's own line terminator

- **Context**: generated glue used `\n` in CRLF files, and the banner did too.
- **Decision and rationale**: `ttc::line_ending` reads the terminator off the file's first line break (the rule TypeScript's own `getNewLineCharacter` and editors use); the rope printer and the CLI banner both use it. Source bytes are still copied verbatim.

## Work log

- 2026-09-14: Audited the compiler black-box and through `--server`; confirmed each defect with a minimal input before changing code.
- 2026-09-14: Added `src/host_input.rs` and rewired the four swc seams; `verify::Failure` became a byte offset. Added `src/stack.rs` and ran the CLI entry and worker threads on it. The server loop reads bytes and answers a non-UTF-8 line with `id: null` instead of exiting; `utf16_column` leaves the `(0, 0)` sentinel alone; `--emit-map` and `emitMap` derive the source kind from the file name.
- 2026-09-14: The rope printer and the banner use `ttc::line_ending`; the concise-arrow compose rewrite indents its glue one level and separates sibling regions, closing its layout scope in the suffix. The result projection puts the synthetic return on its own line so an authored tail expression stays a statement; `ProgramSyntaxError` and `EvaluationError` render as sentences.
- 2026-09-14: Nested decision subjects carry a depth; guards are classified by `guard_shape` and either lowered as a prelude or rejected with `match-placement`. `try` operands are scanned by one primary-operand rule in both forms. Result-return edits keep their mark-carrying suffix when the return has no semicolon.
- 2026-09-14: Added regression tests (`tests/compile/cases_10.rs`, `tests/cli.rs`, unit tests in `host_input`, `stack`, `error`, `verify`, `parser::tests`), updated `docs/ai/tt.md`, and ran the gates.

## Issues and resolutions

### Issue 1: Byte-order mark broke every lowered construct

- **Symptom**: `lowering-plan-failed: MissingOverlay` at 1:1 for any BOM file with a tt construct.
- **Cause**: swc strips the mark; projection spans were three bytes short and no overlay parent matched.
- **Resolution**: Decision 1.

### Issue 2: Deep nesting aborted the process and the server session

- **Symptom**: SIGABRT (exit 134) on 3,500 nested parentheses; `--server` died without answering. After the first push, the hosted `fmt / clippy / test` and `coverage` jobs aborted the same way in the new 20,000-level unit test, because their debug build's parser frames exceeded the 256 MiB thread stack.
- **Cause**: unbounded expression recursion in the parser, and a stack policy that only raised the bound.
- **Resolution**: Decision 2; verified at 50,000 levels in a debug build.

### Issue 3: A non-UTF-8 stdin line ended the server session

- **Symptom**: `--server` exited 0 without answering later requests.
- **Cause**: `BufRead::lines` returns an error for invalid UTF-8 and the loop broke on it.
- **Resolution**: the loop reads bytes and answers such a line as a malformed request.

### Issue 4: A wide character shifted or erased a `verify-failed` position

- **Symptom**: `const e = "한글"; const x = ;` reported no position, or two columns right.
- **Cause**: `col_display` counts CJK and emoji as two columns; the byte walk counted characters.
- **Resolution**: Decision 1 (byte offsets throughout).

### Issue 5: `emitMap` lowered `.ttx` as `.ts`

- **Symptom**: a `$tt_recovery` IIFE in the emitted map for a JSX container match.
- **Cause**: `emit_mapped` fixes `SourceKind::TypeScript`; the CLI and server never passed the file name.
- **Resolution**: both derive the kind from the file name with `emit_mapped_with_kind`.

### Issue 6: The position-only end became `endLine: 0, endCol: 1`

- **Cause**: `utf16_column` mapped line 0 onto line 1 through `saturating_sub`.
- **Resolution**: a line the text does not have keeps the column it was given.

### Issue 7: A result tail expression without `;` cascaded into `lowering-plan-failed` with an internal dump

- **Cause**: the projection appended `return undefined;` on the same line, and the failure was formatted with `Debug`, including the whole projection.
- **Resolution**: the synthetic return starts a new line; both error enums implement `Display`.

### Issue 8: A `return a` without `;` at the end of a `result` block was an internal compiler error

- **Symptom**: `Result return start has no matching end`.
- **Cause**: the suffix edit after the argument is empty when the statement ends at the argument, and the erasure filter dropped every empty edit, including the one carrying the end mark.
- **Resolution**: only erasure edits are filtered for emptiness.

### Issue 9: A `match` in a guard omitted its scrutinee; a `match` scrutinee shadowed the outer subject

- **Resolution**: Decision 4.

### Issue 10: Statement-form `try` claimed the whole initializer

- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` (all stages)

## Result

Changed files: `src/host_input.rs` (new), `src/stack.rs` (new), `src/lib.rs`, `src/lib/api.rs`, `src/verify.rs`, `src/error.rs`, `src/server.rs`, `src/main/{build,command,loading,modes}.rs`, `src/parser/{host,parse,tries,tests}.rs`, `src/program_syntax.rs`, `src/program_syntax/{collector,projection,protocol,visit}.rs`, `src/core_ir/{mod,lower}.rs`, `src/evaluation_ir.rs`, `src/codegen/rope.rs`, `src/codegen/rope/{builder,tests}.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{helpers,host,pattern,result}.rs`, `tests/compile.rs`, `tests/compile/cases_10.rs` (new), `tests/cli.rs`, `docs/ai/tt.md`.

Known limitations left for later tasks, each confirmed by the audit:

- `(try a()) + (try b())` reports `try-placement` (overlapping source captures) while `try a() + (try b())` compiles.
- A `source-not-typescript` failure prevents the lowering plan, so plan-level diagnostics (`try-placement`, `match-placement`) elsewhere in the same file are not reported in that run.
- `declare namespace N { variant P { Q } }` and `export declare variant` emit a runtime constructor object in an ambient context.
- `val` on a rest parameter is not recognized.
- `if (c) try a();` (an unbraced statement body) passes the `try` through.
