# TASK-383: Audit tt and ttx developer workflows

> TASK-384 implements repairs for these baseline findings; see that record for their current status.

- **Status**: Complete
- **Started**: 2026-09-18
- **Completed**: 2026-09-18
- **Commit**: —

## Purpose

Audit the latest main for reproducible compiler, CLI, and editor problems affecting `.tt` and `.ttx` development. This is a discovery task; the defects below remain unfixed.

## Scope

- Included: local gates, compiler composition and runtime probes, actual VS Code extension-host tests, CLI output safety, declaration placement, and watch lifecycle.
- Excluded: repairs, publishing, local extension reinstallation, and claims of exhaustive correctness for all programs or platforms.
- Baseline: `069a5fb` on macOS arm64, Rust 1.98.0, repository TypeScript `7.1.0-dev.20260826.1`, and freshly built debug/release compilers. The installed native extension is `typescriptteam.native-preview-0.20260826.1`; its bundle predates the latest repository patch.

## Decisions

### Decision 1: Confirm failures with executable inputs

- **Context**: Passing existing tests does not establish correctness for combinations they omit.
- **Alternatives considered**: source-only review; running only existing tests.
- **Decision and rationale**: Combine source review, existing suites, targeted reproductions, and comparison checks. The numbered findings distinguish observable failure families, not a claim that all have independent root causes.

### Decision 2: Keep audit evidence separate from repairs

- **Context**: The user requested discovery across the development workflow.
- **Alternatives considered**: immediately patch individual defects while auditing.
- **Decision and rationale**: Preserve the main baseline, record runnable evidence, and make no compiler or extension fixes. Explicit placement rejections are separated from internal errors; a stale installed extension is separated from the current repository source.

## Work log

- 2026-09-18: `./scripts/doctor` passed. Fast-forwarded main from `6f33edd` to `069a5fb`. Preserved untracked `.task-agent-disabled`.
- 2026-09-18: Created `audit/tt-developer-383`; the default `codex/` prefix is blocked by an existing branch named `codex`.
- 2026-09-18: Ran full local CI. Rust, native, agents, and extension passed. npm dependency resolution and website preview were blocked by sandbox networking/listen restrictions; reran only npm and website outside the sandbox, and both passed.
- 2026-09-18: Ran actual VS Code suites against the current tt client/server and installed native extension. Basic, filesystem, ownership, and pattern suites passed; native-only diagnostics exposed the installed bundle mismatch described in E1.
- 2026-09-18: Ran 1,815 compilation combinations both in a temporary directory and under the repository toolchain. Both produced the same failure totals. Reduced three failure families to individual inputs.
- 2026-09-18: Reproduced declaration collisions, source replacement, missed watch inputs, missing declarations for new files, and generated-input watch errors.
- 2026-09-18: Confirmed F9 with a config-event control; ran the corrected runtime matrix with 1,011 accepted programs and no evaluation-trace failures.
- 2026-09-18: Added reproducible audit scripts under [evidence/TASK-383](./evidence/TASK-383/README.md). Discarded initial runtime runs whose harness omitted the emitted runtime import location, then corrected the harness.

- 2026-09-18: The user expanded the audit to complex production-style examples: asynchronous service logic, generic inference, mixed-source JSX modules, and incremental builds.
- 2026-09-18: Built and ran async services, React 19 SSR through Vite, monorepo imports, type-only dependency invalidation, and positive/negative inference controls. Added F10–F16 and confirmed F12/F13 in actual VS Code.
- 2026-09-18: Built the current release compiler and distinguished the release parse error from the debug-only assertion in F16.
- 2026-09-18: Final durable-fixture verification exposed F17: explicit source-extension include patterns silently exclude typed modules. Confirmed both extensions against directory/glob controls and native content mappers.

## Issues and resolutions

### F1 — P1: A pipeline followed by a match aborts compilation

