# TASK-512: Bring single-file compile cost back within the CI budget

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

After TASK-510, the CI performance job of PR #130 (run 36501036037, head `8cbe0a1`) still failed `single_file` against the merge base `f358f64`: median +13.1%, fastest +14.8%, over a 10% budget. The project cases passed (−7.5% each). TASK-510 left `single_file` at +8.6% instructions and attributed the rest to the token-facts machine lexing three whole texts per compile: the source, the host projection, and the output. Find what is avoidable in that cost and in the rest of a compile, and remove it in the layer that owns it, with every output, diagnostic, and token fact unchanged.

## Scope

- Included: the host syntax checks that run before SWC parses the projection and the output, the lexer and the facts machine, and the HIR source map that every compile fills.
- Excluded: benchmark-specific paths, a second lexing mode without facts (TASK-491 Decision 2), reordering a check behind the parse it guards, and any change to emitted text, diagnostics, or token facts.

## Decisions

### Decision 1: Measure instructions, as TASK-510 did

- **Context**: Timed rounds on this 4-core VM vary by 10–40%.
- **Decision and rationale**: Each revision's bench binary (`cargo bench --no-run` with `CARGO_PROFILE_BENCH_STRIP=none` and `CARGO_PROFILE_BENCH_DEBUG=line-tables-only`) ran under `valgrind --tool=callgrind` with `TT_BENCH_ITERS=1`. `single_file` is the inclusive cost of `ttc::compile::compile_mapped` (`--toggle-collect`, four compiles); the project cases are the inclusive costs of their bench closures in a full run. At the start, `single_file` was 35.81 M instructions against 32.93 M at `f358f64` (+8.7%). Of that, the host syntax checks of the projection and the output were 4.84 M (13.5%), of which 4.07 M was lexing and 0.47 M the conflict-marker text search; the source's lexing for the parse was 1.83 M.

### Decision 2: The host syntax checks are not redundant with SWC's parse

- **Context**: The output's check lexes the whole generated module before SWC parses it for the verify self-check, and the projection's check does the same before the projection's parse. If a clean SWC parse implied that the check passes, the check could run only after a failed parse, to name the cause as before, and a successful compile would lex neither text.
- **Alternatives considered**: (a) Parse with SWC first and lex only when it reports an error. (b) Keep the check first.
- **Decision and rationale**: (b). The implication is false. A differential test lexed 1,939 corpus files (the repository's tests, stdlib, editors, and website, and npm's, eslint's, and prettier's packages) as TypeScript, each unmutated and with 300 single-byte insertions or deletions. The unmutated corpus never trips the check. Of the 148,571 mutated texts the check rejected, SWC parsed seven without any error; an earlier run under both kinds, with 30 mutations per file, found two more under TSX. 200,000 random strings of delimiters, quotes, and operators produced no counterexample. Minimal forms: `export namespace A {\n  const x: Lit)eral;\n}\n` and `export namespace A {\n  type T = B;\n` both parse with no SWC error, and the delimiter check rejects both. For the rest, SWC is not a safe first reader: it asserts after conflict-marker recovery (TASK-322), reaches `unreachable!` on a TSX namespaced member (TASK-365), and backtracks for seconds on deeply unbalanced type arguments (TASK-366). Running SWC first would therefore change which malformed texts compile and would expose those failures, so every guarded text is still lexed before its parse. The same test showed the checks are not only for malformed user input. In the test suite, the output check rejects `$tt_ap(\u{6}, ')` generated from the source `\u{6}|>'\u{b}`: the source's unterminated string swallows the `)` the lowering writes after it, so a source that passes the check can still lower to text that fails it. `docs/design/compiler-architecture.md` records the rule.

### Decision 3: The projection's tokens are not derived from the source's

- **Context**: The projection is the source with claimed constructs replaced by placeholders at known spans, so its token stream could be assembled from the source's tokens and the placeholders' tokens instead of lexed.
- **Alternatives considered**: (a) Derive the projection's tokens from the source's tokens and the placeholders. (b) Keep lexing the projection.
- **Decision and rationale**: (b). The tokens of a copied span are the source's only when the lexer reaches it in the same state. The regular-expression decision at a `/` (and the JSX decision at a TSX `<`) is the machine's, and the machine's stack before a copied span holds the placeholder's frames, not the construct's. An unterminated string or template at a span's end also swallows the placeholder text after it, as in Decision 2's example. An exact derivation would have to fall back to lexing wherever a copied span holds a `/`, a template, or an unterminated token. That fallback makes the saving depend on what a file contains, which is a benchmark-shaped path.

