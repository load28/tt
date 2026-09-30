# TASK-638: Hold TypeScript's own test cases to byte-identical passthrough

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-638`

## Purpose

Contract 1 says every valid TypeScript file is a valid `.tt` file that comes
back byte for byte. `tests/corpus.rs` checks it over this repository's own
TypeScript and the installed library declarations, with swc as the judge of
validity. typescript-go checks itself against TypeScript's whole test suite
and keeps every known difference in two reviewed lists. This task runs
ttc over the same suite, at the commit the pinned TypeScript was built from,
with TypeScript itself as the judge, and keeps the differences it finds in
two lists of the same kind.

## Scope

- Included: `tests/typescript-cases.json` (the pin), `scripts/fetch-typescript-cases`,
  a second test in `tests/corpus.rs`, `tests/passthrough-accepted.txt`,
  `tests/passthrough-triaged.txt`, the `check` and `exhaustive` jobs of
  `.github/workflows/ci.yml`, the `rust` stage of `scripts/ci`, and
  `CONTRIBUTING.md`.
- Excluded: fixing the divergences (listed under TASK-638 until tasks are
  planned for them), and TypeScript's other suites (`fourslash`,
  `transpile`), which exercise the language service and emit, not parsing.

## Sources modelled

- microsoft/typescript-go at `16c25522e1230b69b11210cfad066d779e6319ba` (the
  last commit before the repository's closure notice): `.gitmodules` pins
  `_submodules/TypeScript` (branch `tsgo-port`, commit `5848bc51`), and
  `internal/testutil/baseline/baseline.go` compares each baseline with the
  submodule's, writes the difference to `submodule/`, `submoduleAccepted/`
  or `submoduleTriaged/` by the names listed in `testdata/submoduleAccepted.txt`
  and `testdata/submoduleTriaged.txt`, and fails with "diff file ... is in
  both submoduleAccepted and submoduleTriaged; it should only be in one". A
  listed difference that disappears leaves a reference file no run writes,
  which its unused-baseline tracking reports.
- microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`, the
  `gitHead` of `typescript@7.1.0-dev.20260826.1` that `package.json` pins
  (read from `node_modules/typescript/package.json`). typescript-go was
  merged into this repository under `tsc/`; the test cases are at
  `tsc/testdata/tests/cases/{compiler,conformance}` (6,902 and 5,931 files),
  with the repository's `LICENSE.txt` (Apache-2.0) and `NOTICE.txt`.
- `tsc/internal/testrunner/test_case_parser.go` at that commit: lines split
  on `\r?\n`; the option regex ``(?m)^\/{2}\s*@(\w+)\s*:\s*([^\r\n]*)``;
  option lines, and `// @link: a -> b` lines, are removed from the content;
  content before the first `@filename` is discarded; leading blank lines of
  a unit are dropped; a case without `@filename` is one unit named after
  the file.
- `tsc/internal/checker/grammarchecks.go` and
  `tsc/internal/diagnostics/diagnostics_generated.go` at that commit: of the
  187 messages the grammar checks report, 150 have codes 1000-1999, the
  range TypeScript numbers syntax and grammar errors in.

## Decisions

### Decision 1: Pin to the package's own commit; fetch into a cache, commit only the pin

- **Context**: The corpus has 12,833 files (about 55 MB). The pin has to
  match the pinned TypeScript, and CI must be able to reproduce it.
- **Alternatives considered**: (a) A git submodule, as typescript-go does:
  every clone and CI checkout pays for it, and `cargo package` and the
  release jobs would have to skip it. (b) Vendoring a size-bounded sample
  with attribution: a fixed few hundred files, reviewed diffs of third-party
  code, and no path to the full set. (c) A committed manifest and a fetch
  script.
