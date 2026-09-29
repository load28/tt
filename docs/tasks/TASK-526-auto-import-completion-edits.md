# TASK-526: Carry auto-import edits from completion resolve onto the tt source

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Completion in a `.tt` buffer offered exports of modules the file does not
import (`helperFn`, detail "Add import from "./util""), but accepting one
inserted only the name. The resolved item's `additionalTextEdits` — the import
declaration — were dropped, so the result was TS2304.

## Scope

- Included: `Project::completion_resolve`, `CompletionDetail`, the completion
  probe's record (`ProbeDoc`), the `completionResolve` server answer, and the
  extension's `onCompletionResolve`.
- Excluded: Offering completions where none are offered today (the unfinished
  `try` operand, TASK-527).

## Decisions

### Decision 1: Map the edits through the projection that produced the item

- **Context**: The service computes the edits over the text it was served —
  the emitted projection, or a completion probe with `$tt_probe` spliced in —
  so their ranges are in that text's coordinates. A `.tt` projection can
  begin with generated glue (the `@tt/runtime` import of a pipeline).
- **Alternatives considered**: Writing the edit at the source's first line
  unconditionally would guess where TypeScript meant to insert; it would put
  a second import above a user's existing imports' block, or inside a
  directive prologue.
- **Decision and rationale**: `source_edit` maps each edit's range through
  the emit mappings (`to_source_span`, which already treats a zero-width
  range at a mapping boundary as the boundary), and for a probe removes the
  placeholder's length from offsets after the splice. A probe now records its
  mappings, its source, and the splice offset; resolve only uses a probe
  built from the buffer's current text.

### Decision 2: An entry's edits are applied whole or not offered

- **Context**: An edit whose range lands in glue has no source counterpart.
- **Decision and rationale**: If any edit fails to map, the answer carries
  none — the same whole-or-nothing rule rename follows. The entry still
  resolves its detail and documentation.

## Work log

- 2026-09-29: Reproduced with a project holding `util.ts` and a `.tt` file
  starting with pipelines: `completionResolve` answered only
  `signature: "Add import from "./util""`. Added `TextEdit`,
  `CompletionDetail::additional_edits`, `source_edit`, the probe fields,
  `additionalEdits` in the server answer and `additionalTextEdits` in the
  extension. The probe now answers
  `import { helperFn } from "./util";` at 0:0 for a `.ts` target and
  `import { ttHelper } from "./lib.tt";` for a `.tt` target.
- 2026-09-29: Added `an_auto_import_completion_carries_its_import_edit_onto_the_source`
  (`tests/native/cases_06.rs`) and the LSP case "accepting an auto-import
  completion adds the import to the tt source" (`server.test.ts`, whose
  `open` helper now takes sibling files).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native`: 106 passed
- [x] `editors/vscode`: `npm run compile`, then all server and client tests:
  221 passed

## Result

Changed `src/engine/language.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`, `src/engine/mod.rs`, `src/server.rs`,
`editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/server.ts`,
`tests/native/cases_06.rs` and `editors/vscode/server/src/test/server.test.ts`.
Accepting an auto-import completion in a `.tt` buffer now writes the import.
