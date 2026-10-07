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

### Decision 10: Pattern completion narrows at empty positions and after a wildcard

- **Context**: Pattern completion listed every tag in scope at
  `if let § = o`, `else if let § = o`, and in a `match (s) {` left open at
  the end of the file; it offered `_` and uncovered cases after an
  unguarded `_` arm; and `val` was missing from the statement keywords.
- **Decision and rationale**: An `if let` or let-else pattern requires its
  parentheses, so the placeholder tag spliced in to ask the subject's type
  is `Tag()` when no `(` follows, as nested patterns already did. When the
  lowering still fails because the construct never closes, the brackets
  its recovered range leaves open are closed at the placeholder, the repair
  TASK-772 decision 4 gives the completion probe (TypeScript's
  `parseExpected` assumes a missing closer). An unguarded `_` arm written
  before the cursor covers every case, as Rust's usefulness check treats
  arms after a wildcard, so every case is marked covered and no second
  `_` is offered; `ArmHeader` carries its arm's start to tell before from
  after. `val` is a statement-start keyword like `let-else`. A literal
  after an unclosed opening quote (`match (d) { "§`) is answered through
  the same closing repair, and its replacement ends at the cursor rather
  than at the end of the unterminated token, as TypeScript's
  `createTextSpanFromStringLiteralLikeContent` bounds an unterminated
  literal (`services/utilities.ts`); the token's end would have replaced
  the rest of the line.

### Decision 11: `.\` and `..\` begin a relative specifier (C3)

- **Context**: `import { Shape } from ".\shape.tt"` was neither rewritten
  nor followed: the build kept `.\shape.tt` in its output, and the match
  over `Shape` was not checked against the imported variant.
- **Decision and rationale**: TypeScript reads a specifier as relative when
  it is `.`/`..` or starts with `./`, `../`, `.\`, or `..\`
  (`pathIsRelative` in `compiler/path.ts`, `tspath.PathIsRelative` in
  tsgo), and its resolution reads `\` as a separator on every platform
  (`normalizeSlashes`). One predicate, `is_relative_specifier`, now decides
  relativity for import lifting and the reverse rewrite, and every place that
  joins a specifier to a directory goes through `TtImport::path`, which reads
  `\` as `/`. The written specifier is kept as written: the rewrite changes
  only its extension (`.\shape.js`), as TypeScript's
  `rewriteRelativeImportExtensions` does.

### Decision 12: A hand-written file is measured as TypeScript reads it (C4)

- **Context**: A hand-written `a.ts` with bytes that are not UTF-8 got its
  type errors from `--check-types` with no line, column, or snippet, and two
  errors on one line merged into one. `tsc` reported `a.ts(2,14)`.
- **Decision and rationale**: TypeScript's coordinates of a hand-written
  file are converted against its text, and that text was read as UTF-8 or
  not at all. tsgo keeps the bytes and counts one UTF-16 unit for each byte
  that is not valid UTF-8 (`ast.ComputePositionMap` decodes with
  `utf8.DecodeRuneInString`, which reads such a byte as `RuneError` of
  width one; a WTF-8 surrogate is one unit of three bytes,
  `stringutil.DecodeJSStringRune`). `lines::typescript_text` decodes a file
  the same way, and both the report's conversion and the CLI's snippet use
  it, so the positions are `tsc`'s.
- **Not changed**: `--check` and the build still refuse such a file. A
  passed-through file is rewritten as text and its ownership record stores
  that text, so writing it byte for byte would need a byte-level output and
  record path; the refusal names the file.

### Decision 13: An in-place build reports a hand-written twin as `-o` does (C6)

- **Context**: With `src/a.tt` beside a hand-written `src/a.ts`, `ttc -o out
  src` reported "multiple inputs claim this output", but `ttc src` reported
  "output is not owned by this input or has been edited".
- **Decision and rationale**: In place, a hand-written `.ts` passes through
  to itself, so it claims the same output `a.tt` compiles to. `build_jobs`
  dropped every in-place pass-through that a compiled output targets, which
  is right only for a file ttc wrote earlier; it now drops one only when an
  ownership record names it, so a hand-written twin reaches the claim check
  and both layouts give the same sentence. An output already reported as
  claimed twice is not reported again by the ownership check. `--check`
  stays silent, as TASK-770 decided: it judges no output layout.

### Decision 14: The content-mapper notes say a tt error hides every file's type errors (C7)

- **Context**: `docs/design/content-mapper.md` said TypeScript skips the
  semantic check of the file with a tt diagnostic. `tsc --runExternalCode`
  reported only the tt error while another file had a type error.
- **Decision and rationale**: A mapper's diagnostics are syntactic, and
  `tsc` asks for semantic diagnostics only when the program has no syntactic
  diagnostic at all (`compiler.GetDiagnosticsOfAnyProgram` in tsgo,
  `emitFilesAndReportErrors` in TypeScript 6). The design note and
  `docs/ai/tt.md` now say so and point at `ttc --check-types`, which reports
  both layers.

### Decision 15: Dependencies under a configuration are what the program reads (C8)

- **Context**: With `include: ["src"]`, `ttc --dependencies src/shape.tt`
  listed `zz/bad.ts` and earlier outputs, which the check never opens.
- **Decision and rationale**: Without a configuration the program is the
  walk of the root, so its files and directories are dependencies. With
  one, TypeScript decides the program; the files are those it and ttc read
  and the directories its globs watch. The walk's files now join only in
  the first case, as its directories already did.

### Decision 16: Findings kept as they are (C5, C9, C10)

- **C5** (`--check-types x.ts` is refused): TASK-770 decided that the typed
  modes take tt sources and report a hand-written file through the project
  they open; `tt_only_modes_name_the_file_and_the_extensions_they_accept`
  pins it.
- **C9** (a build writes an import of a `.tt` file that is not an input):
  an imported `.tt` outside the inputs is a supported layout that is built
  separately (TASK-658 rebuilds its importers in watch mode). Emitting it
  as `tsc` would changes the output root and moves every output.
- **C10** (`-p` points `@tt/std` at `./tt/` beside the file): `-p` prints
  what a build of that one input writes, and the build writes `tt/` there
  (`ttc -o out src/deep/a.tt` writes `out/tt/option.ts` and imports
  `./tt/option.js`). The bundler adapter asks with `rewriteImports: "off"`
  and serves `@tt/std` virtually.

### Decision 17: A governed statement keeps the line breaks between its arms (K1)

- **Context**: `// @ts-expect-error` above a match written over several
  lines suppressed a type error in an arm two lines below it. The lowering
  of a statement that starts on a governed line was written on one output
  line, arms included, so the directive governed every arm.
