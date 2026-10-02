# TASK-580: Report dependency directories apart from files and register each as its bundler expects

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: the `TASK-580` commit on this branch

## Purpose

The Vite dev server answered 500 for every `.tt` module (including the
create-tt scaffold) with `Failed to resolve import "/…/app" from
"src/app.tt?lang.ts"`, and after `server.close()` a Vite 6 process never
exited. `ttc --dependencies` mixed the directories TypeScript listed into its
list of files, and the adapter gave every path to `this.addWatchFile`.

## Scope

- Included: the TypeScript host reports read files and listed directories
  separately; `Project::dependencies` returns both kinds; `--dependencies` and
  the server's `dependencies` print `{"files", "directories"}`; the unplugin
  adapter registers each kind through the host's API and keeps Vite's dev
  server out of `addWatchFile`; Rust, adapter and create-tt e2e tests; docs.
- Excluded: esbuild's `watchDirs` (unplugin's esbuild bridge passes only
  `watchFiles`; TASK-581 gives esbuild its own `onLoad`), HMR pushes for a
  created or deleted file (Vite calls `handleHotUpdate` only for updates; the
  module is invalidated, which is the contract TASK-384 set), and the
  server/CLI unification of TASK-583.

## Decisions

### Decision 1: Directories are a separate kind in the protocol, decided by the host

- **Context**: `docs/ai/tt.md` calls the output "project input paths for build
  integrations". Since TASK-384 it also carried the directories the host
  listed (`getAccessibleEntries`), which matter: a file added to an `include`
  directory changes the program. But no bundler watches a directory through
  its file API the way it watches a file: Rollup's `addWatchFile` accepts
  both, webpack separates `fileDependencies` from `contextDependencies`,
  esbuild separates `watchFiles` from `watchDirs`, and Vite's dev server
  resolves whatever `addWatchFile` names as an import of the module.
- **Alternatives considered**: (a) Drop directories from the output: loses
  the membership invalidation TASK-384 added. (b) Classify paths in Rust by
  `is_dir()` when answering: a directory that was deleted since it was listed
  would be reported as a file, and the classification would be a guess about
  what the host did. (c) Mark directories inside one list (a trailing
  separator): a string-shape convention every consumer must parse.