```tt
const values = [(1 |> ((x: number) => x + 1)), match (1) { 1 => 1, _ => 0 }];
```

- **Reproduction**: Save as `case.tt`; run `target/debug/ttc -p --no-banner case.tt`.
- **Symptom**: Exit 101, `ValueReadBeforeItIsProduced`. No TypeScript is emitted. The same failure family appears in 679 matrix cases.
- **Cause evidence**: `src/evaluation_ir/planning.rs:66` resolves an earlier expression to a value slot; `src/evaluation_ir/validation.rs:160` skips non-statement-region values, while `:199` checks that a referenced slot has been produced. Inline and statement-region values do not agree on the slot production contract. This identifies the violated compiler boundary; a repair still needs to model the mixed schedule correctly.
- **Resolution**: Unfixed. Captured in the composition probe and reduced input above.

### F2 — P1: Sibling calls containing matches abort compilation

```tt
const value = Number(match (1) { 1 => 1, _ => 0 })
            + Number(match (2) { 2 => 2, _ => 0 });
```

- **Reproduction**: Compile with `ttc -p --no-banner`.
- **Symptom**: Exit 101, `EvaluationCountChanged`. The matrix contains 32 examples of this family.
- **Cause evidence**: `src/evaluation_ir/validation.rs:249` rejects overlapping materialized captures. In this input, the complete left call capture overlaps its already captured `Number` callee. The schedule does not express the already evaluated portion of a compound sibling.
- **Resolution**: Unfixed. `compiler-and-watch.py` preserves the exact stderr and source.

### F3 — P1: Flow/match and JSX compositions fail contextual projection

```tt
const value = [(flow |> ((x: number) => x + 1))(1), match (1) { 1 => 1, _ => 0 }];
```

```ttx
const view = <main a={(1 |> ((x: number) => x + 1))}>
  {match (1) { 1 => 1, _ => 0 }}
</main>;
```

- **Reproduction**: Compile each source with its corresponding `.tt` or `.ttx` extension.
- **Symptom**: Exit 1 with `error[other]: contextual projection lost a successfully lowered module`; no output. The matrix contains 256 failures with this message.
- **Cause evidence**: `src/typescript/contextual.rs:246` recompiles with `defer_to_checker: true` and imports unchanged, then discards the underlying diagnostic if emission fails. Initial lowering and contextual analysis projection disagree. This may share a root cause with F1; it is not counted as 256 distinct defects.
- **Resolution**: Unfixed. The reduced inputs are recorded by `compiler-and-watch.py`.

### F4 — P1: Relative declaration inputs flatten directories and overwrite names

Fixture:

```text
src/a/model.tt  -> export const value: number = 1;
src/b/model.tt  -> export const value: string = "hello";
```

Run from the fixture root:

```sh
ttc --types src -o types
```

- **Symptom**: Exit 0 and only `types/model.tt.d.ts` exists, containing the string declaration. Both expected subdirectories are missing; one module silently overwrites the other.
- **Cause**: The engine supplies canonical absolute source paths. `src/main/output.rs:258` compares these against the original relative directory `src`; `strip_prefix` fails and the function returns only the basename. Named files also fall back to the basename.
- **Impact**: Broken declaration imports and incorrect public types; equal basenames lose one declaration and its map.
- **Resolution**: Unfixed. `cli.py` records the output tree and file contents.

### F5 — P1: Compiling a named tt file overwrites a hand-written sibling ts file

```text
src/a.tt -> export const value = 1;
src/a.ts -> // authored source\nexport const original = 2;
```

```sh
ttc src/a.tt
```

- **Symptom**: Exit 0; `src/a.ts` is replaced with generated output, losing the authored contents. This probe operates only on synthetic files under `target/audit-383`.
- **Cause**: Output collision checking only covers the selected jobs (`src/main/build.rs:239`). The later protection (`:356`) rejects a source writing onto itself, but a sibling `.ts` not named as an input is not protected.
- **Resolution**: Unfixed. `cli.py` preserves the post-build contents.

