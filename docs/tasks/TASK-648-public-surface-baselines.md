# TASK-648: Baseline the Rust API, the server protocol, and the extension's capabilities

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-648`

## Purpose

Three surfaces are consumed outside this repository's compiler: the `ttc`
library API (`ttc::engine` is what embeddings build on), the `ttc --server`
JSON-lines protocol (the VS Code extension and the bundler adapters), and
the LSP capabilities the extension's language server advertises. A change
to any of them is visible today only if someone reads the code. TypeScript
makes its API a baseline, so every change is a reviewed diff; this task
does the same for all three.

## Scope

- Included: `tests/public_api.rs`, `tests/baselines/reference/api/`
  (`ttc.api.txt`, `server-protocol.txt`, `lsp-capabilities.json`), the `api`
  root in `scripts/check-baselines`, building the extension's language
  server before `cargo test` in `scripts/ci` (`rust` stage) and the `check`
  job of `.github/workflows/ci.yml`, and `CONTRIBUTING.md` ("The public
  surface").
- Excluded: the extension's `package.json` contributions (settings,
  grammars) and the npm packages' JavaScript APIs.

## Sources modelled

- microsoft/TypeScript `release-6.0` at
  `050880ce59e30b356b686bd3144efe24f875ebc8`,
  `src/testRunner/unittests/publicApi.ts` (lines 9 to 27): `verifyApi`
  reads the built `typescript.d.ts`, normalizes line endings, and runs
  `Harness.Baseline.runBaseline("api/typescript.d.ts", ...)` in a test named
  "should be acknowledged when they change". The reference is
  `tests/baselines/reference/api/typescript.d.ts` at that commit (blob
  `0f39eae7`). The API text is the compiler's own declaration output, not
  a hand-kept list.

## Decisions

### Decision 1: rustdoc's HTML on the pinned toolchain, not `cargo public-api` or a source walk

- **Context**: The analog of the compiler's own `.d.ts` is the compiler's
  own view of the public API: what is reachable, through which re-exports,
  with which signatures.
- **Alternatives considered**: (a) `cargo public-api`: not installed and
  not available offline, and it reads rustdoc JSON, which needs a nightly
  toolchain (`-Z unstable-options --output-format json`); the repository
  pins stable 1.98.0 (`rust-toolchain.toml`), and forcing nightly features
  on stable with `RUSTC_BOOTSTRAP` is unsupported. (b) A source-derived
  listing (parse `src/` with `syn`, follow `pub mod` and `pub use`, glob
  re-exports included, and attach inherent `impl` blocks by name): a second
  implementation of name resolution in a test, wrong in exactly the cases
  that matter (`pub use api::*` from a private module, impls in files that
  `use super::*`). (c) Run stable `cargo doc --no-deps --lib` and read the
  item pages rustdoc lists in `all.html`.
- **Decision and rationale**: (c). rustdoc resolves visibility and
  re-exports; the test only extracts text. For each listed item it keeps the
  `item-decl` declaration (fields and variants included; private fields are
  rustdoc's `/* private fields */`), inherent methods under their `impl`,
  trait and auto-trait implementation headers, trait methods, and
  implementors; blanket implementations are left out because dependencies'
  blanket traits (`Instrument`, `IntoEither`) would put their changes in
  this crate's baseline. Items are ordered by page path. rustdoc's HTML is
  not a stable format, so a toolchain bump can change the baseline; the
  toolchain is pinned and bumping it is a task (AGENTS.md), which then
  reviews that diff. Measured: re-documenting the crate after a change
  takes about 4 seconds; the first `cargo doc` of a fresh target also
  checks the dependencies (about 75 seconds here, sharing the target
  directory with `cargo test`, so there is no extra target directory).
  Running `cargo doc` from inside a test binary does not wait on cargo's
  build lock: `cargo test` releases it before running tests.

### Decision 2: The protocol as request and answer shapes from a live server

- **Context**: The protocol has no schema; `src/server.rs` documents it in
  prose and the dispatcher is a `match` on method names.
- **Alternatives considered**: A hand-written schema file (a second source
  that drifts); recording whole answers (paths, positions, and TypeScript's
  wording change for reasons unrelated to the protocol).
- **Decision and rationale**: `tests/public_api.rs` sends one or more example
  requests per method to `ttc --server` over a two-file project and
  baselines each request's and answer's shape: object keys, JSON types,
  array element shapes merged, a key absent from some objects marked `?`.
  Two checks keep the examples complete: the set of methods the dispatcher
  in `respond` matches must equal the set of example methods, and every
  `params["..."]` the server reads must be sent by some example. An
  unknown method's answer (`{ id, error }`) is baselined too. A shape is
  what the examples produced, so a field that is `null` in every example
  shows as `null`; a later example can widen it.

### Decision 3: The capabilities from the built extension server, required in the gates

- **Context**: The capabilities are computed in
  `editors/vscode/server/src/server.ts` from constants and library enums;
  only the running server has the final object. It needs the extension's
  `node_modules` and a compile, which `cargo test` does not do.
- **Alternatives considered**: Extracting the object literal from the
  TypeScript source (a parser for one expression, which would not evaluate
  the imported enums); keeping the baseline in the extension's own
  `node --test` suite (the `check` job, which owns baseline tracking, does
  not run that suite, and TASK-635 had to exclude an extension-owned
  baseline for that reason).
- **Decision and rationale**: The Rust test starts
  `editors/vscode/server/out/server.js --stdio`, sends `initialize`, and
  baselines the result with sorted keys. Without a built server it skips;
  `TT_REQUIRE_EXTENSION=1` turns the skip into a failure, and
  `./scripts/ci rust`, `node scripts/check-baselines --run`, and the CI
  `check` job set it. `./scripts/ci rust` and the `check` job build the
  server first (`npm ci` and `npm run compile` in `editors/vscode`); the
  tracking check reports the baseline unused in any run that skipped it.

## Work log

- 2026-09-30: Read `publicApi.ts` and the API baseline at `050880c`;
  checked that neither `cargo-public-api` nor a nightly toolchain is
  installed.
- 2026-09-30: Added `tests/public_api.rs`, generated the three baselines
  (`UPDATE_EXPECT=1 TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 cargo test
  --test public_api`), and read them.
- 2026-09-30: Registered the root in `scripts/check-baselines`, built the
  server in `scripts/ci` and CI, documented the section.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this task adds surface baselines and fixes no bug. The
baselines' failures are shown under Verification.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 cargo test --test
  public_api` passes twice in a row after generation (5 seconds, 1 second
  with rustdoc's output current).
- [x] Negative check: a new `pub fn api_probe() {}` in `src/lib.rs` fails
  with "modified baseline: .../api/ttc.api.txt" and `+ // fn
  ttc::api_probe` / `+ pub fn api_probe()`; reverted.
- [x] Every baseline read: 261 items in `ttc.api.txt`, 26 dispatched
  methods and 16 read parameters in `server-protocol.txt`, and the full
  `initialize` result in `lsp-capabilities.json`.
- [x] `cargo clippy --test public_api -- -D warnings`; `bash -n scripts/ci`.
- [x] The full gate ran over the final tree of TASK-647 to TASK-651; its
  results are in TASK-651.

## Result

Changed files: `tests/public_api.rs`, `tests/baselines/reference/api/**`,
`scripts/check-baselines`, `scripts/ci`, `.github/workflows/ci.yml`,
`CONTRIBUTING.md`, `docs/tasks/INDEX.md`, and this record.
