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
supported platforms. `ttc` requires a current source build of typescript-go.

```sh
git clone https://github.com/microsoft/typescript-go.git
cd typescript-go
npm ci
mkdir -p built/local
go build -o built/local/tsgo ./cmd/tsgo
npx tsc -b _packages/native-preview
export TTC_TSGO_ROOT="$PWD"
```

Keep `TTC_TSGO_ROOT` in the environment that runs `ttc`, and launch VS Code
from the same shell. The executable and API client must come from the same
checkout.

```sh
bunx @load28/create-tt@next my-app
bunx @load28/create-tt@next init       # in an existing TypeScript project
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

For a manual compiler-only install, first complete the typescript-go build
above and keep `TTC_TSGO_ROOT` exported:

```sh
bun add -d @load28/tt-lang@next
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
- Build value and function pipelines with `|>` and `flow`.
- Mark bindings and parameters with `val` when mutation through them must be rejected.

The rest of the file is ordinary TypeScript. Existing TypeScript types, modules, tooling, and runtime behavior remain the foundation.

For the reasoning behind the language, read [Why I built tt](./docs/why-tt.md).

## Develop tt

The compiler is a Rust crate with a small public API and an `ttc` CLI. Rust
1.98 or newer is required — the version `rust-toolchain.toml` pins, and the
one every build here is checked with. The complete local environment also
needs Bun, Node.js, Go, and a typescript-go checkout.

```sh
git clone https://github.com/load28/tt.git
git clone https://github.com/microsoft/typescript-go.git
cd tt
./scripts/setup --tsgo-root ../typescript-go
```

`scripts/setup` builds the current typescript-go checkout, a release `ttc`,
and the VS Code extension. Later runs reuse `.tt-dev/toolchain.json`; the
script never updates either Git checkout.

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
