# TASK-738: Exercise an expanding mixed-source React application in VS Code

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Develop a realistic React application with `.tt`, `.ttx`, `.ts`, and `.tsx`
modules in VS Code, and repair reproducible editor, CLI, and compiler defects
in the responsible layer.

## Scope

- Included: An extensible order and inventory application, interactive editor
  checks, CLI and bundler checks, regression cases, and structural fixes.
- Excluded: Publishing the application or changing release versions.

## Decisions

### Decision 1: Use one growing application and preserve each reproduction

- **Context**: Isolated syntax examples do not exercise mixed-source development.
- **Alternatives considered**: Independent snippets are cheaper but miss module
  boundaries and incremental edits; a full production service adds unrelated work.
- **Decision and rationale**: Build a local React application in `.tt-dev/`, keep
  focused regression cases and verification results in the repository, and
  expand features after each editor/CLI/runtime verification cycle.

### Decision 2: Start from the fetched main branch

- **Context**: The user explicitly requested the latest main as the baseline.
- **Alternatives considered**: Continuing the existing task branch risks testing
  changes that differ from the merged implementation.
- **Decision and rationale**: Fast-forward local main to `01d7dd29` and create
  `task-738-react-vscode-dogfood`. The usual `codex/` prefix cannot be used because
  an existing `codex` branch occupies that Git ref namespace.

### Decision 3: Keep the application local

- **Context**: The user requested that the test application not be included in
  the repository. This supersedes the application snapshot portion of Decision 1.
- **Alternatives considered**: Keeping the snapshot would preserve a standalone
  example but contradict the requested scope.
- **Decision and rationale**: Keep `.tt-dev/react-ops` as the ignored development
  workspace. Retain compiler/editor regression tests and compact test results.

## Work log

- 2026-10-03: `./scripts/doctor` passed. Connected to the real VS Code window.
  Fetched origin/main and created the task branch. Preserved `.task-agent-disabled`.
- 2026-10-03: Built the release compiler and created the mixed React application
  in `.tt-dev/react-ops`. Expanded it through orders, inventory/reporting, and
  editing/permissions. A portable snapshot was initially recorded and subsequently
  removed at the user's request; the working application remains local.
- 2026-10-03: Verified real VS Code definition navigation, a cross-file rename,
  one authored TS2322 and its removal; verified browser order submission,
  reservation, permission rejection, invalid quantity and recovery.
- 2026-10-03: CLI watch detected a TS dependency error at `api.ts:4:105`, then
  cleared it after restoration. Type checks, production builds, declaration
  generation, and runtime assertions passed.
- 2026-10-03: Ran the actual extension-host matrix, corrected stale assertions,
  reproduced the remaining pipeline bug through the engine, and fixed the shared
  completion context model. Investigated and rejected a temporary-document
  identity hypothesis; those experimental changes were removed.

- 2026-10-03: Rebuilt the release compiler, reloaded the working VS Code window,
  and confirmed String member suggestions after the pipeline dot. Removed the
  temporary incomplete file; the Problems panel returned to no problems. Closed
  the unused `tt-sample` window at the user's request, retaining `react-ops`.

- 2026-10-03: Removed the tracked application snapshot at the user's request.
  Kept the local development workspace and all focused regression coverage.

## Issues and resolutions

### Issue 1: Pipeline shorthand completions returned global names

- **Symptom**: In the real VS Code extension host, a script containing
  `const result = "hello" |> .;` did not offer `toUpperCase`.
- **Cause**: `is_member_context` only scanned backwards for a dot, so a nonempty
  global TypeScript answer was classified as a member answer. The engine
  therefore never requested its existing compiler-owned completion probe.
- **Resolution**: Reuse the lexer's operand-completion facts and the shared
  member-dot query. A served TypeScript member access requires a receiver;
  incomplete tt shorthand reaches the existing probe. No new syntax, diagnostic
  suppression, or application-specific fallback was introduced.

### Issue 2: Extension-host assertions described an older diagnostic contract

