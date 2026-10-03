# TASK-734: Use TypeScript file identities in reachability and contextual dependencies

- **Status**: Complete
- **Started**: 2026-10-02
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Make the typed and fourslash oracles preserve files reached by an import on
case-insensitive hosts, using the same file identities as TypeScript. Apply the
same identity contract to contextual storage dependency traversal, where a
missed declaration causes joins to retain an unsettled `any` type.

## Scope

- Included: oracle reachability, contextual dependency traversal in
  `src/typescript/host.mjs`, and oracle/compiler regressions.
- Excluded: changing module resolution, inference rules, or accepted-difference lists.

## Decisions

### Decision 1: Ask the active TypeScript API for canonical file names

- **Context**: A declaration handle's path uses TypeScript's canonical spelling.
  On macOS `/users/...` does not equal the supplied `/Users/...` string.
- **Alternatives considered**: Lowercase paths on every platform, infer host
  behavior from the OS, or use `API.getCanonicalFileName`.
- **Decision and rationale**: Use the active API's canonical names as lookup keys
  and return the original member spelling. This preserves case-sensitive hosts
  and the caller's path-based unit selection. The user approved this approach.

## Work log

- 2026-10-02: The unmodified fourslash sample failed both reference questions of
  `isDefinitionSingleImport`: `a.ts` was wrongly renamed to `a.tt` although
  `b.ts` imports it. A standalone oracle probe printed the canonical lowercase
  declaration path, the original mixed-case member path, and `matched: false`.
- 2026-10-02: Read the pinned API's initialization and `getCanonicalFileName`.
  Interrupted preliminary full runs rather than mixing oracle revisions.

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

- 2026-10-02: The first standard gate passed agents, npm, website, native,
  and extension stages but failed the existing CLI storage-dependency regression.
  Reproduced it alone and traced the second canonical-path membership mismatch
  in the production contextual host. Fixed that layer using the same approved
  TypeScript identity rule, and verified the old CLI regression and a new
  multi-file compiler case before restarting the final standard gate.

- 2026-10-02: The final standard gate passed npm, website, native, and
  extension. Its agents stage overlapped a documentation edit and correctly
  rejected the changed working-tree status; a subsequent standalone agents
  stage passed. Rust reached an existing `engine_cache` expectation that
  compares logical `/var/...` workspace paths with canonical `/private/var/...`
  scan results. The isolated test passes with `TMPDIR=/private/tmp`; restarted
  the complete Rust gate with that physical temporary root. The alias-sensitive
  test expectation remains unchanged and is tracked in TASK-732.

## Issues and resolutions

### Issue 1: Imported files lose their identities in the comparison

- **Symptom**: TypeScript reference results disagree after the harness renames an imported file.
- **Cause**: A canonical declaration path is tested against an uncanonicalized member set.
- **Resolution**: Canonicalize every reachability candidate with the initialized
  API and recover the member's supplied spelling from a map. Imports, references,
  augmentations, relative candidates, and declaration-map sources use one lookup.

### Issue 2: Contextual joins miss unsettled storage through canonical paths

- **Symptom**: The standard gate's existing CLI regression
  `types_join_storage_after_the_storage_its_values_read` succeeds with no
  diagnostics where three errors are required; a targeted rerun reproduces it.
- **Cause**: `readsPending` compares a canonical declaration handle's path to
  the uncanonicalized project member set, the same identity mismatch as Issue 1.
- **Resolution**: Apply the active TypeScript API's identity to contextual
  project membership. Keep dependency traversal and inference rules unchanged.

## Regression test (fails before the fix)

- **Path**: `tests/corpus.rs::oracle_reachability_preserves_the_members_file_spelling`;
  upstream `isDefinitionSingleImport_test.go`.
- **Observed failure**: Before modifying the oracle,
  `cargo test --test corpus oracle_reachability_preserves_the_members_file_spelling`
  failed with `left: Array []`, while the right side contained
  `MixedCase/Imported.ts` under the original mixed-case workspace path.
  The upstream test also failed both references questions. Both pass after the fix.

- **Path**: `tests/cases/compiler/storageDependenciesRespectFileIdentity.tt`
  and `tests/cli.rs::types_join_storage_after_the_storage_its_values_read`.
- **Observed failure**: The original host makes the CLI test fail at `!ok`
  (it reports no diagnostics). After generating the new case's expected
  baselines, restored `host.mjs` alone from HEAD and reran the case: exit 101,
  with emitted storage and hovers retaining `any`/`any[]` instead of `number[][]`
  and `any[][]`, and the required three TS2339 errors absent. Restored the fix;
  both tests pass. Reviewed the emitted code, types, and diagnostic baselines.

## Verification

- [x] Direct regression before and after
- [x] Filtered upstream comparison: 5 selected, 3 compared, 3 questions,
  zero differences; 2 ineligible under existing harness rules.
- [x] Typed-parity sample: 80 selected, 55 compared, zero differences,
  25 skipped under existing eligibility rules.
- [x] `node --check tests/typescript-diagnostics.mjs`.
- [x] Existing CLI regression and new multi-file compiler case pass.
- [x] New `.ts`, `.types`, and `.errors.txt` baselines reviewed.
- [x] `node --check src/typescript/host.mjs`.
- [x] Local gates

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

- `src/typescript/host.mjs`
- `tests/typescript-diagnostics.mjs`
- `tests/corpus.rs`
- `tests/cases/compiler/storageDependenciesRespectFileIdentity.tt`
- `tests/baselines/reference/storageDependenciesRespectFileIdentity.ts`
- `tests/baselines/reference/storageDependenciesRespectFileIdentity.errors.txt`
- `tests/baselines/reference/storageDependenciesRespectFileIdentity.types`
- `docs/tasks/TASK-584-joins-wait-for-settled-inputs.md`
- `docs/tasks/TASK-732-pr-131-follow-up.md`
- `docs/tasks/INDEX.md`

## Final verification (2026-10-03)

- `./scripts/ci` passed all six stages: agents, rust, npm, website, native, and extension.
- Default macOS temporary root; `RUST_TEST_THREADS=2 GOMAXPROCS=2`. Disposable test commits used process-local `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false`.
- 419 library tests, 172 native tests, and 238 extension tests passed. Baseline audit: 5,468 compared, none unused; 5,793 unsampled matrix baselines remain outside this standard gate.
- Log: `/private/tmp/tt-737-final-ci-2.log`. Full nightly matrices and complete upstream sweeps remain separate TASK-732 work.

## Result

Canonical TypeScript file identities govern oracle reachability and contextual dependencies. The fail-before regressions and all required local gates passed.
