# TASK-792: Fix the round-eight editor findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-792: Fix the round-eight editor findings`

## Purpose

The eighth audit found eight editor findings (E1–E8). TASK-790 fixed E5
(compile time of nested pipeline heads). This task reproduces the other
seven on `f90ef7a2` with editor cases and TypeScript twins. It fixes the
ones that reproduce and records why the others need no change.

## Scope

- Included: rename inside a captured operand (E3), member completion
  after an unfinished arm (E7), and the cost of diagnostics and references
  on large files (E4).
- Excluded: E1, E2, E6, and E8, which do not reproduce (Decision 4).

## Decisions

### Decision 1: Restated text follows its source in a rename (E3)

- **Context**: Renaming `box` in `add(box, try parse("1"))` was refused.
  The operand moves into a capture typed `typeof box`, and TypeScript's
  rename also edits that `box`, which no mapping leads back to the source.
- **Decision and rationale**: The capture's type query is a restatement:
  the lowering writes it from the operand's source text, and records it
  (the emission's `restatements`). An edit inside restated text is covered by
  the edit of its source, so the rename skips it (`restated_target`)
  instead of refusing the whole rename.

### Decision 2: A completion probe closes lists where TypeScript ends them (E7)

- **Context**: After `A => try h(` in an unfinished match, `const w = o.`
  offered no members. The TypeScript twin offers `name`. The probe closed
  every open bracket at the cursor, so `const w = o.$tt_probe` stayed
  inside `h(...)`, where it is not TypeScript, and the probe failed.
- **Evidence**: TypeScript's parser ends a list at a token that starts a
  statement in an enclosing context (`abortParsingListOrMoveToNextToken`,
  the rule `src/parser/pipes.rs` already models for pipeline steps).
- **Decision and rationale**: `closed_at` closes an open argument list or
  index, and the expression brackets around it (a match body among them),
  before a statement keyword written directly in it. What remains open
  closes at the cursor, as before. Closers written before the cursor would
  shift the probe's coordinates, so the probe's mappings are mapped back to
  the text without them (`without_closers`). The pattern-completion path,
  which asks at a position before the cursor, uses a closed text only when
  no closer precedes that position.

### Decision 3: Measure a document once per answer (E4)

- **Context**: `tsDiagnostics` took 2.4 s for 3,200 diagnostics, and
  `references` 8.3 s for 8,000 uses, both growing with the square of the
  count. Under callgrind, `LineMap::new` and the UTF-16 conversions ran once
  per diagnostic or location over the whole text. Each location also
  re-read and compared its document's text, and was deduplicated by a
  linear search.
- **Decision and rationale**:
  - `ServiceDoc` answers positions and ranges with the line and UTF-16
    maps it already measures once (`source_range`, `source_offset`,
    `code_offset`).
  - Mapping one answer's targets back reads each document once
    (`ServiceSession::answering`), since the texts cannot change within
    an answer.
  - Locations, rename edits, and merged references are deduplicated
    through hashed keys.
  - ttc's instruction count now doubles with the count: 1.30G → 2.60G for
    4,000 → 8,000 references, and 0.39G → 0.83G for 1,600 → 3,200
    diagnostics.

### Decision 4: Findings that do not reproduce (E1, E2, E6, E8)

- E1: Signature help at the audit's positions, in a pipeline head
  (`add(v, v, try h(|)) |> String`) and on a nested callee
  (`try h(|k(1, try g()))`), answers `h` and `add` as TypeScript does on
  `cee1e118`.
- E2: Diagnostics inside moved operands (`m.a.b`, `g(1)`, and JSX in a
  `.ttx` file) have the same codes and ranges as the TypeScript twins.
- E6: After an unfinished guard (`Click(x) if x.`), the guard completes
  `at` and the statements after the match are typed (hover answers). The
  `match-not-exhaustive` report is the language rule: a guarded arm covers
  no case.
- E8: Signature help with the cursor before a `try` callee answers nothing,
  as TypeScript does outside an argument list.

## Work log

- 2026-10-08: Reproduced E1–E8 with editor cases and TypeScript twins in a
  worktree.
- 2026-10-08: Fixed E3 (`src/engine/language/service/targets.rs`,
  `src/engine/language/project.rs`), E7 (`src/engine/language/service.rs`,
  `src/engine/language/project/completion.rs`, `src/parser/partial.rs`,
  `src/parser/mod.rs`), and E4 (`src/engine/language.rs`,
  `src/engine/language/service.rs`, `src/engine/language/project.rs`,
  `src/engine/language/project/completion.rs`), profiling E4 with
  callgrind through `ttc --server`.

## Issues and resolutions

### Issue 1: The first E7 probe answered at the wrong byte

- **Symptom**: With closers written before the keyword, the probe
  compiled, but completion answered globals instead of `o`'s members.
- **Cause**: The probe mapped the cursor through mappings of the text with
  the closers, so the cursor landed two bytes early, on `o`.
- **Resolution**: Decision 2's `without_closers`.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/renameInACapturedOperand.tt`
- **Observed failure**: `prepareRename: refused null`, `rename: no edits`.
- **Path**: `tests/cases/editor/memberCompletionAfterAnUnfinishedArm.tt`
- **Observed failure**: `completion: 0 item(s), member false`; the TypeScript twin offers `name`.
- **Path**: `src/engine/language/tests.rs::diagnostics_and_references_measure_their_document_once_whatever_their_count`
- **Observed failure**: `left: 138, right: 258` line measurements for 20 and 40 diagnostics and references.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`

## Result

E3, E4, and E7 are fixed. E1, E2, E6, and E8 do not reproduce (Decision 4).