### F6 — P1: Typed watch ignores TypeScript dependencies and configuration changes

Start with `src/dep.ts` exporting a string and `src/main.tt` assigning that import to a string. Run:

```sh
ttc --check-types --watch src
```

Change the exported type in `dep.ts` to number, without touching `.tt` files.

- **Symptom**: Watch remains at `0 reported`; a fresh `ttc --check-types src` immediately reports TS2322. Independently, adding `noUnusedLocals: true` to `tsconfig.json` also leaves watch unchanged, while a fresh check reports TS6133.
- **Cause**: `src/main/typed.rs:168` watches `project.scan()` timestamps; `src/engine/project.rs:161` scans only `.tt` and `.ttx`. Host TypeScript sources and configuration are absent from the change set.
- **Impact**: An apparently passing watch process can conceal newly introduced type errors. Declaration watch uses the same trigger loop.
- **Resolution**: Unfixed. `cli.py` records both watch logs and fresh-check diagnostics.

### F7 — P2: Declaration watch never emits newly created tt files

Start with only `src/a.tt`, run `ttc --types --watch /absolute/path/to/src -o types`, then create `src/b.tt` exporting `newValue`.

- **Symptom**: Watch reports two files and zero errors, but only `types/a.tt.d.ts` exists. There is no declaration for `b.tt`. An absolute source directory deliberately avoids F4.
- **Cause**: `Project.requested` is fixed from the initial collection (`src/engine/project.rs:114`). Subsequent scans discover new files but declaration matching still filters by that original set (`:434`).
- **Resolution**: Unfixed. `compiler-and-watch.py` records the log and output tree.

### F8 — P2: In-place build watch treats its own output as new source

Create `src/a.tt`, then run `ttc --watch src` without `-o`.

- **Symptom**: The first round succeeds. The next round sees generated `src/a.ts` and reports `output would overwrite the input`. Editing `a.tt` produces another successful tt build followed by the same generated-input failure.
- **Cause**: `src/main/output.rs:191` rescans with TypeScript included; output exclusion exists only for an explicit output directory. The watcher has no ownership record for adjacent generated files.
- **Resolution**: Unfixed. `compiler-and-watch.py` captures the alternating success/error log.

### F9 — P1: Editor diagnostics remain stale after external mts/cts edits

For each `.tt`/`.ttx` consumer, import a string from an unopened `.mts`/`.cts` dependency and use it as a string. Confirm hover resolves to string, then replace the dependency on disk with a number export.

- **Symptom**: All four host/consumer combinations keep zero diagnostics after the dependency changes. Touching `tsconfig.json` as a control immediately produces TS2322 and TS2339 in all four cases, confirming that the missing filesystem event is the trigger defect.
- **Cause**: `editors/vscode/client/src/extension.ts:35` watches only `tt,ttx,ts,tsx,json`; `mts` and `cts` are missing even though they are recognized TypeScript source extensions by the engine.
- **Impact**: Branch changes, generators, or another editor can change a dependency without updating the consuming tt editor's errors.
- **Resolution**: Unfixed. The isolated actual-editor reproduction is `run-editor.mjs` plus `editor.cjs`.

### F10 — P1: Vite resolves a workspace package tt import relative to the importer

```ts
import { value } from "@acme/domain/model.tt";
```

- **Production scenario**: A shared workspace package exposes `model.tt` through its package `exports`, and a Vite application bundles the package with `ssr.noExternal: true`.
- **Symptom**: Build fails looking for `apps/web/src/@acme/domain/model.tt` instead of the package's real file. An otherwise identical package exporting `model.ts` builds successfully.
- **Cause**: `integrations/unplugin/index.js:139–144` treats every non-absolute `.tt`/`.ttx` specifier as a relative filesystem path. It never delegates bare package resolution to Vite/Rollup. This is a package-import integration gap; relative source imports remain the documented primary workflow.
- **Resolution**: Unfixed. `bundler.mjs` creates a real package manifest with exports and runs the failing build plus the `.ts` control.

