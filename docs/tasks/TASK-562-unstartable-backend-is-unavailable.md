# TASK-562: A backend that cannot start is unavailable

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-562: Treat a backend that cannot start as unavailable`

## Purpose

With TypeScript installed but no working `node`, `ttc --check`, `ttc -p` and
plain builds failed on ordinary tt code, while the same file compiled with no
TypeScript installed at all. TASK-354 settled that an unavailable backend
yields the unrefined output; a host that cannot start is an unavailable
backend.

## Scope

- Included: where the standalone contextual pass decides availability, and
  how the native backend classifies a host that dies before it acknowledges
  the project
- Included: the same decision in the typed engine's snapshot, which runs
  the contextual pass before `check`
- Excluded: what the contextual pass computes when the backend runs

## Decisions

### Decision 1: Availability includes starting the host

- **Context**: `contextual::standalone` decided availability by resolving the
  toolchain (`NativeBackend::new`) only. The host process is started lazily
  by the first `ask`, so a spawn failure (`cannot run node`) arrived later as
  a backend failure, which the pass returns to its caller unchanged — a
  compile error.
- **Alternatives considered**: Turn every `Unavailable` failure returned by
  `materialize` into the unrefined emit (it would decide availability after
  the project inputs are read, so an unreadable sibling would be reported
  or not depending on whether `node` exists); cache "unavailable" per root
  (a long-running consumer would never notice a runtime appearing).
- **Decision and rationale**: `NativeBackend::open` makes the host serve a
  project, starting it if needed; `ask` and `configured_mappers` use it, and
  `standalone` calls it at its availability point, before reading project
  inputs. An `Unavailable` failure there returns the unrefined emit, exactly
  like a missing toolchain; anything after that point still reaches the
  caller unchanged, as TASK-353 established.

### Decision 2: A host that dies before its acknowledgement could not start

- **Context**: With `node` pointing at Bun, the host loads the API and then
  dies inside the client's `SyncRpcChannel` constructor (exit 1), before it
  acknowledges the open request. `host_died` treated every exit other than
  2 as an internal compiler failure.
- **Alternatives considered**: Catch the client's constructor in `host.mjs`
  and exit 2 (covers this runtime only; a runtime that cannot even parse the
  host still exits 1, and `host.mjs` is being edited concurrently).
- **Decision and rationale**: Classify by phase in the Rust boundary. Before
  the acknowledgement the compiler has not been reached, so a death then is
  `Unavailable`, except exit 3 — the host rejecting the open request ttc
  wrote, which is ttc breaking its own protocol. After the acknowledgement
  the existing classification is unchanged.

### Decision 3: The typed engine decides it at the same point

- **Context**: `Project::snapshot` runs `contextual::materialize` whenever a
  file has a contextual slot and the toolchain resolved, and turned any
  failure into a `Blocked` snapshot. `--check-types` with no `node` printed
  `error: cannot run node` with an empty location and no tt-level
  diagnostics, while with no toolchain it prints the tt layer and "the
  TypeScript layer did not run". `Project::check` already maps an
  `Unavailable` failure from `ask` to that report.
- **Decision and rationale**: The snapshot opens the backend before the
  pass; `Unavailable` leaves the projections unrefined, and `check` then
  reports the backend as unavailable exactly as for a missing toolchain.
  An internal failure still blocks the snapshot.

## Work log

- 2026-09-29: Reproduced under `target/probe4-cli/p1` with
  `env PATH=/usr/bin:/bin ttc --check src/a.tt`, `-p`, and `-o out src`:
  all three exited 1 with `error[other]: cannot run node`. With `node`
  linked to Bun: `the TypeScript backend failed: ... stdout._handle.fd`.
- 2026-09-29: Added `NativeBackend::open` and the start phase to
  `host_died` (`src/typescript/native.rs`); `standalone` computes its
  configuration before deciding availability and opens the backend there
  (`src/typescript/contextual.rs`).
- 2026-09-29: Added
  `a_backend_that_cannot_start_does_not_stop_a_check_print_or_build`
  (`tests/cli.rs`), which runs all three modes with a `PATH` holding no
  `node` and one whose `node` exits immediately, plus `--check-types`, which
  must exit 2 with "the TypeScript layer did not run". It fails without the
  fix (`cannot run node`), and its `--check-types` half fails without the
  `Project::snapshot` change alone (`src/engine/project.rs`).

## Issues and resolutions

### Issue 1: No usable `node` failed a tt-level compile

- **Symptom**: `error[other]: cannot run node: No such file or directory`,
  exit 1, and a build wrote nothing.
- **Cause**: Availability was decided from the toolchain alone; the spawn
  failure surfaced later as a failure that must reach the caller.
- **Resolution**: Decisions 1 and 2. All three modes now exit 0; `-p`
  prints the unrefined module (`let $tt_v0;` rather than
  `let $tt_v0: number;`), the same output as with no TypeScript installed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test cli --test native --test integration --test engine_cache`

## Result

Changed `src/typescript/native.rs`, `src/typescript/contextual.rs`,
`src/engine/project.rs`, and `tests/cli.rs`. A backend that cannot start
now removes only the contextual refinement, as a missing toolchain does, in
the standalone pass and in the typed engine alike.
