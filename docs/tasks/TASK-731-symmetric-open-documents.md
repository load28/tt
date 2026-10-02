# TASK-731: Hold the same documents open on both sides of the editor comparison

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-731`

## Purpose

TASK-718 D3: in `findAllRefsTripleSlashRef1`, find all references at the
specifier of `import type {JSX} from 'react'` answered only the specifier
on the tt side, while `tsgo --lsp` on the twin also answered `JSX` in
`node_modules/@types/react/index.d.ts`. The task was to match tsgo.

## Scope

- Included: the cause, and the fix in `tests/editor_cases.rs`'s `run`;
  `tests/fourslash-differences.txt`.
- Excluded: engine changes (none needed; see Decision 1).

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/fourslash/fourslash.go` (lines 262-276, 993-1010): the
  harness opens a test's files as it goes to their markers; which files
  are open is part of the state a question is asked in.
- `tsc/internal/ls/crossproject.go` and `findallreferences.go`: references
  are gathered over the projects of the open documents, so a declaration
  file held open adds the references its project finds.

## Decisions

### Decision 1: The difference is the comparison's; both sides now hold the same documents open

- **Context**: `run` opened every `.ts`/`.tsx` unit of the twin in `tsgo
  --lsp` (the two `node_modules` declaration files included) but only the
  `.tt`/`.ttx` units on the tt side. Asked directly, `tsgo --lsp` on the
  twin answers only the specifier when just `index.ts` is open, and adds
  `node_modules/@types/react/index.d.ts` 1:13-1:16 once the two
  declaration files are open. The engine, given the same documents
  through `ttc --server` `openDocument` (the editor forwards open host
  TypeScript buffers the same way, `docs/design/lsp-architecture.md`),
  answers exactly that pair.
- **Alternatives considered**: (a) Add the declaration file in the engine's
  references: an answer tsgo does not give for the same state. (b) Open on
  the twin only the counterparts of the opened tt units: changes the
  established answers of every twin, where (c) changes only the tt side's
  state to match. (c) Open on the tt side, besides the `.tt`/`.ttx` units,
  every `.ts`/`.tsx` unit the twin opens under the same name.
- **Decision and rationale**: (c), in `run`, for every case with a twin
  (the converted fourslash tests and the written cases with a twin), on
  the server and on the in-process engine.

## Work log

- 2026-10-01: Reproduced with `TT_FOURSLASH_FILTER`; rebuilt the case by
  hand; asked `tsgo --lsp` (a small LSP client) with and without the
  declaration files open, and `ttc --server` with and without
  `openDocument` for them (answers above).
- 2026-10-01: Opened the twin's TypeScript units on the tt side in
  `run`; removed the line from `tests/fourslash-differences.txt`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/editor_cases.rs`
  `typescript_fourslash_tests_answer_as_their_twins` with
  `TT_FOURSLASH=all TT_FOURSLASH_FILTER=findAllRefsTripleSlashRef1`.
- **Observed failure**: with the previous `run` and the line no longer
  listed: "findAllRefsTripleSlashRef1: references m differs from the
  TypeScript twin", `ts only: node_modules/@types/react/index.d "JSX"`.
  With the fix: 1 compared, 0 differ.

## Verification

- [x] The filtered run above passes.
- [x] `TT_FOURSLASH=all TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1 cargo test --test editor_cases`: both tests pass; every fourslash test: 4,352 tests, 1,829 compared, 5,808 questions, 48 differ (all listed; TASK-718 had 51, less the three of TASK-730 and this task), 2,523 skipped with the same reasons and counts as TASK-718; the written editor cases with the default 40-case matrix sample pass (with this change and TASK-730's).
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [ ] The full gate: not run (the coordinator asked for targeted checks
  only).

## Result

Changed files: `tests/editor_cases.rs`, `tests/fourslash-differences.txt`,
`docs/tasks/TASK-718-fourslash-editor-parity.md` (the note),
`docs/tasks/INDEX.md`, and this record. TASK-718 D3 is gone; every
TASK-718 defect is fixed or classified.
