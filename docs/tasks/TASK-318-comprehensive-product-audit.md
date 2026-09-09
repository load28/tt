# TASK-318: Comprehensive product-surface audit and structural repairs

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-03
- **Commit**: `TASK-318: repair audited product surfaces`

## Purpose

Audit the complete documented, compiler, CLI, tool-integration, and editor
surface of tt. Fix each confirmed defect in its responsible architectural layer
and add regression coverage that proves the repaired contract.

## Scope

- Included: Every user-facing document and command, all `.tt`/`.ttx`/`.ts`/`.tsx`
  source-graph combinations, compiler success and failure behavior, emitted
  diagnostics, editor language features, packaging, setup, and bundler adapters
- Excluded: Release publication and destructive changes to external services

## Decisions

### Decision 1: Audit against an explicit surface matrix

- **Context**: A passing test suite cannot prove that every promised product
  surface is covered or behaves correctly.
- **Alternatives considered**: Rely on the existing CI suite; sample individual
  features manually; derive a complete matrix from documentation, manifests,
  command surfaces, and implementation entry points.
- **Decision and rationale**: Build an explicit inventory and map every item to
  authoritative executable or documentary evidence. Missing evidence remains an
  open audit item rather than being treated as success.

### Decision 2: Repair causes only in their owning layer

- **Context**: The project contract forbids test-shaped branches, suppression,
  and fallback behavior that hides a defect.
- **Alternatives considered**: Patch observed strings or fixtures; suppress
  diagnostics; change the parser, semantic layer, backend, editor adapter, or
  documentation contract that actually owns the behavior.
- **Decision and rationale**: Diagnose each issue through the relevant model and
  fix the responsible layer, then add a regression at the closest public
  boundary and any required cross-layer integration boundary.

## Work log

- 2026-09-02: Updated from `origin/main`, ran `./scripts/doctor`, confirmed the
  environment is ready, and created the audit branch and task record.
- 2026-09-03: Ran the full local gate. The baseline Rust, native, and editor
  suites passed, while the npm stage exposed a non-hermetic global-cache
  dependency in the new generated-project E2E.
- 2026-09-03: Audited every CLI primary mode against every parsed option,
  introduced one compatibility model, rejected ignored combinations, and
  constrained stdout compilation to one self-contained source.
- 2026-09-03: Made the create-tt E2E use a workspace-local npm cache and
  verified the complete npm stage with registry access.
- 2026-09-03: Rebuilt the VS Code extension from a clean output tree, exposed
  deleted JavaScript tests and stale compiler selection, then repaired the
  build projection and compiler-selection contracts. `./scripts/ci extension`
  passes with 104 current tests and zero stale tests.
- 2026-09-03: Added a repository-wide tracked Markdown link gate and repaired
  every rendered link that still targeted the removed `docs/reference/`
  tree.
- 2026-09-03: Expanded the mixed-source fixture from twelve non-self edges to
  all sixteen directed `.tt`/`.ttx`/`.ts`/`.tsx` combinations. Both typed
  project checking and emitted-tree TypeScript checking pass.
- 2026-09-03: Audited the unplugin package, restored the missing public
  `sourcemap` option type, and added real compiler-backed coverage for every
  adapter, `.tt`, `.ttx`, standard modules, source maps, and diagnostics.
- 2026-09-03: Replaced the remaining generated `addRl` identifier, exercised
  references, rename, signature help, and document symbols through the real
  LSP adapter, and reran the 104-test editor suite.
- 2026-09-03: Audited npm tarballs and protected the main package from
  publishing a machine-local development stamp containing an absolute path.
- 2026-09-03: Type-checked and prerendered all 37 website paths with the Pages
  base path, then added the missing type-check step to the deployment gate.
- 2026-09-03: Installed and ran the optional coverage gate. The full suite
  passed at 87.88% line coverage against the 86.9% floor.
- 2026-09-03: Built the standalone fuzz crate, updated its stale compiler lock
  entry, and added a locked fuzz-target check to local and hosted CI.
- 2026-09-03: Cross-checked every current installation surface and removed
  stale TypeScript range and source-build prerequisites from active guidance.
- 2026-09-03: Re-ran the complete local gate, then found that the website was
  absent from both the local and hosted pull-request gates despite its separate
  post-merge deployment workflow. Added typed and static website builds to both
  merge gates.
- 2026-09-03: Ran the complete installed TypeScript corpus and raw-input fuzz
  target. Fuzzing reduced a codegen crash to seven bytes and exposed the same
  crash on valid string, template, and regex pipeline heads containing `//`.
  Replaced textual comment guessing with source-kind-aware lexical detection
  and made the helper that emits a break own its layout scope. The resulting
  literal regression exposed and drove a second repair to top-level runtime
  import placement around anchored constructs.
- 2026-09-03: Re-ran the complete local gate with registry and loopback access.
  All Rust, npm, website, native-backend, and editor stages passed.

