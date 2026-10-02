# TASK-716: Read unclosed type-shaped brackets in linear time

Follow-up: [TASK-732](./TASK-732-pr-131-follow-up.md) removes the remaining
stack-depth scan in `Machine::yield_operator`. The original task made the
lookaheads linear; querying the enclosing `[Yield]` context is now constant
time as well.

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-716`

## Purpose

The converted case `verifyPreflightsUnbalancedDelimitersBeforeSwc` (a
fuzzed line of unclosed `K<[({[(`) opts out of its `.types` baseline
because the engine's semantic-tokens request times out after 8 s on every
run. This task finds the super-linear path behind it and makes the work
ttc does on such text linear.

## Scope

- Included: the lexer facts machine's lookaheads (`src/lexer/facts.rs`,
  `src/lexer/facts/types.rs`), the JSX tag scan that shares one of them
  (`src/lexer.rs`), and a work-count test (`src/lib/scaling_tests.rs`).
- Excluded: TypeScript's own parser, which is what the case's 8 s timeout
  waits on (Issue 1); the case keeps its `@baselines` opt-out
  (Decision 3).

## Sources

- TypeScript `src/compiler/parser.ts`: `isUnambiguouslyStartOfFunctionType`
  and `skipParameterStart` (the binding pattern after a type's `(`),
  `parseTypeArgumentsInExpression` with `canFollowTypeArgumentsInExpression`
  (a `<` after an operand), each answered by speculative parsing
  (`tryParse`/`lookAhead`), which the lexer facts machine mirrors with
  byte-level lookaheads.
- ECMA-262 §12 (the `InputElementRegExp` goal), which
  `Machine::operand_expected` decides for a `/`, a `.` before a digit, and
  in `.ttx` a `<`.
- TASK-654 (the work-count tests, `crate::work`, and the lexer's bracket
  pair table).

## Decisions

### Decision 1: One scan answers every nested start it passes

- **Context**: `type_arguments_end` (a `<` after an operand) and
  `expression_end` (a computed name or initializer in the binding pattern
  after a type's `(`) each read forward until their brackets close. In
  unclosed text they read to the end of the region, and every nested `<`
  or `[` asks again: quadratic in the openers (n=20 copies of the fuzzed
  line: 1 017 872 expression steps; 2n: 4 147 312).
- **Alternatives considered**: (a) A bracket pair table over the region,
  computed before lexing: the lookaheads read strings, templates, and
  comments by their own approximate rules from where they start, and a
  table computed from the region's start can disagree with that reading
  (a quote inside a regular expression or JSX text), so its answers would
  differ from the lookahead's. (b) A bound on the distance a lookahead
  reads: changes answers for long, valid type arguments. (c) Memoize each
  start's answer, and let one scan record the answer of every nested start
  it passes: a nested start's scan reads the same bytes in the same way as
  the enclosing scan from that point, so its answer is known exactly when
  its bracket closes (or when the enclosing scan stops while it is open).
- **Decision and rationale**: (c), as `Lookaheads`, owned by the machine
  of a region and shared with the JSX tag scan of that region. Every
  answer is the one the unmemoized scan gives; the scans that still run
  start where no earlier scan has read, so they read disjoint stretches.

### Decision 2: The operand probe puts back the frames it reached instead of copying the stack

- **Context**: `operand_expected` copied the whole frame stack for every
  `/`, `.` before a digit, and (in `.ttx`) `<`, then offered the byte to
  the copy. With deeply nested openers the stack is as deep as the text is
  long, so `.ttx` text of 140 000 unclosed openers took 18.6 s (debug).
- **Alternatives considered**: (a) Copy only the top 4 097 frames (the
  probe stops after 4 096 steps): correct only while no step reads deeper
  frames, which `yield_operator` does. (b) Probe the real stack and undo:
  record each original frame the probe pops, then truncate to the lowest
  height reached and push those frames back.
- **Decision and rationale**: (b). It reads the real stack, so every step
  sees what it saw before, and it costs the frames the probe touched. The
  machine's other fields a step can change (`facts`, `skip_next`,
  `last_end`, `statements`) are saved and restored around the probe as the
  copied machine had them.

### Decision 3: The case keeps its opt-out; the timeout is TypeScript's

- **Context**: With the lexer linear, the case's `.types` baseline still
  times out: TypeScript's parser takes 18 to 26 s on the 134-byte line
  (Issue 1).
- **Alternatives considered**: (a) Withhold a file whose delimiters ttc
  finds unbalanced from the TypeScript service: every unclosed `(` while
  typing would lose hover, completion, and signature help. (b) Raise the
  8 s service timeout: the editor would wait 20 s. (c) Keep
  `@baselines: ts, errors.txt, map.txt` and record why.
- **Decision and rationale**: (c). The work ttc does is pinned by the new
  work-count test instead; the case still pins the verify diagnostic and
  the emit.

## Work log

- 2026-10-01: Reproduced the timeout by removing the case's `@baselines`
  line (`TT_CASES=verifyPreflightsUnbalancedDelimitersBeforeSwc cargo test
  --test case_baselines`: "TypeScript language service request
  `textDocument/semanticTokens/full` timed out after 8s").
- 2026-10-01: Timed `ttc --server` `semanticTokens` and `check` on the line
  repeated 400 times (53 600 bytes, 9.9 s) and on 20 000 copies of
  `K<[({[(` (289 s); sampled stacks with gdb: `starts_function_type` →
  `skip_parameter_start` → `binding_pattern_end` → `expression_end`, and
  `type_arguments_follow` → `type_arguments_end`; in `.ttx`,
  `operand_expected` copying the stack.
- 2026-10-01: Added `Lookaheads` and the probe's undo; the same inputs take
  0.15 s and 0.35 s (`.ttx`: 0.18 s and 0.46 s, from 1.5 s and 18.6 s).
- 2026-10-01: Timed `tsc --noEmit` (7.1.0-dev.20260826.1 and 5.9.3) on
  the case's line and its prefixes (Issue 1).

## Issues and resolutions

### Issue 1: TypeScript's parser is exponential on this text

- **Symptom**: `tsc --noEmit --extendedDiagnostics` on the case's line as
  a `.ts` file reports "Parse time: 18.223s" (TypeScript
  7.1.0-dev.20260826.1) and "Parse time: 25.56s" (5.9.3). Prefixes of 60,
  90, and 120 bytes take 0.18 s, 0.44 s, and 10.8 s.
- **Cause**: The speculative parses above nest: each unclosed `(`, `<`,
  and `[` is tried as an arrow function's parameters, type arguments, or a
  pattern, and each failed attempt is parsed again as an expression.
- **Resolution**: Not ttc's to fix (Decision 3); the engine's 8 s service
  timeout bounds what the editor waits. To be reported to TypeScript with
  the line as the reproduction.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`,
  `every_request_does_linear_work_in_unclosed_type_shaped_openers`
  (`cargo test --lib`).
- **Observed failure**: With the changes in `src/lexer.rs`,
  `src/lexer/facts.rs`, and `src/lexer/facts/types.rs` reverted except the
  work counters: "TypeScript expression lookahead steps: 1017872 units for
  n openers, 4147312 for 2n, 16742192 for 4n".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib`, and `--test compile --test passthrough --test
  case_baselines --test corpus --test editor_cases` with
  `TTC_REQUIRE_TSGO=1`; the full gate is recorded in TASK-715.
- [x] Baseline changes reviewed and committed with the change (none).

## Result

Changed `src/lexer.rs`, `src/lexer/facts.rs`, `src/lexer/facts/types.rs`,
and `src/lib/scaling_tests.rs`. The lexer facts machine's lookaheads and
operand probe do work linear in the input however its brackets nest or
stay open; the case's remaining timeout is TypeScript's parser (Issue 1).
