# TASK-569: Answer the server's buffer check without the TypeScript backend

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

The server's `check` request is `--check` for a buffer and the editor asks
it on every edit, but it still called `compile_report`, which refines the
emission through the TypeScript backend. TASK-564 moved `--check` to
`check_report` and left the server for later.

## Scope

- Included: `check` in `src/server.rs`.
- Excluded: The typed requests, which need the backend.

## Decisions

### Decision 1: The server's `check` uses the report `--check` uses

- **Context**: TASK-564 established that no `--check` diagnostic depends on
  the refinement, which only annotates output `check` discards.
- **Decision and rationale**: `check` calls `ttc::check_report`, so the
  request and `--check` answer from the same report and neither starts the
  backend.

## Work log

- 2026-09-29: Changed the call and added
  `the_servers_check_does_not_start_the_typescript_backend`
  (`tests/cli.rs`), which records whether a `node` on PATH was started; it
  fails with `compile_report` and passes now.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests

## Result

Changed `src/server.rs` and `tests/cli.rs`.
