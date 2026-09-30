# TASK-677: Keep the TypeScript host's stdin blocking so an idle host waits instead of spinning

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: this record's commit (`TASK-677: …`)

## Purpose

TASK-654 found a `node host.mjs` process, left by a `ttc --check-types -w`
watch, that had used more than 13 hours of CPU: the host's `lineReader`
retried `fs.readSync` on `EAGAIN` while it waited for the next request, so
every idle host kept a core busy and slowed every other typed compile on
the machine.

## Scope

- Included: how the host (`src/typescript/host.mjs`) holds its stdin, and a
  regression test in `src/typescript/native.rs`.
- Excluded: the host protocol (unchanged), the TypeScript API client's own
  channel to `tsgo` (it already sets its pipes to blocking), and
  `writeLine`'s retry loop (Decision 3).

## Decisions

### Decision 1: The cause is that the host opened its own stdin as a Node stream

- **Context**: `fs.readSync` on a pipe returns `EAGAIN` only when the
  descriptor is in non-blocking mode. ttc creates the host's stdin with
  `Stdio::piped()`, a blocking pipe, so something inside the host switched
  it.
- **Findings**:
  - `/proc/<host>/fdinfo/0` of an idle watch host read `flags: 02004000`
    (`O_NONBLOCK` set), and the host used 15 s of CPU in its first 16 s.
  - `strace -f -e trace=execve,fcntl,ioctl` of the watch showed the host
    process run `ioctl(0, FIONBIO, [1])` at startup, before it spawned
    `tsgo`.
  - Bisecting the host's top level with a probe that spawns a script with
    piped stdio and reads `/proc/<pid>/fdinfo/0`: a module containing only
    `import process from "node:process"` (or `import { env } from
    "node:process"`) leaves fd 0 and fd 1 non-blocking; a module using the
    global `process`, or importing `node:fs`, `node:path`, `node:url`,
    `node:crypto`, or the TypeScript API client, leaves them blocking.
- **Why**: Node's ECMAScript modules documentation (Modules: ECMAScript
  modules → Built-in modules, https://nodejs.org/api/esm.html#builtin-modules)
  states that "when importing built-in modules, all the named exports (i.e.
  properties of the module exports object) are populated even if they are
  not individually accessed". Populating `node:process` reads its lazy
  `stdin` and `stdout` getters, which construct `net.Socket`s on fd 0 and
  fd 1 (the Node.js `process.stdin` documentation: "It is a `net.Socket` …
  unless fd 0 refers to a file", https://nodejs.org/api/process.html#processstdin).
  A socket over a pipe is a libuv pipe handle, and libuv's `uv_pipe_open`
  documentation says "Changed in version 1.2.1: the file descriptor is set
  to non-blocking mode" (https://docs.libuv.org/en/v1.x/pipe.html#c.uv_pipe_open;
  `src/unix/pipe.c` calls `uv__nonblock(fd, 1)`). The descriptor is then
  shared by two readers: Node's stream, which never reads because nothing
  listens to it, and `lineReader`, whose `readSync` now fails with `EAGAIN`
  whenever no request is waiting and retried immediately.
- The `import` and the `EAGAIN` retry arrived together in TASK-073's first
  host; the retry compensated for the mode the import had set.

### Decision 2: Leave fd 0 as ttc created it, and stop retrying `EAGAIN`

- **Context**: The host must read stdin synchronously. The TypeScript API
  client is synchronous (`dist/api/syncChannel.js` drives `tsgo` with
  `fs.readSync`/`fs.writeSync`), and the host's file-system callbacks run
  inside those calls: `getAccessibleEntries` asks ttc `ownedOutputs` and
  needs ttc's answer before it returns, with the event loop not running.
  A blocking read is therefore required, and the question is only how to
  block rather than spin.
