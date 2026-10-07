# TASK-783: Fix the seventh audit's command-line findings

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-783: Fix the seventh audit's command-line findings`

## Purpose

The seventh audit of the command line (a read-only agent against a release
build of `a71c5729`) found seven defects where `ttc` disagreed with
TypeScript or with its own one-shot build. Fix each in the layer that owns
it.

## Scope

- Included: D1–D7 of this round.
- Excluded: the round's observations kept as they are (`--project` takes
  the file, `-j1`, tsgo's `{0}` in TS18000, diagnostic order under
  TS18003, the content mapper publishing `@tt/std`, `tt/cjs/package.json`
  under a `tsc` `outDir`, and the server's `print` returning decoded text).

## Decisions

### Decision D1: The standard-library declarations `--types` writes are recorded outputs

- **Context**: `--types` wrote `tt/{index,option,result}.d.ts` with no
  ownership record, so a later build in the same tree copied them as
  hand-written TypeScript (`src/tt/index.d.ts → out/tt/index.d.ts`).
  TASK-770 decision 7 records every sidecar for this reason.
- **Decision and rationale**: Each one is recorded like a sidecar, under the
  support identity `@tt/std/<module>.d.ts`, so the build's scans leave it
  out.

### Decision D2: The typed check serves a projection through a symlinked path

- **Context**: `import { s } from "./link/s.tt"` (a linked directory) or
  `"./fl.tt"` (a linked file) reported TS2307 under `--check-types`, while
  `tsc --runExternalCode` and the build resolve them. The host serves each
  projection at its source's canonical path; TypeScript asked for the path
  it resolved through the link and the host fell back to the disk, which
  has only the `.tt`.
- **Decision and rationale**: The host answers `fileExists`, `readFile` and
  `realpath` for a path whose directory, or whose `.tt` source, is a link
  to a served projection with that projection, as TypeScript itself
  resolves a symlink through `realpath`.

### Decision D3: `--dependencies` lists the probes of each named input

- **Context**: The typed check finds each named input's configuration from
  that input's own location (`project_groups`), but `--dependencies`
  probed only from the collected files' common directory, which a link out
  of `src` moves up, so `src/tsconfig.json` was missing although creating
  it changed the check.
- **Decision and rationale**: The list also holds the probes from each named
  file and directory, as `docs/ai/tt.md` promises every probed path.

### Decision D4: `extends` resolves a package configuration as TypeScript does

- **Context**: The build reads `jsx` through `extends` without TypeScript,
  and refused `"extends": "pkg/sub"` through a package's `exports`, and
  `".\\base.json"`. TypeScript normalizes slashes first
  (`getExtendsConfigPath`) and resolves a bare name with NodeNext
  resolution in CommonJS mode (`nodeNextJsonConfigResolver`: conditions
  `require`, `types`, `node`, and `default`), honoring `exports` subpaths,
  `*` patterns, condition objects in written order and fallback arrays.
- **Decision and rationale**: The reader does the same: backslashes become
  slashes, and a package with `exports` resolves only through them; one
  without keeps the `tsconfig` field and `tsconfig.json` lookup it had.
  Conditions are matched in the order they are written, read from the raw
  JSON since the parsed map does not keep key order.

### Decision D5: A watch round checks output claims across every input

- **Context**: With `src/x.tt` watched, writing `src/x.ts` and editing
  `x.tt` built `out/x.ts` from `x.tt`, while a one-shot build refuses the
  pair ("multiple inputs claim this output"). The round checked claims
  only among the files it recompiled.
- **Decision and rationale**: Every round that recompiles anything checks
  claims across the whole input set first and writes nothing when two
  inputs claim one output; the next round after an edit then recompiles
  every input, as the one-shot build would.

### Decision D6: A watch round rebuilds when the support package's module type changes

- **Context**: Changing `package.json` from `"type": "module"` to `{}` ran
  no round, and later rounds left `out/a.mts` importing `./tt/option.js`,
  now CommonJS. TASK-764 decision 2 already rebuilds every output when
  `jsx` changes.
- **Decision and rationale**: The watch reads whether the support modules'
  package is an ES module each round and recompiles every output when it
  changes, as it does for `jsx`.

### Decision D7: A directory walk leaves out every package folder TypeScript's include leaves out

- **Context**: `src/bower_components/r.tt` was built and tt-checked but
  never type-checked: TypeScript's include patterns leave out
  `node_modules`, `bower_components` and `jspm_packages`
  (`commonPackageFolders`), and the walk left out only `node_modules`.
- **Decision and rationale**: The walk leaves out all three.

## Work log

- 2026-10-07: Ran the seventh audit's command-line agent; reproduced each
  finding and compared with `tsc` (`--showConfig`, `--runExternalCode`).
- 2026-10-07: Fixed D7 (`src/engine/project.rs`), D1
  (`src/main/{typed,ownership}.rs`), D4 (`src/engine/config.rs`), D3
  (`src/engine/{project,mod}.rs`), D2 (`src/typescript/host.mjs`), D5 and
  D6 (`src/main/{build,output}.rs`), each with a test.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Each test below was run on `a71c5729` with this task's tests copied in.

- **Path**: `tests/native/cases_10.rs::types_records_the_standard_library_declarations_it_writes` (D1)
- **Observed failure**: the build copied the declarations (`ttc: std → out/tt`
  with `out/tt/*.d.ts` written).
- **Path**: `tests/native/cases_10.rs::a_tt_source_imported_through_a_symlink_is_type_checked` (D2)
- **Observed failure**: `error[ts2307]: Cannot find module './link/s.tt'`.
- **Path**: `tests/cli.rs::dependencies_list_the_configuration_a_named_directory_would_find` (D3)
- **Observed failure**: the answer did not list `src/tsconfig.json`.
- **Path**: `src/engine/config.rs::tests::a_configuration_extends_what_typescript_resolves_through_package_exports` (D4)
- **Observed failure**: `cannot find the configuration it extends, "conditional/strict"`.
- **Path**: `tests/cli.rs::a_watch_round_refuses_two_inputs_claiming_one_output_as_a_build_does` (D5)
- **Observed failure**: no "multiple inputs claim this output" line (timeout).
- **Path**: `tests/cli.rs::a_watch_round_follows_the_package_type_the_support_modules_take` (D6)
- **Observed failure**: no round ran after the `package.json` change (timeout).
- **Path**: `tests/engine_cache.rs::source_walk_skips_the_package_folders_typescript_include_skips` (D7)
- **Observed failure**: the walk returned the `bower_components` and
  `jspm_packages` sources.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Complete. D1–D7 are fixed with regression tests that fail on `a71c5729`, and the full gate passes.
