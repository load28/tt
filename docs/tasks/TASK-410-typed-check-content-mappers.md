# TASK-410: Serve tt modules without the project's tt content mapper in the typed check

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

After TASK-388 made the typed check report program diagnostics, `ttc --check-types src` failed with `error[ts100024]: Content mappers require the '--runExternalCode' command line flag to be enabled` on the tsconfig docs/ai/tt.md documents and create-tt generates (a top-level `contentMappers` entry for `.tt`/`.ttx`). The edit loop in tt.md tells users to run exactly this command in that setup.

## Scope

- Included: The configuration text the TypeScript host serves (`src/typescript/host.mjs`).
- Excluded: Running external content mappers from the typed check.

## Decisions

### Decision 1: The host is the `.tt`/`.ttx` content provider for its own program

- **Context**: The host projects `.tt`/`.ttx` modules itself through its layered file system and already serves configuration files with their `.tt` patterns rewritten. The project's tt mapper entry asks TypeScript to run `ttc --content-mapper` for the same extensions, which the API refuses without `--runExternalCode`, and which would duplicate what the host already serves.
- **Alternatives considered**: Filtering TS100024 would hide the same diagnostic for a mapper the host does not replace. Enabling external code would make TypeScript spawn a second ttc for modules the host already projects.
- **Decision and rationale**: In served configurations, `.tt` and `.ttx` are removed from each `contentMappers` entry's `extensions`, and an entry left without extensions is dropped. A mapper for any other extension stays, so its TS100024 is still reported, now without a position because the served text differs from the file on disk (TASK-388 Decision 2).

## Work log

- 2026-09-27: Reproduced with the documented tsconfig and the repository TypeScript. Applied the configuration rewrite; the check passes and still reports TS2322 in `.tt` files. Added two CLI cases; the first fails on the previous host.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/typescript/host.mjs` and `tests/cli.rs`.
