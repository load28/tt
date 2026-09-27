# TASK-435: Make parse-only engine requests linear in the number of `match` expressions

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: this commit (`TASK-435: Make parse-only engine requests linear in the number of matches`)

## Purpose

`semanticTokens`, `declarations`, `ttHints`, and `check` grew quadratically with
the number of `match` expressions in one file (measured: `semanticTokens` on one
statement `match` per line took 0.25 s for 500 lines, 0.92 s for 1000, and
3.41 s for 2000; expression-position matches were worse). Every editor keystroke
pays these requests, so the growth has to be removed where it is caused.

## Scope

- Included: every quadratic step profiled on the reported shapes (one statement
  `match` per function, one expression `match` per concise arrow) in the parser,
  program-syntax projection and collection, analysis validation, code emission,
  source-preservation validation, generated-name allocation, semantic tokens,
  and the server's `declarations` offset conversion. Deterministic regression
  guards for the work each fix removed.
- Excluded: output and diagnostic changes (all output is byte-identical; the
  full suite and snapshots pass unchanged). The host-ownership fixed point for
  host declarations named `match` (see Decision 3). Quadratic growth in the
  number of *diagnostics* (each diagnostic's line/column conversion rereads the
  source prefix), which the reported shapes do not exercise.

## Decisions

### Decision 1: Measure before assuming the cause

- **Context**: The report suspected `probe_region` in `src/parser/host.rs`,
  which re-parses a region once per restored candidate.
- **Alternatives considered**: Rewrite `probe_region` first, or profile.
- **Decision and rationale**: Callgrind on the release build showed that
  `probe_region` performs exactly one SWC parse per region for tt matches
  (every candidate's expression probe parses, so nothing is restored). The
  growth came from whole-file scans repeated per match:
  `concise_arrow_boundary_before` (every arrow before a token re-scanned its
  concise body on every `starts_statement` query, in the parser and in the
  projection builder), `engine::tokens::deny_in` (every verbatim segment scanned
  every token), the server's `declarations` (each offset converted by counting
  UTF-16 units from the start of the file), `validate_semantic` (every match
  analysis searched every pattern site), `collect_region_facts` (a linear
  `contains` per segment), `$tt_subject` allocation (each subject probed
  `$tt_subject_1..n` from 1), the emitter's source walk (every cursor step
  scanned every source replacement, owner-slot, compose, and loop rewrite, and
  evaluated `value_anchor` for each), `statement_expr_requires_lowering` and its
  siblings (linear searches by expression), `ParentCollector::finish` (every
  overlay searched every overlay for its enclosing one and every projection
  segment for its source span), and `validate_source_preservation` (a linear
  containment test per printed piece and per owned byte).

### Decision 2: Answer each positional question from an index built once

- **Context**: The scans answer fixed questions (which spans contain a byte,
  which start or end in a range, which arrow bodies end at a token) over data
  that does not change while it is queried.
- **Alternatives considered**: Memoize per query, or restructure the emitter's
  data flow. Memoization keeps the first query per position linear; changing
  the data flow risks the byte-identical output contract.
- **Decision and rationale**: `ConciseArrowBoundaries` computes, in one pass
  over a token stream, the set of tokens a concise arrow body ends at; token
  `at` qualifies exactly when `previous < end <= at`, i.e. `end == at`, so the
  answer is identical. `SpanIndex` (`src/span_index.rs`) orders spans by start
  and end with a maximum-end tree, and returns the same indices a full scan
  would, in index order, so every `find`/`any`/`filter` that consumed a scan
  sees the same sequence. It backs the emitter's replacement, owner-slot,
  propagation, compose, and loop-body lookups, the overlay nesting and segment
  lookups in program-syntax collection, and source-preservation validation.
  Owner-slot and arrow-return rewrites are also keyed by expression.
  `validate_semantic` and `collect_region_facts` use hash sets. Generated-name
  allocation resumes after the last taken suffix of the same base
  (`allocate_after`): the occupied set only grows, so every smaller suffix is
  still taken and the first free name is unchanged. `deny_in` starts at the
  first token that can overlap the segment and stops past it. The server
  converts offsets through `Utf16Offsets`, which records the multi-byte
  characters once and answers exactly what `utf16_offset` answers, including
  the byte-order mark and offsets inside a character.

### Decision 3: Keep the host-ownership fixed point and record its bound

- **Context**: `probe_region` restores one candidate per SWC parse. For host
  methods or functions named `match` whose body is itself claimable as a tt
  match (`{ match(x) { x => 0 } }`), each restored candidate costs one parse of
  the region: 1000 such objects took 1.16 s for `semanticTokens` (release).
- **Alternatives considered**: Restore several candidates per parse, or decide
  each candidate in a smaller region. SWC reports only the first fatal error,
  its recovery and lookahead can cross statement boundaries, and the restored
  set is path dependent, so neither alternative provably yields the same
  ownership claims.
