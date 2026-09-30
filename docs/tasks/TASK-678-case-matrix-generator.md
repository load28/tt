# TASK-678: Generate a case matrix of tt constructs in host positions, each run against a TypeScript twin

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-678`

## Purpose

tt's case suite had about 150 hand-written cases. TypeScript holds its
compiler to thousands of conformance cases, each feature written in many
contexts. This task builds the infrastructure to reach that scale for tt's
own constructs: a committed spec, a deterministic generator that writes
ordinary case files, and an oracle for every generated case, so that a
baseline records a behaviour something independent has agreed with rather
than whatever ttc happened to do. TASK-679 and TASK-680 populate it.

## Scope

- Included: `// @twin`, `// @expectErrors`, and `// @baselines` in
  `tests/case_baselines.rs`, `.ttx`/`.tsx` entries for `// @run`, the
  expected-failure list `tests/oracle-failures.txt`, the generator
  `scripts/generate-cases` and the spec's shared parts under `tests/matrix/`
  (`positions.mjs`, `variants.mjs`, `harness.ts`), sampling of the generated
  matrix with the nightly `exhaustive` step, the `unsampled` tracking state
  in `tests/common/baseline.rs` and `scripts/check-baselines`, and
  `CONTRIBUTING.md`.
- Excluded: the construct specs and the cases themselves (TASK-679,
  TASK-680), and every compiler defect the matrix finds (reported in those
  records, fixed by a later batch).

## Sources modelled

- microsoft/TypeScript `release-6.0` at `050880c`,
  `src/testRunner/compilerRunner.ts`: `CompilerBaselineRunner` enumerates
  `tests/cases/conformance` (5,908 files there, grouped by feature:
  `conformance/es6/destructuring/` holds 147) and `tests/cases/compiler`
  recursively, one baseline set per case; `src/harness/harnessIO.ts`,
  `getFileBasedTestConfigurations` and `splitVaryBySettingValue`: the
  `varyBy` fan-out that TASK-649 already follows, at most 25 configurations.
  A generated case here is an ordinary case file in that layout.
- microsoft/TypeScript `src/harness/evaluatorImpl.ts` and
  `src/testRunner/unittests/evaluation/` (TASK-652): a program is compiled,
  executed, and judged by what it observed. The twin extends that from "the
  baseline a reviewer read" to "what a hand-written program prints".
- D. R. Kuhn, R. N. Kacker, Y. Lei, *Practical Combinatorial Testing*, NIST
  Special Publication 800-142 (2010), chapters 1 and 5: t-way (here 2-way,
  "all pairs") coverage of a system's factors detects the faults that
  depend on the interaction of at most t factors, with a covering array far
  smaller than the full product, and constraints exclude invalid
  combinations. The greedy construction is the one-test-at-a-time strategy
  of AETG (D. M. Cohen, S. R. Dalal, M. L. Fredman, G. C. Patton, "The AETG
  System: An Approach to Testing Based on Combinatorial Design", IEEE TSE
  23(7), 1997), which SP 800-142 describes among the covering-array
  generators.

## Decisions

### Decision 1: The spec is a set of ES modules, the cases are committed

- **Context**: The matrix needs a form per construct (tt text and twin
  text), host positions that wrap a value or statements, and placement
  rules; the output must be reviewable and runnable by the existing case
  runner.
- **Alternatives considered**: (a) JSON with string templates: every form
  needs code in two languages with substitution points, which JSON can only
  hold as escaped strings with an ad hoc substitution syntax. (b) A new DSL:
  another parser to maintain. (c) Generating cases at test time: nothing to
  review, and a failure names a case that exists nowhere.
- **Decision and rationale**: `tests/matrix/*.mjs` modules whose default
  export describes a construct (its forms, each a pair of small template
  functions of the operand), `positions.mjs` (host positions),
  `variants.mjs` (a variant declaration and its twin, from the documented
  lowering), and `harness.ts` (the logging and driving helpers). The
  generator is plain Node with no dependencies, deterministic (no clock, no
  randomness, sorted inputs), and writes ordinary case files under
  `tests/cases/conformance/matrix/<construct>/`; `--check` fails when the
  committed cases differ, and `tests/case_baselines.rs`
  (`the_case_matrix_is_what_its_spec_generates`) runs it, so a hand edit to
  a generated case, or a spec change without regeneration, fails the suite.

