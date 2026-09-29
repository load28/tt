# TASK-556: Keep the pipeline runtime's helpers out of completion

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-556: Keep the pipeline runtime's helpers out of completion`

## Purpose

Completion in a `.tt` buffer with a pipeline offered `$tt_ap` or `$tt_fl`
as an auto-import from `@tt/runtime`, and accepting it wrote `$tt_ap, ` at
0:0, corrupting the first line of the user's file. Generated names must
never surface, and an edit that changes glue must never reach the source.

## Scope

- Included: Which completion entries the service answer keeps
  (`ts_completions`), how the prelude is recorded as glue written at a
  source point (`insert_declarations_at_source`, `InsertedGlue`), and which
  edits `source_edit` maps back.
- Excluded: Where the prelude is placed (TASK-527), and completion entries
  of user modules.

## Decisions

### Decision 1: Every export of the runtime is generated

- **Context**: `ts_completions` drops the names the emission allocated
  (`generated_names`). A file that imports `$tt_fl` allocates only it; the
  runtime's other export is not a name of this file, so TypeScript's
  auto-import of it from `@tt/runtime` passed the filter.
- **Alternatives considered**: (a) Drop every label with the `$tt_` prefix:
  a module may declare its own `$tt_` names (the emission renames around
  them), which would then vanish from completion. (b) Add the runtime's
  export names to the file's generated set: the user's own binding of the
  same name would be dropped too.
- **Decision and rationale**: `@tt/runtime` is compiler-owned (docs/ai/tt.md):
  every export of it is a helper the emitter calls under a generated name.
  An entry is dropped when TypeScript offers it as an import from that
  package (`StdPackage::Runtime`), whatever its name.

### Decision 2: An edit is at a glue point only between the prelude's declarations

- **Context**: TypeScript answered the auto-import with an edit inside the
  generated `import { $tt_fl } from "@tt/runtime";` (adding the name to the
  import). `source_edit` (TASK-527, Decision 5) mapped every zero-width
  edit anywhere inside the prelude to its source point, so an edit that
  changes glue was written into the user's text.
- **Alternatives considered**: Accept only an edit at the prelude's start
  or end: TypeScript inserts a new import after the existing imports, which
  is between the runtime import and the helper functions, inside the
  prelude.
- **Decision and rationale**: The prelude is a sequence of declarations
  that each stand at the source point. The rope inserts each as its own
  `InsertedGlue` (`insert_declarations_at_source`), and `source_edit` maps a
  zero-width edit only at a record's start or end. An edit inside a
  declaration changes glue and has no source counterpart, so the entry
  offers no edits (TASK-526's whole-or-nothing rule). This narrows TASK-527,
  Decision 5; that record says so.

## Work log

- 2026-09-29: Reproduced through `ttc --server` with
  `const trimmed = flow |> String |> .trim();\nconst t = ;\n` (completion at
  1:10 answered `$tt_ap`, resolve answered `$tt_ap, ` at 0:0) and with a
  `cat(` step at the end of the file (`$tt_fl`, `, $tt_fl` at 0:0). The raw
  entries carry `data.autoImport.moduleSpecifier: "@tt/runtime"`.
- 2026-09-29: Added `imports_from_runtime` to the completion filter, the
  per-declaration prelude (`src/codegen/core/mod.rs`,
  `src/codegen/rope/builder.rs`), and the boundary rule in `source_edit`.
- 2026-09-29: Tests: `completion_never_offers_the_pipeline_runtime_helpers`
  (`tests/native/cases_10.rs`) and
  `an_edit_of_the_prelude_maps_only_between_its_declarations`
  (`src/engine/language/tests.rs`). Both failed with the old filter and
  mapping rule (`["$tt_ap"]`; an edit inside `import { ` mapped to 0:0).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --lib --test native --test snapshot --test emit_map`

## Result

Changed `src/engine/language/service.rs`, `src/codegen/core/mod.rs`,
`src/codegen/rope/builder.rs`, `src/codegen/rope/tests.rs`,
`src/engine/language/tests.rs`, `tests/native.rs`,
`tests/native/cases_10.rs`, and the TASK-527 record. Completion no longer
offers the runtime's helpers, and an auto-import edit that would change the
generated prelude is not offered.
