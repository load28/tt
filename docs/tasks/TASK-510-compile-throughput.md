# TASK-510: Restore compile throughput lost on the branch

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: (see the work log)

## Purpose

The CI performance job of PR #130 fails against `main` (merge base `f358f64`): `single_file` +46.8–50.6%, `project_first_snapshot` +52.7–53.3%, `project_recheck_one_file` +40.9–43.4%, over a 10% budget (runs 36483044609 and 36485284203). Find the commits that added the cost, fix each cause in the layer that owns it, and keep every output byte-identical.

## Scope

- Included: bisecting the branch (TASK-388 through TASK-508), profiling the culprits, structural fixes in the responsible layer, and a regression guard.
- Excluded: benchmark-specific shortcuts, caches that change behaviour, any change to emitted text, diagnostics, or token facts, and a redesign of the token-facts machine itself.

## Decisions

### Decision 1: Measure instructions, confirm with time

- **Context**: Wall-clock medians on this 4-core VM moved by 10–40% between identical rounds (for example `single_file` at the merge base: 807/888 µs in one round, 1158/1379 µs in the next), so a bisect by time would have chased noise.
- **Alternatives considered**: `scripts/bench-compare` per commit (one timed run per side, as noisy as above); interleaved timed rounds (still ±15%).
- **Decision and rationale**: Each revision's `benches/compile.rs` binary was built once (`cargo bench --no-run`, the release profile `bench-compare` uses) and run under `valgrind --tool=callgrind` with `TT_BENCH_ITERS=1`. Instruction counts are deterministic to the instruction, and on the branch head they track the CI result (+59% instructions for `single_file` against +47–51% time on CI). Per-case counts come from `--toggle-collect` on `ttc::compile::compile_mapped` (`single_file`: 4 compiles) and from the inclusive cost of the two project closures. Time was checked with interleaved rounds and with `scripts/bench-compare` at the end.

### Decision 2: The attribution (bisect)

- **Context**: The TASK-491 check against `48dff35` had reported no regression.
- **Decision and rationale**: Whole-run instructions of one bench pass, relative to `f358f64` (2.330 G):

  | Commit | Task | Δ (percentage points) |
  |---|---|---|
  | `6ebaca6` | TASK-391 ASI around pipelines and statement constructs | +10.3 |
  | `305daf6` | TASK-453 non-ASCII identifier test (`identifier_char_len` called per identifier byte, not inlined) | +10.0 |
  | `5ee5404` | TASK-450 generator value regions (a whole-prefix scan per value region); `f75e788`/`32d3cda` later took −6 back | +6.6 |
  | `21d89ee`, `23d5218` | TASK-470, TASK-480 | +1.6, +1.2 |
  | `d19a479` | TASK-490 `val` modifier in the parser | +5.6 |
  | `8cd1695` | TASK-491 token-facts machine | **+31.4** |
  | `031c968`, `e67dcaa` | TASK-491 function-body facts, codegen queries over tokens | −5.0, +2.1 |
  | `3fdd7e9` | TASK-491 pragmas and trivia through the scanner primitives | +5.9 |
  | `d35a90e`…`a48338c` | TASK-498…508 | ±0.4 |

  The head was +66.2% (3.870 G). TASK-491's machine made one lexing about 2.7 times as expensive per token (≈160 → ≈440 instructions, trivia and identifier scans included), and the pipeline lexed the same source up to five times per compile (parse, Core lowering, semantic checks, the projection's automatic-semicolon table, the projection's host syntax check), while the engine parsed every project file six times (compile plus five probes). The later TASK-391 code the table blames was replaced by the TASK-491 facts, so its cost today is the machine's.

### Decision 3: Lex and parse each text once

- **Context**: Every duplicate lexing of the source was the same pure function of the same text and kind.
- **Alternatives considered**: A per-compile memo keyed by text; keeping the extra lexings.
- **Decision and rationale**: The parse's token stream is passed to the consumers that lexed on their own: `core_ir::lower_semantic`, `sema::check_all` (the `Checker` borrows the text and the tokens instead of copying them), and `codegen::lowering_plan` → `ProgramSyntax::build_with` → `ProjectionBuilder` (automatic semicolons) and the source host syntax check (`lexer::host_syntax_error_in`). Core lowering and the checks always lexed as TypeScript, so they read `lexer::TypeScriptTokens::of`, which is the parse's stream for a `.tt` file and a separate TypeScript lexing for `.ttx`, as before. The engine's `ProjectedDocument::project_for_snapshot` parses once and hands the program to `compile_projection_report_parsed` and to crate-internal forms of the probes (`scan_module_of`, `literal_matches_of`, `tag_matches_of`, `payload_probes_of`, `val_probes_with_emit`); the public entry points parse and call the same functions. The recovered-projection path reuses the parse instead of parsing again.

