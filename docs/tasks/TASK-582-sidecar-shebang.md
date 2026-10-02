# TASK-582: Keep a shebang first in a declaration sidecar

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: the `TASK-582` commit on this branch

## Purpose

`ttc --types` wrote `.tt-types/cli.tt.d.ts` for a source starting with
`#!/usr/bin/env node` with the `// @generated … by ttc --sidecar` banner on
line 1 and the shebang on line 2, which TypeScript rejects (TS18026 "'#!' can
only be used at the start of a file", then TS1005).

## Scope

- Included: the sidecar's banner placement and its map; one banner rule
  shared by compiled outputs and sidecars; tests; `docs/ai/tt.md`.
- Excluded: what TypeScript's declaration emit writes (the shebang comes from
  it and stays).

## Decisions

### Decision 1: Keep the shebang, as TypeScript's own declaration emit does

- **Context**: A declaration file never runs, so the shebang could also be
  dropped. The task asked to decide by TypeScript's own declaration emit.
- **Alternatives considered**: (a) Drop the shebang from the sidecar: the
  sidecar would then differ from the declarations TypeScript emitted for the
  same module, by a rule of ttc's own. (b) Keep it first and put the banner
  below it.
- **Decision and rationale**: The pinned TypeScript
  (`7.1.0-dev.20260826.1`) keeps a source's shebang as the first line of the
  `.d.ts` it emits: `tsc -p` with `declaration`, `emitDeclarationOnly` and
  `declarationMap` over `#!/usr/bin/env node\nexport const q = 1 as number;`
  wrote `#!/usr/bin/env node\nexport declare const q: number;` and a map
  whose first line is unmapped. ttc's sidecar body is that emit, so the
  sidecar keeps it first and the banner goes below it, which is also where a
  compiled module's banner goes (`ttc help workflow`).

### Decision 2: One banner rule for compiled modules and sidecars

- **Context**: `write_banner` in `src/main/build.rs` already placed a
  compiled module's banner after a shebang, and reported the lines it added
  and where, for the source map. `src/sidecar.rs` prepended its banner and
  shifted its map by a fixed `;`.
- **Decision and rationale**: `write_banner` and `BannerPlacement` moved
  unchanged to the library (`src/banner.rs`, `ttc::banner`); the binary uses
  them from there, and `build_sidecar` writes its banner with the same
  function and inserts the empty mapping lines the placement reports at the
  line it reports, instead of a leading `;`. The banner is written after the
  `sourceMappingURL` line is appended, so a declaration file that is only a
  shebang does not gain an empty line.

## Work log

- 2026-09-29: Reproduced in a copy of the finder's `probe4-cli/sb`: the
  sidecar had the banner above the shebang; `tsc -p` on the project reported
  it. Measured TypeScript's own declaration emit for a shebang source
  (Decision 1).
- 2026-09-29: Moved `write_banner`/`BannerPlacement` to `src/banner.rs`
  (`src/lib.rs`, `src/main.rs`, `src/main/build.rs`), used it in
  `build_sidecar` (`src/sidecar.rs`). The probe's sidecar is now
  `#!…`, banner, declaration, `sourceMappingURL`, with the mapping
  `;;AACa,qBAAA;` (the declaration on generated line 2 maps to source line 1),
  and `tsc -p` passes.
- 2026-09-29: Tests and `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: `tsc` refused a file argument beside a `tsconfig.json`

- **Symptom**: The new test's `tsc --noEmit <sidecar>` failed with TS5112.
- **Cause**: TypeScript 7 does not load a `tsconfig.json` in the directory
  when files are named on the command line, and says so as an error.
- **Resolution**: The test passes `--ignoreConfig`, which checks exactly the
  named declaration file.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (full run recorded in TASK-583)
- [x] New `a_shebang_stays_first_and_the_banner_goes_below_it` and
  `a_file_that_is_only_a_shebang_keeps_it_first` (`tests/sidecar.rs`): the
  declarations' lines and the name segment's generated/source positions.
- [x] New `a_sidecar_of_a_source_with_a_shebang_is_a_declaration_file_typescript_reads`
  (`tests/native.rs`): `ttc --types` then the pinned `tsc --noEmit` over the
  sidecar. With the previous `src/sidecar.rs` it fails.

## Result

Changed files: `src/banner.rs` (new), `src/lib.rs`, `src/main.rs`,
`src/main/build.rs`, `src/sidecar.rs`, `tests/sidecar.rs`,
`tests/native.rs`, `docs/ai/tt.md`, `docs/tasks/INDEX.md`, this record.

A sidecar of a source that opens with a shebang keeps it first, as
TypeScript's declaration emit does, with the banner and the map shifted below
it by the same rule compiled modules follow.
