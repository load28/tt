# TASK-730: Keep a declaration map's source in the fourslash comparison

Follow-up: TASK-733 and TASK-737 resolve the authored `.tt` declaration-map
target coordinates and installed-mapper serving contract left open by this task;
its harness decision remains unchanged.

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-730`

## Purpose

TASK-718 D2: in `declarationMapGoToDefinition` and
`declarationMapsGoToDefinitionRelativeSourceRoot`, go to definition from
`mymodule` into `indexdef.d.ts` answered the declaration file on the tt
side, while `tsgo --lsp` on the twin followed `indexdef.d.ts.map` to the
method in `index.ts`. The task was to make the engine follow declaration
maps as tsgo does.

## Scope

- Included: the cause, and the fix in the comparison's oracle
  (`tests/typescript-diagnostics.mjs`); `tests/fourslash-differences.txt`.
- Excluded: engine changes (none needed; see Decision 1).

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/ls/source_map.go`: definition results go through
  `getMappedLocation`, which asks `tryGetSourcePosition`; that follows the
  declaration file's position mapper and returns nothing when the mapped
  source file does not exist (`if _, ok := l.ReadFile(newPos.FileName); !ok`),
  so the answer stays in the `.d.ts`.
- `tsc/internal/sourcemap/source_mapper.go`, `GetDocumentPositionMapper`
  (line 229): the map is the declaration file's `sourceMappingURL` (a
  base64 `data:` URL included) or `<file>.map`; `convertDocumentToSourceMapper`
  rejects a map without `sources`, `file` or `mappings`, or with inlined
  sources; `createDocumentPositionMapper` (line 51) resolves each source
  against `sourceRoot` (itself resolved against the map's directory) or
  the map's directory.
- `tsc/internal/sourcemap/util.go`, `TryGetSourceMappingURL`: the URL is
  the last `//# sourceMappingURL=` (or `//@ `) comment line, scanning from
  the end over blank lines and stopping at any other line.

## Decisions

### Decision 1: The difference is the comparison's; the engine already follows the map through tsgo

- **Context**: TASK-718 Decision 2 renames every `.ts` unit no other unit
  reaches by an import, an augmentation, or a reference. `index.ts` is
  reached only by `indexdef.d.ts.map`'s `"sources": ["index.ts"]`, so the
  tt project had `index.tt` and a map naming a file that does not exist,
  and tsgo itself answers the `.d.ts` there (`tryGetSourcePosition`). With
  `index.ts` kept, the engine's `definition` (through `ttc --server`) at
  `instance.methodName` answers `index.ts` 3:5-3:15, the method, as the
  twin does; with `index.tt` it answers `indexdef.d.ts`, as tsgo would.
- **Alternatives considered**: (a) Map `.d.ts` locations in the engine:
  a second implementation of what the engine's language service, `tsgo
  --lsp`, already does, and it would map to a file the project does not
  have. (b) List the questions as by design: the projects differ only
  because the harness renamed a file another file names.
- **Decision and rationale**: A declaration map's sources reach their
  files, as an import does: the oracle resolves each unit that is a
  declaration file to its map the way `GetDocumentPositionMapper` does and
  counts the sources it names as reached, so they keep their names on the
  tt side. The oracle is shared with TASK-717's corpus run, where no case
  has a declaration map, so nothing changes there.

## Work log

- 2026-10-01: Reproduced with `TT_FOURSLASH_FILTER`; rebuilt the case by
  hand and asked `ttc --server` `definition` with `index.ts` and with
  `index.tt` (answers above); read `source_map.go`, `source_mapper.go`
  and `util.go`.
- 2026-10-01: Added `declarationMapSources` and `mapSources` to the
  oracle's `check`, removed the two lines from
  `tests/fourslash-differences.txt`.

## Issues and resolutions

### Issue 1: The first oracle edit did not parse

- **Symptom**: Every converted test with the change was skipped as "the
  conversion failed".
- **Cause**: The line-separator alternatives of the line split were
  written to the file as the raw U+2028 and U+2029 characters, which end a
  regular expression literal.
- **Resolution**: Written as ` ` and ` `; `node --check` passes.

## Regression test (fails before the fix)

- **Path**: `tests/editor_cases.rs`
  `typescript_fourslash_tests_answer_as_their_twins` with
  `TT_FOURSLASH=all` and `TT_FOURSLASH_FILTER` naming the twelve fourslash
  tests that write a declaration map (33 tests match).
- **Observed failure**: with the previous oracle and the two lines no
  longer listed: "4 compared (5 questions, 2 differ)" and
  "declarationMapGoToDefinition: definition 1 differs from the TypeScript
  twin" (and the same for
  `declarationMapsGoToDefinitionRelativeSourceRoot`), `tt only:
  indexdef.d "methodName"`, `ts only: index /*2*/ "methodName"`. With the
  fix: 4 compared, 5 questions, 0 differ; the skip counts are unchanged.

## Verification

- [x] The filtered fourslash run above passes.
- [x] `node --check tests/typescript-diagnostics.mjs`.
- [ ] The full fourslash and corpus runs: not run (the coordinator asked
  for targeted checks only). No TypeScript compiler case writes a
  declaration map (`grep -rl 'sourceMappingURL\|\.d\.ts\.map'` over the
  corpus finds none).

## Result

Changed files: `tests/typescript-diagnostics.mjs`,
`tests/fourslash-differences.txt`, `docs/tasks/TASK-718-fourslash-editor-parity.md`
(a note), `docs/tasks/INDEX.md`, and this record. TASK-718 D2 is gone. Open:
a declaration map whose source is a `.tt` file (a `.d.ts` emitted by
`tsc --runExternalCode` with `declarationMap`) maps into the `.tt` file's
own text, which the engine's `map_target` reads as served coordinates;
not observed here and not examined.