- **Decision and rationale**: TASK-363 (Issue 7) accepted "one parse plus one
  retry per actual host declaration" as the cost of AST-proven ownership.
  Identical claims are a hard requirement here, so the fixed point stays. It is
  bounded by the number of host declarations named `match`, not by the number
  of tt matches; a regression test now fixes that tt matches never add host
  parses.

### Decision 4: Guard the removed work with deterministic counts

- **Context**: Wall-clock bounds are flaky on shared runners.
- **Alternatives considered**: Timed tests, or counting work at the fixed
  sites.
- **Decision and rationale**: `src/work.rs` provides a test-only per-thread
  counter (`tick` compiles to nothing outside tests). The parser's concise-arrow
  scans, host parses, generated-name probes, emitter anchor lookups, and denied
  token visits are counted; `src/lib/scaling_tests.rs` runs compile, semantic
  tokens, declarations, and hints on 150 and 300 matches of both shapes and
  requires the counts to at most double. Reverting the generated-name cursor
  makes the test fail (22,956 probes for n, 90,906 for 2n). `SpanIndex` and
  `Utf16Offsets` are checked against the scans they replace.

## Work log

- 2026-09-27: Reproduced with `ttc --check` and `ttc --server` on generated
  files; profiled with callgrind on a release build with line tables.
- 2026-09-27: Replaced `concise_arrow_boundary_before` with
  `ConciseArrowBoundaries` (`src/flow/syntax.rs`, `src/flow/mod.rs`,
  `src/parser/parse.rs`, `src/program_syntax/projection.rs`); bounded
  `deny_in` (`src/engine/tokens.rs`); added `Utf16Offsets`
  (`src/lib/mapped.rs`, `src/server.rs`).
- 2026-09-27: Indexed pattern sites and region candidates
  (`src/analysis/mod.rs`, `src/parser/host.rs`); added `allocate_after`
  (`src/generated_names.rs`, `src/evaluation_ir/evaluation.rs`).
- 2026-09-27: Added `SpanIndex` (`src/span_index.rs`) and used it in the
  emitter (`src/codegen/core/mod.rs`, `src/codegen/core/emitter/*.rs`),
  source-preservation validation (`src/codegen/rope.rs`), and program-syntax
  collection (`src/program_syntax.rs`, `src/program_syntax/{collector,
  projection,protocol,visit}.rs`).
- 2026-09-27: Added `src/work.rs` and `src/lib/scaling_tests.rs`.
- 2026-09-27: Ported onto TASK-451. The emitter's owner blocks for unbraced
  bodies (TASK-447) added two scans of every block-required owner per source
  step (`next_owner_end`, `close_owner_blocks_at`); both now read the owners
  keyed by end (`block_required_by_end`), which yields the same owners.

## Issues and resolutions

### Issue 1: The reported suspect was not the cause

- **Symptom**: `probe_region` did not appear in the profile of the reported
  shapes.
- **Cause**: tt match candidates parse as expression probes, so the fixed
  point converges after one parse per region.
- **Resolution**: Fixed the scans the profile named (Decision 1); recorded the
  remaining bound of the fixed point (Decision 3).

### Issue 2: Shared disk ran out during builds and tests

- **Symptom**: `No space left on device` from the linker and from TypeScript
  writing into `/tmp` during integration tests.
- **Cause**: Several worktrees share one volume.
- **Resolution**: Removed this worktree's release and incremental outputs and
  re-ran the affected test binary, which passed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (all binaries, snapshots unchanged)

Measurements (debug `ttc --server`, one request each; before on release where
noted):

| Request | Before (release) | After (debug) |
| --- | --- | --- |
| `semanticTokens`, statement matches | 1000: 0.076 s, 2000: 0.463 s | 2000: 0.18 s, 4000: 0.25 s |
| `declarations`, expression matches | 1000: 0.162 s, 2000: 0.636 s | 2000: 0.15 s, 4000: 0.19 s |
| `ttHints`, expression matches | 1000: 0.123 s, 2000: 0.473 s | 2000: 0.14 s, 4000: 0.19 s |
| `check`, statement matches | 1000: 0.313 s, 2000: 1.198 s | 2000: 0.66 s, 4000: 1.28 s |
| `check`, expression matches | 1000: 0.227 s, 2000: 0.862 s | 2000: 0.56 s, 4000: 1.44 s |

Instruction counts (callgrind, debug) now double with the input:
`ttHints` 385 M → 593 M instructions for 1000 → 2000 expression matches
(`e1000`/`e2000`), `check` 1.76 G → 3.80 G for 1000 → 2000 statement matches.

## Result

All four parse-only requests are linear in the number of `match` expressions
for the reported shapes, with byte-identical output. The host-ownership fixed
point remains proportional to host declarations named `match` (Decision 3).
