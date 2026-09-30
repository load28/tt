# TASK-591: Forward termination signals from the npm launcher to ttc

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-591: Forward termination signals from the npm launcher to ttc`

## Purpose

`npm/tt-lang/bin/ttc.js` ran the native binary with `spawnSync`. A
SIGTERM, SIGINT or SIGHUP sent to the launcher's node process (a process
supervisor stopping `npx ttc -w`, `kill <pid>`, a closed terminal) ended
the launcher and left `ttc` running, re-parented to PID 1.

## Scope

- Included: the launcher (`npm/tt-lang/bin/ttc.js`) and a test in the npm
  package tests (`npm/scripts/*.test.mjs`, run by `scripts/ci` and CI).
- Excluded: signals Node.js cannot observe (SIGKILL, SIGSTOP), which no
  process can forward.

## Decisions

### Decision 1: Spawn asynchronously and forward the three termination signals

- **Context**: `spawnSync` blocks the event loop until the child exits, so
  a signal listener can never run; without a listener, Node.js applies the
  default action and exits (Node.js `process` docs, "Signal events":
  installing a listener for SIGTERM/SIGINT removes the default exit
  behavior; SIGHUP is delivered on Windows when the console closes). A
  child does not receive a signal sent to its parent's pid.
- **Alternatives considered**: (a) Spawn the child `detached` in its own
  process group and signal the group: changes terminal job control (the
  child would leave the foreground group and stop receiving the
  terminal's own Ctrl-C). (b) `exec`-style replacement of the node
  process: Node.js has no `execve`.
- **Decision and rationale**: `child_process.spawn` with `stdio:
  "inherit"`, and listeners for SIGINT, SIGTERM and SIGHUP that call
  `child.kill(signal)` (Node.js `child_process` docs, `subprocess.kill`).
  On the child's `exit`, the listeners are removed and a signal death is
  re-raised on the launcher itself, so its parent observes the same
  termination; otherwise the launcher exits with the child's status, as
  before. A terminal Ctrl-C reaches the child through the process group
  and through the forward; ttc's default SIGINT action ends it either way.

## Work log

- 2026-09-30: Reproduced: `kill -TERM` on the launcher left the child alive
  under PID 1.
- 2026-09-30: Rewrote the launcher on `spawn`; added
  `npm/scripts/tt-lang-launcher.test.mjs` (status pass-through, and each
  of the three signals ends launcher and binary, with the launcher dying of
  the same signal). With the previous launcher the three signal tests fail
  and the status test passes. The tests are skipped on Windows, where
  POSIX signals and shebang binaries do not apply.

## Issues and resolutions

None.

## Verification

- [x] `node --test npm/scripts/tt-lang-launcher.test.mjs` (4 passed)
- [x] Full gate run once at the end of the TASK-587–592 series; see TASK-592.

## Result

Changed `npm/tt-lang/bin/ttc.js`; added
`npm/scripts/tt-lang-launcher.test.mjs`.