- **Symptom**: The error-range check expected `value` instead of `result` in TT
  sources, and JSX text checks rejected ordinary word suggestions named like cases.
- **Cause**: Assertions did not distinguish TypeScript binding diagnostics from
  an older mapping or text suggestions from tt constructors. A failed assertion
  also left the temporary error in the buffer for the next check.
- **Resolution**: Assert the same authored binding range across all four source
  types, distinguish Text items from pattern constructors, and restore temporary
  consumer edits in `finally`.

### Ruled-out observations

- A `result` block without a direct `try` was invalid fixture usage under the
  documented contextual grammar. The fixture was corrected.
- A rename appeared absent on disk while its TSX buffer was still unsaved.
  The buffer contained both edits; saving it restored CLI agreement.
- An initial sandboxed gate could not inspect processes or create its temporary
  Git metadata. A later gate overlapped extension rebuilding during investigation
  and read an intermediate failing regression. Neither is a completed gate.

## Regression test (fails before the fix)

- **Path**: `editors/vscode/test/editor.cjs`, `introduce and clear unsaved error`
  across the mixed-source matrix; followed by `dependency edits refresh untouched consumer`.
- **Observed failure**: On unmodified main, the real VS Code extension host reported
  `'result' !== 'value'` for a valid TS2322 binding range. The failed assertion left
  `result: number` in the consumer and the next check then failed to clear that
  unrelated error. Evidence: `evidence/TASK-738/editor-before.json`.
- **Path**: `tests/native/editor_service.rs::installed_mapper_completes_pipeline_members_in_scripts_and_modules`.
- **Observed failure**: Before the production fix, `cargo test --test native
  installed_mapper_completes_pipeline_members_in_scripts_and_modules` failed with
  `module=false: missing String members`. It passes after the token-grammar fix.
- The editor case `tests/cases/editor/pipelineShorthandMembers.tt` also holds the
  adapter's String completions to a reviewed baseline. Production diagnostic
  behavior remains unchanged.


## Verification

- [x] Interactive VS Code checks: definition, rename, diagnostic recovery, and
  pipeline String completions in the reloaded local window. The real extension
  host passed all 71 checks (`evidence/TASK-738/editor-after.json`).
- [x] Mixed-source CLI, build, and runtime checks: TypeScript project checking,
  `ttc --check-types`, declaration generation, watch recovery, Vite production
  build, and runtime assertions. The portable fixture also passed independently.
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

The final exclusive `./scripts/ci` run exited with status 0: `agents rust npm
website native extension` all passed. The extension suite passed 238 tests with
zero skipped tests. Baseline checking compared 5,469 artifacts and found none
unused; the remaining 5,793 unsampled matrix baselines were explicitly unjudged
by that gate. The local full log is `.tt-dev/task738-ci-final.log`.

After removing only the application snapshot, `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `scripts/check-task-index`, and
`git diff --check` passed again. Compiler, editor, and regression sources are
identical to the full-gate run above, so its test results remain applicable.

## Changed files

- `src/engine/completions.rs`, `src/engine/language/service.rs`: shared lexical
  member context and completion-probe routing.
- `src/engine/language/tests.rs`, `tests/native/editor_service.rs`,
  `tests/cases/editor/pipelineShorthandMembers.tt`, and its reference baseline:
  lexical, installed-mapper, and editor regression coverage.
- `editors/vscode/test/editor.cjs`: current diagnostic-range contract, cleanup
  after assertion failure, and JSX word-suggestion distinction.
- `docs/design/lsp-architecture.md`, this task, `docs/tasks/INDEX.md`, and
  `docs/tasks/evidence/TASK-738/`: architecture rationale, results, and the
  compact verification results.

## Result

The pipeline completion defect is fixed in the shared engine context model.
Three application expansion cycles and the final revalidation found no further
reproducible toolchain defects in the exercised flows. The ignored local application
remains available for subsequent feature expansion; this is not a claim that
arbitrary future applications are defect-free.
