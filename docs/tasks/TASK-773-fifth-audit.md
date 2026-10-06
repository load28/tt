# TASK-773: Fix defects found by the fifth audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

The fifth audit of the CLI, the compiler, and the editor engine found new
defects. This task fixes them in the layer that owns each one.

## Scope

- Included: the CLI findings C1–C10, the editor findings E1–E7 and the
  smaller completion gaps, and the compiler findings of the same round.
- Excluded: tsgo's own defects (the content-mapper diagnostic cost, a
  signature-help timeout that a plain `.ts` file shows too).

## Decisions

### Decision 1: Paths are compared by an identity measured once (C2)

- **Context**: `--check`, the build, and `--watch` were quadratic in the
  number of input files: 3,200 empty files took 29.4 s for `--check`, and
  `--watch` over 1,600 took 43.7 s to start and 34.5 s per saved file.
  `strace` counted about 1.3 million `readlink` and `getcwd` calls for 800
  files. `build_jobs` and `compile_jobs` compared every job with every
  earlier one through `same_file`, which canonicalizes both paths per call.
- **Decision and rationale**: `file_identity` computes what `same_file`
  compared — the canonical path of an existing file, the canonical
  directory and name of one that does not exist yet, and the normalized
  path otherwise — once per path, and the job deduplication, the
  compiled-output filter, and the overlapping-root check hash those
  identities. `same_file` is the equality of two identities, so every
  answer it gave is unchanged. After the change 3,200 files check in
  0.24 s, and `--watch` over 1,600 starts in 0.42 s and rebuilds an edit in
  0.43 s.

### Decision 2: A file outside the program that cannot be read does not stop the check (C1)

