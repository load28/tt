# TASK-667: Write tt's specifier for a tt module in auto-imports, and reject its served names

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-667`

## Purpose

Completing a name exported by `shapes.tt` wrote `import { shapeK } from
"./shapes.tt.ts"` (bundler resolution with `allowImportingTsExtensions`
when another import already ends in `.ts`) or `"./shapes.tt.js"`
(`nodenext`), where tt imports the module as `./shapes.tt`, as
import-path completion offers it (TASK-609). `ttc -o` then emitted
`./shapes.tt.js`, a file that does not exist (the output is
`shapes.js`), and `ttc --check-types` accepted it while `tsc` on the
output reports TS2307. This record also holds the full gate over TASK-667
to TASK-676.

## Scope

- Included: `tt_module_specifier` and `tt_specifiers_in`
  (`src/engine/language/service.rs`) applied to completion entries'
  `labelDetails.description` and resolved `additionalTextEdits`
  (`src/engine/language/project.rs`); `loweredModuleSpecifiers` in
  `src/typescript/host.mjs`; completion resolve in the editor cases
  (`tests/editor_cases.rs`); the editor cases `autoImportSpecifierEnding`
  and `autoImportSpecifierNodeNext` with their twins; the compiler case
  `generatedModuleSpecifier`; `docs/ai/tt.md` and
  `docs/design/lsp-architecture.md`.
- Excluded: code actions (the engine offers none), and a directory import
  of an `index.tt` (`./dir`), which TypeScript writes only with the
  `minimal` ending and which resolves the same way before and after
  emit.

## Sources

- TypeScript, Modules Reference, "Module resolution" and the
  `moduleResolution` options (typescriptlang.org/docs/handbook/modules/reference.html):
  a relative specifier is resolved by trying the TypeScript extensions for
  the path (`bundler`, `node10`), and a `.js` ending is substituted by its
  TypeScript counterpart (`.js` → `.ts`/`.tsx`, "file extension
  substitution"); `.ts` endings
  are accepted with `allowImportingTsExtensions`. Module specifier
  generation (`importModuleSpecifierEnding`: `minimal`, `index`, `js`, and
  a `.ts` ending when the project already writes them) picks the ending
  for the file it imports, here the served `x.tt.ts`.
- `host.mjs`, "How a lowered module reaches the compiler": a tt module is
  served as `x.tt.ts` (`x.ttx.tsx`), also as an unlisted alias in the
  content-mapper arrangement.

## Decisions

### Decision 1: The engine writes TypeScript's specifier for a served tt module in tt's form

- **Context**: TypeScript generates an import for the module it knows,
  the served `shapes.tt.ts`, with the ending its preference picks:
  none (`./shapes.tt`, correct by accident), `.ts`, or `.js`.
- **Alternatives considered**:
  - Change the preference TypeScript uses (`importModuleSpecifierEnding`
    in the served configuration): it is the user's setting for their
    TypeScript files too, and no ending yields `./shapes.tt` for `.js`
    under `nodenext`.
  - Rewrite in the adapter: the edit reaches every engine client, and the
    module model (which paths are served tt modules and under what name)
    is the engine's.
- **Decision and rationale**: `tt_module_specifier(importer, specifier)`
  inverts the engine's module model: a relative specifier whose ending is
  one TypeScript writes for the lowered file (the lowered extension or its
  JavaScript counterpart) and whose remainder names a tt source the
  session serves (or that exists) is written as that remainder,
  `./shapes.tt`. It is applied to an entry's `labelDetails.description`
  (the module shown beside the entry, TASK-612) and to each string literal
  of the resolved `additionalTextEdits` (lexed with tt's lexer, so only a
  literal's contents change). The entry's `source` is left as TypeScript
  sent it, because it identifies the entry TypeScript resolves (TASK-629).

### Decision 2: The typed check reports a tt module reached through its served name, as TS2307

- **Context**: `./shapes.tt.js` resolves in the served program only
  because `.js` → `.ts` substitution reaches the served `shapes.tt.ts`, a
  name that exists nowhere outside ttc. After `ttc` emits, the specifier
  is copied verbatim (only `.tt`/`.ttx` specifiers are rewritten, contract
  1) and names no file; `tsc` on the output reports TS2307.
- **Alternatives considered**:
  - Accept it: the typed check would pass a program whose output does not
    build, the opposite of what it is for.
  - Rewrite `./x.tt.js` on emit: widens contract 1's single exception to
    specifiers that are not tt's.
  - A new tt diagnostic: the fact is TypeScript's own (a module that
    cannot be found), stated in TypeScript's words by `tsc` on the output;
    contract 2 gives it to TypeScript.
- **Decision and rationale**: The backend, which owns the served file
  system, reports what TypeScript reports once the served names are gone:
  for each relative import, export, or `import()` specifier of a served
  module that reaches a served tt module other than by its source name
  (`x.tt.ts`/`x.tt.js`, `x.ttx.tsx`/`.jsx`/`.js`), and that names no file
  on disk, `loweredModuleSpecifiers` adds TS2307 "Cannot find module
  '...' or its corresponding type declarations." at the specifier. The
  case shows the typed check and `tsc` on the output reporting the same
  two diagnostics at the same places.

## Work log

- 2026-09-30: Added the probe's two cases with twins and a
  `completionResolve` step to the editor cases (each entry that names a
  module is resolved through both transports and its edits printed), and
  generated the baselines with the unfixed engine: `from "./shapes.tt.ts"`
  and `from "./shapes.tt.js"` in both the description and the edit.
- 2026-09-30: Implemented Decision 1; the descriptions and edits read
  `./shapes.tt`. `autoImportEntry` gains its two resolved edits
  (`./shapes.tt` and `./lib`, unchanged in substance).
- 2026-09-30: Reproduced the leak with the compiler case
  `generatedModuleSpecifier` (`ttc --check-types` exit 0, `tsc` on the
  output TS2307); implemented Decision 2; the case now reports TS2307 at
  both specifiers in the typed check.
- 2026-09-30: Full gate over TASK-667 to TASK-676 (Verification).

## Issues and resolutions

### Issue 1: The gate found two baselines out of date

- **Symptom**: The first full run failed `editor_cases`
  (`deprecatedCompletionTag.baseline` gained `resolve oldAdd from
  "./util": 1 edit(s)` / `1:13-1:13 "" -> ", oldAdd"`) and `public_api`
  (`ttc.api.txt` gained `pub exported: bool` in `hir::LetElseStmt`).
- **Cause**: The first is this task's resolve step, which reaches every
  case with an auto-import entry; the case was added by TASK-672 before
  the step existed. The second is TASK-676's (its Issue 3).
- **Resolution**: The first is accepted here after reading it (the edit
  adds the name to the existing `import { api }`); the second is TASK-676's
  own second commit.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/autoImportSpecifierEnding.tt`,
  `tests/cases/editor/autoImportSpecifierNodeNext.tt` (their baselines
  under `tests/baselines/reference/editor/`), and
  `tests/cases/compiler/generatedModuleSpecifier.tt`
  (`tests/baselines/reference/generatedModuleSpecifier.errors.txt`).
