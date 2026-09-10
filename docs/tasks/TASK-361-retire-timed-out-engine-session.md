# TASK-361: Retire timed-out editor engine sessions

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-361: fix(editor): recover timed-out engine sessions`

## Purpose

Audit the editor-to-compiler process boundary under a stalled request and make
the next semantic request recover instead of reusing a permanently blocked
session.

## Scope

- Included: VS Code engine session ownership, request timeout recovery, pending
  request settlement, and deterministic process-level regression coverage
- Excluded: TypeScript service internals already repaired by TASK-359 and
  per-feature editor presentation

## Decisions

### Decision 1: A timed-out request retires its whole conversation

- **Context**: The JSON-lines compiler protocol is ordered over one process. If
  one request never completes, later requests cannot establish that the same
  conversation can make progress.
- **Alternatives considered**: Drop only the timed-out request; keep retrying the
  same process; or retire the process and let the next request start a new one.
- **Decision and rationale**: Retire the process. This matches the TypeScript
  service boundary, settles every request queued behind the stall, and gives the
  next editor action an independent session without classifying the compiler as
  unsupported.

## Work log

- 2026-09-10: Ran `./scripts/doctor`; the pinned environment was ready.
- 2026-09-10: Confirmed `main` at `fd03025` and opened an independent branch.
- 2026-09-10: Traced engine request timeouts and found that they removed only
  one pending callback while retaining the same live child process.
- 2026-09-10: Centralized engine-session retirement for timeout, process, write,
  and explicit shutdown paths, settling every queued request before termination.
- 2026-09-10: Added a deterministic fake-compiler regression that proves queued
  requests settle and the next request starts a fresh process.
- 2026-09-10: Ran the complete local CI gate successfully.

## Issues and resolutions

### Issue 1: One stalled request poisons every later editor type request

- **Symptom**: After one engine request times out, later requests reuse the same
  process and time out behind the stalled conversation.
- **Cause**: The timeout handler resolves only its own promise and does not
  transfer or end process ownership.
- **Resolution**: A timeout now retires the ordered compiler conversation,
  settles every queued request as unavailable, and leaves the next editor action
  to create a fresh process. Timeouts do not count as evidence that the compiler
  lacks server support.

## Verification

- [x] VS Code session tests: 6 passed
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`: agents, Rust, npm, website, native, and extension passed;
  171 VS Code tests passed

## Result

The editor no longer keeps an ordered compiler process after a request proves
that conversation cannot make progress. Queued operations finish promptly, and
the next operation recovers on a new process with the open-buffer state replayed.