- **Decision and rationale**: (c). `tests/typescript-cases.json` holds the
  repository, the commit, the TypeScript version, and the git tree id of
  both directories; `scripts/fetch-typescript-cases` makes a sparse,
  blobless, depth-1 checkout of those two directories and the two notice
  files into `target/typescript-cases/<commit>/` and verifies every tree id
  (3.7 seconds cold, 0.3 seconds when present). GitHub is reachable from
  Actions, so CI fetches it too. The test fails when the manifest names
  another TypeScript than `package.json`, or when the installed package's
  `gitHead` differs from the pinned commit, so a TypeScript bump carries the
  corpus with it. Nothing from the corpus is committed; the list files
  name cases by path only.

### Decision 2: TypeScript decides which units are TypeScript

- **Context**: The corpus is full of negative tests. Only units that are
  TypeScript are subjects of contract 1.
- **Alternatives considered**: swc's verdict, as `tests/corpus.rs` uses for
  its own corpus: swc is the component under suspicion here. TypeScript's
  full check: almost every negative-free case still reports semantic
  errors (unresolved names, missing libraries), which say nothing about
  parsing.
- **Decision and rationale**: Two runs of the pinned `tsc` per batch of
  4,000 units, each unit written as `u<N>.ts` or `u<N>.tsx` (the name the
  unit would have as ttc's output, so a `.d.ts` unit is judged as the
  `.ts` file a `.tt` becomes): `noCheck` (any error in the file excludes it:
  a parse error), then a full check of the rest (an error TS1000-TS1999
  excludes it: a grammar error). The second run is separate because `tsc`
  reports no semantic diagnostic while any file has a syntactic one. Of
  15,383 TypeScript units, 12,779 pass.

  A batch is one program, so the units of different cases share a global
  scope there, and whether the checker reports a grammar error in one file
  can depend on another file's reference to the same global being checked
  first (Issue 4). Every unit the run would report, a difference or a
  listed unit the batch rejected, is therefore judged again with only its
  own case's units, as TypeScript's harness compiles a case, and that
  verdict decides.

### Decision 3: The passthrough verdict is the CLI's and the editor's

