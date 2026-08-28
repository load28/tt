# @openload28/tt-lang

**tt** is a tiny preprocessor language that compiles to TypeScript. It adds
Rust-style tagged unions and pattern matching, concise `Option`/`Result`
control flow, pipelines, and mutation-safe bindings while keeping valid
TypeScript valid.

This package installs `ttc`, the tt compiler, as a prebuilt native binary —
no Rust toolchain required.

For a complete Bun + Vite project, or to add tt to an existing TypeScript
project, use the initializer instead:

```sh
bunx @openload28/create-tt@next my-app
bunx @openload28/create-tt@next init
```

`ttc` drives TypeScript 7 and takes it from your project's own
`node_modules` — the same package your build uses, and the only place it
looks. The TT installer does not add it for you:

```sh
bun add -d @openload28/tt-lang@next typescript@7.1.0-dev.20260826.1
```

Declaration output (`ttc --types`, editor `.tt.d.ts` sidecars) and content
mappers (below) use APIs that arrived in TypeScript 7.1; everything else
works on 7.0. See the
[installation guide](https://github.com/load28/tt/blob/main/docs/getting-started.md).

```sh
bunx ttc -o build src/     # compile a source tree to TypeScript
bunx ttc --check src/      # check without writing anything
bunx ttc --types src/      # editor/typecheck declarations
```

On TypeScript 7.1+, `.ts` files can import `.tt` directly — this package
is a TypeScript **content mapper**, so the compiler holds the transform
virtually (no sidecar files). Declare it once in `tsconfig.json` and run
`tsc` with `--runExternalCode`:

```jsonc
"contentMappers": [
  { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }
]
```

Using a bundler? [`@openload28/unplugin-tt`](https://github.com/load28/tt/tree/main/integrations/unplugin)
reads `.tt` files directly in Vite, Rollup, webpack, Rspack, esbuild and
Farm, and finds this package's binary automatically.

For editor support, download `tt-language-<version>.vsix` from the newest
[GitHub Releases](https://github.com/load28/tt/releases) pre-release and use
**Extensions: Install from VSIX...** in VS Code.

## Supported platforms

Prebuilt binaries ship as optional dependencies; npm installs the one
matching your machine.

| Package | OS | CPU |
|---------|----|----|
| `@openload28/tt-lang-linux-x64` | Linux | x64 (static musl build) |
| `@openload28/tt-lang-linux-arm64` | Linux | arm64 (static musl build) |
| `@openload28/tt-lang-darwin-x64` | macOS | x64 |
| `@openload28/tt-lang-darwin-arm64` | macOS | arm64 |
| `@openload28/tt-lang-win32-x64-msvc` | Windows | x64 (MSVC) |

On other platforms, build from source
(`cargo install --git https://github.com/load28/tt`) and set the
`TTC_BINARY` environment variable to the resulting binary.

## API

The package exports one helper for tools that want to spawn the compiler
directly:

```js
const { binaryPath } = require("@openload28/tt-lang");
binaryPath(); // absolute path to the ttc binary for this platform
```

## Local development install

In a checkout of the [tt repository](https://github.com/load28/tt),
`./scripts/setup` stamps this directory for local installs. A project can
then use the work-in-progress compiler like any other dependency:

```sh
pnpm add -D file:/path/to/tt/npm/tt-lang
```

The launcher runs the repository's `target/release/ttc`; TypeScript still
comes from the consuming project, exactly as it does for a published
install. Published installs are unaffected (the stamp file is not committed
or published).

## Documentation

Language reference, CLI options, standard library and error index:
<https://github.com/load28/tt#readme>.

## License

MIT