### Decision 4: Index the enclosing function instead of rescanning the prefix

- **Context**: `Lowering::node_in_generator` (TASK-450) called `flow::user_function_target_at`, which walks every token before the node and scans every concise arrow body before it, once per value region.
- **Decision and rationale**: `flow::FunctionTargets` computes, in one pass, the innermost enclosing braced function for every token position and the end of every concise arrow body; `at` answers with a lookup and a backward walk over the arrows. Core lowering builds it lazily, once per file. `user_function_target_at` remains the single-query form, and a test holds the index to it at every position of several sources under three ownership sets.

### Decision 5: Make the scanner primitives cheap on ASCII

- **Context**: `identifier_char_len` (TASK-453) and `skip_trivia` (TASK-491) were out-of-line calls per byte or per token with the non-ASCII and comment paths in the same frame.
- **Decision and rationale**: An ASCII identifier table, `#[inline]` fast paths for `identifier_char_len`, `ident_end`, `space_len`, `line_terminator_len`, and `skip_trivia` (white space and line breaks), with comments and non-ASCII code points in out-of-line functions that run the unchanged loop; `line_end` and `contains_line_terminator` scan bytes directly. The token vector is sized for one token per three bytes: tt source and generated TypeScript measure 3.7 and 4.3 bytes per token, so the old one-per-six estimate grew and copied the vector three times per compile.

### Decision 6: Answer lexical questions from the bytes when no token could change the answer

- **Context**: Codegen's token queries and the host syntax check lex snippets whose answer can be decided without tokens.
- **Decision and rationale**: Exact prefilters, each a necessary condition of a `true` answer: `contains_await` needs the bytes `await`; `has_top_level_comma` needs one of `,()[]{}<>`; `Rope::last_line_has_line_comment` needs `//`; `host_syntax_error` needs conflict-marker text or a delimiter byte before its token checks can report anything, so it lexes only then (a variant field type such as `number` is checked without lexing). The marker search runs once per check instead of twice.

### Decision 7: Compute the machine's look-ahead only where a rule reads it

- **Context**: `operand_word` peeked at the next token for every identifier operand and `statement_word` read the next word for every statement start, although only a few keywords consult them; `statement_only_keyword` ran before the cheap `cfg.statement` flag.
- **Decision and rationale**: The peeks are pure, so they moved into the guards and arms that read them, and the flag is tested first. `TokenFacts` are unchanged: a dump of every token and fact for 759 corpus files under both source kinds, whole files and 400-byte windows every 997 bytes (the repository's tests, stdlib, editors, website, integrations, packages, docs, the TypeScript package's `lib`, and npm, eslint, and prettier from `/opt/node22`), is identical to the branch head's.

### Decision 8: What is left is the machine's own cost

- **Context**: After the fixes, `single_file` executes 8.6% more instructions than the merge base.
- **Decision and rationale**: The remainder is lexing: the three whole texts a compile still lexes (source, host projection, output) cost about 2.2 times what the pre-TASK-491 lexer paid for them, which is +2.1 M of the +2.9 M instructions of four compiles (the output check alone +1.9 M), with the rest within ±0.35 M per stage. The machine's profile is flat (no source line above 4.3% of lexing; `push` ≈ 60 and `step` ≈ 70 instructions per call), so what remains is the recognizer TASK-491 chose for correctness, not an avoidable repetition. Removing it would mean a second lexing mode without facts for the host syntax checks, which TASK-491 Decision 2 rejected. The project cases are now faster than the merge base because a projection parses once.

## Work log