### F11 — P1: Vite keeps stale match code after a type-only variant dependency changes

```tt
import type { State } from "./model.tt";
export function render(state: State) {
  return match (state) { Ready(value) => value, Empty => 0 };
}
```

- **Production scenario**: A UI module consumes a shared discriminated state through a type-only import. The state gains `Loading` while the Vite development server is running.
- **Symptom**: The watcher observes the dependency change, but transforming the consumer returns the identical cached code without a diagnostic. Calling that cached function with `{kind: "Loading"}` throws `tt match: unexpected case`. A fresh `ttc -p` correctly rejects the consumer as non-exhaustive.
- **Cause**: The plugin's `load` registers only the source file (`integrations/unplugin/index.js:168`). Compilation depends on imported variant declarations, but the type-only import is erased from JavaScript and therefore does not create a Vite runtime dependency edge. Compiler-only dependencies are not registered as watched dependencies of the consumer.
- **Resolution**: Unfixed. `bundler.mjs` records the actual watcher event, unchanged output, runtime exception, and fresh compiler diagnostic. This is not a missed operating-system filesystem event.

### F12 — P1: Exhaustive JSX matches receive false non-exhaustiveness errors

```ttx
type State = {kind: "A"} | {kind: "B"} | {kind: "C"};
declare const state: State;
const view = match (state) {
  A => <p>A</p>, B => <p>B</p>, C => <p>C</p>
};
```

- **Production scenario**: A React cart renders loading, failure, and loaded branches. All three branches exist.
- **Symptom**: The real cart bundles and server-renders correctly, but `ttc --check-types` reports the present `Failed` branch as missing. The reduced example reports the present `B` branch as missing. Actual VS Code shows the same false `match-not-exhaustive` error. Fresh release and debug compilers both reproduce it.
- **Cause**: The typed coverage path reparses the source with `parser::parse(source)` (`src/analysis/coverage.rs:50`), losing the `.ttx` source kind. That TypeScript-mode parse does not preserve JSX arm boundaries even though the main TSX lowering does.
- **Resolution**: Unfixed. `react-and-library.mjs`, `run-editor-production.mjs`, and `fixtures/jsx-coverage.ttx` preserve the application and reduced reproduction.

### F13 — P1: Generic variants declared in ttx emit ambiguous TSX arrows

```ttx
export variant Box<T> { Value(value: T), Empty }
```

- **Production scenario**: A React component colocates a generic loading/result state with its view.
- **Symptom**: Compilation and typed checking fail with `verify-failed`; VS Code displays the same error. `--no-verify` exposes the invalid generated constructor `Value: <T>(value: T): Box<T> => ...`. Both release and debug builds reproduce the failure.
- **Cause**: `src/codegen/core/emitter/helpers.rs:318–320` inserts `adt.generics` unchanged in an arrow's type-parameter list. An unconstrained single type parameter needs TSX disambiguation, such as `<T,>`; the emitter does not carry that distinction.
- **Resolution**: Unfixed. `fixtures/jsx-generic.ttx` reproduces it without React or any external dependency. Equivalent `.tt` generic variants passed the comparison matrix.

### F14 — P1: Nested React row calculations abort the release compiler

- **Production scenario**: In the `Loaded` arm of the React cart, `items.map` renders rows. A title uses a pipeline, while a price cell computes `Number(match(item.quantity){...}) + Number(match(item.price){...})`.
- **Symptom**: Both `--check-types` and Vite compilation abort with exit 101, `validate_source_preservation / SourceOmitted`. The fresh release binary also aborts. The reported missing source range starts at the inner `item.quantity` scrutinee. The cart without these two match calculations server-renders successfully (although F12 still affects its typed diagnostic).
- **Cause evidence**: The source-preservation invariant catches original expression bytes omitted by nested lowering (`src/codegen/rope.rs:413`, `:494`). This identifies a lowering failure, not invalid React input. The exact schedule/emitter repair has not been determined; it must not be replaced by disabling verification.
- **Resolution**: Unfixed. `react-and-library.mjs` preserves both full cart versions. This is a distinct observed failure from F2's evaluation-count rejection and may share underlying scheduling defects.