### Decision 2: The oracle is a twin unit in the same case

- **Context**: A generated case needs a correctness oracle, so its baseline
  does not freeze wrong behaviour.
- **Alternatives considered**: (a) Baselines only: they record what ttc
  does; the first run of thousands of cases would be accepted unread.
  (b) Comparing with ttc's emit of a different form of the same program:
  both sides come from ttc. (c) A twin in a separate file tree: the runner
  would pair files by name, and the case would stop being self-contained.
- **Decision and rationale**: `// @twin: <unit>` names a `.ts`/`.tsx`
  unit of the case. The runner compiles the case once (the twin is
  hand-written TypeScript, which ttc passes through), emits JavaScript once,
  runs the `@run` entry and then the twin under the same sandbox, and the
  case disagrees when their stdout or exit status differ, or when the twin
  prints nothing (an empty twin proves nothing). A case that does not
  compile cleanly disagrees too: a twin case is written to be valid. The
  twin of each form is written in the spec from the construct's documented
  semantics, never from ttc's output: a `match` is a conditional chain over
  a temporary that holds the scrutinee (evaluated once, arms tested in
  order, guards only after their pattern); a `try` is `unwrap(r)`, which
  throws the failure out, and `caught(error)` around the body of the
  function the `try` leaves turns it back into that function's return value
  (so nothing after the failure runs, and `finally` and `using` run as a
  `return` runs them; a user `catch` in the twin rethrows it); a `result`
  block is a function run on the spot with the same wrapper, awaited when
  the block awaits; a let-else and an `if let` are the test, the else or
  body, and the destructuring. Every program logs its side effects through
  `note` and prints one line per input, so the comparison sees evaluation
  order, counts, and short-circuiting, not only values.

### Decision 3: Invalid combinations are diagnostic cases with `@expectErrors`

- **Context**: Some combinations are rejected by documented placement rules
  (`match` in a parameter default, `try` in a loop test or a generator); the
  task requires them kept as diagnostic cases rather than dropped.
- **Decision and rationale**: the spec states each rule where the
  documentation states it (`rejects` on a position, keyed by the construct
  or by a companion such as `yield`; `unbraced` on a statement form), and
  the generator writes such a row as a case with
  `// @expectErrors: <code>` and no twin. The runner requires every listed
  code to appear as `error[<code>]` in what ttc reports; a case that
  compiles cleanly, or reports other codes only, disagrees. Combinations
  that are not TypeScript at all (`await` in a class field initializer,
  `yield` inside a nested function, a companion that would wrap a module's
  top level) are infeasible, not rejected, and are not generated.

### Decision 4: Known disagreements are listed, and the list is exact

- **Context**: A disagreement is a compiler defect or a twin error. Twin
  errors are fixed; defects are not fixed in this batch, and the suite must
  still pass while saying so.
- **Alternatives considered**: (a) A generated `failingTwins.txt` baseline
  like the editor's `failingParity.txt`: the default run samples the
  matrix, so a whole-list baseline could only be compared in the exhaustive
  run. (b) Skipping known cases: a fixed defect would go unnoticed.
- **Decision and rationale**: `tests/oracle-failures.txt`, in the format of
  `tests/passthrough-triaged.txt`: the case name, the observation the
  failure prints, and the task that tracks it. A disagreeing case that is
  not listed fails with the diff or the diagnostics; a listed case that
  agrees fails ("remove the line"), a listed case observed differently
  fails, and in an unfiltered run a line that names no case fails. Only the
  cases a run executed are judged, so a sample judges its sample and the
  nightly run judges the whole list.

### Decision 5: Matrix cases keep `.stdout` and `.errors.txt` only

- **Context**: Thousands of cases would add thousands of `.ts`, `.types`,
  and `.map.txt` baselines.
- **Alternatives considered**: Every kind: the emitted TypeScript of
  near-identical programs churns on every lowering change, the `.types`
  hovers are the suite's most expensive part (TASK-652, Issue 2), and none
  of them is an oracle.
