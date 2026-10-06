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
