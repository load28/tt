# TASK-771: Keep a logical test's narrowing and fix the fourth audit's remaining editor findings

- **Status**: Complete
- **Started**: 2026-10-06
- **Completed**: 2026-10-06
- **Commit**: —

## Purpose

TASK-770 moved these fourth-audit findings here. A tt value in the right
operand of a `&&` or `||` test loses the narrowing TypeScript gives the
test's body. Several editor requests grow superlinearly with file size or
with the number of TypeScript errors. Nested patterns under a generic
payload field are invisible to editor features. Or-patterns and
unreachable-arm hints answer inconsistently.

## Scope

- Included: the narrowing of an `if`, `while`, or `for` test whose
  right operand holds a tt value (compiler); per-request cost in the
  editor engine and typed check (E3–E5); nested patterns under a generic
  payload field (E6); or-pattern hover, completion, and document symbols
  (E8); unreachable-arm hints beside duplicate-arm errors (E9).
- Excluded: new language surface.

## Decisions

### Decision 1: A logical test is lowered as guards, so its body keeps the narrowing

- **Context**: `if (typeof x === "string" && match …) x.toUpperCase()` was
  TS2339. The operation was lowered into `let $tt_v2` before the `if`, and
  the `if` tested that slot. TypeScript narrows only from the condition it
  sees, and a `let` alias carries no narrowing. Loops lowered their test
  the same way.
- **Alternatives**: Annotate or alias the condition, which TypeScript
  narrows through only for `const` aliases of unreassigned references
  (aliased-condition analysis); or duplicate the `else` branch.
- **Decision and rationale**: When a `&&` or `||` with a tt value in its
  right operand is the whole test of a statement, the left operand is
  written as the statement's own guard, the right operand is lowered after
  it, and its value is tested last. Every path to the body then passes
  TypeScript a test it narrows on, and the code after the statement sees
  the same flows as the source.
  - `while`/`for` with `&&`: `if (!(A)) break; <right>; if (!(r)) break;`.
  - `while`/`for` with `||`: `if (!(A)) { <right>; if (!(r)) break; }`.
  - `if` without `else`, `&&`: `if (A) { <right> if (r) S }`.
  - `if` without `else`, `||`: `l: { if (!(A)) { <right>; if (!(r)) break
    l; } if (true) S }`. The code after the statement is reached with A
    and r false, as in the source.
  - `if` with `else`, `&&`: a flag records that S ran, and `if (f) {} else
    E` keeps the authored `else E`. The narrowing E has in the source is
    the union of A false and A true with r false, which is no narrowing of
    A, so E loses nothing.
  - `if` with `else` and `||` keeps the slot form. A guard form would have
    to write E before S or duplicate one of them.
  The `if` facts (test, consequent, alternate) are read from SWC
  (`ProgramSyntax::if_tests`).

### Decision 2: Function-body placement is computed once per statement stream

- **Context**: `check`, `ttSymbol`, and `ttHints` grew quadratically with
  the number of `try`, let-else, and `if let` statements (2.2 s at 16k
  lines). Each statement asked `in_function_body`, which scanned the token
  stream from its start.
- **Decision and rationale**: The parser keeps one incremental table per
  token slice (`flow::FunctionBodies`) and extends it only as far as a
  question reaches, so a nested region still scans no further than it
  did. Two more scans had the same shape. `edits_after_statement` looked
  at every statement of the body for one holding an edit, where only the
  neighbours can (statements of a body are in source order and do not
  overlap). `completion_scopes` searched every scope for each construct,
  where one sweep in source order keeps only the open scopes. Measured on
  the audit's files: let-else at 16k lines went from 2156 ms to 92 ms for
  `ttSymbol`, 1235 ms to 277 ms for `check`, and 1502 ms to 62 ms for
  `ttHints`.

### Decision 3: An or-pattern's shared binding is a document symbol

- **Context**: `documentSymbols` listed every pattern binding except an
  or-pattern's shared one, which the emission writes as one glue name for
  several source occurrences.
- **Decision and rationale**: A symbol whose name lies in a shared binding
  maps to the binding's first occurrence, as diagnostics already do
  (`shared_binding_origin`).

### Decision 4: The alternatives an arm already writes are completion evidence

