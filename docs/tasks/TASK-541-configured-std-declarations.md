# TASK-541: Write the `@tt/std` declarations under a TypeScript configuration

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

`ttc --types` wrote `.tt-types/tt/*.d.ts` only when the project had no
`tsconfig.json`. With any configuration, `written` listed the module
sidecars alone, and a consumer that maps `@tt/std` to those files with
`paths` failed with TS2307. `write_declarations` promises that the
standard-library declarations mirror the generated `tt/` package.

## Scope

- Included: Where the engine takes the standard-library declarations from
  (`match_declarations`), and one shared answer to which standard-library
  packages a snapshot serves (`projection::served_std_packages`).
- Excluded: The module sidecars, which keep coming from the compiler's
  declaration emit; the layout under `-o`.

## Decisions

### Decision 1: The standard-library declarations are the package's own

- **Context**: The engine serves the standard library as a package under
  `<root>/node_modules/@tt/std` (and `@tt/runtime`). Without a
  configuration the host opens those modules directly, so they are roots
  of the inferred program and TypeScript emits declarations for them. In a
  configured program they are reached through `node_modules`, TypeScript
  treats them as an external library, and `getDeclarationEmit` emits
  nothing for them, so `match_declarations` found no standard-library entry.
- **Alternatives considered**: Opening the standard-library modules as
  roots of the configured program would change the user's program to make
  the compiler emit files it deliberately does not emit. Serving them
  outside `node_modules` would change how the bare specifier resolves.
- **Decision and rationale**: The package already carries TypeScript's
  declaration emit of each module (`StdModule::declaration`, the `cjs/`
  entry points), and `tests/content_mapper.rs` holds it byte-identical to
  `tsc --declaration` over the sources. The declarations the compiler
  emitted without a configuration are the same bytes. `match_declarations`
  now takes one declaration per module of each package the snapshot serves,
  so the output no longer depends on whether a configuration exists.
  `projection::assemble` and `match_declarations` ask the same
  `served_std_packages`, so the written set is exactly the served set.

## Work log

- 2026-09-29: Reproduced with a `.tt` file importing `@tt/std` and
  `@tt/std/option`: without a configuration `written` holds
  `tt/index.d.ts`, `tt/option.d.ts`, `tt/result.d.ts`; with one it holds only
  `a.tt.d.ts` and its map. Compared the files written without a
  configuration with `StdModule::declaration`: identical.
- 2026-09-29: Replaced the emit-path match in
  `src/engine/semantics/declarations.rs`, added `served_std_packages` in
  `src/engine/projection.rs` in place of `std_module_path`, and updated the
  call in `src/engine/project.rs`.
- 2026-09-29: Added
  `cli_types_under_a_configuration_writes_the_std_declarations_it_maps`
  (`tests/integration/cases_03.rs`), which writes the consumer configuration
  before `--types` and fails before the change. The consumer configuration
  is shared with `cli_types_sidecars_typecheck_the_source_tree`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test integration --test native --test cli`

## Result

Changed `src/engine/semantics/declarations.rs`, `src/engine/projection.rs`,
`src/engine/project.rs` and `tests/integration/cases_03.rs`. `--types`
writes the `@tt/std` declarations whether or not the project has a
`tsconfig.json`.