### F15 — P1: Typed CLI cannot resolve a shared tt module outside the app config root

```text
apps/web/tsconfig.json
apps/web/src/main.tt
packages/domain/src/models.tt
```

The application imports the real `../../../packages/domain/src/models.tt` and `apps/web/tsconfig.json` includes `src`.

- **Symptom**: `ttc --check-types src` from `apps/web` reports TS2307 for the existing module. Plain `ttc -p` compiles the same consumer. An equivalent `.ts` dependency passes the typed CLI. The official `tsc --runExternalCode` content-mapper configuration also passes with the original external `.tt` source.
- **Cause**: `src/engine/mod.rs:178–185` populates the layered `.tt` filesystem by scanning the chosen config root and explicitly named files. It does not discover the imported sibling package outside that root, so the typed CLI backend never receives its projection.
- **Resolution**: Unfixed. `monorepo.mjs` records all four paths with `allowImportingTsExtensions` enabled and removes the generated `.ts` control before the native mapper check, avoiding a stale sibling-file explanation.

### F16 — P1: JSX-list expression arms fail parsing in production and assert in debug

```ttx
const view = match (ready) {
  true => <p>{items.map(item => <p>{Number(item.quantity) + Number(item.price)}</p>)}</p>,
  false => <p>Empty</p>
};
```

- **Production scenario**: A state branch directly returns a JSX list instead of wrapping the arm in a statement block.
- **Symptom**: The fresh release compiler rejects this valid shape with `source-not-typescript: Expression expected`. Debug compilation exits 101 at `vendor/swc_ecma_parser/src/parser/expr.rs:2573` (`items.len()` is 2, expected 1). Replacing only the outer `match` with a normal TypeScript conditional compiles successfully. The inner expressions need no tt syntax to reproduce this failure.
- **Cause evidence**: Nested JSX and arrow expressions are not preserved through this match projection/parsing path. The assertion location identifies the failing parser state; the exact upstream projection defect remains to be traced. The debug-only assertion must not be described as the release behavior.
- **Resolution**: Unfixed. `fixtures/jsx-nested-arrow.ttx` and `fixtures/jsx-nested-arrow-oracle.ttx` preserve the paired reproduction.

### F17 — P1: Explicit tt/ttx include patterns silently skip type checking

```json
{"compilerOptions":{"strict":true,"noEmit":true},"include":["src/**/*.tt"]}
```

```tt
export const count: number = "wrong";
```

- **Production scenario**: CI narrows a package's tsconfig include patterns to `.tt` or `.ttx` sources.
- **Symptom**: Fresh release `ttc --check-types src/main.tt` exits 0 without diagnostics. Both `src/**/*.tt` and the exact `src/main.tt` include fail silently; `.ttx` equivalents do the same. Changing only the include to `src` or the corresponding generated `*.ts`/`*.tsx` extension reports TS2322. The official native content-mapper check correctly reports TS2322 using the original `.tt`/`.ttx` include patterns.
- **Cause**: `src/engine/projection.rs:176` serves files as `main.tt.ts` / `main.ttx.tsx`. `src/typescript/host.mjs:237–261` opens the unchanged config and admits only modules in that configured program. Original source-extension include patterns do not match the virtual names; even a file explicitly passed on the command line is omitted from the typed program, and the CLI reports success.
- **Impact**: A CI type-check job can stay green while checking none of the requested tt modules. This is more severe than a visible missing-module error.
- **Resolution**: Unfixed. `include-patterns.mjs` compares four include forms for each source extension and the official native mapper. The issue was found while validating a durable JSX reproduction, whose config now uses `jsx-coverage.*` so it actually reaches the intended typed check.