## Issues and resolutions

### Issue 1: CLI modes silently accepted options they did not execute

- **Symptom**: Standalone and tooling modes accepted incompatible flags;
  `--check --print` emitted code, `--print --source-map file` emitted a dangling
  map URL, and multiple printed modules were concatenated on stdout.
- **Cause**: Argument parsing stored only resulting values, and each dispatch
  branch maintained a partial hand-written conflict list. Explicit default
  options could not be distinguished from absent options.
- **Resolution**: Record every parsed option as a canonical `CliOption`, select
  the primary mode once, and validate it against that mode's allowed set before
  doing work. Enforce the stdout mode's source and map cardinality explicitly.

### Issue 2: Generated-project E2E depended on the global npm cache

- **Symptom**: `./scripts/ci npm` failed before packing with `EPERM` when the
  user's global npm cache contained entries owned by another account.
- **Cause**: Bun's cache and the temp directory were isolated, but `npm pack`
  inherited the process-wide npm cache.
- **Resolution**: Point `npm_config_cache` at the same per-case temporary tree.

### Issue 3: Deleted editor tests and code survived TypeScript builds

- **Symptom**: The editor suite ran 134 tests, including deleted `.rl`-era test
  files that no longer had TypeScript sources.
- **Cause**: `tsc -b` does not remove output for deleted inputs, and both local
  and CI builds reused `client/out` and `server/out` indefinitely.
- **Resolution**: Add a cross-platform exact-output cleaner, make compilation
  invoke it, make CI call the package compile contract, and assert that emitted
  test files equal the current source test set.

### Issue 4: Editor E2E mixed current server code with a stale compiler

- **Symptom**: After stale JavaScript was removed, completion, hover, semantic
  tokens, and diagnostics returned empty answers or timed out.
- **Cause**: The resolver preferred `target/release/ttc` by fixed order even
  after CI built a newer debug compiler; temporary LSP projects could then fall
  through to another stale compiler on PATH.
- **Resolution**: Select the newest workspace build by modification time and
  pass the exact test compiler through both supported development resolution
  paths to the spawned LSP server.

### Issue 5: Current documentation linked to a removed reference tree

- **Symptom**: User, design, changelog, and task links targeted files under
  `docs/reference/`, which no longer exists.
- **Cause**: The user contract moved to `docs/ai/tt.md` without a repository
  gate for local Markdown targets.
- **Resolution**: Retarget rendered links to the current contract and add a
  test over every tracked Markdown file, excluding code examples.

### Issue 6: The mixed-source matrix omitted same-kind imports

- **Symptom**: The documented complete matrix asserted only twelve non-self
  edges, leaving `.ts`→`.ts`, `.tsx`→`.tsx`, `.tt`→`.tt`, and `.ttx`→`.ttx`
  outside the shared typed and emission fixture.
- **Cause**: The original matrix treated same-kind behavior as implicit rather
  than proving the full directed product.
- **Resolution**: Add one same-kind module per source kind and assert, type-check,
  sidecar-emit, and TypeScript-check all sixteen edges.

### Issue 7: The bundler runtime and public option type disagreed

- **Symptom**: `sourcemap: false` worked at runtime but was absent from
  `Options`, so a valid bundler configuration failed TypeScript checking.
- **Cause**: The JavaScript JSDoc and implementation gained the option without
  updating the shipped declaration, and no compiler-backed plugin suite ran.
- **Resolution**: Add the option to `index.d.ts` and run the shared plugin hooks
  plus all seven exported adapters against the current compiler in the npm
  gate.

### Issue 8: Generated bundler wrappers retained the former language name

- **Symptom**: New wrappers called their recursive configuration helper
  `addRl`, even though every public package and generated file uses tt.
- **Cause**: The initializer template survived the repository-wide language
  rename because no generated-source assertion covered internal identifiers.
- **Resolution**: Rename the emitted helper to `addTt` and assert every wrapper
  contains no former-language identifier.

### Issue 9: Several advertised editor methods stopped at engine tests

- **Symptom**: References, rename, signature help, and document symbols had no
  current end-to-end test through the LSP protocol after stale outputs were
  removed.
- **Cause**: Engine-level tests covered the semantic operations, but no live
  adapter test proved request registration, parameter conversion, and response
  projection together.
- **Resolution**: Exercise all four methods against a real server and compiler
  in source coordinates.

### Issue 10: A direct npm publish could expose the local checkout path

- **Symptom**: `npm pack` after local setup included `tt-dev.local.json`, whose
  `root` value is an absolute machine path.
- **Cause**: Local `file:` installs require the marker in the package file set,
  while the automated release relies only on its clean checkout being free of
  that ignored file.
- **Resolution**: Preserve local installation semantics but make
  `prepublishOnly` reject any package tree containing the development marker,
  with a regression for clean and stamped trees.