- **Decision and rationale**: Each unit goes through `ttc::compile_report`
  with `rewrite_imports: Off` (the contract's one exception, turned off)
  and `defer_to_checker` (no TypeScript is consulted), and through
  `ttc::emit_mapped_with_kind` (the editor projection). The verdict is
  `crashed`, `rejected <codes>`, `changed`, or `projection changed`, in that
  order; a unit with none of them passes.

### Decision 4: Two exclusive lists, keyed by unit, held exact

- **Context**: typescript-go keys its lists by diff file; here the
  difference is a verdict, not a baseline.
- **Decision and rationale**: `<case path>[#<unit>]`, a tab, the observed
  verdict, a tab, and a note: the reason (accepted) or `TASK-NNN` and a
  description (triaged). The test fails for an unlisted difference, a unit
  listed in both files, a listed unit whose verdict changed, and a listed
  unit that now passes through; the full run also fails for a listed unit
  that is not a TypeScript unit of the pinned cases. A sample run judges
  only the units it ran.

### Decision 5: A fixed-seed sample per pull request, every case nightly

- **Decision and rationale**: `cargo test` runs 400 cases drawn with a
  fixed seed (`0x7473636173657321`), about 5 seconds including both `tsc`
  runs; the `check` job fetches the corpus first and sets
  `TTC_REQUIRE_TYPESCRIPT_CASES=1` so a missing corpus fails. The scheduled
  `exhaustive` job runs every case in a release build
  (`TTC_TYPESCRIPT_CASES=all`; about 120 seconds in a debug build here,
  45 of them re-judging reported units alone).
  `scripts/ci rust` fetches the corpus and warns, rather than failing, when
  it cannot (offline).

### Decision 6: Accepted means "TypeScript itself reports that construct"

- **Context**: All 191 differences are `verify-failed`: ttc's output
  self-check, which parses the emission with swc, rejects the unit. None is
  a changed byte, a tt diagnostic, or a crash.
- **Alternatives considered**: Triaging all 191 (most name constructs
  TypeScript's checker rejects too); accepting all 191 (hides the units
  TypeScript accepts entirely).
- **Decision and rationale**: Each unit was checked against the pinned
  `tsc`'s full check. A unit is accepted, with the TypeScript code in its
  reason, when TypeScript reports an error about the construct swc rejects
  on the same line (for example TS2369 for a parameter property outside a
  constructor, TS2364 for an invalid assignment target, TS18016 for a
  private name outside a class): the file is not valid TypeScript, and
  `verify-failed` is ttc's documented answer for invalid TypeScript passed
  through. Three `with` statements are accepted too: TypeScript 7 is always
  strict (`alwaysStrict=false` is TS5108), and TS1101 is hidden there only
  by the case's `// @ts-ignore`. The other 44 were triaged: TypeScript
  reports nothing about the construct. After the merge of TASK-624, which
  accepts `using` declarations in a `for` head, two of them pass through and
  left the list: 147 accepted, 42 triaged.

## Work log

- 2026-09-30: Found the package's `gitHead` in microsoft/TypeScript, read
  typescript-go's `baseline.go`, `.gitmodules`, and the case parser, and
  measured the sparse fetch.
- 2026-09-30: Added the manifest, the fetch script, and the test; the first
  full run reported 191 differences.
- 2026-09-30: Classified each against the pinned `tsc`'s full check and
  wrote both lists; reproduced every triaged class with the `ttc --check`
  command line.
- 2026-09-30: Wired the `check` and `exhaustive` jobs and `scripts/ci`, and
  documented "TypeScript's own test cases" in `CONTRIBUTING.md`.
- 2026-09-30: Merged `claude/ecstatic-dijkstra-qw5pf9` (TASK-621 to
  TASK-628); the full run reported `usingDeclarationsInFor.ts` and
  `awaitUsingDeclarationsInFor.ts` as "now comes back unchanged", and both
  lines were removed.
- 2026-09-30: Found the batch interference of Issue 4 in repeated full runs
  and added the re-judging of Decision 2.

## Issues and resolutions

### Issue 1: 42 valid TypeScript units that ttc rejects (contract 1)

- **Symptom**: `ttc` reports `verify-failed` for TypeScript the pinned
  `tsc` accepts. Each reproduces with `ttc --check` on a `.tt` file, and
  each of these compiles with no error under the pinned `tsc`:
  - `declare namespace N3 { var static: number; }` then `export {};`
  - `declare const readonly: unknown;` then
    `export const a4 = (readonly as number);` (read as a parameter property)
  - `export { type "x" as "c d" } from "./m";`
  - `<div className= "foo` / `bar" />` (a JSX attribute string over two lines)
  - `export class C9 { #a = 1; b: typeof this.#a = 1; }`
  - `function f1(await: number) { return await; }` in a script
- **Cause**: Two classes. (a) 29 units: the self-check applies module and
  strict-mode rules TypeScript does not apply to the file (`await` as an
  identifier in a script, a call of a function named `await`, `static` and
  `eval` in declarations). (b) 13 units: the vendored `swc_ecma_parser`
  does not parse syntax TypeScript 7.1 parses (`typeof this.#a`,
  `export { type "…" as "…" }`, `await using of` in `for...of` heads,
  `export @dec abstract class`, `import await = …`, import attributes
  after a line break, `throw await` across a line break, a JSX attribute
  string across lines, and `a ? b ? c : (d) : e => f`).
- **Resolution**: Listed in `tests/passthrough-triaged.txt` under TASK-638
  with the construct each rejects; fixing them needs task numbers. Reported.
  Two more of class (b), `using` and `await using` declarations in a
  `for (;;)` head, were fixed by TASK-624 and left the list.

### Issue 2: 147 units ttc rejects before TypeScript can (contract 2)

- **Symptom**: For constructs TypeScript's checker reports (TS2369, TS2364,
  TS2371, TS18016, TS17019, …), ttc reports `verify-failed` with swc's
  wording instead, so the user sees the error from the wrong layer and
  without TypeScript's code.
