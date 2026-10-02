# TASK-540: Report an unreadable root configuration as TS5083 and recover

> **Narrowed by [TASK-588](./TASK-588-follow-configuration-discovery.md):**
> a typed watch whose configuration was discovered, not named with
> `--project`, reopens as the project a fresh run finds when that
> configuration is deleted (an inferred one, or the next one up) instead of
> reporting TS5083. A named configuration keeps this record's behavior.

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

`ttc --check-types -w src` (and `--types -w`) ended with an internal
compiler error, exit 101, when the project's `tsconfig.json` was moved away
or deleted and re-created. The help text reserves 101 for a ttc bug. A
missing configuration is the user's environment, and the same condition for
an `extends` target is already reported as `error[ts5083]` without ending
the watch.

## Scope

- Included: The TypeScript backend host (`src/typescript/host.mjs`): how an
  `ask` answers when the project's own configuration cannot be read, and
  how the project is opened once it can be read again.
- Excluded: The exit policy for real internal backend failures, which stays
  an internal compiler error; configuration syntax errors, which TypeScript
  already reports through its configuration parsing diagnostics.

## Decisions

### Decision 1: The host answers with TypeScript's own TS5083

- **Context**: Every `ask` of a configured project called
  `api.parseConfigFile` on the root configuration. With the file gone the
  API client throws `could not read file`, the host turned that into
  `{ error }`, and the Rust boundary classifies `{ error }` as an internal
  failure (`BackendErrorKind::Internal`), which `typed_pass` panics on.
- **Alternatives considered**: Classifying the failure in Rust by its
  message would infer the kind from text, which `BackendErrorKind`'s
  contract forbids. Recovering from internal errors in `typed_watch` would
  hide real compiler bugs behind a retry.
- **Decision and rationale**: The host first asks TypeScript to read the
  configuration (`api.readConfigFile`). When TypeScript answers with its
  "Cannot read file" diagnostic (TS5083) — the diagnostic `tsc --watch`
  reports for a configuration it cannot read — the host returns it as a
  project diagnostic, exactly as it returns a missing `extends` target, and
  answers no typed questions. The tt layer still reports in full, the pass
  exits 1 like any other project diagnostic, and a watch keeps watching.
  Syntax errors do not take this path: TypeScript can read that file, and
  its parsing diagnostics are reported as before.

### Decision 2: A configuration that comes back is opened afresh

- **Context**: The session opens the project once and afterwards only sends
  file changes. A project whose configuration vanished and returned has no
  reliable incremental state.
- **Decision and rationale**: When the configuration cannot be read after
  the project was opened, the host restarts the compiler (`reconnect`), so
  the next readable pass opens the project from scratch. `opened` is now set
  where the project was actually obtained rather than after every answered
  `ask`, so an answer without a project leaves it unset.

## Work log

- 2026-09-29: Reproduced exit 101 with a mapped project (`mv tsconfig.json`
  during `--check-types -w src`). Probed the API: `readConfigFile` on a
  missing file returns `error.code === 5083` ("Cannot read file '…'."),
  while `parseConfigFile` throws.
- 2026-09-29: Changed `handle` in `src/typescript/host.mjs`; added
  `a_typed_watch_reports_a_missing_configuration_and_recovers_when_it_returns`
  to `tests/native/cases_07.rs`, which fails with the ICE before the change.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native`

## Result

Changed `src/typescript/host.mjs` and `tests/native/cases_07.rs`. A typed
watch reports `error[ts5083]` for a configuration that disappears and checks
the project again when it returns.