- **Decision and rationale**: TypeScript applies a directive to the next
  line only (`Program.getDiagnosticsWithPrecedingDirectives` in tsgo's
  `compiler/program.go` looks up the directive on the line before each
  diagnostic's line). The printer now writes a line break where the source's
  next written piece (a copied span or a pattern's source point) starts on
  a later source line than the last copied one, once per gap, indented as
  the source line is. Each arm then sits on its own line and a directive
  governs only the arms on the line after it. What the lowering writes
  after the arms, the declaration the match initializes, follows the last
  arm's line; `tsDirectiveBeforeLoweredStatement` now writes that match on
  one line, the form in which a directive governs its declaration, and
  `docs/ai/tt.md` says so.
- **Not changed (K2)**: in a let-else whose `else` block spans lines, the
  bindings are declared after the block, so a binding error is reported
  after it. Destructuring before the divergence test would read a field
  of a value that has not been tested.

### Decision 18: Findings recorded without a change (K3, K4, K6, K7, K8)

- **K3, K4** (the scrutinee variable is not narrowed inside an arm; an arm
  for a case narrowed away before the match is TS2678): both follow from
  testing a `const` copy of the scrutinee, which keeps the single
  evaluation the documentation promises and takes the narrowed type of
  the subject at the match. Narrowing the subject itself, or typing the
  copy with the declared variant (which a generic variant cannot spell),
  changes every match's emission and is proposed to the user rather than
  decided here.
- **K6** (`Function.name` of an anonymous function or class written as an
  arm value): the value is stored through a slot, which names it as
  ECMAScript's NamedEvaluation does for an assignment. A conditional
  expression names nothing, but the forms that avoid naming (`(0, f)`) are
  TS2695 in TypeScript. Proposed to the user.
- **K7** (plain `ttc` leaves a call to a name declared twice unjudged):
  `--check-types` resolves the callee's symbol and reports it. The
  documentation now says plain `ttc` judges by name.
- **K8** (time for many matches in one expression): measured on a debug
  build at 200, 400, and 800 matches in one expression, 0.24 s, 0.75 s,
  and 3.2 s; let-else statements in one body are linear (0.04 s at 100,
  0.10 s at 400). Samples put the expression cost in the evaluation-order
  validation, which compares each captured span with every earlier one.
  Not changed in this task.

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
- 2026-10-06: Narrowed pattern completion at empty positions, after a
  wildcard, and in an unclosed literal, and offered `val` (decision 10).
- 2026-10-06: Read backslash specifiers as relative (decision 11).
- 2026-10-07: Measured hand-written files as tsgo does, unified the twin
  report, corrected the content-mapper note, and narrowed dependencies
  under a configuration (decisions 12–15). Changed `--check-types` to
  accept a named `.ts`, then reverted it when
  `tt_only_modes_name_the_file_and_the_extensions_they_accept` showed the
  TASK-770 decision (decision 16).

## Issues and resolutions

- **Commits made with clippy errors**: Two commits (casts, then a complex
  type) went in while clippy reported errors. Cause: the commit command was
  chained with `;` after the clippy run, so it ran whatever clippy printed.
  Resolution: later commits fixed both; commits now run only when the
  captured clippy output is empty and `cargo fmt --check` passes.
- **A commit without its baselines**: The decision 5 commit left out two
  `.types` baselines it changed. Cause: the case runs were filtered with a
  comma list that the case runner does not split. Resolution: the reviewed
  baselines were committed next, and cases are now regenerated one by one.
- **A VS Code test left expecting the old outline**: Decision 9 updated the
  Rust outline expectation but not `editors/vscode/server/src/test/server.test.ts`,
  which still expected `radius` beside `a`. Cause: the VS Code suite was not
  run with that change. Resolution: the test now expects `radius` under `a`,
  committed with decision 10.

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