- **Cause**: The self-check parses the whole emission, and swc treats as
  syntax errors what TypeScript's parser accepts and its checker rejects.
- **Resolution**: Accepted, with the TypeScript code in each reason, because
  the files are not valid TypeScript. Whether passthrough bytes should be
  exempt from the self-check is a design question for a follow-up.

### Issue 3: The first oracle took swc's side

- **Symptom**: With `noCheck` alone, 22 of the 400 sampled units differed;
  most were grammar errors (`'await' expressions are only allowed within
  async functions`) and `.d.ts` units judged as declaration files.
- **Cause**: `noCheck` skips the checker's grammar checks, and a declaration
  file accepts what a `.ts` file does not.
- **Resolution**: Decision 2's second run and the `.ts`/`.tsx` names.

### Issue 4: A unit's grammar verdict depended on the rest of its batch

- **Symptom**: Three full runs of the same tree counted 12,779, 12,779 and
  12,778 units that parse; the unit that moved was
  `conformance/types/spread/objectSpreadNegative.ts`, whose TS1117
  (duplicate property in an object literal) twelve runs of `tsc` on the
  file alone all report. `--singleThreaded` did not change it.
- **Cause**: The batch is one program, the unit is a script, and its
  globals (`o`, `o2`, `duplicated`) merge with other units' globals. The
  likely mechanism, not traced into the checker: when another file's check
  resolves the literal's type first, the grammar check that reports TS1117
  is not run for it again.
- **Resolution**: Decision 2's re-judging. The count of units that parse
  may still move by one between runs, because a unit that passes through
  unchanged is not re-judged; what the run reports does not, and three
  runs after the change reported the same 189 listed differences.

## Regression test (fails before the fix)

Not applicable: this task adds a differential test and its lists and fixes
no compiler bug. Its checks are shown under Verification: an unlisted
difference, a unit in both lists, a changed verdict, a listed unit that
passes, and a listed unit that does not exist each fail the run.

## Verification

- [x] `TTC_TYPESCRIPT_CASES=all cargo test --test corpus typescript_test_cases`:
  "all 12831 cases; 15383 TypeScript unit(s), 12779 parse, 191 differ (191
  listed)", passing, 77 to 123 seconds in a debug build; after the merge and
  Issue 4's change, three runs: "189 differ (189 listed)", about 120
  seconds each.
- [x] `cargo test --test corpus`: both tests pass in 5.3 seconds; "400 of
  12831 cases ... 430 parse, 4 differ (4 listed)".
- [x] Negative checks in one full run: `compiler/2dArrays.ts` listed as
  triaged fails with "now comes back unchanged: remove it"; a listed
  `compiler/doesNotExist.ts` fails with "not a TypeScript-valid unit of the
  pinned cases"; `ClassDeclaration24.ts` listed as `changed` fails with "now
  rejected verify-failed, listed as changed"; a unit in both lists fails
  with "listed in both".
- [x] `scripts/fetch-typescript-cases` cold and warm; `--path`; an unknown
  argument prints the usage and exits 2.
- [x] `bash -n scripts/ci`; `ci.yml` parses as YAML.
- [x] The full gate over TASK-637 to TASK-639 is recorded in TASK-639.

## Result

Changed files: `tests/corpus.rs`, `tests/typescript-cases.json`,
`tests/passthrough-accepted.txt`, `tests/passthrough-triaged.txt`,
`scripts/fetch-typescript-cases`, `scripts/ci`, `.github/workflows/ci.yml`,
`CONTRIBUTING.md`, `docs/tasks/INDEX.md`, and this record. Follow-ups: tasks
for the two classes of Issue 1, and the design question of Issue 2.