### E1 — Installed native extension does not include main's diagnostic-focus fix

- **Symptom**: The native-only diagnostic suite passed 3/4 cases. `ts -> tsx: discard refreshes the newly active consumer` retained TS2322 and TS2339 after the dependency buffer was reverted and closed.
- **Cause evidence**: The installed `dist/extension.bundle.js` sets `diagnosticPullOptions.onChange`, `onSave`, and `onTabs`, but omits `onFocus`. Current `npm/patches/typescript-content-mapper-ownership.patch:74` already adds `onFocus: true`.
- **Resolution**: No extension was installed or modified. This is an observed local-artifact mismatch, not a newly confirmed defect in the current patch. The newly built native VSIX was not validated by this audit.

## Verification

- [x] `./scripts/doctor`
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: 1,235 passed, including integration and native tests.
- [x] `./scripts/ci`: agents, rust, native, extension passed on the initial run; npm and website passed on the retry outside the sandbox, `./scripts/ci npm website`.
- [x] Extension client/server tests: 191 passed, zero skipped.
- [x] Actual VS Code basic suite: 71/71; filesystem: 11/11; ownership: 6/6; patterns: 8/8.
- [x] Native-only actual VS Code diagnostic suite: 3/4; the single observed failure is E1.
- [x] Composition matrix: 1,815 cases; 776 accepted, 711 exit-101 internal errors, 256 contextual projection failures, 72 explicit `match-placement` rejections. The 72 explicit rejections are not counted as new confirmed defects.
- [x] Targeted compiler and CLI probes reproduced F1–F8.
- [x] Production services: 5/5 passed type checking, compilation, and exact runtime output.
- [x] Positive contextual-inference controls: 24/24 passed (12 hosts, paired TypeScript/tt forms). Negative controls: 24/24 rejected nonexistent callback properties with TS2339.
- [x] React/Vite application, package-resolution control, hot-reload cache/runtime check, out-of-root monorepo comparison, and relocated emitted library checked. The emitted library type-checks without its original source tree.
- [x] `cargo build --release`; F12, F13, F14, and F16 compiler behavior checked as described above.
- [x] Actual VS Code reproduced the false JSX coverage error and generic-variant emission error.
- [x] Include-pattern controls: `.tt` and `.ttx` each silently skipped under glob/exact source-extension patterns; directory/generated-extension controls and both official native-mapper checks reported TS2322.
- [x] Durable reduced fixtures verified with the release binary after correcting the fixture include pattern; the nested-arrow TypeScript oracle passes.
- [x] Controlled editor probe: all four `.mts`/`.cts` to `.tt`/`.ttx` combinations missed the change and then reported TS2322/TS2339 after a config filesystem event.
- [x] Corrected runtime matrix: 1,011 accepted programs executed; zero trace mismatches or execution failures. Rejected programs are excluded from this number.
- [x] Reproduction JavaScript syntax checked with `node --check`; Python scripts parsed with `ast.parse`.
- [x] `node scripts/check-task-index` and `git diff --check`.

## Result

Seventeen reproducible failure families are recorded, including eight from the production-style expansion, plus one installed-artifact mismatch. Compiler and editor implementation files are unchanged.

Changed files: this task record, `docs/tasks/INDEX.md`, and the reproduction scripts/readme in `docs/tasks/evidence/TASK-383/`.

Limitations: one macOS arm64 environment; no cross-platform execution, no fresh native-VSIX build, and no exhaustive guarantee over all valid TypeScript or tt programs. Runtime probes validate evaluation traces for accepted programs, not every type-inference or runtime semantic property. Existing CI passing does not cover the failing combinations above.