- **Decision and rationale**: `// @baselines: <kind>, ...` keeps the listed
  kinds (the default is every kind, so hand-written cases are unchanged).
  Generated runnable cases keep `errors.txt` (which must stay absent) and
  `stdout` (what the program printed, the reviewable record of the agreed
  behaviour; `.stderr` goes with it); diagnostic cases keep `errors.txt`.
  Their baselines live under `tests/baselines/reference/matrix/<construct>/`
  so the flat reference directory keeps the hand-written cases. With
  TASK-679's 1,617 cases this is 1,658 files, 0.7 MB.

### Decision 6: Pull requests run a fixed-seed sample; nightly runs all

- **Context**: A generated case costs one `ttc --out-dir`, one
  `ttc --check-types`, two `tsc` runs, and two `node` runs.
- **Decision and rationale**: following TASK-637/638, the default run takes
  `MATRIX_SAMPLE` (120) matrix cases with a splitmix64 shuffle from a fixed
  seed; `TT_MATRIX_CASES=<count>|all` and `TT_MATRIX_SEED=<number>` choose
  another sample, and a `TT_CASES` filter runs every matching case. The
  nightly `exhaustive` job gains a step with `TT_MATRIX_CASES=all`, and its
  timeout goes from 45 to 75 minutes. The baselines of the cases a run did
  not sample are recorded as `unsampled` in the tracking log, so
  `scripts/check-baselines` counts them as accounted for (and says how
  many) without judging them; a baseline whose case no longer exists is
  still unused, because only existing cases' files are recorded.

### Decision 7: Rows are all pairs of form, position, and companion

- **Context**: Forms × positions × companions is several thousand rows per
  construct; the full product adds little over pairs.
- **Decision and rationale**: every feasible (form, position) pair gets a
  row, and its companion is chosen greedily (AETG-style) to cover the most
  uncovered (form, companion) and (position, companion) pairs, ties going
  to the least-used companion; pairs still uncovered then get a row each.
  Feasibility is the constraint set of SP 800-142 §5. The companions are
  nothing, `await` in the operand, `yield` in the operand, an optional
  chain, a spread argument, an operand that throws with a `catch` around
  it, a `using` declaration, and a `finally` block.

### Decision 8: Each case carries only the harness it uses

- **Context**: The harness (logging, the driver, Result helpers) is about
  80 lines; copied whole into 1,617 cases it was 7.2 MB.
