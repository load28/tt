# TASK-698: Point a variant constructor's missing argument at the field it is for

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-698`

## Purpose

TASK-676 Issue 2: `Shape.Circle()` reports TS2554 with the related
information "An argument for 'radius' was not provided" at `Shape`, the
variant's name, in the CLI and in the editor. TypeScript points it at the
parameter's declaration, which in tt is the field `radius: number`.

## Scope

- Included: the constructor parameters in `emit_adt`
  (`src/codegen/core/emitter/helpers.rs`), the compiler case and the editor
  case `variantConstructorMissingArgument`, and the TASK-676 note.
- Excluded: any mapping of the parameter name for navigation, hover, or
  semantic tokens.

## Decisions

### Decision 1: Give each generated parameter a diagnostic origin, not a mapping

- **Context**: `emit_adt` writes a constructor's parameter names as
  generated text (`push_lit`) and copies the field's type from the source.
  A related span over the parameter declaration therefore lies in the
  variant's anchor and maps to its display range, `Shape`. TASK-676 noted
  that mapping the parameter name to the field (`EmitMapping`) would also
  move TypeScript's answers about the parameter (semantic tokens, hover,
  references, rename) onto the field, beside the union member's property
  that already maps there.
- **Alternatives considered**:
  - Map the parameter name as copied text: changes the editor answers
    TASK-676 named, and rename of the field would then also have to rename
    the generated object literal's shorthand.
  - Recognize TS2554's related message in the reporter: a string-shape
    branch (contract 3), and only for one code.
  - Wrap each generated parameter declaration in its own `EmitAnchor` whose
    source range is the field (`name` to the end of its type). An anchor
    records the origin of glue for diagnostics only
    (`src/lib/mapped.rs`, `EmitAnchor`: "deliberately one-way and for
    diagnostics only: navigation and rename must never resolve into
    glue"); anchors nest and the inner one comes first, so every consumer
    that takes the first match (`mapper::diagnostic_origin`,
    `MappedEmit::anchor_at`, the content mapper's span mappings, which
    give anchor spans no navigation features) resolves a span inside the
    parameter to the field.
- **Decision and rationale**: The third. It is the general rule for any
  diagnostic or related place on a generated parameter declaration, it uses
  the existing origin machinery of the CLI, server, and editor, and it
  leaves `mappings` unchanged, so semantic tokens, hover, references, and
  signature help cannot move. The editor case was baselined before the
  change and again after it: the only difference is the related place of
  the diagnostic, in the service and the published list.

## Work log

- 2026-10-01: Reproduced with `tests/cases/compiler/variantConstructorMissingArgument.tt`
  (`----- An argument for 'radius' was not provided.` under `Shape`).
- 2026-10-01: Added `parameter` in `emit_adt`, used by the inline and the
  documented parameter lists.
- 2026-10-01: Wrote `tests/cases/editor/variantConstructorMissingArgument.tt`
  (hover on the field, its use, and the constructor; references of the
  field; signature help; semantic tokens; diagnostics), baselined it
  without the change, applied the change, and compared: two lines differ,
  both the related place.
- 2026-10-01: `UPDATE_EXPECT=1` over case, editor, snapshot, emit map,
  content mapper, compile, public API, and library tests changed no other
  baseline.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/variantConstructorMissingArgument.tt`;
  `tests/cases/editor/variantConstructorMissingArgument.tt`.
- **Observed failure**: Without the change the case's `.errors.txt` labels
  `Shape` (3:16-3:21) for both missing arguments, and the editor baseline
  reads `related 1:16-1:21 "Shape"` instead of
  `related 1:31-1:45 "radius: number"`.

## Verification

- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] `cargo test --test case_baselines --test editor_cases --test snapshot
  --test emit_map --test content_mapper --test compile --test public_api
  --test native --test integration --lib`: passed. The full gate is
  recorded in TASK-699.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/codegen/core/emitter/helpers.rs`, the compiler and
editor cases and their baselines, the TASK-676 note, `docs/tasks/INDEX.md`,
and this record.
