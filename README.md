# tt

[Website](https://load28.github.io/tt/) · [English](./README.md) · [한국어](./README.ko.md)

tt is a small language that adds expressive data and control-flow features to TypeScript, then compiles back to plain TypeScript.

> [!WARNING]
> **Early development:** tt is not yet recommended for production use. APIs and language behavior may change between releases.

```tt
export variant Shape {
  Circle(radius: number),
  Rectangle(width: number, height: number),
  Point,
}

export const area = (shape: Shape): number =>
  match (shape) {
    Circle(radius) => Math.PI * radius ** 2,
    Rectangle(width, height) => width * height,
    Point => 0,
  };
```

Every valid TypeScript file is also a valid `.tt` file, and every valid TSX file is a valid `.ttx` file. tt only transforms syntax it owns, reports language errors such as non-exhaustive matches itself, and emits readable TypeScript or TSX without a runtime dependency.

## Install and use

Install TT's official development packages from npm. Rust is not required on
supported platforms, and neither is a source build of anything: `ttc` uses
the TypeScript your project installed, so TypeScript is the only other
dependency. There is nothing to export and nothing to configure — the
compiler, the editor extension and your build all read the same package.

```sh
bun add -d typescript@7.1.0-dev.20260826.1
```

An exact prerelease on purpose, and the same one this repository tests
against. Two things tt uses arrived in the TypeScript 7.1 line — the
declaration emit API (`ttc --types`, the editor's `.tt.d.ts` sidecars) and
content mappers (`.ts` importing `.tt` with nothing on disk) — and a plain
`typescript@7` resolves to the 7.0 line because npm ranges do not match
prereleases. Naming the version rather than a tag means a nightly published
tonight cannot change how your build behaves tomorrow. When 7.1 is
released this moves to `typescript@7`, everywhere at once.

```sh
bunx @load28/create-tt@0.3.0 my-app
bunx @load28/create-tt@0.3.0 init       # in an existing TypeScript project
```

The automatic installer uses Bun for new projects. The complete automatic and
manual paths are in the [installation guide](./docs/getting-started.md).

The development VS Code extension is not published to the Marketplace.
Download `tt-language-<version>.vsix` from the newest pre-release on
[GitHub Releases](https://github.com/load28/tt/releases), then run
**Extensions: Install from VSIX...** from the Command Palette or:

```sh
code --install-extension ./tt-language-<version>.vsix
```

For a manual compiler-only install:

```sh
bun add -d @load28/tt-lang@0.3.0 typescript@7.1.0-dev.20260826.1
```

Compile a file or source tree, or check it without writing output:

```sh
bunx ttc -o build src
bunx ttc --check src
bunx ttc --check-types src
```

`ttc` emits `.ts` from `.tt` and `.tsx` from `.ttx`. JSX is preserved, so React projects keep using their existing `jsx` compiler option and JSX runtime. For direct `.tt` or `.ttx` imports, use [`@load28/unplugin-tt`](./integrations/unplugin) with Vite, Rollup, webpack, Rspack, esbuild, or Farm.

Prebuilt binaries are available for Linux x64/arm64, macOS x64/arm64, and Windows x64. On another platform, build from source:

```sh
cargo install --git https://github.com/load28/tt
```

Run `ttc --help` for compiler options or `ttc help <topic>` for the built-in language guide.

## The language at a glance

- Model data with Rust-style `variant` declarations and unpack it with exhaustive `match` expressions, including guards, tuples, literals, or-patterns, and nested patterns. TypeScript `enum` declarations remain ordinary TypeScript.
- Work with `TOption` and `TResult` through `try`, `let-else`, `if let`, and `result` blocks. Types come from `@tt/std`; tree-shakeable operations use `@tt/std/option` and `@tt/std/result`.
- Build value and function pipelines with `|>` and `flow`, including JavaScript-style optional postfix steps such as `value |> ?.name`.
- Mark bindings and parameters with `val` when mutation through them must be rejected.

The rest of the file is ordinary TypeScript. Existing TypeScript types, modules, tooling, and runtime behavior remain the foundation.

For the reasoning behind the language, read [Why I built tt](./docs/why-tt.md).

## Develop tt

The compiler is a Rust crate with a small public API and an `ttc` CLI. Rust
1.98 or newer is required — the version `rust-toolchain.toml` pins, and the
one every build here is checked with. The complete local environment also
needs Bun and Node.js.

```sh
git clone https://github.com/load28/tt.git
cd tt
npm ci
./scripts/setup
```

`npm ci` installs TypeScript at the version `package.json` pins; the typed
test suites and the editor both read it, the same way a consumer project's
would. `scripts/setup` then builds a release `ttc` and the VS Code
extension; it never updates the Git checkout.

To test exactly what package consumers receive, publish the local TT packages
to an npm-compatible registry:

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

The second command prints the matching `create-tt` bootstrap command. Full
contributor setup details are in [CONTRIBUTING.md](./CONTRIBUTING.md).

Before submitting a change, run the repository gates:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The compiler can also be embedded as a Rust library:

```rust
use ttc::{compile, Options};

let typescript = compile(source, &Options::default())?;
```

Architecture records live in [`docs/design`](./docs/design).

## License

[MIT](./LICENSE)
