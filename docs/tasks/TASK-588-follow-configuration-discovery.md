# TASK-588: Follow configuration discovery in typed watch and dependencies

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-588: Follow configuration discovery in typed watch and dependencies`

## Purpose

`ttc --check-types -w src` never noticed a `tsconfig.json` being created
(ignored) or deleted (TS5083 "Cannot read file" every round), while a
one-shot run handles both. `--dependencies` had the same blind spot: it
reported `"directories": []` for a project without a configuration, whose
program is formed by walking its root, so a bundler adapter invalidated on
neither a new configuration nor a new file.

## Scope

- Included: configuration discovery's probe list
  (`src/engine/project.rs`), `Project::dependencies`/`dependencies_for`,
  the server's cached dependencies answer (`src/server.rs`), the typed
  watch (`src/main/typed.rs`), `docs/ai/tt.md`, regression tests.
- Excluded: the language server's project identity, which the workspace
  already resolves per request (`src/engine/workspace.rs`).

## Decisions

### Decision 1: Discovery's probed paths are file dependencies of the inputs, existing or not

- **Context**: `find_tsconfig` walks from the inputs' common directory to
  the filesystem root and takes the first `tsconfig.json`, as
  `ts.findConfigFile` does (TypeScript compiler API). Its answer depends
  on each probed path's existence, not on any directory's full listing.
  Which paths it probes depends on the inputs, so one project serving
  several files (the server) answers per request.
- **Alternatives considered**: (a) Report the probed directories as
  `directories`: bundlers watch a directory dependency recursively
  (webpack context dependencies, chokidar through Rollup's
  `addWatchFile`), and the probe reaches `/`. (b) A third list of
  "missing" paths (webpack's `missingDependencies`): a protocol change for
  every consumer, while Rollup's `addWatchFile`, chokidar and watchpack
  already watch a file path that does not exist yet.
- **Decision and rationale**: `files` also lists every probed
  `tsconfig.json` path, the way tsserver watches the config file locations
  of an inferred project's ancestors (TypeScript `src/server/
  editorServices.ts`, config file existence watching for inferred
  roots). `Project::dependencies_for(inputs)` adds them when the
  configuration was discovered rather than named (`--project`);
  `dependencies_of` and the server's cached answer both use it, so the
  CLI and the server keep answering the same object.

### Decision 2: Without a configuration, the root walk's directories are directory dependencies

- **Context**: With a configuration, TypeScript's `include` globbing lists
  directories through the host, and those are reported. Without one, the
  engine's own root walk (`project_sources`) decides the program's
  files, and it listed nothing TypeScript saw.
- **Alternatives considered**: Report the walk's directories always: a
  configured project's membership is TypeScript's, and the extra
  directories would only widen what bundlers watch.
- **Decision and rationale**: `project_tree` returns the walk's directories
  with its files; `dependencies` adds them when the project has no
  configuration.

### Decision 3: The typed watch re-runs discovery each round and reopens on a change

- **Context**: A project is opened as a `(tsconfig, root)` identity; a
  created or deleted configuration changes it, and the running host keeps
  the old configuration (hence TS5083 on deletion).
- **Alternatives considered**: Re-read the configuration inside the
  existing session: the identity (and root) itself changes, which the
  engine models as another project.
- **Decision and rationale**: Each round the watch asks
  `Engine::project_identity` for its inputs and reopens the project when
  the answer differs from `Project::identity`, re-applying overlays. A
  reopen failure is printed once per distinct message.

## Work log

- 2026-09-30: Reproduced with a watch over `src/a.tt` (no configuration):
  creating `tsconfig.json` with `strict: false` left the TS7006 standing;
  deleting a configuration printed TS5083 each round (probe `tw`).
- 2026-09-30: Added `tsconfig_lookup`, `project_tree`,
  `Project::identity`, `Project::dependencies_for`; the watch reopens on a
  new identity. The first attempt stored the probe list on the project,
  which failed `server_dependencies_check_a_file_its_configuration_leaves_out_as_a_root`
  (Issue 1).
- 2026-09-30: Added `typed_watch_follows_configuration_discovery` and
  `dependencies_of_an_inferred_project_name_discovery_and_the_walk`
  (`tests/workflow_repairs.rs`); extended
  `dependencies_of_a_configured_project_are_only_its_inputs` to expect the
  probed, missing `src/tsconfig.json`.

## Issues and resolutions

### Issue 1: The server and the CLI answered different probe lists

- **Symptom**: The server's `dependencies` for `app/out.tt` differed from
  `ttc --dependencies app/out.tt` in the `tsconfig.json` paths.
- **Cause**: The probe list was stored when the project opened, from the
  first document; another file of the same project probes from its own
  directory.
- **Resolution**: The list is computed from each request's inputs
  (`dependencies_for`).

### Issue 2: The TASK-540 watch regression expected TS5083 for a deleted, discovered configuration

- **Symptom**: `a_typed_watch_reports_a_missing_configuration_and_recovers_when_it_returns`
  (`tests/native/cases_07.rs`) failed in the full gate: after the rename,
  the pass reported nothing instead of `error[ts5083]`.
- **Cause**: That is this task's intended change for a discovered
  configuration: a fresh run finds none and checks an inferred project.
  Running the test with `--project tsconfig.json` then exposed a defect of
  the first version: the watch re-ran discovery for a named configuration
  too, and a missing named file resolved to a different identity whose
  reopen failed, so the watch never passed again.
- **Resolution**: The watch re-runs discovery only when the configuration
  was not named; a named configuration that disappears keeps TASK-540's
  TS5083 and recovery. The regression now names its configuration, and
  the TASK-540 record says so at the top.

## Verification

- [x] `cargo test --test workflow_repairs --test cli --test cli_outputs --test integration dependenc`
- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test` (18 passed)
- [x] Full gate and extension tests run once at the end
  of the TASK-587–592 series; see TASK-592.

## Result

Changed `src/engine/project.rs`, `src/engine/mod.rs`, `src/server.rs`,
`src/main/typed.rs`, `docs/ai/tt.md`, and `tests/workflow_repairs.rs`.
