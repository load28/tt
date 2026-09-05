# tt Language — VS Code extension

Language support for `.tt` and `.ttx`. The client starts an LSP server; the
server delegates language semantics to the project's `ttc` engine. TypeScript
checks use the TypeScript installation resolved from that project.

## Installation and mixed-source projects

Install the compiler and the repository-pinned TypeScript in your project:

```sh
npm i -D @openload28/tt-lang typescript@7.1.0-dev.20260826.1
```

Install the matching tt language VSIX from the release. For `.ts` and `.tsx`
consumers of tt modules, also install the release's platform-specific TypeScript
7.1 VSIX with content-mapper support. Its extension ID is
`TypeScriptTeam.native-preview`. Use a compiler and extension from the same
release; an older installed extension does not pick up edits to this checkout.

For the preview-based editor setup, enable both settings:

```json
{
  "js/ts.experimental.useTsgo": true,
  "typescript.experimental.useTsgo": true
}
```

Declare `contentMappers` at the top level of `tsconfig.json`, not inside
`compilerOptions`:

```json
{
  "compilerOptions": {
    "strict": true,
    "noEmit": true
  },
  "contentMappers": [
    { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }
  ],
  "include": ["src"]
}
```

For command-line checking, run `npx tsc --runExternalCode`. The editor registers
the tt content-mapper contribution when the TypeScript extension exposes that
API. Mapped modules stay virtual; new TypeScript 7.1 projects do not need
declaration sidecars or generated-source `paths`/`rootDirs` wiring.

Legacy TypeScript hosts without content mappers can use declaration output from
`ttc --types -w src` and configure their resolver for that output. The extension's
save-time sidecar refresh is a separate compatibility feature, not a replacement
for the content-mapper setup.

## Language features

- TypeScript/TSX-derived grammars, tt semantic tokens, Markdown/MDX tt fences,
  and file icons supported by the active icon theme.
- Source-located tt and TypeScript diagnostics, including typed exhaustiveness
  and `val` checks. Quick fixes carry compiler-authored suggestions.
- Completion for tt patterns, constructors, snippets, and TypeScript members;
  completion-item details, hover, signature help, references, and definitions.
- Rename for supported symbols and document symbols. Import aliases follow
  TypeScript rename semantics; a local rename need not rename the exported symbol.
  Unsafe or unsupported rename targets can be rejected.

Incomplete member expressions can use a compiler-owned completion probe. Such
probes answer completion requests; they are not the source used for diagnostics.
Recovery and source mappings belong to the compiler, not to a second parser in
the editor adapter. See [the LSP architecture](../../docs/design/lsp-architecture.md).

### Diagnostics and project state

The tt server combines syntax checks, language-service results, typed compiler
diagnostics, and hints before publishing a complete validation generation. Old
generations are discarded. Structured compiler results can use `source: ttc`
and codes such as `ts2322`; consumers must not assume that every TypeScript-derived
diagnostic has `source: ts` or a numeric code.

Open `.tt`, `.ttx`, `.ts`, and `.tsx` buffers are synchronized into the tt engine.
TypeScript retains ownership of `.ts`/`.tsx` editor providers. A dependency edit
or close schedules revalidation of open tt consumers. Host overlays are frozen
with typed snapshots and served at their authored paths for language queries.

Source/configuration filesystem events reload cached projects and replay open
buffers before subsequent checks. This covers external edits and module
creation/deletion without requiring an edit in each consumer. The replay does
not save user buffers. Support files under `node_modules` and Git metadata are
not treated as user source events.

The tt adapter currently revalidates all open tt documents conservatively. This
is a correctness policy, not a measured large-project latency guarantee.

### Toolchain resolution and unavailable features

The compiler is resolved in this order:

1. `tt.compilerPath`, when explicitly configured.
2. The newest workspace `target/release/ttc` or `target/debug/ttc` build.
3. The workspace's `@openload28/tt-lang` package via its `binaryPath()` API.
4. `ttc` on PATH.

An explicit path or a workspace development build can differ from `npx ttc`.
Check the selected compiler when diagnosing editor/CLI disagreements.

The engine resolves the TypeScript API and native executable through the
project's `node_modules`, walking upward. Neither compiler is bundled in the tt
language extension. Missing tools limit semantic features; an empty completion
list or unavailable typed checker is not evidence that a program is correct.
Inspect the **tt Language Server** output channel for failures. Grammar-based
highlighting does not require a working compiler.

## Settings

| Setting | Default | Effect |
| --- | --- | --- |
| `tt.compilerPath` | `""` | Explicit compiler executable; otherwise use the resolution order above. |
| `tt.verify` | `true` | Verify emitted syntax during syntax checks. |
| `tt.typeDiagnostics` | `true` | Include TypeScript type diagnostics for tt documents. |
| `tt.typedChecks` | `true` | Include type-dependent tt checks. |
| `tt.sidecar` | `"refresh"` | On save, refresh existing sidecars; `always` creates them, `off` disables refresh. |
| `tt.sidecarDir` | `""` | Sidecar directory relative to the workspace; empty means adjacent to sources. |
| `tt.trace.server` | `"off"` | LSP trace level. |

## Development and verification

From the repository root, run `./scripts/doctor` first. Build the compiler with
`cargo build`; then use these commands in `editors/vscode`:

```sh
npm ci
npm run compile
npm test
npm run test:editor
```

`npm test` runs unit and real LSP tests. `npm run test:editor` launches the local
`code` executable in an isolated profile. `VSCODE_EXECUTABLE` can select another
executable. No user extension is reinstalled and no normal profile setting is
changed. Missing or incomplete reports fail rather than silently skip tests.

The default editor matrix covers all four dependency extensions with tt/ttx
consumers: 8 directed edges and 32 checks. Set `VSCODE_TYPESCRIPT_EXTENSION` to a
locally available TypeScript 7.1 extension directory to include ts/tsx consumers:
16 edges and 64 checks. The runner loads it as a second development extension;
it never downloads an extension automatically.

Each edge exercises activation, complete/incomplete completion, definitions,
application of rename edits, source-located diagnostics, unsaved dependency
changes, changed member types, and discarded edits. The tt fixtures contain a
variant and match; JSX fixtures contain JSX.

Set `TT_EDITOR_TEST_SUITE=filesystem` for the separate external-edit,
create/delete/recreate, and tsconfig-change suite. Profiles, fixtures, and JSON
reports are retained under `target/editor-tests/` in the repository.

Use `./scripts/ci` at the repository root for the product gate. Packaging uses
`vsce package --no-dependencies`; `.vscodeignore` excludes test sources and
selects the compiled client/server and required LSP runtimes. Installation with
`./scripts/setup` is a separate, explicit operation, not a prerequisite for
testing the development extension.

## Validation boundaries

Passing these matrices does not establish correctness for every program,
multi-root layout, third-party extension combination, or large workspace.
Known contextual-typing failures for scoped/sibling match continuations remain
tracked in [TASK-324](../../docs/tasks/TASK-324-scoped-contextual-continuations.md).
Editor diagnostics can expose those compiler limitations; they are not hidden
to make the editor appear clean.
