# TASK-536: Share unsaved buffers with every open project

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

An unsaved edit reached only the project its file belongs to. When
`test/b.tt` (its own `tsconfig.json`) imports `../src/a.tt`, editing `mk`'s
parameter type in the open `a.tt` left `b.tt`'s TypeScript diagnostics,
typed check and hover showing the old signature until `a.tt` was saved.
Inside one project the same edit is seen at once.

## Scope

- Included: where the engine keeps open documents, and how every project
  reads them.
- Excluded: the editor's revalidation schedule. It already revalidates every
  open document on any change (`documents.onDidChangeContent` in
  `editors/vscode/server/src/server.ts`).

## Decisions

### Decision 1: One document store per engine, read by every project

- **Context**: `Project::open_document` wrote the buffer into that
  project's own `overlays` map, and `openDocument` in `src/server.rs` only
  called it on the project the file belongs to. Every other project read
  the file from disk. This covered projection (`Project::update`), the
  language service's served modules (`serve`, `serve_one`), host `.ts`
  overlays and extern collection. typescript-go keeps its overlay
  filesystem on the session, so every project sees an open buffer
  (`docs/design/engine-architecture.md` §B.2).
- **Alternatives considered**:
  - Have the server copy each `openDocument`/`updateDocument`/`closeDocument`
    into every open project. The unsaved text would be a fact the server
    replicates by hand, and a project opened later would miss earlier
    buffers until the client replays them.
  - Sync each project from a workspace map before each request. This
    copies every open buffer per request, and anything that opens a project
    without the workspace (the engine API, tests) would lose the guarantee.
- **Decision and rationale**: `engine::documents::Documents` is one shared
  map (`Arc<RwLock<…>>`) owned by the `Engine`. Every project the engine
  opens holds a handle to it, and opening, updating or closing a document
  through any project changes the text for all of them. The CLI opens one
  engine per run with one project, so its behavior is unchanged.

### Decision 2: The text is shared; being a root by request is not

- **Context**: An open document is also a *root by request* of its project
  (`Project::roots`). The typed backend checks it even when the
  configuration does not include it, and reports its diagnostics in the
  answer. If this came from the shared store, every open buffer found by a
  project's scan would become a root there. The root project of a
  `src`/`test` layout would then check and report `test/b.tt` in `a.tt`'s
  typed check. The editor reads a typed check whose findings are all in
  other files as "could not run" (`runTypedCheck` in
  `editors/vscode/server/src/ttc.ts`).
- **Decision and rationale**: A project keeps `opened`, the documents
  opened through it, for its per-project uses: request roots, the
  project's own `.tt` files in pattern reference search (`tt_files`), and
  membership (`Project::sees`, TASK-535).
  Every text lookup goes through the shared store.
  `a_document_open_in_another_project_is_not_a_root_of_this_one` fails when
  `roots` reads the shared store instead.

### Decision 3: Invalidation stays content-keyed

- **Context**: The task asked that dependents in other projects be
  invalidated.
- **Decision and rationale**: No eager invalidation is added, because none
  is missing. The projection cache reuses an entry only while its source
  equals the current text. The semantic cache is keyed by content and
  imported declarations. The language service re-serves a module whose
  emitted text changed, and reopens host overlays whose text changed. So a
  project rebuilds exactly what changed at its next question, and the
  editor already revalidates every open document on any change. This is
  the existing request-time guarantee ("a request sees the edits that
  arrived before it", engine-architecture §D), now over the shared store.

## Work log

- 2026-09-29: Reproduced with `target/probe2-editor/nest` and `refs`
  through `ttc --server`. After `updateDocument` changed `mk(n: number)` to
  `mk(n: string)` in `a.tt`, `tsDiagnostics`, `typedCheck` and `hover` for
  `b.tt` in the other project still reported `mk(n: number)`.
- 2026-09-29: Added `src/engine/documents.rs`, gave `Engine` the store and
  `Project` a handle to it plus its `opened` set, and moved every overlay
  read in `src/engine/project.rs` and `src/engine/language/project.rs` to
  the store. `serve` reads the store in its own scope before taking the
  snapshot, so the lock is never taken twice on one thread.
  `Workspace::reload` clears the store with the projects.
- 2026-09-29: A first version used the shared store for request roots too.
  Found the cross-project root problem (Decision 2) by reading
  `host.mjs`'s `outside` roots, and split `opened` out.
- 2026-09-29: The probes now show `mk(n: string)` in `b.tt`'s hover right
  after the edit, TS2345 at `mk(1)` in `tsDiagnostics`, and
  `expected string, found 1` in `typedCheck`, in both layouts.
- 2026-09-29: Added `an_unsaved_edit_reaches_the_importers_in_another_project`,
  `an_unsaved_typescript_module_reaches_the_importers_in_another_project` and
  `a_document_open_in_another_project_is_not_a_root_of_this_one`
  (`tests/native/cases_08.rs`). The first two fail on the TASK-535 tree,
  and the third fails with request roots taken from the store.

## Issues and resolutions

### Issue 1: `Project::new` exceeded clippy's argument limit

- **Symptom**: `clippy::too_many_arguments` (8/7) after adding the store as
  a constructor argument.
- **Cause**: The constructor already took seven arguments.
- **Resolution**: `Project::new` starts with a store of its own, and
  `Engine::open_collected` replaces it with the engine's store. No lint is
  allowed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (1591 passed, 0 failed)
- [x] `editors/vscode`: `npm run compile` and all server and client tests
  (224 passed, three consecutive runs)

## Result

Changed `src/engine/documents.rs` (new), `src/engine/mod.rs`,
`src/engine/project.rs`, `src/engine/language/project.rs`,
`src/engine/workspace.rs`, `tests/native/cases_08.rs` and
`docs/design/engine-architecture.md`. An unsaved buffer is now the file's
text for every open project: diagnostics, typed checks and hover in an
importing project follow the edit at once, and go back to disk text when
the buffer closes.
