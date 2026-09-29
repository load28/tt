# TASK-397: Mirror the source tree in save-time sidecar refresh

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

With `tt.sidecarDir` set, the save-time refresh looked for and wrote `<dir>/<basename>.d.ts`, a flat layout, while `ttc --types <input dir>` mirrors the source tree under its output directory. For nested files, `refresh` never found anything to refresh, and `always` wrote flat files that collide across directories.

## Scope

- Included: `editors/vscode/server/src/sidecar.ts`, `roots.ts`, the `rebuildSidecar` call site in `server.ts`, their tests, and the `tt.sidecarDir` setting description and README row.
- Excluded: the compiler's `--types` layout (`src/main/output.rs` `input_relative`, `src/main/typed.rs` `write_declarations`), which is the contract this task follows.

## Decisions

### Decision 1: The compiler's `--types` layout is the contract the refresh follows

- **Context**: `ttc --types <inputs> -o <dir>` writes each declaration at `<dir>/<path relative to the input directory that contains the source>` (`src/main/output.rs` `input_relative`, `src/main/typed.rs` `write_declarations`), so `ttc --types src` writes `src/a/x.tt` to `.tt-types/a/x.tt.d.ts`. The refresh assumed `<dir>/x.tt.d.ts`. TypeScript resolves a sidecar in a separate tree through `rootDirs`, which merges the listed roots into one virtual directory (TypeScript handbook, "Module Resolution", *Virtual Directories with rootDirs*), so only the mirrored position is ever found by a `.ts` importer.
- **Alternatives considered**:
  - Invoke `ttc --types <input dir> -o <dir>` on every save so the compiler lays out the whole tree itself. Rejected: the extension does not know which input directory the project used (`src`, `.`, or several), and it would re-emit every sidecar of that directory on each save.
  - Read `rootDirs` from `tsconfig.json`. Rejected: the server would need its own implementation of TypeScript's configuration loading (`extends`, JSONC, project selection), which the architecture keeps inside the compiler.
  - Keep the flat layout and document it. Rejected: it contradicts the documented `ttc --types -w src` workflow and collides for same-named files.
- **Decision and rationale**: the refresh identifies an existing sidecar by the record the compiler wrote into it. A sidecar of `x.tt` can only be at `<dir>/<path of x.tt relative to one of its ancestor directories>.d.ts`, because the input directory of a `--types` run is an ancestor of every source it writes. Each such candidate from the file's directory up to its workspace folder is checked, and it is the file's sidecar exactly when its declaration map's `sources` (with `sourceRoot`) resolves to the saved file. This is the map `build_sidecar` writes (`src/sidecar.rs`: "`sources` is the original `.tt` file"), so the layout is read from the compiler's own output rather than guessed from a file name. The write then runs `ttc --types <file> -o <directory of that sidecar>`: for a single file input the compiler writes `<name>.d.ts` directly under `-o` and computes the map's `sources` from that directory itself, so the compiler still produces both files and their relative references.

### Decision 2: `always` creates a missing sidecar at the file's path relative to its workspace folder

- **Context**: with no existing sidecar there is no record to read, and `tt.sidecarDir` is documented as a directory relative to the workspace folder.
- **Alternatives considered**: the flat basename layout (collides across directories, and is not what any `ttc --types <dir>` run writes); inferring the input directory from other files' sidecars (a guess that changes with unrelated files).
- **Decision and rationale**: the base is the workspace folder that `tt.sidecarDir` is resolved against, which is exactly the layout of `ttc --types <workspace folder> -o <dir>`. Two same-named files in different directories get different sidecars. A file outside every folder (only possible with an absolute `tt.sidecarDir`) uses its own directory, which is the compiler's layout for a single-file input and the previous behavior. The setting description and README state the layout.

### Decision 3: The adjacent mode is unchanged

- **Context**: an empty `tt.sidecarDir` places `x.tt.d.ts` beside the source, where the compiler's single-file layout already puts it.
- **Decision and rationale**: the same target and `-o` as before; the existing tests cover it unchanged.

## Work log

- 2026-09-27: Confirmed the compiler layout with the built `ttc`: `ttc --types src` wrote `.tt-types/a/x.tt.d.ts` with `sources: ["../../src/a/x.tt"]`, while `ttc --types src/a/x.tt -o .tt-types` wrote `.tt-types/x.tt.d.ts`.
- 2026-09-27: Added three regression tests to `server/src/test/sidecar.test.ts` (refresh of a tree written by `ttc --types src`; `always` for two same-named files in different directories; a flat sidecar whose map names another source is not refreshed). All three failed against the previous `sidecar.ts` (the refresh skipped the mirrored sidecar, `always` wrote `.tt-types/notice.tt.d.ts`, and the other file's sidecar was overwritten).
- 2026-09-27: Implemented the candidate lookup and per-target write in `sidecar.ts`; `server.ts` passes the file's containing workspace folder to `refreshSidecar`. Updated the `tt.sidecarDir` description in `package.json` and the README settings row.

## Issues and resolutions

### Issue 1: Standard-library declarations follow `-o`

- **Symptom**: none observed. `write_declarations` writes standard-library declarations to `<-o>/tt` when the compiler emits them.
- **Cause**: with a nested `-o`, such declarations would land below the sidecar's directory. In the runs made here (`import * as Result from '@tt/std/result'`) the compiler emitted none, and the adjacent mode already passes the source's own directory as `-o`.
- **Resolution**: recorded as a compiler-side observation; no extension change can place them differently without a compiler option.

## Verification

- [x] `npm run compile` in `editors/vscode`
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js` with `target/debug` on `PATH`
- [x] `node scripts/check-task-index`

## Result

Changed files: `editors/vscode/server/src/sidecar.ts`, `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/test/sidecar.test.ts`, `editors/vscode/package.json`, `editors/vscode/README.md`, `docs/tasks/INDEX.md`, this record. The save-time refresh now updates the sidecar where `ttc --types` wrote it and creates new ones in a collision-free mirror of the workspace folder.