- **Alternatives considered**:
  1. *Use the global `process` and never construct Node's stdin stream*,
     so fd 0 keeps the blocking mode `Stdio::piped()` gave it and
     `fs.readSync` blocks in the kernel until a request or EOF arrives
     (Node.js `fs.readSync`, https://nodejs.org/api/fs.html#fsreadsyncfd-buffer-offset-length-position,
     is the synchronous `read(2)`). Removes the cause; no new mechanism.
  2. *Sleep on `EAGAIN` with `Atomics.wait`* (what the API client does
     for its own pipes, `sleepBuf` in `syncChannel.js`; Node.js
     `Atomics.wait` is available on the main thread). Still a poll: it
     trades CPU for request latency and keeps waking.
  3. *A worker thread that reads fd 0 asynchronously and hands lines to the
     main thread through a `SharedArrayBuffer` and `Atomics.wait`/`notify*
     (Node.js `worker_threads`, https://nodejs.org/api/worker_threads.html).
     Truly blocking, but a second reader of fd 0, a copy protocol, and a
     thread to shut down on EOF, all to work around a mode the host
     should not have set.
  4. *Put the descriptor back into blocking mode* with
     `process.stdin._handle.setBlocking(true)` (what the API client does
     with its child's pipes). Relies on an undocumented handle and keeps
     the unused stream.
  5. *Reopen `/dev/stdin`*. On Linux, opening `/proc/self/fd/0` of a pipe
     creates a new open file description, but on macOS `/dev/fd/0`
     duplicates the existing one (same `O_NONBLOCK`), and on Windows there
     is no such path.
- **Decision and rationale**: Alternative 1. The host already treats fd 0
  as its own: it is the only reader of the protocol. Constructing
  `process.stdin` made Node a second owner. The global `process` is the
  same object the import returned (Node.js `process` documentation: "The
  `process` object provides information about, and control over, the
  current Node.js process", https://nodejs.org/api/process.html), so
  every other use (`process.pid`, `process.execPath`, `process.env`,
  `process.stderr`, `process.exit`) is unchanged. The `EAGAIN` branch is
  removed rather than kept: a blocking `read(2)` never returns `EAGAIN`
  (POSIX `read`, https://pubs.opengroup.org/onlinepubs/9799919799/functions/read.html:
  `EAGAIN` only when `O_NONBLOCK` is set), so the branch
  only ever served as the spin. If a future change made fd 0
  non-blocking again, the read now fails and the session reports it,
  instead of burning a core in silence.
- **Platforms**: Linux and macOS both take libuv's `src/unix/pipe.c`
  path, where only `uv_pipe_open` sets `O_NONBLOCK`; with no stream on
  fd 0 the pipe stays blocking. On Windows, libuv's `uv_pipe_open` sets
  `PIPE_READMODE_BYTE | PIPE_WAIT` (`uv__set_pipe_handle`,
  `src/win/pipe.c`), so the Windows host never received `EAGAIN` from
  stdin, and without the stream nothing changes the pipe's mode at all.
  The `EOF` code (`ERROR_BROKEN_PIPE` on Windows) is still read as end of
  input.

### Decision 3: Keep `writeLine`'s retry loop

- **Context**: The same import had made fd 1 non-blocking, which is why
  TASK-084's `writeLine` retries `EAGAIN`. After this change fd 1 stays
  blocking (probe: `flags: 02000002` instead of `02004002`), so a blocking
  `write(2)` of the whole answer does not return `EAGAIN`.
- **Decision and rationale**: Leave `writeLine` unchanged. Its retry only
  runs while ttc is actively reading an answer, so it never spun while
  idle, and its partial-write loop is still correct for a blocking
  descriptor. Changing it is outside this defect and would touch a path
  TASK-084 pinned for large answers.

## Work log

- 2026-09-30: Reset the worktree onto `claude/ecstatic-dijkstra-qw5pf9`
  (3686b96), `npm ci` in the root and in `editors/vscode`, compiled the
  extension, fetched TypeScript's test cases, built `ttc`.
- 2026-09-30: Reproduced with a `ttc --check-types -w a.tt` watch in a
  scratch project: host `%CPU` 93.7–95.4, `flags: 02004000` on fd 0.
  Traced and bisected the cause (Decision 1).
- 2026-09-30: Added
  `typescript::native::idle_tests::an_idle_host_waits_for_the_next_request_without_spending_cpu`
  and ran it against the unfixed host (fails, see below).
- 2026-09-30: Removed `import process from "node:process"` and the
  `EAGAIN` retry from `host.mjs`; the test passes.
- 2026-09-30: Measured idle CPU and typed compile time before and after
  (below), then ran the full gate.

## Measurements

Machine: 4 CPUs, Linux 6.18, Node v22.22.2, debug `ttc`, TypeScript
`7.1.0-dev.20260826.1`.

- **Idle host CPU**: four `ttc --check-types -w` watches left idle, host CPU
  summed from `/proc/<pid>/stat` (`utime + stime`, `CLK_TCK` 100) over
  10 s: before 3 917 ticks (39.2 CPU-seconds, four cores saturated); after
  0 ticks.
- **Typed compile, idle machine**: `ttc --check-types .` over a 40-module
  project (one `variant` and one `match` each), 10 interleaved runs per
  binary, both binaries built with the same profile: median wall time
  before 0.854 s, after 0.809 s (within run-to-run noise; no regression).
- **Typed compile beside four idle watches**: 8 runs each, median wall time
  before 1.42 s (the idle hosts held every core), after 0.82 s.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/typescript/native.rs`,
  `typescript::native::idle_tests::an_idle_host_waits_for_the_next_request_without_spending_cpu`
  (Unix; reads the host's CPU time with `ps -o time=`, which Linux procps
  and macOS `ps` both print). It opens a host session, asks one question,
  leaves the host idle for 4 s, requires less than 2 s of host CPU time in
  that window, then asks again to show the session still answers. A unit
  test rather than a case file because the fault is in the process, not in
  any compiled output, and only this module holds the host's process id.
- **Observed failure**: `the host spent 4 s of CPU in 4 s with no request
  to answer` (`cargo test --lib idle_tests` with the unfixed `host.mjs`).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast`: 1 795 passed, 0 failed across 50 test binaries
- [x] `node scripts/check-baselines --tracking <dir>`: 325 compared, none
  unused
- [x] Baseline changes reviewed and committed with the change: none changed
- [x] Extension: `npm run compile`, then `node --test
  "server/out/test/*.test.js" "client/out/test/*.test.js"` with the built
  `ttc` on `PATH`: 238 passed, 0 skipped
- [x] `./scripts/ci agents`: passed (doctor reports "not ready" only for the release `ttc` and VSIX that `./scripts/setup` installs, which this task does not run)
- [x] Every host and watch process started for the measurements was
  stopped.

## Result

- `src/typescript/host.mjs`: uses the global `process` instead of importing
  `node:process`, so the host never constructs `process.stdin`/`stdout`
  and its stdio pipes keep the blocking mode ttc created them with;
  `lineReader` no longer retries `EAGAIN`.
- `src/typescript/native.rs`: the idle-host CPU regression test.
- `docs/tasks/INDEX.md`, this record.

An idle host now sleeps in `read(2)`: four idle watch hosts went from
39.2 CPU-seconds per 10 s to none, and a typed compile beside them from a
1.42 s to a 0.82 s median. The protocol is unchanged.
