# TASK-340: Make an unusable compiler visible and recoverable in the editor

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-340: report and recover from a compiler the editor cannot run`

## Purpose

The language server has a recovery path for a compiler it could not run —
it re-arms the engine and re-validates every open buffer when `tt.*`
settings change — but the client never asked for configuration
notifications, so that handler was unreachable. And one way of being
unusable, a `ttc` that exists but cannot be started, was never reported at
all: the Problems panel just stayed empty.

## Scope

- Included: The client's `synchronize.configurationSection`, the spawn
  failures the one-shot `ttc --check` classifies, and what the server says
  about each.
- Excluded: How the engine session decides to give up on a compiler
  (TASK-334), and the typed layer's own failure reporting.

## Decisions

### Decision 1: The client declares the section it synchronizes

- **Context**: `server.ts` handles `onDidChangeConfiguration` by clearing
  `warnedCompilerMissing`, calling `engine.retryEngineServer()` and
  re-validating open documents — the exact recovery someone performs after
  the "ttc compiler not found" notification tells them to set
  `tt.compilerPath`. The notification never arrived.
  `SyncConfigurationFeature.initialize()` in vscode-languageclient 9.0.1
  registers its `workspace.onDidChangeConfiguration` listener only when
  `clientOptions.synchronize?.configurationSection !== undefined`, and
  returns having registered nothing otherwise
  (`client/node_modules/vscode-languageclient/lib/common/configuration.js:109`).
  A workspace-scoped edit still recovered, but only by accident: it writes
  `.vscode/settings.json`, which the `**/*.{tt,ttx,ts,tsx,json}` watcher
  reports. A user-settings edit — the scope the notification's own advice
  leads to — touches no file in the workspace and recovered from nothing.
- **Alternatives considered**: Have the server pull settings on a timer or
  on each request (a poll where the protocol has a notification, and
  `workspace/configuration` still needs a change signal to know when to
  ask); widen the file watcher to the user-settings path (guessing a
  per-platform product path, and still nothing for a change made by a
  settings-sync push).
- **Decision and rationale**: Declare `configurationSection: "tt"`, which is
  what the client library documents as the request for exactly these
  notifications, and which scopes them to our own settings.

### Decision 2: The options are built where they can be read without a host

- **Context**: The section above is a contract with the server, not a
  detail of how the options are spelled, so it should be pinned by a test.
  The options were built inline in `activate()`, which only runs inside an
  extension host.
- **Alternatives considered**: Test it end to end in `test/editor.cjs` (the
  real host, but no gate runs it — no VS Code in CI — and the check it
  could make, "a settings change reaches the server", cannot separate the
  configuration path from the file watcher that also fires on the only
  settings file inside the workspace); assert on the source text (pins the
  spelling, not the value).
- **Decision and rationale**: Move the options into `client/src/options.ts`,
  which imports only types, and run the client's tests in the same gate as
  the server's.

### Decision 3: A compiler that cannot be started is its own answer

- **Context**: `runCheckOnce` mapped `ENOENT` to `not-found` and everything
  else to `failed`. `failed` writes one line to the server's output channel
  and publishes an empty diagnostics list — no notification, no visible
  change. So a `tt.compilerPath` naming a file without its execute bit, a
  directory, or a non-executable, all of which fail with `EACCES` (or
  `EPERM`/`ENOEXEC`/`EISDIR`), silently turned off every tt diagnostic.
- **Alternatives considered**: Report it as missing (the advice would be
  wrong — the path is right and the file is there); probe the path with
  `fs.access(X_OK)` before spawning (a second, racy answer to a question
  the spawn already answers, and one that disagrees with the kernel on
  interpreter and format failures).
- **Decision and rationale**: Classify the spawn failure — `missing` or
  `not-executable` — and let the server say which, since the two need
  different things from the user.

## Work log

- 2026-09-05: Confirmed the languageclient contract in the installed
  package (`configuration.js:109-119`), and confirmed that a non-executable
  file and a directory both fail with `EACCES` even for uid 0, so the
  execute bit is what is missing in both.
- 2026-09-05: Added `client/src/options.ts` and `ttClientOptions`, and
  pointed `activate()` at it.
- 2026-09-05: Added `UnusableCompiler`, `unusableCompiler()` and the
  `reason` field, and split the server's warning in two.
- 2026-09-05: Added `client/src/test/options.test.ts` and
  `server/src/test/compiler.test.ts`, and extended the extension test
  command in `package.json`, `scripts/ci` and `.github/workflows/ci.yml` to
  run the client's tests alongside the server's.

## Issues and resolutions

### Issue 1: `tt.*` setting changes never reached the server

- **Symptom**: Setting `tt.compilerPath` after the "compiler not found"
  notification changed nothing until the window was reloaded. The server's
  `onDidChangeConfiguration` handler never ran.
- **Cause**: `synchronize.configurationSection` was unset, so the client's
  `SyncConfigurationFeature` registered no listener.
- **Resolution**: Decision 1.

### Issue 2: A `ttc` that could not be started disabled diagnostics silently

- **Symptom**: With `tt.compilerPath` pointing at a file whose execute bit
  is unset, the Problems panel stays empty for every `.tt` buffer and
  nothing is shown anywhere.
- **Cause**: Only `ENOENT` was recognised; `EACCES` fell into `failed`,
  which logs to the output channel and publishes no diagnostics.
- **Resolution**: Decision 3.

## Verification

- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js` —
  a missing compiler still reports `missing`, one that cannot be started
  reports `not-executable`, and the client options carry the section
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/client/src/extension.ts`,
`editors/vscode/client/src/options.ts`,
`editors/vscode/client/src/test/options.test.ts`,
`editors/vscode/server/src/ttc.ts`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/compiler.test.ts`,
`editors/vscode/package.json`, `scripts/ci`, `.github/workflows/ci.yml`,
and this record.
