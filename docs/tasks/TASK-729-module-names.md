# TASK-729: Name a `.tt` module as TypeScript names it

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-729`

## Purpose

TASK-717 D4 and TASK-718 D1: TypeScript writes a `.tt` module's name with
its extension (`Namespace '"<dir>/a.tt".P'`, `typeof import("<dir>/f.tt")`)
where it writes `"<dir>/a"` for the `.ts` twin, and the editor offers an
auto-imported default export as `fooBarTt` where the twin offers `fooBar`.
The task was to find where the served name reaches TypeScript and make
TypeScript derive the twin's names structurally, without rewriting
messages.

## Scope

- Included: the investigation, the decision, `docs/ai/tt.md`, and the
  classification of the six list lines.
- Excluded: any change to TypeScript, or to the text of its messages.

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/binder/binder.go` line 768: a source file that is a module
  is bound as `"\"" + tspath.RemoveFileExtension(file.FileName()) + "\""`.
- `tsc/internal/tspath/extension.go` lines 43-54: `RemoveFileExtension`
  removes only `extensionsToRemove` (`.d.ts`, `.d.mts`, `.d.cts`, `.mjs`,
  `.mts`, `.cjs`, `.cts`, `.ts`, `.js`, `.tsx`, `.jsx`, `.json`); a content
  mapper's extensions are not among them. (`outputpaths.go` lines 98 and
  149 and `modulespecifiers` consult `ContentMapperExtensions()`; the
  binder does not.)
- `tsc/internal/checker/nodebuilderimpl.go`, `getSpecifierForModuleSymbol`
  (line 1249): without an enclosing file the module is written by its
  symbol name or its file name.
- `tsc/internal/ls/autoimport/extract.go` lines 330-352 and 463-473, and
  `tsc/internal/ls/lsutil/utilities.go` lines 116-145
  (`ModuleSpecifierToValidIdentifier`): a default export with no usable
  name is named from the declaring file's name with one extension removed
  by `RemoveAnyFileExtension` (`fooBar.tt` gives `fooBar`, `fooBar.tt.ts`
  gives `fooBar.tt`, written `fooBarTt`).

## Decisions

### Decision 1: Report the names TypeScript gives the `.tt` project; classify the differences as by design

- **Context**: The typed host already holds a configured project's `.tt`
  modules under their own names through a content mapper (`host.mjs`,
  TASK-410's arrangement), which is how `tsc --runExternalCode` holds them
  under the documented configuration. Run on the D4 repro (with the
  documented `contentMappers` entry and the mapper package executing this
  build's `ttc --content-mapper`), the pinned `tsc -p . --runExternalCode`
  prints `a.tt(3,19): error TS2694: Namespace '"<dir>/a.tt".P' has no
  exported member 'Missing'.` and `typeof import("<dir>/h.tt")` in TS2322:
  exactly ttc's messages. The editor, asked for completions at `fooB` with
  `fooBar.tt` holding `export default function () {}`, offers `fooBar`
  with the documented configuration (TypeScript holds `fooBar.tt`) and
  `fooBarTt` without one (the language service holds the lowered module
  as `fooBar.tt.ts`, the only name a configured project without a tt
  mapper can resolve `"./fooBar.tt"` to).
- **Alternatives considered**: (a) Serve each lowered module as `a.ts`,
  the one name from which TypeScript derives `"<dir>/a"`: it collides with
  the output `ttc` writes beside the source and with a hand-written
  `a.ts`, and `"./a.tt"` would no longer resolve. (b) Rewrite TypeScript's
  messages and completion labels: forbidden by the error-layer contract
  (AGENTS.md contract 2) and by the task. (c) Change TypeScript so that
  `RemoveFileExtension` also removes a content mapper's extension: an
  upstream change to the pinned compiler, outside this repository. (d)
  Configure the documented mapper in TASK-718's generated `.tt` projects:
  changes what every fourslash comparison measures, for three questions.
- **Decision and rationale**: No change to the compiler. Contract 2 makes
  these TypeScript's messages, and ttc already reports what TypeScript
  reports for the `.tt` project under the documented configuration; a
  structural change on ttc's side would make it disagree with
  `tsc --runExternalCode`. `docs/ai/tt.md` now states how TypeScript names
  a `.tt` module, in messages and for an auto-imported default export,
  and the six lines are `by-design` with that document as the reason.

## Work log

- 2026-10-01: Reproduced D4 with `ttc --check-types`; read the binder,
  `tspath`, the node builder, the auto-import extractor and
  `ModuleSpecifierToValidIdentifier`.
- 2026-10-01: Ran the pinned `tsc --runExternalCode` with the documented
  mapper on the repro (identical messages), and the server's `completion`
  at `fooB` with and without the documented configuration (`fooBar`,
  `fooBarTt`).
- 2026-10-01: Documented the naming in `docs/ai/tt.md`; reclassified the
  lines in `tests/typed-parity-differences.txt` and
  `tests/fourslash-differences.txt`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: no compiler code changed; the investigation found that
ttc reports what TypeScript reports for the same `.tt` project. The list
lines, now `by-design`, keep the suites failing if the signature changes.

## Verification

- [x] `TTC_TYPED_FILTER=bluebirdStaticThis.ts,internalAliasInterfaceInsideLocalModuleWithoutExportAccessError.ts,internalAliasUninitializedModuleInsideLocalModuleWithoutExportAccessError.ts
  TTC_TYPED_CASES=all cargo test --test corpus typescript_cases_type_check_as_typescript_does`:
  3 compared, 3 differ, 3 listed; passes.
- [x] `TT_FOURSLASH=all TT_FOURSLASH_FILTER=completionsImport_defaultAndNamedConflict,completionsImport_default_anonymous,completionsImport_jsxOpeningTagImportDefault
  cargo test --test editor_cases typescript_fourslash_tests_answer_as_their_twins`:
  3 compared, 3 differ, listed; passes.
- [x] `node scripts/check-task-index`.
- [ ] The full gate: not run (the coordinator asked for targeted checks
  only).

## Result

Changed files: `docs/ai/tt.md`, `tests/typed-parity-differences.txt`,
`tests/fourslash-differences.txt`, `docs/tasks/INDEX.md`, and this record.
Follow-up outside this repository: TypeScript's binder could remove a
content mapper's extension from a module's name, as its output paths
already do.
