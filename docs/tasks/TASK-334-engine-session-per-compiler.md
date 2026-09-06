# TASK-334: Keep one engine session per compiler and never crash on a dead pipe

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-334: keep one engine session per compiler`

## Purpose

The language server terminates with an uncaught `EPIPE` as soon as two open
documents resolve different `tt.compilerPath` values, taking every tt
language feature down with it.

## Scope

- Included: The engine session lifecycle in `editors/vscode/server/src/engine.ts`
  and the session-start callback in `server.ts`.
- Excluded: What the server publishes when a layer cannot answer, and the
  one-shot fallback's diagnostic parsing; both are separate records.

## Decisions

### Decision 1: Hold one session per compiler rather than one in total

- **Context**: `engineServerFor` kept a single global session and shut it
  down whenever a different compiler was asked for. Because
  `tt.compilerPath` is a `resource`-scoped setting, two documents in one
  window legitimately resolve different compilers, so the single session was
  torn down and respawned on essentially every request.
- **Alternatives considered**: Keep one session and serialize requests
  behind it; refuse to serve more than one compiler per window.
- **Decision and rationale**: Key sessions, their strike counts, and their
  shutdown on the compiler path — the same key every request already carries.
  A window with one compiler is unchanged; a window with two keeps both
  sessions warm instead of paying a process start and a full document
  re-send per request. This also removes the re-entrancy that caused the
  crash: a session start can no longer shut down the session being started.

### Decision 2: Hand the session's own compiler to the session-start callback

- **Context**: The callback re-sends every open buffer to "the current
  compiler", read from a global the per-document validation overwrites. On a
  fresh session that global frequently named a *different* compiler, so the
  callback opened the documents against another session.
- **Alternatives considered**: Keep the global and have the callback skip
  when it disagrees.
- **Decision and rationale**: The session that just started is the one whose
  documents must be re-sent, so the callback receives its compiler as an
  argument. The global is no longer consulted for this decision at all.

### Decision 3: A stream error ends the session, never the process

- **Context**: `child.stdin` had no `error` listener. Node emits `error` on
  the stream as well as passing it to the write callback, so a write to a
  dead pipe was an uncaught exception.
- **Alternatives considered**: Check `writable` more carefully before
  writing (a race by construction — the kill is asynchronous).
- **Decision and rationale**: Attach `error` handlers to the child's stdin
  and stdout that fail the session the same way a child `error`/`exit` does.
  Pending requests resolve to `null`, which is the module's documented
  contract: "every caller degrades gracefully… the feature simply has no
  answer".

## Work log

- 2026-09-05: Reproduced the crash deterministically by driving the compiled
  engine module the way `server.ts` drives it (script in the scratch
  directory): warm a session for compiler A, leave the global naming A, then
  request against compiler B. B's fresh session calls the callback, which
  opens documents against A, which shuts down B's just-spawned session; the
  outer call returns that dead session and the write raises an uncaught
  `EPIPE`. Confirms the audit report with a reproduction that does not
  depend on timing.

## Issues and resolutions

### Issue 1: An uncaught `EPIPE` kills the language server

- **Symptom**: `Error: write EPIPE` at `engine.js` inside `engineRequest`,
  the server process exits, and the editor loses every tt feature. Reproduced
  5/5 with the driver above.
- **Cause**: Decisions 1–3 above, compounding: the session-start callback
  named the wrong compiler, that shut down the session being constructed, the
  constructor returned it anyway, and the resulting stream error had no
  handler.
- **Resolution**: All three decisions applied.

## Verification

- [x] The reproduction driver survives and answers (`SURVIVED`, answer `ok`)
- [x] Two compilers alternating over three rounds keep both sessions warm
- [x] A session start that opens documents against another compiler no
  longer ends the session being started, and each start names its own
  compiler
- [x] `./scripts/ci extension`: 135 passed, zero failed/cancelled/skipped
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/session.test.ts`, and this record.
Sessions, their strike counts and their shutdown are keyed on the compiler
path; the session-start callback receives the compiler it is for; and a
stream error on the child's pipes ends that session instead of the
language server.