- **Context**: A dangling `.tt` symlink or a non-UTF-8 `.tt` anywhere under
  the project root, outside `include`, stopped `--check-types`, `--types`,
  `--dependencies`, and the server's `typedCheck` with `ttc: No such file or
  directory (os error 2)` (no path) or `cannot read`, and exit 2. `tsc`
  checks the same project normally.
- **Alternatives considered**: Report every unreadable candidate, which
  makes a file the configuration excludes decide whether the check runs;
  skip every unreadable file, which would drop a named input or an
  imported module without a word.
- **Decision and rationale**: The project scan only proposes candidates;
  TypeScript's program decides membership (TASK-764 decision 4), and an
  entry the scan cannot read is one module the checker does not get
  (TASK-356 decision 2). The scan now treats a symlink as its target and
  skips one whose target cannot be read, as TypeScript's directory listing
  does (`getAccessibleFileSystemEntries` in `sys.ts`). A scanned file that
  cannot be read is skipped unless an input names it or a projected file
  imports it; then reading it is part of the check, and the failure is
  reported with its path as before. The command line's own walk of a
  directory input still names an unreadable entry (TASK-387 decision 1),
  so `--check`, the build, and the typed modes agree on inputs.

### Decision 3: A payload column's alphabet is the one asked in that match (E1)

- **Context**: With `Box<T>` matched as `Box<Color>` in one function and
  as `Box<Shape>` in another, `--check-types` and the editor reported the
  second, exhaustive match as missing `Full(item: Red())`. The typed pass
  asks the checker for each nested pattern's alphabet at its own lowered
  receiver, but the answers were keyed by `(constructor, field)` per file,
  and the first answer for `(Full, item)` stood for every match.
- **Decision and rationale**: An answer keeps the offset of the nested tag
  it was asked at (`PayloadAnchor::offset`, `PayloadAlphabet`), and a match's
  coverage reads only the answers asked at nested patterns inside its own
  arm patterns (`payload_columns`). The column's alphabet is then the
  payload's type at that match, which is what the question asked.

### Decision 4: Editor requests measure a text once and look mappings up by position (E2)

- **Context**: On a 28,000-line file of match-heavy functions,
  `documentSemanticTokens` took 19.6 s, `typedCheck` 14.1 s,
  `documentSymbols` 3.7 s, and references 20.8 s, each growing faster than
  the file. Stack samples and request timings pointed at per-item scans:
  the token list's duplicate check (`out.contains`), the token merge's
  comparison of every service token with every tt token, an output-offset
  lookup that scanned every mapping (`to_source`, `to_source_span`,
  `to_source_inclusive`), a UTF-16 conversion that rescanned the served
  text for every position (`ServiceDoc`), and one more per contextual slot.
- **Decision and rationale**:
  - The duplicate check hashes each token's range, type, and modifiers.
  - The merge indexes the service's modifiers by range and type, and keeps
    tt's token columns per line, sorted, with the furthest end so far, so
    whether a service token overlaps one is a binary search.
  - Mappings are in output order and do not overlap
    (`MappedEmit::mappings`), so an output offset is found by binary search.
    Each place that finalizes mappings (`Rope::flatten`, contextual
    refinement, recovery) asserts that order in debug builds
    (`mapper::in_output_order`).
  - `ServiceDoc` measures its source and served code once
    (`source_utf16`, `code_utf16`), as `ProjectedDocument` does since
    TASK-771, and the contextual pass measures each module once per round.
  On the same file the requests now take 3.3 s, 9.1 s, 1.1 s, and 13.4 s,
  and grow about 2.2× per doubling. What remains is TypeScript's own work
  (the server waits on tsgo during `typedCheck` and references).

### Decision 5: A recovery that unclaims a `result` block recovers the block (E3)

- **Context**: In the editor, a `result` block whose only direct `try` sits
  in a misplaced position (`while (try r())`) drew `ts2304 Cannot find name
  'result'` and `ts1005` beside `try-placement`; the command line reports
  only `try-placement`. The editor projection recovers the misplaced `try`
  as a placeholder, and the parser claims a `result` block only by the
  direct `try` it holds, so after the recovery the block's text stayed as
  written and TypeScript read it as code.
- **Decision and rationale**: Each recovery round compares the `result`
  blocks the parser claims before and after it
  (`parser::claimed_result_blocks`). A block the recovery unclaimed is
  itself recovered, as an expression placeholder, before the round goes
  on: one placeholder owns the construct that lost its reading. The tt
  diagnostic stays, and nothing of the block is read by TypeScript, as on
  the command line, where `try-placement` stops the file's projection.
  The cost is that the editor no longer types the names inside such a
  block (the `.types` baselines of
  `aTryExitingAResultFromAParameterInitializer` and
  `aTryInAResultBlockLoopConditionIsAPlacementError` lose them): tt.md
  states that a construct left unread says nothing to TypeScript, and the
  command line types none of that file.

### Decision 6: A duplicate arm owns its match's checker consequences (E4)

- **Context**: A duplicate arm with a nested pattern
  (`Some(value) => 0, ..., Some(value: Point()) => 9`) drew
  `match-duplicate-arm` and two `ts2339` errors on `never`, the second
  reworded as "this scrutinee has none (a plain TypeScript `enum` is not
  one)". The dead arm's lowered test reads a value the earlier arm already
  narrowed away.
- **Decision and rationale**: The duplicate-arm diagnostics now carry the
  match as their owner, as the match's other structural diagnostics do
  (`match-mixed-patterns`, `match-is-wildcard-required`), so checker
  diagnostics on that match's glue are consequences of the tt cause and are
  not reported (`origin_intersects_tt_error`).

### Decision 7: References reach nested tags under a generic payload and built-in cases (E5)

- **Context**: Find References on `Circle` in `Full(item: Circle(r))` over
  `Box<Shape>` returned nothing, references from the top-level `Circle`
  missed that site, and references on `Some`/`Ok`/`Err` in a pattern
  returned nothing although definition reaches `@tt/std`.
- **Decision and rationale**:
  - At a position the parse-only resolution leaves empty, the declaration
    comes from `patternSymbol` (TASK-772 decision 3).
  - After the parse-only pattern references, each nested pattern tag
    spelled as the case is asked through `patternSymbol` and kept when its
    declaration is the target. Only same-named tags are asked, so the cost
    follows the case's uses.
  - A built-in case takes its standard-library declaration
    (`builtin_case_definition`), TypeScript's references to it, and every
    pattern in the project's tt files that resolves to the same built-in
    case.

### Decision 8: A cursor inside recovered text asks through a probe (E6)

- **Context**: Signature help at `match (s) { Circle(radius) => add(§`, at
  the end of the buffer, answered null. The unclosed match is recovered as
  a placeholder, and the cursor at its end still mapped to an output offset
  (the placeholder's end), so the request skipped the probe and asked
  TypeScript about the placeholder.
- **Decision and rationale**: Recovered text is not served, so a cursor
  inside a recovered range has no place in the served text even where an
  offset maps. Signature help then asks through the probe, as completion
  does; the probe closes the construct the cursor sits in (TASK-772
  decision 4).

### Decision 9: The outline nests a declaration under the one whose source holds it (E7)

- **Context**: `documentSymbols` listed `inner` of
  `export const block = result { const inner = ...; }` and the locals of a
  match arm as module-level symbols. Lowering writes those declarations
  before the statement that reads their value, so TypeScript's navigation
  tree of the served code sees them as siblings.
- **Decision and rationale**: TypeScript's navigation bar places what a
  variable's initializer declares under that variable
  (`addNodeWithRecursiveInitializer`, `addNodeWithRecursiveChild` in
  `services/navigationBar.ts`). The outline is mapped to the source and
  then, level by level, a symbol whose source range lies inside another's is
  moved under the innermost one, and each level is put in source order. The
  existing outline test now expects `radius`, written in `a`'s initializer,
  under `a`.

## Work log

- 2026-10-06: Ran the fifth audit as three read-only agents (CLI,
  compiler, editor) against a release build of the branch.
- 2026-10-06: Reproduced C2, traced it to the pairwise `same_file` loops,
  and replaced them with identity hashing (decision 1).
- 2026-10-06: Reproduced C1 with `strace`; made the project scan skip
  unreadable candidates outside the inputs and their imports (decision 2).
- 2026-10-06: Reproduced E1 and scoped payload alphabets to their match
  (decision 3).
- 2026-10-06: Timed every editor request with the audit's harness over
  7,000–28,000 lines, sampled the server with gdb and the host's API calls
  with a timing hook, and removed the per-item scans (decision 4).
- 2026-10-06: Reproduced E3 with an editor case and recovered the unclaimed
  block (decision 5).
- 2026-10-06: Owned the duplicate arm's consequences (decision 6) and
  extended references (decision 7); reviewed the two `.types` baselines
  decision 5 changed.
- 2026-10-06: Sent signature help inside recovered text through the probe
  (decision 8).
- 2026-10-06: Nested outline symbols by source containment (decision 9).

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
