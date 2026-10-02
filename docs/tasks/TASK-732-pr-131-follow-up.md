# TASK-732: Verify PR 131 and fix yield scaling and else-continuation comments

- **Status**: In progress
- **Started**: 2026-10-01
- **Completed**: —
- **Commit**: —

## Purpose

Continue PR [131](https://github.com/load28/tt/pull/131) from commit
`0a382b153b1f401d318573879a9ed282a257e955`, using the final conversation
comment's list of unverified changes and remaining findings.

## Scope

- Included: final-tree gates, full compiler/editor matrices and typed parity,
  reproducible compiler/editor findings, and accurate remaining limitations.
- Excluded: publishing upstream reports, real VS Code UI validation, and
  Windows/macOS validation unavailable in this Linux environment.

## Decisions

### Decision 1: Verify the final PR tree before classifying open findings

- **Context**: The last merged changes had only targeted verification.
- **Alternatives considered**: Rely on earlier intermediate-tree results, or
  repeat the tests on the actual final tree.
- **Decision and rationale**: Run the actual gates and expanded samples;
  intermediate-tree results cannot establish the final tree's correctness.

### Decision 2: Attribute the regression by instruction counts

- **Context**: Time medians of identical binaries vary by 10–40% on this VM
  (TASK-510 Decision 1); the time-based bisect landed on an editor-only commit.
- **Alternatives considered**: Repeating timed runs per commit; a single
  profile of base against head.
- **Decision and rationale**: Build each revision's `benches/compile.rs` once
  and count instructions of one pass (`TT_BENCH_ITERS=1`) with
  `valgrind --tool=cachegrind --cache-sim=no`, as TASK-510 did. A head-only
  profile cannot name commits, because the PR renamed and wrapped many of the
  functions it would compare.

### Decision 3: The attribution

Whole-run instructions of one bench pass, first-parent commits, relative to
`b9b85bd` (1.810 G):

| Commit | Task | Δ instructions |
|---|---|---|
| `86e92d15` | TASK-665 directive and JSDoc comment placement | +34.3 M (+1.9 pt) |
| `f4bbb790` | TASK-654 nesting depth through stack growth | +34.8 M (+1.9 pt) |
| `943982c2` | merge of TASK-681–684 | +27.4 M (+1.5 pt) |
| `b4fea63e` | TASK-687 completion scope in `ProgramSyntax` | +21.1 M (+1.2 pt) |
| `019e328a` | TASK-732 yield context and else-continuation comments | +18.8 M (+1.0 pt) |
| `f05e21b1` interval | not yet refined | +17.0 M (+0.9 pt) |
| `fc1e8062` | TASK-593 earlier declarators before a later value | +16.1 M (+0.9 pt) |
| `1ce50c0f` | merge of TASK-713–716 | +16.4 M (+0.9 pt) |
| others | each under +0.7 pt | the rest |

No single commit explains the budget overrun. Each listed cause still has to
be profiled and fixed in its owning layer with byte-identical output.

### Decision 4: Remove costs no compile needs, without changing any output

- **Context**: The attribution named causes in several layers; each fix had
  to keep emitted text, mappings, and diagnostics identical.
- **Alternatives considered**: Caching results across compiles; skipping
  work by benchmark shape; reverting the features.
- **Decision and rationale**: Three fixes in the owning layers:
  1. `TargetFile::print` (codegen) built two per-piece lookahead arrays for
     single-line directive printing even when no directive governs a
     statement; the arrays are now built only when one does, because a
     piece can print on one line only then.
  2. `sema::check_all` built `FunctionTargets` for every file, although only
     a statement `try` reads it; it is now built on first use, as Core
     lowering already does (`OnceCell`).
  3. The completion scopes were copied twice per compile (`to_vec` from
     `ProgramSyntax`, `clone` from the plan); they are now moved, because
     neither owner reads them afterwards.
  A `FrameStack` that recorded only deciding lists was also tried for the
  TASK-732 yield context; it cost 17 M more instructions than the cached
  context (a check on every pop and truncation) and was reverted.

## Work log

- 2026-10-01: Read PR metadata and its final comment; there are no inline
  review threads. Created `codex/pr-131-followup` at the PR head. Found the
  installed Rust/Bun tools under `/workspace/.tools` and restored their
  command-local PATH; `scripts/doctor` passes without running setup.
- 2026-10-01: Reproduced and fixed the yield scaling and else-comment issues.
  Generated and reviewed the new case's output, mapping, type, and runtime
  baselines, then verified both regressions against the unfixed code.
- 2026-10-01: Corrected section 4 of `ts7-semantic-unification.md`: the variant
  test deliberately uses declared cases, while the literal test uses the narrowed
  type. Added follow-up notes to TASK-712 and TASK-716 and the comment contract to
  `docs/ai/tt.md`.
- 2026-10-01: Started the default gates and full compiler/editor matrices and
  TypeScript typed parity. The TypeScript test checkout was fetched and verified.
  `scripts/ci agents`, `cargo fmt --check`, and all-target Clippy pass.
- 2026-10-02: The final gate passed all 415 library tests, 27 CLI unit tests,
  and two baseline-tracking tests. Full case baselines, compiler/editor matrices,
  and typed parity did not finish: orphaned test subprocesses exhausted the
  environment's process resources. Stopped the remaining test process groups
  to restore command execution. The user requested committing and pushing the
  fixes before restarting the environment and continuing verification. Keep
  this task in progress; these partial results do not establish full-gate success.
- 2026-10-02: Resumed in a fresh environment; `scripts/doctor` reported only
  the missing TypeScript install, resolved with `npm ci`. Reproduced the CI
  fuzz failure: `cargo check --manifest-path fuzz/Cargo.toml --all-targets
  --locked` refused to update `fuzz/Cargo.lock`. Ran `cargo update --workspace`
  for the fuzz package, which added only `stacker` to `swc_ecma_parser`'s
  dependency list (no version changes); the locked check then passed.
- 2026-10-02: Reproduced the performance failure locally with
  `scripts/bench-compare` (single file +16.7%, first snapshot +25.0%) and
  started bisecting the PR range on the single-file benchmark.
- 2026-10-02: A time-based `git bisect` over the PR range named TASK-645, an
  editor-only commit; single timed runs on this 4-core VM varied by more than
  the 10% budget, so that result was discarded (Decision 2).
- 2026-10-02: Counted instructions instead (Decision 2). One bench pass:
  `b9b85bd` 1.810 G, `019e328a` 2.055 G (+13.5%, matching CI's +11.8–14.6%).
  The callgrind profile of the head spreads the increase over lowering
  planning, emission, `ProgramSyntax::build_with`, lexing, and the host syntax
  check rather than one function. Sampled every seventh first-parent commit,
  then every first-parent commit in the five largest intervals (Decision 3).
- 2026-10-02: Profiled adjacent commits with callgrind and compared phase
  costs of `b9b85bd` and the head. Applied the three fixes in Decision 4:
  one pass of the benchmark fell from 3.687 G to 3.607 G instructions
  (+13.5% → +11.1% against `b9b85bd`). The rest is spread over the token
  facts machine and lexing (`lex_region` 486 M → 568 M), rope printing,
  the `ProgramSyntax` collector, and stack-growth checks, each a part of a
  feature rather than a duplicated computation found so far.
- 2026-10-02: With the fixes, `cargo fmt --check`, all-target Clippy, and the
  full `cargo test` (`RUST_TEST_THREADS=2`, exit 0; 415 library tests)
  passed. `scripts/bench-compare` still reported `single_file` +16.0% /
  +26.0%, `project_first_snapshot` +14.1% / +20.1%, and
  `project_recheck_one_file` +13.3% / +14.6% (median / fastest), over the
  10% budget.

### Verification recovery

Restart the environment to clear unreaped processes, preserve this checkout,
and run `scripts/doctor` before continuing. Wrap subsequent test commands in a
child subreaper and serialize extension compilation before editor execution.
Rerun the interrupted final gate, full compiler/editor matrices, and full typed
parity; do not treat the interrupted logs as completed runs.

## Issues and resolutions

### Issue 4: The fuzz lock file is out of date

- **Symptom**: CI's `cargo check --manifest-path fuzz/Cargo.toml --all-targets
  --locked` failed because the lock file needed updating.
- **Cause**: The PR made `swc_ecma_parser` depend on `stacker`, but the
  standalone fuzz package's lock file was not regenerated.
- **Resolution**: Regenerated `fuzz/Cargo.lock` with `cargo update --workspace`;
  the only change is the added `stacker` dependency edge.

### Issue 1: Yield context queries multiply with nesting depth

- **Symptom**: Lexing 100 nested arrays containing 100 yields visits 20,200
  frames; doubling both visits 80,400 frames.
- **Cause**: `Machine::yield_operator` scanned back to the enclosing
  non-inheriting statement list for every query.
- **Resolution**: Each grammar stack entry stores its effective `[Yield]`
  context. Push/pop/truncate and speculative probe restoration preserve
  that context; a query reads the top entry in constant time.

### Issue 2: Else-continuation comments are omitted

- **Symptom**: Comments before and after `else` disappear, including those
  between chained `if let` links.
- **Cause**: Neither the site's arm head nor its body owned the continuation
  trivia, so no lowering emitted it or checked its preservation.
- **Resolution**: HIR records the trivia as the site's trailing span. Core
  already carries that span and includes its comments in source preservation.
  The if-let emitter writes those comments before the continuation.

### Issue 3: Concurrent extension compilation invalidated an editor run

- **Symptom**: The first full editor-matrix run failed 233 cases with
  `a built extension server, checked before the run`.
- **Cause**: The final gate's `npm run compile` deletes `server/out` before
  rebuilding it. Running that mutation alongside tests which launch that
  server made the initial preflight insufficient.
- **Resolution**: Serialize extension compilation and editor-matrix execution;
  rerun the matrix after the gate finishes. This was a verification orchestration
  error, not evidence of an editor defect.

### Remaining findings from the PR comment

- TASK-692(a,c): The literal-zero type and the syntactic truthiness/nullishness
  diagnostics conflict with the current storage lowering and plain-TypeScript
  contract; TASK-692 records the attempted alternatives and their regressions.
- TASK-697: A directive's governed line takes precedence over JSDoc attachment;
  its Decision 2 explains the incompatible TypeScript leading-comment rules.
- TASK-699: The possible type-argument completion position remains open.
- TASK-709: The typed diagnostic cascade on an invalid nested or-pattern remains
  open; a recovery model for the rejected alternative is needed.
- TASK-715: The content-mapper wire schema still lacks diagnostic labels.
- TASK-716: TypeScript's exponential parser remains an upstream issue. No report
  was published, because the user did not request messaging upstream.
- TASK-721: Error recovery for a flow's first step can still lose the return type.
- TASK-725: The unified exhaustiveness contract uses the outer missing case;
  its finer nested witness is a separate enhancement.
- TASK-727: The TypeScript API forces a full check for the first global stage.
- TASK-730: A local multi-file reproduction with a declaration map naming
  `api.tt`, where a lowered variant precedes `work`, returned no definition
  location. The authored and served coordinate spaces need explicit provenance;
  no speculative coordinate heuristic was added. The temporary case was removed
  rather than committing an empty answer as the expected contract.
- TASK-641: Four vendored SWC grammar gaps remain listed in
  `tests/passthrough-triaged.txt`; they require AST/module/context model changes.
- Upstream JSX-spread semantic tokens and content-mapper module naming remain
  TypeScript behaviour. Real VS Code UI, large-project latency, and macOS/Windows
  editor behaviour cannot be established by these Linux test runs.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`,
  `yield_context_queries_do_linear_work_in_nested_expressions`.
- **Observed failure**: With only the work counter added to the original scan:
  `yield context probes: 20200 units for n matches but 80400 for 2n`.
- **Path**: `tests/cases/compiler/ifLetElseContinuationComments.tt`.
- **Observed failure**: Generated the fixed `.ts`/`.map.txt` baselines, restored
  the four non-test comment changes to the PR head, and reran the case: both
  baselines failed, with all four continuation comments absent (19 mappings
  became 15). The runtime result remains `2,6,0`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [ ] Full `cargo test` with baseline tracking and `node scripts/check-baselines`
- [ ] Full compiler and editor matrices
- [ ] Full TypeScript typed parity
- [ ] `scripts/ci agents` and extension tests
- [x] `scripts/ci agents`
- [x] New baseline changes reviewed

## Result

In progress.
