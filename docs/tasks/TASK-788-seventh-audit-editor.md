# TASK-788: Fix the seventh audit's editor findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-788: Fix the seventh audit's editor findings`

## Purpose

The seventh audit of the editor (at `a71c5729`) reported seven defects,
D1–D7. This task fixes all seven in the layer that owns each one.

## Scope

- Included: D1 (generated names of another script in completion), D2
  (signature help on an argument moved out of its call), D3 (compile time
  quadratic in the `try` operands of one expression), D4 (semantic tokens and
  document symbols quadratic in the length of one line), D5 (references inside
  a recovered construct), D6 (the similar-case quick fix introducing a
  duplicate arm), D7 (pattern completion after `if let` with a prefix).
- Excluded: the audit's observations that it did not count as defects (NUL
  and directory paths, `else if let` chain output size, `any`-typed let-else
  definitions, CRLF auto-import, which TypeScript shares).

## Decisions

### Decision 1: Completion hides every served file's generated names (D1)

- **Context**: A script's generated storage and helpers (`$tt_v0$total`,
  `$tt_show`) are global, so TypeScript offers them in every other file of
  the program. The completion filter only knew the names of the document
  being completed.
- **Decision and rationale**: `ts_completions` (`src/engine/language/service.rs`)
  drops a `$tt_` label that any served document generated. The names come
  from each document's own emission, so a user name that merely starts with
  `$tt_` is kept.

### Decision 2: The emitter records where a moved-out operand is read (D2)

- **Context**: `add(v, try g())` copies `v` into `const $tt_v2: typeof v =
  (v);` before the call, so a cursor on `v` maps outside the call and
  TypeScript has no signature to show. Only the emitter knows which slot read
  stands in an operand's place.
- **Alternatives considered**: Searching the output for the slot's name would
  tie the editor to a text shape (contract 3).
- **Decision and rationale**: The rope gains a `RelocatedOperand` mark pair:
  the slot read written in an operand's place (a source replacement, a
  value's slot, a deferred match's selected arm values) records the operand's
  source span and the read's output span, and `MappedEmit` carries them
  (`relocated_operands`, shifted with the other output offsets by contextual
  annotation and with the source offsets by recovery). `signature_position`
  asks at the read when the cursor lies in a moved operand, no invocation
  inside that operand holds it, and the read is an argument of a call; a
  cursor with no mapping (a match pattern) inside such an operand asks at the
  end of its read under the same condition. Every position of the
  audit's three repros now answers as TypeScript does for the same call
  written without tt.

### Decision 3: Shared input chains are read once per reader (D3)

- **Context**: The sibling steps of one call or operator chain share their
  input prefix (`Segments`), and four readers walked the whole chain at every
  step: the emitter's scheduled-step captures, the planner's replacement and
  closing-tag lists (which also built O(n²) duplicate replacements), and the
  nested-input index. Three emitter lookups also scanned every slot, Core
  expression, or compose action per query.
- **Decision and rationale**:
  - `Segments::fresh_under` reads a segment once per key; the planner keys by
    what the result depends on (`step.parent`, `callee_tested_step`), so the
    output is unchanged.
  - The emitter's captured set (`CapturedSlots`) remembers the segments whose
    captures it wrote. Their slots are all captured, so a later step skips
    them.
  - The planner's containment test for consumed values uses a `NestedOrder`.
    The emitter indexes slot names, piped steps, and deferred arm values once.
  - The emitter's replacement, value, statement, and nested-input walks use
    `NestedOrder` (`src/span_index.rs`) instead of scanning.
  - Instruction counts double with the input now: `try g() + …` 127M → 254M →
    509M → 1021M at 400 → 3200 operands, and `h(try g(), …)` 98M → 194M →
    386M → 772M. The emitted text is byte-identical to `6462aac7` on every
    benchmark.

### Decision 4: An ASCII line's columns are its byte offsets (D4)

- **Context**: Converting a byte offset to a UTF-16 or code-point column
  re-scanned the line from its start, so one long line made semantic tokens
  and symbols quadratic.
- **Decision and rationale**: `LineMap`/`LineIndex` record per line whether
  it is ASCII. Column conversions on an ASCII line take constant time. N=4000
  on one line: semantic tokens 3117 ms → 163 ms.

### Decision 5: Recovered text answers no name request (D5)

- **Context**: References at a recovered `match` returned the placeholder's
  mapped range. TASK-773 decision 8 says recovered text is not served.
- **Decision and rationale**: The service-name lookups return nothing when the
  cursor intersects a recovered span, and `locations()` drops a mapped target
  inside a recovered span of its document.

### Decision 6: The replacement skips cases the site covers; the error stays (D6)

- **Context**: `Delta` in a match that covers every case got the suggestion
  `Beta`, and applying it made a duplicate arm. GAP-1
  (`docs/design/rust-parity-analysis.md`) makes the suggestion the license to
  report, so dropping covered candidates from it also dropped the error (see
  Issue 1).
- **Decision and rationale**: The license stays the nearest declared case.
  The replacement offered as `help:` and as a quick fix is the nearest case no
  unguarded top-level sibling arm already covers. When none is left, the
  error is reported without a suggestion.

