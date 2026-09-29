# TASK-531: Await the expected republish in the sidecar re-arm test

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

"the server's own sidecar writes do not re-arm the project, a hand-written
declaration does" failed once while `cargo test` ran beside the extension
tests, and passed twice alone. It gave the expected publish two seconds.

## Scope

- Included: That case in `editors/vscode/server/src/test/server.test.ts`.
- Excluded: The server's re-arm behaviour, which the failure did not
  implicate.

## Decisions

### Decision 1: Only the absence of a publish is observed over a window

- **Context**: The case asserts two things: no republish after the server's
  own sidecar write, and a republish after a declaration written by hand.
  Both went through one helper that raced the publish against a
  two-second timer.
- **Alternatives considered**: A longer timer only moves the load at which
  the positive assertion fails.
- **Decision and rationale**: Silence can only be observed for a bounded
  time, so the negative assertion keeps its window (`silent`). The positive
  one awaits the publish with the case's own timeout, registered before the
  change is announced, so its outcome no longer depends on machine load.

## Work log

- 2026-09-29: Observed the failure in the full extension run next to
  `cargo test` (TASK-530 verification); read the case and found the shared
  two-second race. Split it into `silent` and an awaited `publish`.

## Issues and resolutions

None.

## Verification

- [x] `npm run compile`, then the case alone: passed.
- [x] Full server and client suites: 223 passed.

## Result

Changed `editors/vscode/server/src/test/server.test.ts`.
