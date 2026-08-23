# tt

[Website](https://load28.github.io/tt/) · [English](./README.md) · [한국어](./README.ko.md)

tt is a small language that adds expressive data and control-flow features to TypeScript, then compiles back to plain TypeScript.

> [!WARNING]
> **Early development:** tt is not yet recommended for production use. APIs and language behavior may change between releases.

```tt
export enum Shape {
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

Install the prebuilt compiler from npm. Rust is not required on supported platforms.

```sh
bunx @load28/create-tt@latest my-app
bunx @load28/create-tt@latest init       # in an existing TypeScript project
```

The automatic installer uses Bun for new projects. The complete automatic and
manual paths are in the [installation guide](./docs/getting-started.md).

For a manual compiler-only install:

```sh
bun add -d @load28/tt-lang typescript@7
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

- Model data with Rust-style `enum` declarations and unpack it with exhaustive `match` expressions, including guards, tuples, literals, or-patterns, and nested patterns.
- Work with `TOption` and `TResult` through `try`, `let-else`, `if let`, and `result` blocks. Types come from `@tt/std`; tree-shakeable operations use `@tt/std/option` and `@tt/std/result`.
- Build value and function pipelines with `|>` and `flow`.
- Mark bindings and parameters with `val` when mutation through them must be rejected.

The rest of the file is ordinary TypeScript. Existing TypeScript types, modules, tooling, and runtime behavior remain the foundation.

## Develop tt

The compiler is a Rust crate with a small public API and an `ttc` CLI. Rust 1.88 or newer is required. Node.js and TypeScript are needed for the full integration suite.

```sh
git clone https://github.com/load28/tt.git
cd tt
cargo build
cargo test
```

Before submitting a change, run the repository gates:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For local compiler, TypeScript, and VS Code extension setup, run `./scripts/setup --tsgo-npm`. Contribution workflow and project rules are in [CONTRIBUTING.md](./CONTRIBUTING.md).

The compiler can also be embedded as a Rust library:

```rust
use ttc::{compile, Options};

let typescript = compile(source, &Options::default())?;
```

Architecture records live in [`docs/design`](./docs/design).

## License

[MIT](./LICENSE)
