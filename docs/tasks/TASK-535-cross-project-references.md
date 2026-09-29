# TASK-535: Find references and rename across every open project

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Find All References and Rename from a declaration answered only from the
project that owns the requesting file. With a solution `tsconfig.json` whose
`pb` project references `pa`, references and rename at `mk` in `pa/a.tt`
returned only the declaration, while TypeScript on the same `.ts` layout
returns and renames the uses in `pb/b.ts` too.

## Scope

- Included: the server's project collection, the cross-project combination
  of references and rename, and their regression tests.
- Excluded: loading projects that no open document belongs to (TypeScript's
  `loadAncestorProjectTree`), and unsaved text reaching another project
  (TASK-536).

## Decisions

### Decision 1: Search every open project the way TypeScript's project service does

- **Context**: `src/server.rs` routed each request to the one project its
  file belongs to (`semantic()` via `project_for()`). Each project runs its
  own language service over the files its graph reaches, so `pa` never
  serves `pb/b.tt` and cannot report its uses. A solution-style layout
  (`files: []` plus `references`) always splits a declaration from its
  consumers this way.
- **Alternatives considered**:
  - One `tsgo --lsp` for the whole server, serving every project's
    documents. It would let tsgo's own multi-project search run, but it
    replaces the per-project service arrangement (content mapper, project
    root) and the typed backend is per project anyway. Much larger than
    the defect, with no gain for the other requests.
  - Ask every other open project, and let each language service decide
    what it can see. Tried first. It made every unrelated open project
    start its language service and serve a foreign file (Issue 1).
  - Ask every other project at the requested position. A project that
    reaches the declaration but not the requesting file would be asked
    about a file outside its program. The definition is the one place
    every project using the symbol reaches, which is why tsserver anchors
    there.
- **Decision and rationale**: Follow tsserver's `getPerProjectReferences`
  (`session.ts`), which tsgo mirrors. The requesting file's default project
  answers at the requested position. Each other open project whose program
  contains that answer's definition is then asked at the definition. A
  project that does not contain it but does contain the requesting file is
  asked at the requested position. The results are merged without
  duplicates. The collection is a new engine type, `engine::Workspace`: the
  analogue of tsgo's `ProjectCollection`, keyed by `(tsconfig, root)` like
  the server's old map. The server delegates to it, and the combination
  logic stays in the engine rather than in the protocol adapter.

### Decision 2: `Project::sees` is tt's `containsFile`

- **Context**: tsserver asks only projects that `containsFile` the
  location. A tt project's TypeScript program can contain a `.tt` module
  only if the project serves it, and it serves exactly its projection
  graph: its candidates and their tt imports. Hand-written TypeScript is
  admitted by TypeScript alone.
- **Decision and rationale**: `Project::sees(path)` is true for a document
  opened through the project, a tt module in the graph of a fresh snapshot
  of its candidates (`update(initial_files())`, the same graph `serve`
  starts from, updated first as tsserver's `updateProjectIfDirty` does), or
  a file its compiler read in a typed check (`dependencies`, TypeScript's
  own answer). For `.tt` sources this is exact. A `.ts` declaration is
  found in another project once that project's compiler has read it.

### Decision 3: The anchor for other projects is TypeScript's definition, stopping at aliases for rename

- **Context**: tsserver asks other projects at `getDefinitionLocation`: the
  first definition for references, and for rename the definition with
  `stopAtAlias`. So renaming at an import alias renames only the local
  binding (`mk as next`), and the declaration is not renamed in other
  projects. The LSP surface tt drives has no `stopAtAlias`.
- **Decision and rationale**: References use the first reference the
  project marks `is_definition`. Rename uses the first definition that the
  default project's own edits rename. The declaration a rename edits is
  exactly the declaration of the symbol being renamed, which is what
  `stopAtAlias` names. When the rename stops at an alias, no definition is
  among the edits. The other projects are then asked at the requested
  position, as tsserver asks the projects that contain the requesting file.
  The probe confirmed that `tsgo` on the `.ts` layout renames an import use
  as `mk as mk2` in `b.ts` only.

### Decision 4: A combined rename stays whole or nothing

- **Context**: `Project::rename` refuses a rename it cannot perform
  whole. Merging several projects' answers adds two new ways to break
  that: one project refuses, or two projects edit the same text
  differently.
- **Decision and rationale**: Any project returning `None` refuses the whole
  rename. An edit identical to one already collected (same place, same
  text) is dropped. An edit that overlaps a collected one in any other way
  refuses the whole rename. Errors from any project propagate, because a
  failed request must not look like a complete answer.

## Work log

- 2026-09-29: Reproduced with `target/probe2-editor/refs` through
  `ttc --server`. References and rename at `mk` in `pa/a.tt` answered only
  the declaration. An engine probe asking `pb`'s project at the same
  declaration returned the declaration and both uses in `pb/b.tt`, so each
  project's answer was right and only the combination was missing.
- 2026-09-29: Added `src/engine/workspace.rs` (`Workspace`, `ProjectIdentity`)
  and moved the server's project map, open-document routing,
  `reloadProjects` and `project_for` into it. `references` and `rename` now
  go through `Workspace`, and every other request still goes to one project.
- 2026-09-29: After the change, the probe answers references `pa/a.tt:1:16`
  (definition), `pb/b.tt:0:9`, `pb/b.tt:1:10` and renames all three from
  the declaration. From the use in `pb/b.tt` it still renames the local
  alias only, as tsgo does.
- 2026-09-29: Added
  `references_and_rename_reach_every_open_project_that_sees_the_symbol` and
  `the_server_answers_references_from_every_open_project`
  (`tests/native/cases_08.rs`). Both fail when the workspace asks no other
  project.
- 2026-09-29: The editor suite exposed Issue 1. Added `Project::sees` and
  `Workspace::searched` (Decision 2), and
  `a_project_sees_the_modules_its_graph_reaches`.
- 2026-09-29: Recorded the partial reversal at the top of TASK-086 and
  under `docs/design/engine-architecture.md` §D.

## Issues and resolutions

### Issue 1: Asking every open project timed out the editor's rename test

- **Symptom**: With the first version (every other project asked),
  `renaming a destructuring shorthand keeps TypeScript's expansion`
  (`editors/vscode/server/src/test/engine.test.ts`) took 12.8 s and
  sometimes returned `null` after 15 s. The suite shares one server across
  tests that each open their own project.
- **Cause**: About a dozen unrelated projects were each asked to rename in
  a file outside their programs. Each one served the foreign file and its
  imports to its own `tsgo --lsp`, starting that service if it was not
  running, only to return duplicates of the default project's edits.
- **Resolution**: Decision 2. A project that sees neither the definition
  nor the requesting file is not asked. The test takes 2.0 s, and the
  `engine.test.js` file passed on three consecutive runs.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (all suites pass; `native` 112)
- [x] `editors/vscode`: `npm run compile` and all server and client tests
  (224 passed)

## Result

Changed `src/engine/workspace.rs` (new), `src/engine/mod.rs`,
`src/engine/project.rs`, `src/server.rs`, `tests/native.rs`, `tests/native/cases_08.rs` (new),
`docs/design/engine-architecture.md` and
`docs/tasks/TASK-086-engine-architecture.md`. References and rename from a
declaration now include every open project that uses it. Loading projects
with no open document (tsserver's `loadAncestorProjectTree`) is still not
done. Unsaved text reaching other projects is TASK-536.
