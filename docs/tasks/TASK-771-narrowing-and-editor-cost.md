# TASK-771: Keep a logical test's narrowing and fix the fourth audit's remaining editor findings

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
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

## Work log

- 2026-10-06: Started from the findings TASK-770 moved here.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