- **Decision and rationale**: The host already keeps the two in separate maps
  (`dependencies` for `readFile`, `listings` for `getAccessibleEntries`); it
  now answers them as `dependencies` and `directories`
  (`src/typescript/host.mjs`, three lines). `Answers::directories` carries
  them, `Project::dependencies()` returns `Dependencies { files,
  directories }`, and `watch_paths()` (typed watch, the server's stamps) stays
  their union because a directory's modification time does change with its
  entries. `--dependencies` prints `{"files": [...], "directories": [...]}`,
  and the server answers the same object from the same value
  (`Dependencies::to_json`).

### Decision 2: Each bundler gets each kind through its documented API; Vite's dev server gets its watcher

- **Context**: The adapter must register both kinds for every host.
- **Alternatives considered**: `addWatchFile` everywhere, which is what broke
  Vite dev; or skipping directories in dev, which loses membership
  invalidation there.
- **Decision and rationale**: Rollup, Rolldown, Vite's build and Farm take
  both kinds through `this.addWatchFile` (Rollup documents a directory as a
  valid id; Vite 6 on Rollup and Vite 8 on Rolldown were measured to rebuild
  when a file is added to a listed directory). webpack and Rspack take
  directories as the loader's `addContextDependency`, from unplugin's
  `getNativeBuildContext()`. In Vite's dev server, `addWatchFile` from `load`
  enters `_addedImports`, which `vite:import-analysis` resolves as imports:
  that resolved the project root as a module (the 500), and
  `ensureWatchedFile` added the root itself to chokidar as an "outside"
  path, which Vite 6 never released on close (the hang). The dev server's own
  watcher already covers everything under the root, so the adapter adds only
  the dependencies outside the root to `server.watcher` (Vite's documented
  `configureServer` watcher) and `watchChange` — which Vite calls for
  `update`, `create` and `delete` — invalidates a module when a file it read
  changed or an entry appeared in or left a directory it listed. The
  bundler's framework comes from unplugin's `meta.framework`.

## Work log

- 2026-09-29: Reset the worktree to `claude/ecstatic-dijkstra-qw5pf9`, ran
  `npm ci` and `npm ci --prefix integrations/unplugin`, built debug `ttc`.
- 2026-09-29: Reproduced with the finder's probes: `/src/main.ts` then
  `/src/app.tt?import&lang.ts` answered 500 on Vite 8 (`probe4-cli/app`);
  Vite 6 (`probe4-cli/vp`) left 29 `FSWatcher` handles after `close()`.
  `ttc --dependencies` for `vp/src/main.tt` listed `vp` and `vp/src` among
  its files.
- 2026-09-29: Split the host's answer, added `Answers::directories`,
  `Project::dependencies`, `Dependencies`, and moved `--dependencies` and the
  server to the object (`src/typescript/host.mjs`, `backend.rs`, `native.rs`,
  `src/engine/project.rs`, `mod.rs`, `src/main/modes.rs`, `src/server.rs`).
- 2026-09-29: Adapter: `dependenciesOf`, `watchDependencies`, `dependsOn`,
  `watchChange` with the event (`integrations/unplugin/index.js`).
- 2026-09-29: Probes after the fix: both dev servers answer 200 and their
  processes exit after `close()`; adding a file beside the module invalidates
  it in both; `vite build --watch` rebuilds after an edit and after a file is
  added to the listed directory on Vite 6 and Vite 8.
- 2026-09-29: Tests and docs (below).

## Issues and resolutions

### Issue 1: The dev request passed when the module was requested first

- **Symptom**: The first e2e version requested `/src/app.tt?import&lang.ts`
  directly and passed even with the old plugin and compiler.
- **Cause**: Not established in detail; the failure appears when the module is
  first transformed after its importer, which is the order a browser uses.
- **Resolution**: The e2e requests `/src/main.ts` first, then the module. With
  the old plugin and the pre-fix compiler it fails with exactly the reported
  `Failed to resolve import "…/app" from "src/app.tt?lang.ts"`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (full run recorded in TASK-583, the last
  task of this branch); subsets here: `workflow_repairs` dependencies tests,
  `cli` `server_dependencies_answer_what_dependencies_prints`
- [x] New `dependencies_list_directories_apart_from_files`
  (`tests/workflow_repairs.rs`): fails on the old array output
- [x] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix integrations/unplugin test`:
  16 passed, including the new `dependency files and directories are
  registered as each host defines them` and `an entry added to or removed from
  a listed directory invalidates the module`, and the Windows test extended
  with a `create` event
- [x] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix packages/create-tt run test:e2e`:
  1 passed; the scaffold's dev server serves `src/app.tt` with 200 and the
  script exits by itself after `close()`. With the old plugin and the pre-fix
  release compiler the same test fails with the reported 500.

## Result

Changed files: `src/typescript/host.mjs`, `src/typescript/backend.rs`,
`src/typescript/native.rs`, `src/engine/project.rs`, `src/engine/mod.rs`,
`src/main/modes.rs`, `src/main.rs`, `src/server.rs`,
`tests/workflow_repairs.rs`, `tests/cli/server_print.rs`,
`integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
`integrations/unplugin/test/plugin.test.mjs`,
`integrations/unplugin/test/server.test.mjs`,
`integrations/unplugin/test/windows.test.mjs`,
`packages/create-tt/e2e/scaffold.test.mjs`, `docs/ai/tt.md`, `CHANGELOG.md`,
`docs/tasks/INDEX.md`, this record.

`--dependencies` and the server's `dependencies` answer files and directories
separately, and the adapter registers each with the bundler as that bundler
defines it. The Vite dev server serves `.tt` modules and closes cleanly;
`vite build --watch` keeps rebuilding on edits and on directory membership.