### Issue 11: Pages deployment did not type-check the website

- **Symptom**: The production build succeeded independently of TypeScript's
  `noEmit` check, so a typed website regression could reach deployment.
- **Cause**: The Pages workflow installed dependencies and called Vite without
  invoking the existing `typecheck` script.
- **Resolution**: Run `bun run typecheck` before the static build and verify the
  current site under the same `/tt/` base path as deployment.

### Issue 12: The standalone fuzz lock drifted from the compiler package

- **Symptom**: The first fuzz-crate check rewrote `fuzz/Cargo.lock`, changing
  the path dependency from compiler version `0.3.0-dev.6` to `1.0.0`.
- **Cause**: The fuzz crate is outside the main workspace and no ordinary merge
  gate built it with `--locked`.
- **Resolution**: Refresh the lock entry and run a locked all-targets fuzz check
  in both local and hosted Rust gates.

### Issue 13: Hosted CI omitted locally required tool integrations

- **Symptom**: The local npm gate tested unplugin and deliberation tooling, but
  the hosted merge gate tested neither; hosted extension builds also bypassed
  the clean-output compile script.
- **Cause**: The workflow and `scripts/ci` evolved independently after the
  original parity contract was written.
- **Resolution**: Add both tool suites to hosted CI and build the extension
  through its package compile contract.

### Issue 14: Active setup guidance contradicted the pinned toolchain

- **Symptom**: `CONTRIBUTING.md` and the compact AI contract instructed users
  to install `typescript@7`, while the repository contract says that range
  resolves to 7.0 and lacks required 7.1 APIs. The website also claimed typed
  modes still required a source-built tsgo checkout.
- **Cause**: The exact-version gate covered published copyable commands but not
  the contributing guide or compact prose, and one pre-npm toolchain limitation
  survived in website copy.
- **Resolution**: Name the exact pin on every active setup path, extend the
  version test to the contributing and AI contracts, and remove the obsolete
  source-build prerequisite.

### Issue 15: Website regressions were detected only after merge

- **Symptom**: The Pages workflow type-checked and built the site only on a
  `main` push, while pull-request CI and the default local gate omitted the
  website completely.
- **Cause**: Website deployment and merge validation were treated as one
  workflow even though only validation belongs before merge.
- **Resolution**: Add a default local `website` stage and mirror its frozen
  install, type check, and `/tt/` static build in hosted pull-request CI. Keep
  deployment itself in the post-merge Pages workflow.

### Issue 16: Double slashes in pipeline values caused a codegen crash

- **Symptom**: Valid pipelines such as `"//" |> String` and `` `//` |>
  String `` raised `LayoutScopeMissing`; raw-input fuzzing found the same
  internal error on a seven-byte malformed input.
- **Cause**: The rope helper treated any `//` on the final textual line as a
  line comment, including occurrences inside strings, templates, regexes, and
  JSX. It then emitted a layout break without opening the scope required by
  the target IR contract.
- **Resolution**: Reconstruct the rope's lexical text, use the existing
  source-kind-aware lexer to isolate trailing trivia, and classify only a real
  trailing line comment. The guarding helper now scopes the break it owns, and
  regressions cover literal contexts, JSX, split rope pieces, and both source
  kinds through the public compiler boundary.

### Issue 17: A file consisting of one effectful pipeline emitted an invalid import

- **Symptom**: After comment classification was repaired, `` `//` |> String ``
  emitted its runtime import immediately after the lowered call and failed the
  generated-TypeScript parser.
- **Cause**: Runtime import insertion searched only top-level source pieces.
  When the complete source was nested in a top-level tt anchor, the search
  found no insertion point and appended the import after the construct.
- **Resolution**: Treat the outermost anchor or layout scope containing the
  first eligible source byte as an atomic top-level insertion boundary. The
  import is now placed before the complete construct without splitting its
  mapping or layout structure.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — `agents`, `rust`, `npm`, `website`, `native`, and
      `extension` passed
- [x] Documentation inventory and example verification
- [x] Mixed-source `.tt`/`.ttx`/`.ts`/`.tsx` project matrix
- [x] CLI command and diagnostic matrix
- [x] Editor completion, hover, definition, references, rename, diagnostics,
      semantic tokens, and sidecar matrix
- [x] Tool, installer, package, and bundler integration matrix

## Result

The audited product surface now has executable coverage at each public
boundary. Seventeen confirmed defects were repaired in their owning compiler,
CLI, editor, integration, packaging, documentation, or CI layer, including the
raw-input codegen crash and its runtime-import follow-up.

Changed files: `.github/workflows/`, `src/codegen/`, `src/main.rs`,
`src/main/command.rs`, `tests/`, `fuzz/Cargo.lock`, `editors/vscode/`,
`integrations/unplugin/`, `packages/create-tt/`, `npm/`, `scripts/ci`,
`website/src/content.json`, the active user and design documentation, and this
task record and index.