- 2026-09-28: Reproduced. Callgrind, one pass: head `a48338c` `single_file` 52.5 M instructions (+59.1%), `project_first_snapshot` 2954.9 M (+66.1%), `project_recheck_one_file` 75.3 M (+62.7%) against 33.0 M, 1778.6 M, 46.3 M. Interleaved timed rounds (150 iterations, 3 rounds): `single_file` 919/1084 µs (min/median) at the base against 1130/1521 µs, `project_first_snapshot` 42.4/57.0 ms against 63.9/85.3 ms.
- 2026-09-28: Bisected by building each revision's bench binary from `git archive` into one target directory and counting instructions (Decision 2); 47 revisions.
- 2026-09-28: Profiled the head with symbols (`CARGO_PROFILE_BENCH_STRIP=none`, `CARGO_PROFILE_BENCH_DEBUG=line-tables-only`); lexing was 44.8% of all instructions and `host_syntax_error` alone 14.8%.
- 2026-09-28: Fixes, measured after each step as `single_file` / `project_first_snapshot` / `project_recheck_one_file` instructions relative to the base:
  - scanner ASCII fast paths (Decision 5, first part): +40.5% / +45.0% / +42.3%;
  - one source lexing per compile (Decision 3, compile half): +21.9% / +30.4% / +29.1%;
  - one parse per projection (Decision 3, engine half): +22.0% / −7.9% / −7.2%;
  - function-target index (Decision 4): +18.0% / −10.6% / −9.7%;
  - inlined `skip_trivia`/`ident_end` fast paths: +15.5% (single file);
  - `validate_source_preservation` classifies white space only for the bytes it reports and consults the rewritten index only there: +13.7% / −14.2% / −13.6%;
  - byte prefilters (Decision 6): +12.1% / −15.3% / −14.6%;
  - lazy look-ahead and one marker scan (Decision 7): +9.8% / −17.1% / −16.2%;
  - token vector sizing (Decision 5): **+8.6% / −18.0% / −16.8%**.
  Lexer-only instructions over the same pass went 423.4 M → 278.0 M.
- 2026-09-28: Added the regression guard: `work::tick("whole-text lexes")` in `lex_with_kind` and `work::tick("source parses")` in `lex_and_parse_with_kind`, and `compiling_a_file_lexes_its_source_projection_and_output_once_each` and `projecting_a_file_for_a_snapshot_parses_it_once` in `src/lib/scaling_tests.rs` (one parse and three whole-text lexings for modules of 1, 4, and 8 variants), plus `the_function_target_index_answers_every_position_as_the_scan_does` in `src/flow/tests.rs`. Documented the rule in `docs/design/compiler-architecture.md`.

## Issues and resolutions

### Issue 1: Wall-clock noise hid the regression's shape

- **Symptom**: Identical binaries differed by up to 40% in median time between rounds; a local `bench-compare` of TASK-491 against `48dff35` reported no regression although it added 31 points of instructions.
- **Cause**: A shared 4-core VM; one timed run per side.
- **Resolution**: Instruction counts for attribution and every intermediate measurement (Decision 1); time only as confirmation.

### Issue 2: The token vector estimate

- **Symptom**: 5% of lexing was `realloc` copying token vectors, three growths per compile.
- **Cause**: The capacity assumed one token per six bytes; tt source and generated TypeScript are denser.
- **Resolution**: One per three (Decision 5). An intermediate one-per-four still grew twice per compile.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; every suite passed, snapshots unchanged.
- [x] `./scripts/ci extension`: exit 0; 212 tests passed, none skipped.
- [x] `scripts/check-task-index`: the index and the records agree.
- [x] Token and fact dump (Decision 7): identical to the branch head on 1517 file/kind pairs.
- [x] `./scripts/bench-compare f358f64`: exit 0, "No case is slower than its own noise floor allows" (see the Result for the numbers).

## Result

Changed `src/scanner.rs`, `src/lexer.rs`, `src/lexer/validation.rs`, `src/lexer/queries.rs`, `src/lexer/facts/expressions.rs`, `src/lexer/facts/statements.rs`, `src/parser/parse.rs`, `src/sema.rs`, `src/sema/checker.rs`, `src/core_ir/lower.rs`, `src/core_ir/mod.rs`, `src/flow/mod.rs`, `src/flow/syntax.rs`, `src/flow/tests.rs`, `src/program_syntax/projection.rs`, `src/program_syntax/tests.rs`, `src/evaluation_ir/tests.rs`, `src/codegen/core/mod.rs`, `src/codegen/rope.rs`, `src/codegen/rope/builder.rs`, `src/lib/compile.rs`, `src/lib/api.rs`, `src/lib/mapped.rs`, `src/probe.rs`, `src/engine/projection.rs`, `src/lib/scaling_tests.rs`, `docs/design/compiler-architecture.md`, `docs/tasks/INDEX.md`, and this record.

Against the merge base `f358f64`, one pass executes `single_file` +8.6%, `project_first_snapshot` −18.0%, and `project_recheck_one_file` −16.8% instructions (from +59.1%, +66.1%, +62.7%). The remaining `single_file` cost is TASK-491's token-facts machine lexing the source, the host projection, and the output once each (Decision 8).