- **Context**: At `match (s) { Circle | § }` the parse-only list offered
  every case in scope, and `Circle` was not marked covered. The finished
  arms were evidence, but the arm being written was not.
- **Decision and rationale**: The tags written before the cursor in the
  current arm count as evidence and as covered, the same way an `if let`
  pattern's alternatives do (TASK-770 decision 21).

### Decision 5: An or-pattern's field label hover is not changed

- **Context**: The audit saw `hover` return null on `width` in
  `Sq(width: q) | Rect(width: q)`.
- **Decision and rationale**: `hover` is the TypeScript layer. The editor
  asks `ttSymbol` first for a tt name and shows its answer
  (`editors/vscode/server/src/server.ts`, `onHover`), and `ttSymbol`
  answers this label. Not a defect.

### Decision 6: A file's lines and UTF-16 offsets are measured once per check

`ttc --check-types` grew quadratically with the number of TypeScript
errors (400 errors: 2.1 s, 1600: 15.3 s). Sampling the release binary
showed every reported diagnostic rebuilding a line table
(`crate::line_col`) and rescanning the emitted text for a UTF-16 offset
(`mapper::from_utf16`), the CLI renderer measuring the whole file again per
diagnostic, and the host walking every top-level statement of the file to
find the node a diagnostic starts at.

- TypeScript measures a file once (`getLineStarts` caches the line map on
  the source file; tsgo's `ast.PositionMap` records only the non-ASCII
  characters and answers both directions by binary search). `lines::Utf16Map`
  is that structure, and `ProjectedDocument` caches its source `LineIndex` and
  its emitted code's `Utf16Map` in `OnceLock`s, as it already caches its
  exported variants. The report measures a hand-written file once per path.
- `render::render_measured` and `engine_diagnostic_measured` draw against a
  measured `LineMap`; the CLI measures each reported file once. `render` and
  `engine_diagnostic` keep their signatures and measure for one call.
- The host's position lookups (`contextualMismatch`, `lookupReceiver`,
  `smallestExpressionCovering`) use `walkContaining`, which binary-searches a
  node's cached children for the ones holding the range, as
  `getTokenAtPositionWorker` does, instead of visiting every sibling.
- Rejected: a cache keyed by text inside `line_col`. The owner of the text
  is the projected document, so the measurement lives with it.

After the change 1600 errors take 4.0 s and 6400 take 21.6 s. The remaining
growth is inside tsgo: `getSemanticDiagnostics` on a content-mapped file
takes 1.2 s for 3200 diagnostics and 12.6 s for 12800 through the API alone
(no ttc involved), and plain `tsc --runExternalCode` with the same identity
mapper takes 2.5 s and 25.9 s, its CPU profile spent in
`diagnosticwriter.newOriginalTextFile` recomputing `ComputeECMALineStarts`
per diagnostic. Unmapped, the same check takes 0.5 s. It is recorded as an
upstream issue and not worked around.

### Decision 7: Projection segments are recorded without shifting, and host spans are looked up by index

`ttc --check` on a file of N match expressions was quadratic (8000 matches:
2.6 s, 16000: 9.3 s). Two places scanned or shifted the whole file per match:

- `emit_decision_region` and the result region recorded their placeholder
  segment with `Vec::insert(0, ..)`, moving every segment written so far.
  Consumers read segments in order (`find`, `in_segment_order`), so the order
  is kept exactly: `SegmentList` holds the front-recorded segments in their
  own stack and yields the same sequence a vector would
  (`a_segment_list_keeps_the_order_a_vector_would` compares them under random
  operations). A statement decision's `insert(segment_index, ..)` keeps its
  index semantics, shifting only the segments written after its start.
- `completion_scopes` mapped each decision's subject span by scanning every
  segment. It now reads the segments starting inside the span from the
  projection's `SpanIndex`, which the if-test collector already builds; the
  two share one index.

After the change 16000 matches take 2.7 s.

### Decision 8: A nested pattern position under a generic payload is completed from TypeScript

- **Context**: `Has(item: §)` over `Opt<Shape>` offered nothing. The
  parse-only list follows the field's declared type, and `T` names no
  variant (by design: a type parameter does not transfer ownership,
  `docs/ai/tt.md`). The typed request (`patternCompletions`) asked the
  checker only at the arm's top level.