### Decision 7: A typed case prefix after `if let` keeps the subject's type (D7)

- **Context**: `if let G| = s {}` listed every known case. The statement
  position with a prefix was completed as if nothing followed it.
- **Decision and rationale**: `complete_pattern_at` replaces the prefix with
  the statement placeholder (with `()` when the case is not called), as it
  already did without a prefix, so the subject's type decides the cases.

## Work log

- 2026-10-08: Reproduced D1–D7 with the audit's scripts under
  `scratchpad/audit7/editor`.
- 2026-10-08: D1, D5, D6, D7 fixed and checked against their repros.
- 2026-10-08: D4: per-line ASCII flags in `src/lines.rs`.
- 2026-10-08: D3: `NestedOrder` walks in the emitter, deduplicated planning
  chains, then (profiling `try` sums and argument lists with callgrind at 800
  and 3200 operands) the slot-name, piped-step, deferred-arm and consumed-value
  lookups and the shared-segment reads. Output compared byte for byte with a
  `6462aac7` build on every benchmark.
- 2026-10-08: D2: `RelocatedOperand` marks, the signature-position redirect,
  and the unmapped-cursor redirect.
- 2026-10-08: Regression tests and baselines.

## Issues and resolutions

### Issue 1: Filtering the suggestion removed the unknown-case error

- **Symptom**: After the first D6 change, `ttc --check` on a match covering
  every case plus `Delta` exited 0 with no diagnostic.
- **Cause**: The resolver records an unresolved name only when it can name a
  suggestion (GAP-1). With every case covered, the filtered candidate list was
  empty.
- **Resolution**: Separated the license (nearest over all cases) from the
  replacement (nearest over uncovered cases); `replacement` is now optional
  and `resolution_errors` adds the suggestion only when there is one.

### Issue 2: A relocated read can hold another

- **Symptom**: Marking a deferred match's selected arm values, which hold arm
  values that are themselves slot reads, would have closed the outer mark on
  the inner entry.
- **Cause**: The printer matched an end mark with the last entry.
- **Resolution**: The printer keeps a stack of open relocated operands.

### Issue 3: A redirect lost the call that moved with the operand

- **Symptom**: `tests/native/editor_service.rs`
  `signature_help_names_a_stored_callee_as_the_source_call_does` failed:
  `two(match (s) { ... }, |)` answered `None` instead of `two(a, b)` at
  parameter 1.
- **Cause**: There the whole call moves into the arms (a completed call), so
  the value's read `$tt_v0` stands for the call, not for an argument of it.
  The redirect moved the cursor to that read (`return $tt_v0`), which is in no
  call; before the redirect the probe answered.
- **Resolution**: A redirect is taken only when the read is itself inside an
  invocation of the served code, so it lands in the call whose argument the
  operand was; otherwise the cursor keeps its place (and the probe). Every
  position of that statement answers as `a71c5729` did.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`
  `compiling_does_linear_work_in_the_number_of_try_operands_of_one_expression`
  (D3); `src/lines/tests.rs`
  `columns_on_one_long_ascii_line_take_constant_work_per_position` (D4);
  `tests/cases/compiler/anUnknownCaseIsNotReplacedByACaseAnotherArmCovers.tt`
  (D6); `tests/cases/editor/signatureHelpInAMovedOutArgument.tt` (D2),
  `completionOmitsAnotherScriptsGeneratedNames.tt` (D1),
  `referencesInsideARecoveredMatch.tt` (D5),
  `patternCompletionAfterAnIfLetPrefix.tt` (D7).
- **Observed failure**: With the same work counters placed on `6462aac7`:
  `captured slot checks: 5461 units for n matches but 21478 for 2n` (D3) and
  `column scan bytes: 10274896 for n statements on one line but 41055200 for
  2n` (D4).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`: 31 of 31 test binaries pass.
- [x] Baseline changes reviewed and committed with the change: only the new
  cases' baselines were added; no existing baseline changed.
- [x] The audit's repros (`c4`, `c5`, `c6`, and `two(match ..., |)`) answer
  at every position as TypeScript does for the same call, or as `a71c5729`
  did where it already answered.

## Result

All seven editor findings are fixed. Changed files: `src/chain.rs`,
`src/span_index.rs`, `src/lines.rs`, `src/resolve/mod.rs`, `src/sema.rs`,
`src/analysis/mod.rs`, `src/codegen/rope.rs`, `src/codegen/rope/builder.rs`,
`src/codegen/contextual.rs`, `src/codegen/core/{mod,planning}.rs`,
`src/codegen/core/planning/rewrites.rs`,
`src/codegen/core/emitter/{mod,host,source,pattern}.rs`,
`src/lib/{mapped,compile,recovery}.rs`, `src/engine/language.rs`,
`src/engine/language/{service,project,tests}.rs`,
`src/engine/language/project/completion.rs`, `docs/ai/tt.md`, and the tests
listed above. The CI `performance` check on `6462aac7`
(`project_first_snapshot` 13% slower than `main`) is a separate task.