### Decision 4: No lexing mode that computes facts lazily

- **Context**: The host checks read token kinds and spans, not facts. The machine could run only up to the last `/`, TSX `<`, or `.digit` where the lexer needs its answer.
- **Decision and rationale**: Rejected. It would be a second, fact-free lexing mode (TASK-491 Decision 2). Its saving would also depend on where a text's last division or JSX element falls, so it would help the benchmark's `/`-free output and not real code.

### Decision 5: Remove the repetition that remained in the checks

- **Context**: With the three lexings kept, a check still did work its inputs already had.
- **Decision and rationale**:
  - The conflict-marker search ran four substring searches over each checked text. A run of seven equal bytes covers one of every seven consecutive positions, so `has_conflict_marker_text` reads every seventh byte and measures the run only around one of `=`, `<`, `>`, and `|`. The answer is exact: `the_marker_scan_answers_what_a_search_for_each_marker_answers` holds it to the four searches over every text of up to ten bytes from `{=, <, a}`, and over runs of 1–16 of each marker byte between other text.
  - The TSX namespaced-member check lexed its text a second time as TSX, although the check's own tokens are that lexing. It now reads the tokens the check lexed, which removes one whole-text lexing from every TSX check, including the source check of every `.ttx` compile. Text with no delimiter byte, no TSX `<`, and no conflict-marker run cannot fail any of the three checks, so it is not lexed (`text_that_cannot_fail_the_check_is_not_lexed`).
  - `verify_emit` lexed the output for the syntax check and, when a statement boundary needed the output's facts, again for `verify_statement_boundaries`. `lexer::host_syntax_check` returns the tokens it lexed, and the boundary check reads them. `generated_text_that_continues_a_source_statement_is_rejected` now counts one whole-text lexing for a module whose boundary check reads tokens.

### Decision 6: Make the facts machine's per-token costs smaller without touching its rules

- **Context**: Every token paid for a char-boundary-checked `&str` slice of its text, although the machine's rules read the text of words only. Every regular-expression, JSX, and `.digit` decision cloned the whole frame stack into a new heap allocation.
- **Decision and rationale**: `Tok::text` is the word's spelling for `Tk::Word` and empty for every other kind. Every read of `text` in the machine is behind a `Tk::Word` match or guard, which was checked at each of the 37 reads. `Machine::operand_expected` copies the stack into a buffer the machine keeps between queries. Marking `step` `#[inline(always)]` was tried and raised `single_file` by 0.39 M instructions, so it was reverted. A dump of the tokens, their facts, and the host check's answer for 2,890 files under both kinds (the repository's tests, stdlib, editors, website, integrations, packages, and docs, TypeScript's `lib`, and npm, eslint, and prettier), over whole files and 400-byte windows every 997 bytes (55,131 lexings), is identical to `8cbe0a1`'s.

### Decision 7: The HIR source map indexes nodes by their dense IDs

- **Context**: After Decisions 5 and 6, `single_file` was still +7.0%. The largest remaining cost that no rule requires was `HirSourceMap::record_node` at 1.0 M instructions per four compiles (2.8%). It inserted each node's span and origin into two `HashMap`s under SipHash, and they rehashed as they grew. The lowering numbers nodes densely from 0 (`Lower::node`). This is not a regression from `f358f64`, which pays the same cost.
- **Alternatives considered**: A faster hasher for the maps; a dense table.
- **Decision and rationale**: A dense table. `HirSourceMap` keeps one `Vec<Option<(Span, AstOrigin)>>` indexed by `NodeId`, and `node_span`, `ast_origin`, and `first_node_span` read it. `first_node_span` used to iterate a randomly ordered map, so among equal starts it could return any of them; it now returns the lowest ID. No test or output depended on the old order, which varied from run to run. `single_file` fell by 1.65 M instructions.

## Work log

- 2026-09-29: Read TASK-510, TASK-491, TASK-225, and `scripts/bench-compare`. Built the bench binaries of `8cbe0a1` and `f358f64` with symbols and counted instructions (Decision 1): `single_file` 35.81 M against 32.93 M (+8.7%).
- 2026-09-29: Tested whether SWC's parse implies the host check (Decision 2): 200,000 random fragment strings, the corpus, and 30 mutations per corpus file. Reproduced the minimal counterexamples in a scratch test. Logged every firing of the projection's and the output's checks across `TTC_REQUIRE_TSGO=1 cargo test`: the projection's never fired, and the output's fired on three texts (two kinds of `$tt_ap(\u{6}, ')` and TASK-366's input), each also rejected by SWC. The two conflict-marker and TSX tests panicked inside SWC when the scratch probe parsed first, which confirms the order.
- 2026-09-29: Instruction counts after each step (`single_file`, four compiles):
  - the marker scan, the shared TSX tokens, and the output tokens shared with the boundary check: 35.44 M (+7.6%);
  - word-only `Tok::text` and the kept probe stack: 35.24 M (+7.0%);
  - `#[inline(always)]` on `Machine::step`: 35.63 M, reverted;
  - the dense HIR node table: 33.59 M (+2.0%);
  - final tree: **33.74 M (+2.5%)**.