- **Decision and rationale**: The emitter now records where it writes the
  tag literal a nested pattern's receiver is compared with
  (`PayloadTemp::tag`, the `"Circle"` of `$tt_m.item.kind === "Circle"`),
  beside the receiver it already recorded for typed exhaustiveness. A
  nested position (`TypedSite::Nested`) lowers the arm with a placeholder
  tag, replaces that literal with the probe, and asks TypeScript's
  completions there, as the top-level path asks after `(scrutinee).kind
  === `. The arm's own condition narrows the receiver, so the answer is
  the payload's type at that arm, at any depth. When exactly one visible
  variant has every tag TypeScript offers, its cases supply the details.
  A field whose declared type names a variant keeps its parse-only answer,
  in declaration order: that answer is already exact, and the typed path
  is asked only where it is empty.
- **Not changed here**: hover and definition of a nested tag under a
  generic payload. `ttSymbol` is parse-only by its protocol contract
  (`src/server.rs`), and a typed answer needs a new project-backed
  request and the VS Code client asking it. Proposed as a follow-up task.

### Decision 9: An unreachable-arm hint beside a duplicate-arm error is kept

- **Context**: A duplicate arm draws the `match-duplicate-arm` error and
  the editor's unreachable-arm hint over the same arm.
- **Decision and rationale**: They state two facts at two layers. The
  error is the rule the build enforces; the hint is the editor's dimming of
  code that cannot run (`src/engine/hints.rs`), which a duplicate arm also
  is. An editor shows a diagnostic and an "unnecessary" tag on the same
  range the same way TypeScript reports TS7027 or TS6133 beside an error.
  Not a defect.

## Work log

- 2026-10-06: Started from the findings TASK-770 moved here.
- 2026-10-06: Lowered a logical statement test as guards (decision 1).
- 2026-10-06: Made function-body placement, statement placement, and
  completion scopes linear (decision 2); fixed or-pattern document symbols
  and arm completion (decisions 3 and 4); reviewed or-pattern hover
  (decision 5).
- 2026-10-06: Profiled `--check-types` (E4) with symbol-carrying release
  builds and gdb stack samples; cached line and UTF-16 measurements,
  measured CLI rendering once per file, and bounded the host's position
  walks (decision 6). Profiled tsgo with `--pprofDir` and request timings
  and traced the rest to tsgo's content-mapper path.
- 2026-10-06: Profiled `--check` on match-heavy files (E3); removed the
  segment shifting and the full scans of host spans (decision 7).
- 2026-10-06: Added typed nested-position completion (decision 8) and
  reviewed the hint beside a duplicate-arm error (decision 9).

## Issues and resolutions

- **Guard condition rendered as the result slot**: the guard's condition
  printed `$tt_v2` in place of its operand. Cause: the condition was
  rendered outside a conditional region, so a nested value read its slot.
  Resolution: the guard raises `conditional_region_depth` while it writes
  the condition and the right operand.
- **An `||` loop guard with a tt condition printed `$tt_v2{ work(); }`**.
  Cause: the replacement-capture path wrote the operation's slot where the
  test was skipped. Resolution: a guarded loop operation's replacement is
  empty text.
- **The first function-body cache was quadratic in nesting depth**: it
  rebuilt a full table per slice. Resolution: the table is incremental and
  kept per slice.
- **E4's remaining growth is in tsgo**: after decision 6, 6400 errors still
  took 21.6 s. Cause: tsgo's semantic diagnostics on a content-mapped file
  (API 12.6 s for 12800 diagnostics; `tsc --runExternalCode` profile in
  `diagnosticwriter.newOriginalTextFile`, which recomputes line starts per
  diagnostic). Resolution: recorded as an upstream issue; no workaround.
- **A cached UTF-16 map outlived the emit it measured**: the first full
  run showed diagnostics on user code reported as glue (`(in code ttc
  generated for this construct)` lost or gained, translated wording on
  TS2339). Cause: `Project` replaces a document's emit after contextual
  refinement, and the cloned `OnceLock` kept the measurement of the earlier
  text. Resolution: `ProjectedDocument::replace_emit` replaces the emit and
  resets its measurement; both replacement sites use it.
