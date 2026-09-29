# TASK-543: Leave the backend's own mapper package out of project dependencies

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

`ttc --dependencies` listed ttc's own temporary content-mapper package
(`<tmp>/ttc-host-<hash>/typed-engine-mapper-<hash>/package.json`) whenever
the project had a `tsconfig.json`. `docs/ai/tt.md` describes the array as
the project's input paths for build integrations, and a typed watch also
stat'ed the file on every poll through `Project::watch_paths`.

## Scope

- Included: What the TypeScript backend host (`src/typescript/host.mjs`)
  records as dependencies and directory listings.
- Excluded: TypeScript's own library files, which the compiler reads as
  inputs of the program and stay listed.

## Decisions

### Decision 1: The host leaves out the packages it publishes

- **Context**: A configured project names the identity content mapper
  `@tt/typed-engine-mapper`. The host writes that package into its own
  temporary directory and links it into the project's `node_modules`
  through the layered file system (`links`). The layered `readFile` records
  every file the compiler reads that the engine did not serve, so the
  compiler's read of the package manifest became a project dependency.
- **Alternatives considered**: Filtering the paths in the engine or the CLI
  would need them to recognise the host's temporary layout, which only the
  host knows, and would leave `tsgo` concepts leaking out of
  `src/typescript/`.
- **Decision and rationale**: The host is the layer that creates the
  package and its link, so the layered file system records neither a read
  nor a directory listing under a linked path or its target. Every other
  read is recorded as before.

## Work log

- 2026-09-29: Reproduced: `--dependencies src` in a configured project
  printed the mapper package's `package.json` from the system temporary
  directory.
- 2026-09-29: Added `published` to `layeredFileSystem` and applied it to
  `readFile` and `getAccessibleEntries`. Added
  `dependencies_of_a_configured_project_are_only_its_inputs`
  (`tests/workflow_repairs.rs`), which fails before the change with that
  path as the only one outside the repository.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/typescript/host.mjs` and `tests/workflow_repairs.rs`.
`--dependencies` and a typed watch no longer report or poll the backend's
own mapper package.
