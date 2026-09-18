# TASK-383 reproduction artifacts

Run from the repository root after `cargo build` and `npm --prefix editors/vscode run compile`.
The Python probes create disposable fixtures under `target/audit-383`; the source-overwrite probe only overwrites its own synthetic fixture.
Run each Python probe once per clean fixture directory. For another run, rename or remove only that probe's generated fixture directory first.

```sh
python3 docs/tasks/evidence/TASK-383/cli.py
python3 docs/tasks/evidence/TASK-383/compiler-and-watch.py
node docs/tasks/evidence/TASK-383/composition.mjs
node docs/tasks/evidence/TASK-383/runtime.mjs
VSCODE_TYPESCRIPT_EXTENSION="$HOME/.vscode/extensions/typescriptteam.native-preview-0.20260826.1" node docs/tasks/evidence/TASK-383/run-editor.mjs
```

The editor reproduction uses the actual VS Code executable (`code`, or `VSCODE_EXECUTABLE`) in an isolated profile. It intentionally exits unsuccessfully when external `.mts`/`.cts` edits fail to update diagnostics. Each result records diagnostics before the edit, after the edit, and after touching `tsconfig.json` as a control. It does not install or replace any extensions.

The composition matrix is a compiler acceptance probe, not a claim that every rejected placement must be supported. The runtime matrix checks evaluation traces only for cases that compile. Its runtime module is materialized at the output import location; initial exploratory runs without this module were discarded.

## Production-style expansion

```sh
node docs/tasks/evidence/TASK-383/production.mjs
node docs/tasks/evidence/TASK-383/bundler.mjs
node docs/tasks/evidence/TASK-383/react-and-library.mjs
node docs/tasks/evidence/TASK-383/monorepo.mjs
node docs/tasks/evidence/TASK-383/include-patterns.mjs
VSCODE_TYPESCRIPT_EXTENSION="$HOME/.vscode/extensions/typescriptteam.native-preview-0.20260826.1" node docs/tasks/evidence/TASK-383/run-editor-production.mjs
```

The Vite and React probes use the existing dependencies in `website/node_modules` and `integrations/unplugin/node_modules`. They create isolated application directories under `target/audit-383` and link the existing React packages; they do not install dependencies or change the real website. Vite runs in middleware mode with HMR networking disabled; its watcher uses polling so the observed filesystem event is explicit. The cached consumer is evaluated after a variant change to demonstrate the missed recompile's runtime consequence.

`production.mjs` covers asynchronous orders, sequential/parallel failures, resource disposal, finally overriding a result, short-circuiting, generator batches, and twelve contextual-inference hosts with plain-TypeScript controls. To reproduce the negative inference check, replace `input.id` with `input.missing` in each generated inference fixture and run `ttc --check-types src` there; all 24 variants must report TS2339. Restore the source after checking.

`react-and-library.mjs` builds and server-renders a React 19 cart through Vite, then adds nested match calculations. It also verifies a generated library after temporarily moving its original source folder out of the module resolution path. `monorepo.mjs` compares an out-of-project `.tt` import, an equivalent `.ts` import, and the official native content-mapper check.

Reduced JSX inputs are in `fixtures/`. For the coverage case, run `ttc --check-types --project docs/tasks/evidence/TASK-383/fixtures/tsconfig.json docs/tasks/evidence/TASK-383/fixtures/jsx-coverage.ttx`. Compile the other fixtures with `ttc -p <file>`. The nested-arrow oracle replaces the outer tt match with a TypeScript conditional. Use a freshly built `target/release/ttc` to distinguish production errors from debug assertions.

`include-patterns.mjs` requires `cargo build --release`. It compares directory, explicit source-extension, and generated-extension patterns for both `.tt` and `.ttx`, then checks the same incorrect assignment through the official native content mapper.
