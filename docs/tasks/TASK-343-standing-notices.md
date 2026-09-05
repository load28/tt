# TASK-343: Give the server's standing notices one owner and one reset

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-343: one ledger for the notices the server says once`

## Purpose

Three `warned*` booleans each said "we have already told the user about
this". Only one of them was ever cleared, so a compiler that appeared, a
TypeScript install that arrived, or a typed failure that was fixed left the
other two standing for the rest of the session — the server had stopped
reporting a condition that was no longer true, and would not report the
next one either.

## Scope

- Included: The three `warned*` flags, where they are raised, and what
  clears them. The second copy of `rearmProject()`'s body in the
  watched-files handler.
- Excluded: What each notice says, and which conditions raise it.

## Decisions

### Decision 1: "Already said" is one concept with one owner

- **Context**: `warnedCompilerMissing` was cleared in two places;
  `warnedTypedCheckUnavailable` and `warnedTypedCompilerFailure` in none.
  Nothing in the code connected the three, so a notice added later opted
  out of recovery by default — which is how two of the three came to be
  permanent.
- **Alternatives considered**: Add the two missing assignments (the same
  drift returns with the fourth notice, and nothing marks the omission);
  clear the flags on a timer (a notice would come back while its condition
  still stands, which is the noise the flags exist to prevent).
- **Decision and rationale**: One `NoticeLedger` in `notices.ts` with
  `raise(id)` and `reset()`. Re-arming becomes a property of the ledger
  rather than of remembering to write a line, and a new notice is a new
  member of `NoticeId` — nothing else.

### Decision 2: The watched-files handler re-arms through `rearmProject`

- **Context**: `onDidChangeWatchedFiles` ended with a verbatim copy of
  `rearmProject()`'s four statements. It is the same event class — where
  the server looks may have changed — and the copy would fall behind the
  original the first time either grew a step. It already had: the ledger
  reset added here would have had to be written twice.
- **Alternatives considered**: Leave the copy and add the reset to both.
- **Decision and rationale**: Call `rearmProject()`. The engine shutdown
  that precedes it (an executable replaced at the same path) stays where it
  is, since it is specific to that event.

## Work log

- 2026-09-05: Added `server/src/notices.ts` and replaced the three flags.
  Folded the watched-files handler's copy into `rearmProject()`.
- 2026-09-05: Added `server/src/test/notices.test.ts`.

## Issues and resolutions

### Issue 1: Two of three notices never came back

- **Symptom**: After the server said "typed checks unavailable" once, it
  never said it again — including for a different project, a different
  cause, or after the user installed TypeScript and broke it again.
- **Cause**: `warnedTypedCheckUnavailable` and `warnedTypedCompilerFailure`
  were set once and never cleared; only `warnedCompilerMissing` had a
  reset, written by hand at its two call sites.
- **Resolution**: Decision 1.

## Verification

- [x] A notice is said once, notices do not silence one another, and one
  `reset()` re-arms every one of them —
  `server/src/test/notices.test.ts`
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js`
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/notices.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/notices.test.ts`, and this record.