- **Decision and rationale**: cases stay self-contained (as TypeScript's
  are), and the generator writes into each only the harness declarations
  the programs reference, with their dependencies, and imports only those
  names (5.2 MB for TASK-679's cases). The twin's Result helpers are named
  `ok`/`err`/`Success`/`Failure`, so no harness name coincides with a tt
  pattern tag (`Ok(value: v)`) and is imported into a tt program.

### Decision 9: The fuzz and incremental corpora leave the matrix out

- **Context**: `tests/fuzz_regressions.rs`, `tests/incremental.rs`, and
  `scripts/fuzz-seed-corpus` take every case under `tests/cases`; the
  nightly mutation pass mutates every character of every unit.
- **Decision and rationale**: they skip `tests/cases/conformance/matrix`.
  The matrix's programs are repetitions of one spec, which would multiply
  the nightly mutation and incremental runs and the fuzzer's seeds without
  new shapes, and they have their own runner. Their pull-request samples
  are unchanged.

## Work log

- 2026-09-30: Reset onto `claude/ecstatic-dijkstra-qw5pf9` at `3686b96`,
  `npm ci` at the root and in `editors/vscode`, fetched the TypeScript
  cases, built the extension's server.
- 2026-09-30: Read `compilerRunner.ts` and `harnessIO.ts` at `050880c`,
  TASK-634/638/649/650/652, `docs/ai/tt.md`, and
  `docs/design/try-result-scopes.md`.
- 2026-09-30: Added the directives, the verdicts, the failure list, and
  sampling to `tests/case_baselines.rs`; wrote the generator and the shared
  spec; ran TASK-679's matrix repeatedly while correcting twin and harness
  errors (Issues 1 to 3).
- 2026-09-30: Merged `claude/ecstatic-dijkstra-qw5pf9` at `ba58c12`
  (TASK-677).
- 2026-09-30: Probed the oracle with a deliberate miscompile (below).

## Issues and resolutions

### Issue 1: Harness names and host names collided with the programs' own

- **Symptom**: `seen.push(seen)` failed with TS18046 in statement positions
  whose form declared its own `seen`; a sed over the spec missed names
  written after `\n`.
- **Cause**: The host and the form both declared `seen` in one scope.
- **Resolution**: Hosts write `collected`; forms keep `seen`.

### Issue 2: Narrow literal types made test arms incomparable

- **Symptom**: TS2367/TS2678 in generated code for a tuple arm `(_, Slow)`
  when the second scrutinee was `note("second", Speed.Fast)` (typed
  `{ kind: "Fast" }`), for `false =>` over `note("arm", true)`, for
  `case note("case", 0)` over a string, and TS2349 for
  `absent?.(...)` where `absent` was narrowed to `undefined`.
- **Cause**: The programs, not ttc: TypeScript reports the same comparison
  in hand-written code. (The twins did not, because their temporaries are
  `any`.)
- **Resolution**: `note<Speed>(...)`, `note<boolean>(...)`,
  `note("case", 0 as unknown)`, and a `callee(present)` helper whose type is
  `typeof pack | undefined`.

### Issue 3: `try` with a `yield` companion was expected to be rejected inside a `result` body

- **Symptom**: `tryStatement_propagate_resultBody_yield` reported only
  `result-yield-crossing`.
- **Cause**: The spec rejected function-targeted `try` in a generator
  everywhere, but a `try` in a `result` body targets the block, which is
  legal in a generator (try-result-scopes §4.5).
- **Resolution**: A position that retargets `try` (`resultBody`) does not
  apply the construct's function-target rules.

## Regression test (fails before the fix)

Not applicable: this task adds test infrastructure and fixes no compiler
defect. The oracle was shown to catch a miscompile instead: with
`emit_conditional_operation` (`src/codegen/core/emitter/host.rs`) testing
`LogicalAnd` where it tests `LogicalOr` (which swaps the branches of both),
`TT_CASES=_logical` failed 54 of its 56 generated cases with
"prints what its twin does not" (the other two are TASK-679's Issue 4), for
example:

```
match_tagBindings_logicalAnd_exception: prints what its twin does not
the entry's stdout (+) against twin.ts's (-):
- logicalAnd({"kind":"Circle","r":2}) => 4 | flip true; risky {"kind":"Circle","r":2}; circle 4
- logicalAnd({"kind":"Rect","w":3,"h":4}) => false | flip false
+ logicalAnd({"kind":"Circle","r":2}) => true | flip true
+ logicalAnd({"kind":"Rect","w":3,"h":4}) => 12 | flip false; risky {"kind":"Rect","w":3,"h":4}; rect 12
```

The change was reverted.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test case_baselines` with TASK-679's matrix present:
  both tests pass; "120 of 1617 matrix cases, seed 8391452271030987128".
- [x] The full gate is recorded in TASK-680, which ends this batch.
- CI time (debug build, four workers, this container): `case_baselines`
  takes 23 seconds without the matrix (`TT_MATRIX_CASES=0`) and 61 seconds
  with the default sample of 120, so a pull request pays about 40 seconds;
  the whole matrix of TASK-679 (1,617 cases) takes 544 seconds, about 0.34
  seconds of wall time per case on four workers.

## Result

Changed files: `tests/case_baselines.rs`, `tests/common/baseline.rs`,
`scripts/check-baselines`, `scripts/generate-cases`,
`tests/matrix/{positions.mjs,variants.mjs,harness.ts}`,
`tests/fuzz_regressions.rs`, `tests/incremental.rs`,
`scripts/fuzz-seed-corpus`, `.github/workflows/ci.yml`, `CONTRIBUTING.md`
("Adding a test case", "The case matrix"), `docs/tasks/INDEX.md`, and this
record. Follow-ups: TASK-679 and TASK-680 populate the matrix.
