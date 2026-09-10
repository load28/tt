# TASK-356: Separate a missing toolchain from a project the pass cannot read

> TASK-357 supersedes the partial-scan and unreadable-input skipping policy.
> It also extends support-package lookup to respect ancestor installations.

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-356: fix(compiler): serve the standard library and keep input failures visible`

## Purpose

Two review findings on PR 115, both reproduced. TASK-354 caught every
`Unavailable` failure from the contextual pass, but the pass raises that
kind for a project it could not read as well as for a toolchain that is not
there — so a real failure was silently discarding annotations the emitted
code needs. And the emit fixture TASK-355 pinned depended on an ambient
`node_modules/@tt/std` that `npm ci` does not install.

## Scope

- Included: where the "no toolchain" policy is applied, how the sibling
  scan treats an input it cannot read, and how `@tt/std` reaches the checker
- Excluded: what the contextual pass computes when it runs

## Decisions

### Decision 1: The policy belongs where the cause is known

- **Context**: `contextual::standalone` returns `Unavailable` for three
  different things — no toolchain, a directory walk that failed, a sibling
  that could not be read. A caller sees one kind and cannot tell them apart.
- **Alternatives considered**: Add a fourth `FailureKind` and keep deciding
  at the callers (two call sites, one policy, still duplicated); inspect the
  message text (a string is not a contract).
- **Decision and rationale**: The pass answers with the unrefined emit when
  there is no toolchain, and the callers report every failure that reaches
  them. One policy, applied once, at the boundary that knows why it failed.

### Decision 2: The sibling scan is enrichment, not this file's compilation

- **Context**: A sibling whose bytes are not UTF-8, or a tree with a
  dangling symlink, aborted the whole pass — so an unrelated broken file in
  the directory cost the file being compiled its slot types, and the emitted
  code then failed `tsc` with TS7034.
- **Alternatives considered**: Report the sibling (compiling it reports it
  in its own right, and failing *this* file over it helps nobody).
- **Decision and rationale**: An entry the scan cannot read is one module
  the checker does not get, not an answer the file loses.

### Decision 3: The standard library is served, not required

- **Context**: The fixture's annotation needs `@tt/std` to resolve. It
  resolved only because an earlier session's content mapper had written the
  package into this checkout's `node_modules`; `npm ci` does not.
- **Alternatives considered**: Commit a copy under the fixtures (it drifts
  from the compiler's own sources); have the suite write the package before
  running (it fixes the test and leaves every user's project needing the
  package materialized before its types can be inferred).
- **Decision and rationale**: ttc owns these modules — a project only ever
  gets a copy. The pass serves them to the checker through the layered
  filesystem the host already runs on, so nothing is written to disk and the
  annotation no longer depends on what happens to be installed. A package
  the project has on disk is left alone, so a project that manages its own
  copy keeps it.

## Work log

- 2026-09-10: Read the four review threads. Two were already resolved by
  TASK-353; reproduced the two new ones against this head.
- 2026-09-10: Confirmed `node_modules/@tt/{std,runtime}` exists in this
  checkout, is declared by neither `package.json` nor `package-lock.json`,
  and is dated to the audit session that wrote it — so the TASK-355
  verification ran against a contaminated environment.
- 2026-09-10: Moved the toolchain policy into the pass, made the sibling
  scan skip what it cannot read, and served the standard library from the
  compiler's own modules.

## Issues and resolutions

### Issue 1: A real failure was read as a missing toolchain

- **Symptom**: With the pinned TypeScript installed and working, a sibling
  `.tt` holding invalid UTF-8 made `ttc -p main.tt` exit 0, emit
  `let $tt_v0;` where it had emitted `let $tt_v0: number[];`, and say
  nothing. Checking that output reports TS7034 and TS7005.
- **Cause**: TASK-354 caught `FailureKind::Unavailable` at both compile
  entry points, and the pass raises that kind for an unreadable input as
  well as for a missing toolchain.
- **Resolution**: The pass returns the unrefined emit itself when there is
  no toolchain, and skips an input it cannot read. Both callers report
  whatever still reaches them.

### Issue 2: A fixture pinned an annotation only an ambient package allowed

- **Symptom**: With the ambient `@tt/std` removed and TypeScript present,
  `TTC_REQUIRE_TSGO=1 cargo test --test snapshot` failed:
  `tests/fixtures/emit/try-and-result/expected.ts is out of date`, the
  compiler emitting `let $tt_v0;` instead of the pinned union. TASK-355's
  guard asks only about TypeScript, so it could not make this deterministic.
- **Cause**: The contextual pass required `@tt/std` to be resolvable on
  disk, and this checkout had a copy no dependency declares.
- **Resolution**: The pass serves the package from the compiler's own
  modules. The fixture regenerates byte-identically with the ambient copy
  removed, and passes with it present.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — all six stages pass, with `node_modules/@tt` removed
      from this checkout for the whole run.

Each finding was re-run against the case that exposed it:

| case | before | after |
| --- | --- | --- |
| unreadable sibling beside `main.tt` | `let $tt_v0;`, exit 0 | `let $tt_v0: number[];` |
| snapshot without ambient `@tt/std` | fixture "out of date" | 4 passed |
| snapshot with a project's own copy | 4 passed | 4 passed |
| `--check`/`-p` with no toolchain | exit 0 (TASK-354) | exit 0 |

## Result

The contextual pass decides for itself what a missing toolchain means and
reports everything else. The standard library reaches the checker from the
compiler that owns it, so a fixture — or a user's project — no longer needs
the package installed for its generated storage to be typed.
