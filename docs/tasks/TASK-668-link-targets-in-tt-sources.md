# TASK-668: Point `{@link}` targets in documentation at the `.tt` source

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-668`

## Purpose

A JSDoc `{@link other}` in hover documentation (and in completion and
signature help documentation) linked to the generated module:
`[other](file:///.../main.tt.ts#32,17-32,22)`, a file that does not exist
on disk, at a line of the emission. Following the link in the editor went
nowhere. TypeScript's twin links to `main.ts` at the declaration.

## Scope

- Included: `source_links` in `src/engine/language/service.rs`, its use for
  hover, completion resolve, and signature help documentation
  (`src/engine/language/project.rs`), the editor case
  `jsdocLinkTargets` with its twin, and `linked_places` in
  `tests/editor_cases.rs` (parity compares link targets by place).
- Excluded: links TypeScript writes as plain text (`{@linkplain}` without
  a target), which carry no location.

## Decisions

### Decision 1: Map each link target through the emit mapping, as a navigation target

- **Context**: tsgo renders a `{@link}` whose target resolves to a
  declaration as a markdown link to the declaring file's URI with the
  declaration's range as a one-based fragment
  (`file:///path#startLine,startCol-endLine,endCol`), in the hover,
  completion, and signature documentation it sends (LSP 3.17
  `MarkupContent`). For a declaration in a `.tt` file that file is the
  served emission (`x.tt.ts`), so both the file and the range are
  generated.
- **Alternatives considered**:
  - Ask for plain-text documentation: the link and its target are lost,
    and TypeScript's twin keeps them.
  - Drop the link target and keep the name: a regression from the `.ts`
    twin, where the link opens the declaration.
- **Decision and rationale**: `source_links` finds each link whose target
  is a `file://` URI with a range fragment, and when the URI names a served
  tt document it maps the range with `map_target` in navigation mode, the
  mapping every go-to-definition answer takes (a declared name in glue
  falls back to the name as written). The link then names the `.tt` file
  at the source range, in the same one-based fragment form. A target that
  is not a served tt document, or has no source counterpart, is left as
  TypeScript wrote it. The rewrite is applied where the engine returns
  documentation (`Project::hover`, `completion_resolve`,
  `triggered_signature_help`), so the server protocol and the adapter
  relay it unchanged.

### Decision 2: Parity compares a link by the place it names

- **Context**: The twin's link names the twin's file under a different
  temporary directory, so a raw comparison always differs and would put a
  machine-dependent path into the baseline.
- **Decision and rationale**: `linked_places` rewrites each link target in
  the parity view to the unit stem and covered text, as `parity_view`
  compares locations; both links now read `[other](main "other")`.

## Work log

- 2026-09-30: Added `tests/cases/editor/jsdocLinkTargets.{tt,ts}` from
  `target/probe7-editor/cases` and generated its baseline with the unfixed
  engine: links to `main.tt.ts#32,17-32,22` and `shapes.tt.ts#10,17-10,26`,
  and parity differences naming the twin's absolute path.
- 2026-09-30: Added `source_links` and `linked_places`; regenerated the
  editor baselines (only this case changed).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/jsdocLinkTargets.tt`
  (`tests/baselines/reference/editor/jsdocLinkTargets.baseline`).
- **Observed failure**: Without the engine change the hover documentation
  read `*@see* — [other](file://$DIR/main.tt.ts#32,17-32,22)` and `Area.
  See [perimeter](file://$DIR/shapes.tt.ts#10,17-10,26).`, and parity
  reported both hovers as differing; the committed baseline links
  `main.tt#8,17-8,22` and `shapes.tt#4,17-4,26` with parity `same`.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases`: only the new case.
- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/language/service.rs`,
`src/engine/language/project.rs`, `tests/editor_cases.rs`,
`tests/cases/editor/jsdocLinkTargets.{tt,ts}` and its baseline,
`docs/tasks/INDEX.md`, and this record.
