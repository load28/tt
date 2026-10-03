# TASK-733: Preserve authored declaration-map target coordinates

- **Status**: Complete
- **Started**: 2026-10-02
- **Completed**: 2026-10-03
- **Commit**: —

> TASK-737 supersedes the temporary same-URI limitation below: normal source
> documents now contain authored text, and probes have distinct virtual identities.

## Purpose

Continue TASK-732 and reproduce TASK-730's unresolved declaration-map navigation
into a `.tt` source whose preceding variant changes its emitted coordinates.

## Scope

- Included: target coordinate provenance when authored and served identities
  differ, editor case regressions, and current-tree verification.
- Excluded: changing TypeScript's declaration-map algorithm or module naming,
  and the shared-URI installed-mapper serving contract (Issue 2, carried by TASK-732).

## Decisions

### Decision 1: Identify the coordinate space before translating a target

- **Context**: `map_target` treats every `.tt` URI as projected text, including
  declaration-map targets that TypeScript has already mapped into source text.
- **Alternatives considered**: Guess from whether a range maps, parse declaration
  maps again, or use document identity and the service's actual served documents.
- **Decision and rationale**: Model authored versus projected target coordinates
  explicitly. Preserve TypeScript's source-map resolution and map a projection
  exactly once. The user approved this design before implementation.

## Sources

The pinned TypeScript implementation at `5739027c9a7df24e27123f453a50c011b37717b6`:
`tsc/internal/ls/source_map.go` follows declaration maps before building the LSP
range; `tsc/internal/ls/definition.go` returns that range in a standard Location
or LocationLink, without a coordinate-origin discriminator.

## Work log

- 2026-10-02: Doctor passed. Fetched origin and confirmed main at `67c98aa8`.
  Read PR 131, TASK-732, the matrix and upstream-parity contracts, and the pinned
  TypeScript `tsc/internal/ls/source_map.go` implementation.
- 2026-10-02: A temporary multi-file server probe returned `api.ts` line 2,
  columns 17–21 for its TypeScript twin, but no location for `api.tt` with a
  variant before the same function. No baselines were accepted during discovery.

- 2026-10-02: Started a nightly-sized run with full compiler/editor matrices,
  typed parity, and upstream passthrough. Stopped it during compiler cases to
  return to the repository's standard full gate plus focused expanded checks;
  the interrupted run is not a completed sweep. The standard gate runs with
  `RUST_TEST_THREADS=2 GOMAXPROCS=2` and temporary-process-only
  `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false`:
  the npm harness's disposable commits otherwise invoke this host's 1Password
  signer. No user or repository signing configuration was changed. The gate
  runs outside the sandbox because its CPU check uses `ps` and the website
  prerenderer listens on localhost.

## Issues and resolutions

### Issue 1: Declaration-map targets are translated twice

- **Symptom**: Definition of an imported function returns no location.
- **Cause**: Authored `.tt` source coordinates are interpreted in lowered text.
- **Resolution**: Classify target coordinates by virtual document identity and the
  exact URI served by the session. Apply inverse mappings and shared-binding
  expansion only to projected targets. Authored targets retain their ranges.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/declarationMapAuthoredTarget.tt`, with its
  `.ts` twin and `tests/baselines/reference/editor/declarationMapAuthoredTarget.baseline`.
- **Observed failure**: Before changing `src/`,
  `TT_CASES=declarationMapAuthoredTarget cargo test --test editor_cases
  every_editor_case_matches_its_baseline` failed the expected baseline:
  `definition: 0 location(s)` instead of `api.tt 2:17-2:21 "work"`, and
  `definition mapped: differs; ts only: api /*target*/ "work"`. The direct
  import was already correct. After the fix both agree with the twin; the
  expanded `.ttx` source-root question also agrees.

### Issue 2: An installed mapper can serve transformed text under the authored URI

- **Symptom**: With a genuinely installed mapper, a configured content-mapper
  project that also directly imports `api.tt` still returns no declaration-map
  target, while its direct import navigates correctly.
- **Cause**: Both the served projection and declaration-map target use the same
  URI. TypeScript's location protocol does not carry the coordinate provenance
  needed to distinguish them. Treating every source URI as authored would
  regress direct-import navigation.
- **Resolution**: Completed by TASK-737 using TASK-735's approved authored-buffer
  contract, exact contextual projections, and distinct virtual probe identities.
  Genuine installed-mapper regressions cover direct and declaration-map targets.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Editor case: authored `.tt`, direct projected import, and `.ttx` with
  a spaced relative source root all agree with the TypeScript twin.
- [x] Every upstream fourslash file containing a declaration map was selected:
  33 tests matched the 12 filename filters, 4 compared, 5 questions, zero
  differences; 29 skipped under the harness's existing eligibility rules.
- [x] Baseline changes reviewed

### Earlier verification attempts (2026-10-02)

- Standard gate: npm, website, native, and extension passed on the final source.
- Rust retry with `TMPDIR=/private/tmp`: formatting and clippy passed; compiler
  baselines, CLI, corpus comparisons, editor cases, engine cache, native service,
  and snapshot suites passed. The last integration binary, `workflow_repairs`,
  has 19 passes and one fixture-creation failure: this filesystem rejects the
  non-UTF-8 filename before ttc runs. The full Rust gate is therefore **not
  passing**, and its subsequent baseline-tracking audit was not reached.
- Logs: `/private/tmp/tt-733-734-final-standard-gate.log` and
  `/private/tmp/tt-733-734-rust-final.log`. The platform-dependent test issues
  remain in TASK-732; none was skipped or accepted as a changed baseline.
- Standalone final agents gate passed; all 35 documentation tests and
  `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked` passed.
- Full nightly matrices and full upstream parity remain unverified.

## Changed files

- `src/engine/language/service.rs`
- `tests/cases/editor/declarationMapAuthoredTarget.tt`
- `tests/cases/editor/declarationMapAuthoredTarget.ts`
- `tests/baselines/reference/editor/declarationMapAuthoredTarget.baseline`
- `docs/design/lsp-architecture.md`
- `docs/tasks/TASK-730-declaration-map-sources.md`
- `docs/tasks/TASK-732-pr-131-follow-up.md`
- `docs/tasks/INDEX.md`

## Final verification (2026-10-03)

- `./scripts/ci` passed all six stages: agents, rust, npm, website, native, and extension.
- Default macOS temporary root; `RUST_TEST_THREADS=2 GOMAXPROCS=2`. Disposable test commits used process-local `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false`.
- 419 library tests, 172 native tests, and 238 extension tests passed. Baseline audit: 5,468 compared, none unused; 5,793 unsampled matrix baselines remain outside this standard gate.
- Log: `/private/tmp/tt-737-final-ci-2.log`. Full nightly matrices and complete upstream sweeps remain separate TASK-732 work.

## Result

Authored declaration-map targets retain their source coordinates. TASK-737 also resolves the installed-mapper collision. All required local gates passed.
