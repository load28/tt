# TASK-767: Fix defects found by an editor audit

- **Status**: In progress
- **Started**: 2026-10-05
- **Completed**: —
- **Commit**: —

## Purpose

An audit of `ttc --server` found answers placed on the wrong source text
after an edit, completions read in the wrong context, and diagnostics the
editor merged that the command line reports separately. Fix each in the
layer that owns it.

## Scope

- Included: scoped materialization reuse across source edits, the extent of
  a value `try` operand ending in a member dot, completion in a recovered
  construct, completion in a tuple pattern slot, and the editor's merging of
  checker diagnostics.
- Excluded, recorded for a later round: completion inside a variant whose
  body is not closed (the recovery replaces the whole declaration), tokens
  and the unused-parameter hint around a declaration whose name is missing,
  and the editor's wording of checker errors on glue (the service path has
  no structured mismatch facts; the typed pass the adapter runs reports the
  command line's wording).

## Decisions

### Decision 1: A scoped materialization is reused only for its own source

- **Context**: After an edit that left a module's projected TypeScript
  unchanged (a malformed variant's stand-in does not depend on its text),
  hover, definition, diagnostics and tokens were shifted by the edit's
  length.
- **Alternatives considered**: Key the reuse on the whole lowered emission;
  add the module's source.
- **Decision and rationale**: The reuse key was the served text, which is
  what the checker reads, but the reused emission also carries mappings to
  the source. Mappings are a function of the source and the code, so the
  entry now also records its module's source digest; the whole-project path
  already compares the complete emission.

### Decision 2: A member dot without a name belongs to the `try` operand

- **Context**: `const v = try o.` completed the members of the `try`'s
  result, because the operand stopped before the dot.
- **Decision and rationale**: typescript-go's `parseRightSideOfDot` reads a
  `.` or `?.` not followed by a name as a property access with a missing
  name, so the operand includes the dot (not a dot followed by a private
  name, or `?.` followed by a call or an index).

### Decision 3: Completion in a recovered construct asks the probe

- **Context**: `Tri(p: |)` listed values: the cursor mapped into the
  malformed variant's stand-in, read as an expression position.
- **Decision and rationale**: A recovery placeholder is not authored text;
  hover already refuses to answer there. Completion skips the plain answer
  inside a recovered range and asks the probe, whose projection reads the
  field as a type.

### Decision 4: A tuple pattern slot is typed by its own scrutinee

- **Context**: `(North, |)` listed every visible variant's cases.
- **Decision and rationale**: The slot's position is counted from the
  tuple's top-level commas; the tags written at that position in the other
  arms are its evidence, and the typed question asks about that position's
  scrutinee, repairing an empty slot with `_` rather than a whole arm.

### Decision 5: Only diagnostics on glue are merged

- **Context**: Two TS2678 errors on two literal arms were shown as one.
- **Decision and rationale**: The command line merges diagnostics that mean
  one thing about one construct only when they land on generated code
  (`semantics::report`); the editor applied the merge to diagnostics on the
  user's own text too. It now follows the command line.

## Work log

- 2026-10-05: Reproduced each defect with a JSON-lines client against
  `ttc --server`, read the matching typescript-go parser code, and fixed
  `src/engine/project.rs`, `src/parser/{tries,partial,mod}.rs`,
  `src/engine/completions.rs`, `src/engine/declarations.rs`, and
  `src/engine/language/{project.rs,project/completion.rs}`. Built the VS
  Code adapter (`npm ci --prefix editors/vscode`, `npm --prefix
  editors/vscode run compile`) to run the editor cases.

## Issues and resolutions

### Issue 1: The first tuple slot stayed untyped

- **Symptom**: `(|, Slow)` still listed every case.
- **Cause**: The empty first slot does not project, and the retry inserted
  a whole wildcard arm (`_ =>`) inside the tuple.
- **Resolution**: A tuple slot is repaired with `_`.

## Regression test (fails before the fix)

- **Path**: `src/engine/language/tests.rs`
  `an_edit_that_keeps_the_projection_keeps_the_answers_on_the_new_source`;
  `tests/cases/editor/memberCompletionAfterAValueTry.tt`,
  `emptyVariantFieldTypeCompletion.tt`, `tuplePatternSlotCompletion.tt`,
  `literalArmDiagnosticsAreNotMerged.tt`.
- **Observed failure**: Without the source changes the hover range was
  `3:19-3:24` instead of `3:21-3:26`, and each editor case differed from its
  baseline (number members, value completions, every visible case, one
  TS2678).

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