- 2026-09-29: Final full-run counts against `f358f64`: `project_first_snapshot` 1,369.5 M against 1,775.4 M (−22.9%), `project_recheck_one_file` 36.27 M against 46.07 M (−21.3%). TASK-510 left them at −18.0% and −16.8%.
- 2026-09-29: Ran the token, facts, and host-check dump on `8cbe0a1` and on the final tree (Decision 6): identical. Updated `docs/design/compiler-architecture.md`.
- 2026-09-29: `./scripts/bench-compare f358f64`, before the commit: `single_file` 1.27 ms → 1.25 ms (median −1.2%, fastest +10.9%, budget 22.6%), `project_first_snapshot` 58.73 ms → 53.52 ms (−8.9% / −9.0%), `project_recheck_one_file` 1.94 ms → 1.90 ms (−2.1% / +4.4%). After the commit: `single_file` 1.22 ms → 1.88 ms (median +54.4%, fastest +7.7%, budget 10.0%), `project_first_snapshot` −9.9% / +7.8%, `project_recheck_one_file` −14.0% / −14.1%. Both runs printed "No case is slower than its own noise floor allows" and exited 0. Four interleaved rounds of 150 iterations put `single_file`'s median between −22.1% and +11.8%, so time on this VM decides nothing at this size.
- 2026-09-29: Because time is too noisy here, ran callgrind's cache and branch simulation over the four `single_file` compiles of `f358f64`, `8cbe0a1`, and the final tree. Weighting L1 misses by 10, last-level misses by 100, and mispredicted branches by 15 cycles gives 46.1 M, 50.9 M (+10.4%), and 48.4 M (+4.9%). CI measured `8cbe0a1` at +13.1% against that model's +10.4%.

## Issues and resolutions

### Issue 1: The obvious redundancy was not one

- **Symptom**: The output's host check and SWC's verify parse looked like two readings of one fact.
- **Cause**: SWC recovers from some unbalanced delimiters without recording an error, and it panics or backtracks on others (Decision 2).
- **Resolution**: The checks keep their order. The cost came out of the checks' own repeated work (Decision 5), the machine's per-token overhead (Decision 6), and the HIR source map (Decision 7).

### Issue 2: Inlining the machine's dispatch cost more than it saved

- **Symptom**: `#[inline(always)]` on `Machine::step` raised `single_file` from 35.24 M to 35.63 M instructions.
- **Cause**: `step` is also called from `operand_expected`'s probe loop, so both callers carried the whole dispatch, and `push` spilled more registers.
- **Resolution**: Reverted.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 42 suites, 1,574 tests, 0 failures; snapshots unchanged.
- [x] `./scripts/ci extension`: exit 0; 219 tests passed, none skipped.
- [x] `scripts/check-task-index`: the index and the records agree.
- [x] Token, facts, and host-check dump: identical to `8cbe0a1` on 55,131 lexings of 2,890 files under both kinds.
- [x] `./scripts/bench-compare f358f64`: exit 0, "No case is slower than its own noise floor allows".
- [x] The TASK-510 guards (`compiling_a_file_lexes_its_source_projection_and_output_once_each`, `projecting_a_file_for_a_snapshot_parses_it_once`) pass unchanged: one parse and three whole-text lexings.

## Result

Changed `src/lexer.rs`, `src/lexer/validation.rs`, `src/lexer/facts.rs`, `src/verify.rs`, `src/hir/mod.rs`, `docs/design/compiler-architecture.md`, `docs/tasks/INDEX.md`, and this record.

Against `f358f64`, one pass now executes `single_file` +2.5%, `project_first_snapshot` −22.9%, and `project_recheck_one_file` −21.3% instructions (from +8.7%, −18.0%, −16.8%). The lexing a compile pays is 0.57 M instructions lower (Decisions 5 and 6); the three whole-text lexings remain, for the reasons in Decisions 2 through 4. The rest of the drop is the HIR source map (Decision 7).