- **Nested completions over a declared variant changed order**: the typed
  path listed them in TypeScript's order. Resolution: the typed path is
  asked only where the parse-only answer is empty (decision 8).
- **A clippy failure reached a commit**: the commit command ran after a
  clippy pipe whose status it did not check. Resolution: fixed in the next
  commit; gates are run and read before committing.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aLogicalTestKeepsItsNarrowingBesideATtValue.tt`;
  `src/lib/scaling_tests.rs`
  (`every_request_does_linear_work_in_the_number_of_statement_decisions`,
  `a_check_measures_each_file_once_however_many_diagnostics_it_reports`,
  `every_request_does_linear_work_in_the_number_of_statement_matches`,
  `every_request_does_linear_work_in_the_number_of_expression_matches`,
  `every_request_does_linear_work_in_the_nesting_depth_of_matches`);
  `tests/native/editor_service.rs` (`an_or_pattern_binding_is_a_document_symbol`);
  `tests/cases/editor/` (`orPatternArmCompletionCountsItsWrittenAlternatives`,
  `nestedPatternCompletionUnderAGenericPayload`).
- **Observed failure**: the narrowing case failed with TS2339 on
  `x.toUpperCase()` in the test's body. The statement-decision scaling test
  failed on missing linear work ticks. The measurement test failed with
  `line measurements: 160 for 80 diagnostics but 320 for 160`. The match
  scaling tests failed with `completion host segments: 360300 units for n
  matches but 1440600 for 2n`. The document-symbol test found no symbol for
  the shared binding. The or-pattern completion case listed every case in
  scope without `covered`. The nested completion case had no
  `patternCompletions` items.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --no-fail-fast` with `TTC_REQUIRE_TSGO=1`: every target
  passed (exit 0) after the fixes in "Issues and resolutions".
- [x] Baseline changes reviewed and committed with the change: the loop
  and `if` guard outputs (decision 1), the new editor cases, and the public
  API additions (`render_measured`, `engine_diagnostic_measured`,
  `PayloadTemp::tag`).
- Measurements: `--check-types` with 1600 TypeScript errors 15.3 s → 4.0 s;
  `--check` with 16000 matches 9.3 s → 2.7 s.

### Changed files

- Compiler: `src/program_syntax.rs`, `src/program_syntax/projection.rs`,
  `src/program_syntax/projection/segments.rs`,
  `src/program_syntax/completion.rs`, `src/codegen/core/planning.rs`,
  `src/codegen/core/emitter/` (host, source, pattern),
  `src/codegen/rope.rs`, `src/codegen/rope/builder.rs`,
  `src/codegen/contextual.rs`, `src/flow/`, `src/parser/`,
  `src/lines.rs`, `src/lib/mapped.rs`, `src/typescript/mapper.rs`.
- Engine and CLI: `src/engine/projection.rs`, `src/engine/project.rs`,
  `src/engine/semantics/report.rs`, `src/engine/semantics/translate.rs`,
  `src/engine/completions.rs`, `src/engine/language/service.rs`,
  `src/engine/language/project/completion.rs`, `src/render.rs`,
  `src/main/typed.rs`, `src/typescript/host.mjs`.
- Tests and baselines: `src/lib/scaling_tests.rs`, `src/lines/tests.rs`,
  `src/program_syntax/tests.rs`, `tests/native/editor_service.rs`,
  `tests/cases/compiler/aLogicalTestKeepsItsNarrowingBesideATtValue.tt`,
  `tests/cases/editor/orPatternArmCompletionCountsItsWrittenAlternatives.tt`,
  `tests/cases/editor/nestedPatternCompletionUnderAGenericPayload.tt`,
  `tests/baselines/reference/`.

## Result

Complete. The logical-test narrowing (decision 1) and the editor and
check costs (E3–E5, decisions 2, 6, 7) are fixed; or-pattern answers
(E8) are fixed or reviewed; nested completion under a generic payload
(E6) is fixed, with typed hover and definition proposed as a follow-up;
the hint beside a duplicate-arm error (E9) is kept by decision 9. The
remaining `--check-types` growth is tsgo's content-mapper path, recorded as
an upstream issue.
