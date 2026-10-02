# TASK-644: Go to a user variant's tag or field declaration through the engine's definition

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-644: Go to a user variant's tag or field declaration through the engine's definition`

## Purpose

TASK-639's `matchHoverAndDefinition` baseline recorded `definition /*tag*/`
answering no location for `Circle` in `match (s) { Circle(r) => ... }`,
while `ttSymbol` at the same position named the case declaration. Go to
definition on a user variant's tag or payload field in a pattern must land
on the declaration through every consumer of the engine, not only through
the VS Code adapter.

## Scope

- Included: `Project::definition` in `src/engine/language/project.rs`, the
  editor cases `matchHoverAndDefinition` (and its TypeScript twin) and
  `variantTagDefinition`.
- Excluded: The VS Code adapter's definition handler (its answer does not
  change), references and rename (already answered through
  `tt_declaration`), built-in tags (TASK-610).

## Decisions

### Decision 1: The cause is a missing path in the engine, not a regression

- **Context**: The report said local variant tags navigated in round 5.
- **Alternatives considered**: Bisecting `Project::definition` for a
  removed branch.
- **Decision and rationale**: `git log -S` over `project.rs` and
  `server.ts` shows the engine's `definition` never consulted tt's own
  names: it asks the service, then the standard library for a built-in case
  (TASK-610), then the match analysis' bindings. The VS Code adapter asks
  `ttSymbol` first and returns its `definition` (TASK-107, kept by TASK-610
  Decision 1), which is why the editor navigated in round 5. The engine API
  and the `definition` method of `ttc --server`, which the editor cases ask,
  had no such step. The emission gives TypeScript nothing to answer with: a
  tag lowers to a string literal (`case "Circle":`), for which TypeScript
  answers no definition (TASK-610 Decision 2).

### Decision 2: The engine resolves a tt name before asking the service, as the adapter does

- **Context**: The rule "where is a tt name declared" already exists once,
  in `engine::names::symbol_at`, which the engine's `references` uses
  through `tt_declaration`.
- **Alternatives considered**: (a) Ask `symbol_at` only when the service
  finds nothing: at a shorthand payload binding (`Circle(radius)`) the
  service answers the binding in the lowered destructuring, so the engine
  and the adapter would give different answers at the same position.
  (b) Leave the adapter as the only place: every other engine consumer
  (the server protocol, an embedding, the editor cases) stays wrong, and
  the rule lives in a protocol adapter, which `docs/design/lsp-architecture.md`
  rejects ("LSP is not the language engine").
- **Decision and rationale**: `Project::definition` first asks
  `tt_name_declaration`, which returns `symbol_at`'s `definition` for a
  `.tt`/`.ttx` file, read as the project holds it open. The order is the
  adapter's, so both answer the same. It also matches TypeScript: for a
  name in an object binding pattern, go to definition returns the property
  declaration of the destructured type, not the binding
  (`typescript-go/internal/ls/definition.go`, the branch for
  `IsPropertyName(node) && IsBindingElement(node.Parent) &&
  IsObjectBindingPattern(node.Parent.Parent)`), and a payload pattern is
  tt's destructuring of the case. A built-in case has no `definition` in
  `symbol_at` and still falls through to TASK-610's path.

### Decision 3: The TypeScript twin has no `tag` marker

- **Context**: `matchHoverAndDefinition.ts` put `/*tag*/` on `"Circle"` in
  `case "Circle":`. TypeScript answers no definition for a discriminant
  literal, so the fixed answer would be a parity difference.
- **Alternatives considered**: List the difference in `failingParity.txt`:
  that list is for answers that should match TypeScript's and do not; this
  one is a tt name with no TypeScript counterpart.
- **Decision and rationale**: The marker is removed from the twin, so
  `hover tag` and `definition tag` are not compared; every other parity
  line is unchanged.

## Work log

- 2026-09-30: Read `Project::definition`, `builtin_case_definition`,
  `tt_declaration`, `engine::names`, and the adapter's `onDefinition`.
  `git log -S builtin_case_definition` and `git log -S engine.ttSymbol`
  confirmed the path was never in the engine.
- 2026-09-30: Added `Project::tt_name_declaration` and asked it first in
  `Project::definition`.
- 2026-09-30: Added the editor case `variantTagDefinition` (an imported
  variant's tag in a match, an `if let` and a let-else, an aliased imported
  field, a local variant's tag and aliased field) and removed the twin's
  `tag` marker; regenerated with
  `UPDATE_EXPECT=1 TT_CASES=<name> cargo test --test editor_cases` and read
  both baselines.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/variantTagDefinition.tt` and
  `tests/cases/editor/matchHoverAndDefinition.tt` (`tests/editor_cases.rs`)
- **Observed failure**: With `src/engine/language/project.rs` restored to
  its previous revision, `TT_CASES=matchHoverAndDefinition cargo test
  --test editor_cases` failed with "modified baseline ...
  matchHoverAndDefinition.baseline is out of date", `- definition: 1
  location(s)` / `- matchHoverAndDefinition.tt 1:24-1:30 "Circle"` / `+
  definition: 0 location(s)`; `variantTagDefinition` failed the same way
  with `+ definition: 0 location(s)` for the imported tag.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test editor_cases` (every case)
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native -- definition navigat
  builtin_tag prototype` (4 passed)
- [x] Full gate, recorded in TASK-646 (run once over TASK-644 to TASK-646)
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/engine/language/project.rs`,
`tests/cases/editor/matchHoverAndDefinition.ts`,
`tests/cases/editor/variantTagDefinition.tt`, the two baselines, and the
task index. Go to definition on a user variant's tag or field in a pattern
lands on its declaration through the engine, the server, and the editor.
