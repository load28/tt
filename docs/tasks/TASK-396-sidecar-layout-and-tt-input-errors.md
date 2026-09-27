# TASK-396: Read sidecar declarations from tsc's layout and name rejected tt-only inputs accurately

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`--sidecar <dir>` looked declarations up by base name only, so files with the
same name in different folders received another file's declarations or none.
The tt-only modes also rejected a TypeScript input with a message that
contradicted itself, and the engine-backed modes omitted the file name.

## Scope

- Included: The declaration path that `sidecar_mode` reads
  (`src/main/modes.rs`, `src/main/command.rs`, `--sidecar` help in
  `src/main.rs`); the rejection error for a named non-source file
  (`src/engine/project.rs`) and how the build path prints walk errors
  (`src/main/build.rs`).
- Excluded: Reading `rootDir` from a tsconfig (see Decision 2), the sidecar
  map's URL encoding (see Result), and which modes accept TypeScript inputs.

## Defects

1. With `src/a/x.tt`, `src/b/x.tt`, and `tsc --emitDeclarationOnly --outDir
   decl` output `decl/a/x.d.ts` and `decl/b/x.d.ts`, `ttc --sidecar decl src`
   read `decl/x.d.ts` for both inputs (a stray file from another build), or
   failed when it did not exist.
2. `ttc --symbols x.ts`, `--emit-map x.ts`, and `--sidecar d x.ts` reported
   `ttc: x.ts: not a tt or TypeScript source (expected .tt, .ttx)`: the file is
   a TypeScript source, and the listed extensions are tt only.
   `--check-types x.ts`, `--types x.ts`, and `--dependencies x.ts` reported
   the same text without the file name.

## Decisions

### Decision 1: Mirror the input path under tsc's common source directory

- **Context**: The TypeScript TSConfig reference, `outDir`: "the directory
  structure of the original source files is preserved"; `rootDir`: "Default:
  The longest common path of all non-declaration input files." A declaration
  for `<root>/a/x.ts` is therefore `<outDir>/a/x.d.ts`.
- **Alternatives considered**: Using the directory input itself as the root
  (the `-o` mirroring rule) matches tsc only when every subtree is populated;
  for a tree whose sources all sit in `src/a`, tsc writes `decl/x.d.ts`, which
  the base-name lookup found and an input-directory root would miss. Trying
  several candidate paths would guess instead of modelling the layout.
- **Decision and rationale**: The root is the deepest directory shared by all
  non-declaration sources in the inputs (`.tt`, `.ttx`, and host TypeScript,
  excluding `.d.ts`, `.d.mts`, and `.d.cts`). This is tsc's default when it
  compiles the same tree. Every case the base-name lookup answered correctly —
  all sources in one directory — yields the same path.

### Decision 2: Do not read a configured `rootDir`

- **Context**: A project may set `rootDir` explicitly.
- **Decision and rationale**: `--sidecar` is a legacy step that receives only a
  declaration directory. The help now states the layout it expects; a project
  with a different `rootDir` can pass the declaration subdirectory that
  corresponds to its inputs. `--types`, which drives TypeScript itself, is
  unaffected.

### Decision 3: The source walk names every entry it rejects

- **Context**: Every other error from `collect_sources_in` is wrapped by
  `named(entry, …)`; the extension rejection was the only exception. The build
  path compensated by prefixing the input, while the engine path
  (`collect_tt`, used by `--check-types`, `--types`, and `--dependencies`)
  printed the bare message.
- **Decision and rationale**: The rejection names its entry and states what the
  walk accepts in that mode: "not a tt source (expected .tt, .ttx)" without
  TypeScript, and the unchanged "not a tt or TypeScript source (expected .tt,
  .ttx, .ts, .tsx, .mts, .cts)" with it. The build path prints the walk's
  error as `ttc: <error>`. Named-file rejections print exactly as before; a
  failure inside a directory input now reads `ttc: src/bad.tt: …` instead of
  `ttc: src: src/bad.tt: …`, the same form the engine path already used.

## Work log

- 2026-09-27: Reproduced both defects with the `sc` investigation material and
  a scratch `x.ts`. Added `sidecars_read_declarations_from_the_layout_tsc_emits`
  (duplicate base names with a decoy `decl/x.d.ts`, a flat directory, and a
  tree whose common directory includes a host `.ts`) and
  `tt_only_modes_name_the_file_and_the_extensions_they_accept` to
  `tests/cli_outputs.rs`. Before the change the sidecar for `src/a/x.tt` held
  the decoy's `export declare const decoy: 0;`, and `--symbols x.ts` printed
  "not a tt or TypeScript source (expected .tt, .ttx)".
- 2026-09-27: Implemented Decisions 1 and 3 and updated the `--sidecar` help
  text. Both tests passed. Re-ran the `sc` material: `src/a/x.tt.d.ts` now has
  `X` and `src/b/x.tt.d.ts` has `Y`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/main/modes.rs`, `src/main/command.rs`, `src/main/build.rs`,
`src/main.rs`, `src/engine/project.rs`, and `tests/cli_outputs.rs`. Sidecars
receive the declarations tsc emitted for their own file, and a rejected input
is named with the extensions the mode accepts.

Follow-up (not in this task): sidecar maps written by `--sidecar` and
`--types` still name their `sources` and `sourceMappingURL` with raw paths
(`relative_path` in `src/main/modes.rs` and `ttc::build_sidecar`), the same
URL defect TASK-395 fixed for `--source-map`. `relative_path` is also used for
`@tt/std` module specifiers, which are not URLs, so the fix needs its own
task.
