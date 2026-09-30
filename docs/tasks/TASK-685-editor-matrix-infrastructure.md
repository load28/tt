# TASK-685: Generate editor cases from the case matrix, each asked against a TypeScript twin

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-685`

## Purpose

The editor suite held about 30 hand-written cases (TASK-639, TASK-675). The
case matrix (TASK-678) writes every tt construct into many host positions
for the compiler, each run against a TypeScript twin. This task lets the
same spec write editor cases, so that `.tt` and `.ttx` editing is held to
what `.ts` and `.tsx` editing answers at many points, with the twin's
answers as the oracle; `|>` is the pilot construct, and TASK-686 extends
the matrix to the others.

## Scope

- Included: fourslash markers in the spec (`/*@name*/`), the editor output
  of `scripts/generate-cases` (`tests/cases/editor/matrix/<construct>/`),
  `tests/matrix/editor.mjs` (marker kinds and their verbs, the host
  positions and constructs that get editor cases),
  `tests/matrix/editor-harness.ts`, the pipeline spec's markers, the JSX
  hosts' markers in `tests/matrix/positions.mjs`, the runner changes in
  `tests/editor_cases.rs` (recursive cases, `@parityIgnores`, twins that
  share the case's other units, marker-anchored locations, per-file verbs
  on twins whose text differs, the matrix's sampling, baselines, and
  known-difference list), `tests/common/matrix.rs` (the sampler, now shared
  with `tests/case_baselines.rs`), `tests/editor-matrix-differences.txt`,
  the fuzz corpora's exclusion of the editor matrix, the nightly step, one
  sentence in `docs/ai/tt.md`, and `CONTRIBUTING.md`.
- Excluded: the markers of the other constructs (TASK-686), and fixing any
  engine or compiler defect the matrix finds (reported below).

## Sources modelled

- microsoft/typescript-go at `16c25522e1230b69b11210cfad066d779e6319ba`,
  `internal/fourslash/test_parser.go`: markers `/*name*/` are removed from
  the content and name positions; `fourslash.go`,
  `VerifyBaselineHover`/`VerifyBaselineFindAllReferences`/
  `VerifyBaselineGoToDefinition`/`VerifyBaselineSignatureHelp`, one answer
  per marker. The generated cases are ordinary TASK-639 cases in that
  format.
- The same commit's `internal/fourslash/_scripts/failingTests.txt` (a list
  of test names that are known to fail, which `SkipIfFailing` consults and
  `updateFailing.mts` rewrites from a full run): the model for a tracked
  list of known differences. Here the list also states a class and a
  reason per line, and is exact in both directions (TASK-678 Decision 4),
  since a fixed difference must be noticed.
- microsoft/TypeScript `release-6.0` at `050880c`, `tests/cases/fourslash/`
  (for example `findAllRefsForObjectBindingPattern*.ts`,
  `renameDestructuringDeclarationInFor.ts`, `quickInfoForDestructuring*`):
  TypeScript's own editor tests put markers at bindings, uses, and
  destructured properties, the points the matrix marks.
- TASK-678 for the matrix, its sampling knobs, and the nightly job;
  TASK-639 and TASK-675 for the runner and `parity_view`.

## Decisions

### Decision 1: Markers live in the spec templates, stripped from compiled cases

- **Context**: The generated editor case must mark the same points in the
  tt program and in its twin, and the twins are written from the same
  templates as the compiled matrix.
- **Alternatives considered**: (a) A separate editor spec per construct:
  a second description of each form, free to drift from the compiled one.
  (b) Markers computed by the generator from the text (first identifier
  after `(`, and so on): a guess about each template's shape.
- **Decision and rationale**: a template writes `/*@name*/` before a point;
  the compiled matrix strips every such marker, so its 3,286 cases are
  byte-identical (checked with `--check` after every spec edit), and the
  editor case turns the first occurrence of each name into `/*name*/` (a
  template instantiated twice marks only its first instance). The name's
  leading lowercase letters are its kind, and `tests/matrix/editor.mjs`
  maps kinds to verbs: `operand` (the companion's input, which every row
  has) asks hover, definition, references, rename, completions, and
  signature help; `bind` hover, references, rename; `use` hover,
  definition, completions; `call` hover, definition; `arg` signature help;
  `member` hover, definition, completions; `field` hover, definition;
  `attr` hover, completions. Every case also asks `semanticTokens` and
  `diagnostics` for its unit. The generator fails when a marker is on one
  side only, so every question has an oracle.

### Decision 2: The editor twin is the runtime twin unless the form gives an `edit` template

- **Context**: A runtime twin models evaluation, not editing: a `match`
  twin reads `t0.r` from an `any` temporary where the tt program binds `r`,
  and `unwrap` returns `any` where `try` has the Ok type.
- **Decision and rationale**: a form may carry `edit`, the TypeScript a
  user would write for the same construct (TASK-686 writes them, for
  example `const { r } = t0` in an arm's block), and a declaration may
  carry `edit` the same way; `tests/matrix/editor-harness.ts` replaces
  harness declarations by name for editor cases (a generic `unwrap`, typed
  as `try` is documented). Editor twins do not run, so they only need to
  be valid TypeScript whose answers at the markers are the ones tt
  promises. The editor program is the compiled program without its
  driving line (`await drive(...)`), which keeps `drive` and its
  dependencies out of every file: 5.4 MB became 0.69 MB for the pilot's
  264 cases, and a twin names only its own `.ts`/`.tsx` unit and shares
  the case's `tsconfig.json` and `harness.ts`.

### Decision 3: The oracle's normalizations

- **Context**: `parity_view` (TASK-639) reduced a location to its unit
  stem and covered text, which cannot tell a binding from its use, and it
  skipped semantic tokens and diagnostics whenever the twin's text is not
  the source's, which is every generated case.
- **Decision and rationale**: a location that starts at a marker is shown
  with it (`main /*use*/ "r"`), in both views, so references and
  definitions are compared place by place wherever the case marks places;
  when the twin's text differs, semantic tokens are compared as the token
  at each marker both files have, and diagnostics by location (stem,
  marker, covered text) with the project directory written `$DIR`;
  completion lists leave out the labels a case's `@parityIgnores` names,
  which the generator computes as the names only one side declares (the
  twin's temporaries, `Result` or `unwrap` imports) — they are the twin's
  plumbing, not scope the user sees — and the entries that import from
  `@tt/std`, a package only a tt project has. The hand-written cases'
  baselines are unchanged by these rules.

### Decision 4: What is stored

- **Context**: Full baselines of 1,800 cases would repeat what the twin
  already says and bury a reviewer (TASK-678 Decision 5).
- **Decision and rationale**: a generated case stores a baseline only when
  it differs from its twin, holding only the differing questions (the
  marker's line with a caret and the `tt only`/`ts only` lines), under
  `tests/baselines/reference/editor/matrix/<construct>/`; an agreeing case
  must have none (`expect_absent`). Each differing question is listed in
  `tests/editor-matrix-differences.txt` with its class, `by-design` (the
  reason must cite `docs/ai/tt.md`) or `defect` (the reason must name the
  task that records the repro). A case name there may use `*`, so a
  difference caused by one form or position is one line; a line fails the
  suite when any executed case it matches agrees at that question, and,
  in an unfiltered run, when it names no question of any case. A
  difference no line lists fails with the command that shows it. The
  pilot stores 60 baselines (36 KB) and 8 list lines.

### Decision 5: Cost

- **Context**: Each case starts `ttc --server`, the VS Code adapter (whose
  published list settles after 1.5 s of quiet, TASK-675 Decision 2), and a
  `tsgo --lsp` for the twin; the first full run took 5.5 s of worker time
  per case.
- **Decision and rationale**: a generated case asks only the server
  transport (the engine-versus-server agreement stays pinned by the
  hand-written cases, which still ask both), does not resolve auto-import
  entries (they feed only the full baseline), and asks the adapter only
  for its published diagnostics; the suite runs eight workers instead of
  four, since most of a case is waiting on processes. A pull request runs
  a fixed-seed sample of 40 generated cases (`MATRIX_SAMPLE`, seed
  `0x7474656469746f72`) through `tests/common/matrix.rs`, the sampler the
  compiled matrix now shares, with the same `TT_MATRIX_CASES` and
  `TT_MATRIX_SEED` knobs; `TT_CASES` runs every matching case. The nightly
  `exhaustive` job builds the adapter and runs every case; its timeout
  goes from 75 to 120 minutes. Unsampled cases' baselines are recorded as
  `unsampled` for `scripts/check-baselines`.

### Decision 6: The fuzz corpora leave the editor matrix out

- **Decision and rationale**: as TASK-678 Decision 9 did for the compiled
  matrix, `tests/fuzz_regressions.rs` and `scripts/fuzz-seed-corpus` skip
  `tests/cases/editor/matrix`, whose programs repeat the compiled matrix's.

## Work log

- 2026-09-30: Reset onto `claude/ecstatic-dijkstra-qw5pf9` at `e44a7e0`,
  `npm ci` at the root and in `editors/vscode`, fetched the TypeScript
  cases, built the extension's server; the existing editor suite passed in
  18 s.
- 2026-09-30: Read typescript-go's fourslash parser, harness, and failing
  list, TASK-639, TASK-675, and TASK-678 to TASK-680. Prototyped one
  editor case by hand (a `match` in a declaration initializer) with an
  IIFE twin that destructures: 21 of 23 questions agreed, and the two that
  did not were a twin-only temporary and a tt defect (TASK-686 Issue 1).
- 2026-09-30: Added the markers, the editor output, and the runner
  changes; `node scripts/generate-cases --check` confirmed the compiled
  matrix unchanged. Timed the pipeline cases: 362 s for 264 cases with
  both transports and four workers, 196 s with the server only, 123 s
  with eight workers.
- 2026-09-30: Ran every construct's editor cases three times (1,508 s,
  1,466 s, 1,458 s for 1,853 cases) while correcting twin and oracle
  errors (Issues 1 and 2); TASK-686 records the rest. Narrowed this
  commit to the pipeline and its list lines.

## Issues and resolutions

### Issue 1: Twin plumbing that differed from the program

- **Symptom**: The first full run reported, only in twins, TS6133 for
  unused module temporaries (`let t0: any` in forms that use none) and for
  an unused import `first` (a `flow` twin's parameter named `first`
  shadowed the harness function), TS2306 for `import {  } from
  "./harness.js"` over an empty harness when a program used no harness
  name, and TS2304 for temporaries an `edit` template assigned but the
  generator did not declare.
- **Cause**: The generator's twin plumbing, not tt.
- **Resolution**: An editor twin declares exactly the temporaries it
  assigns, a program that uses no harness name has no harness import, and
  (TASK-686) the `flow` twins' parameter is `head`.

### Issue 2: TypeScript classifies nothing inside a JSX spread attribute

- **Symptom**: In 29 of 40 `.ttx` cases in a spread attribute
  (`<div {...{ value: <construct> }} />`), the twin had no semantic token
  at any marker, while tt classified each one.
- **Cause**: `tsgo` returns no semantic token for an identifier inside a
  JSX spread attribute even in plain TSX (a probe with identical `.ttx` and
  `.tsx` text, `export const a = <div {...{ value: f(1) }} />;`, lists no
  token for `f` on either side). tt's answer differs only because the
  lowering moves the construct out of the attribute.
- **Resolution**: The oracle cannot judge that position, so it withholds
  `semanticTokens` there (`editorWithholds` on the position). Not a tt
  finding; a candidate report for typescript-go.

### Issue 3: A postfix step after an optional-chain head joins the chain

- **Symptom**: `pipeline_postfixStep_*_optionalChain`: signature help on
  `.toFixed(` shows `string | undefined` where the twin's
  `(box(input)?.inner.value!).toFixed(1)` shows `string`.
- **Cause**: A compiler defect found by the editor oracle. For
  `declare const box: { inner: { value: number } } | undefined;`,
  `box?.inner.value |> .toFixed(1)` emits `box?.inner.value.toFixed(1)`:
  the step becomes part of the optional chain, so when `box` is
  `undefined` the pipeline yields `undefined` where `x |> .toFixed(1)`,
  documented as the postfix chain on the piped value, would throw on
  `(undefined).toFixed`; its type gains `| undefined`, and a checker error
  the parenthesized form reports (`'...' is possibly 'undefined'`) is
  lost. The compiled matrix did not see it because `box` never returns
  `undefined` there.
- **Resolution**: Listed as a `defect`; not fixed in this task.

## Regression test (fails before the fix)

Not applicable: this task adds test infrastructure and fixes no product
defect. The oracle was shown to catch what it should: the hand-built
prototype and the pilot found Issue 3, and a listed line that stops
matching fails (`tests/editor-matrix-differences.txt` lines are judged
against every executed case).

## Verification

- [x] `node scripts/generate-cases --check`: the compiled matrix is
  unchanged by the markers; 264 editor cases.
- [x] `TT_MATRIX_CASES=all TT_REQUIRE_EXTENSION=1 cargo test --test
  editor_cases`: passes, 225 s for the 264 pipeline cases and the 30
  hand-written cases, whose baselines and `failingParity.txt` did not
  change.
- [x] The default run (40 sampled cases): 60 s, against 18 s without the
  matrix.
- [x] `cargo test --test case_baselines` with the shared sampler: passes.
- [x] `cargo test --lib the_machine_reads_the_corpus_as_swc_does`: passes
  with the new files (markers inside JSX tags).
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] The full gate is recorded in TASK-686, which ends this batch.

## Result

Changed files: `scripts/generate-cases`, `tests/matrix/{editor.mjs,
editor-harness.ts,pipeline.mjs,positions.mjs}`, `tests/editor_cases.rs`,
`tests/common/{matrix.rs,mod.rs}`, `tests/case_baselines.rs`,
`tests/fuzz_regressions.rs`, `scripts/fuzz-seed-corpus`,
`tests/editor-matrix-differences.txt`, `tests/cases/editor/matrix/**`,
`tests/baselines/reference/editor/matrix/**`, `.github/workflows/ci.yml`,
`docs/ai/tt.md`, `CONTRIBUTING.md`, `docs/tasks/INDEX.md`, and this record.
Follow-ups: TASK-686 populates the other constructs; Issue 3 is a compiler
defect for a later batch; Issue 2 may be reported upstream.