- **Observed failure**: Without the engine change the editor baselines had
  `shapeK (Variable, 16) from "./shapes.tt.ts"` and the resolved edit
  `import { shapeK } from \"./shapes.tt.ts\";` (and `.tt.js` under
  `nodenext`); without the backend change the compiler case's
  `ttc --check-types` section was empty (exit 0) while `tsc` on the output
  reported TS2307. Each is a modified baseline against the committed one.

## Verification

Full gate over TASK-667 to TASK-676, on this tree, serialized
(`CARGO_BUILD_JOBS=2`):

- [x] `cargo fmt --check`: clean.
- [x] `cargo clippy --all-targets -- -D warnings`: clean.
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast`: every suite passed except the two baselines of Issue 1
  (`editor_cases`, `public_api`); after accepting them, both suites passed
  unfiltered with the same tracking directory. No `SKIP`.
- [x] `node scripts/check-baselines --tracking <dir>`: "baselines: 324
  compared, none unused".
- [x] Extension: `npm run compile`, then `node --test
  "server/out/test/*.test.js" "client/out/test/*.test.js"` with this
  build's `ttc` on `PATH`: 238 tests, 238 passed, 0 skipped.
- [x] `./scripts/ci agents`: passed (warnings: rolldown not on `PATH`, and
  doctor reports the checkout not ready, both environmental).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/language/service.rs`,
`src/engine/language/project.rs`, `src/typescript/host.mjs`,
`tests/editor_cases.rs`, `CONTRIBUTING.md`, the three cases and their
baselines, `tests/baselines/reference/editor/deprecatedCompletionTag.baseline`,
`tests/baselines/reference/editor/autoImportEntry.baseline`,
`docs/ai/tt.md`, `docs/design/lsp-architecture.md`,
`docs/tasks/INDEX.md`, and this record.
