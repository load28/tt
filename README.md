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

## Get started

### New project

```sh
bunx @openload28/create-tt@next my-app
cd my-app
bun run dev
```

### Existing TypeScript project

```sh
cd existing-project
bunx @openload28/create-tt@next init
bun run tt:check
```

See the [installation guide](./docs/getting-started.md) for automatic and
bundler-specific manual setup.

### VS Code extension

Install `tt-language-<version>.vsix` from the latest pre-release on
[GitHub Releases](https://github.com/load28/tt/releases):

```sh
code --install-extension ./tt-language-<version>.vsix
code --install-extension ./tt-typescript-preview-<version>-<platform>.vsix
```

After installing both VSIX files, enable TypeScript 7 in the editor:

```jsonc
// .vscode/settings.json (or user settings)
"js/ts.experimental.useTsgo": true,
"typescript.experimental.useTsgo": true
```

### Use the CLI

```sh
bun add -d @openload28/tt-lang@next typescript@7.1.0-dev.20260826.1
bunx ttc -o build src        # emit TypeScript
bunx ttc --check src         # check tt
bunx ttc --check-types src   # check tt and TypeScript
```

- Output: `.tt` → `.ts`, `.ttx` → `.tsx`
- Bundlers: [`@openload28/unplugin-tt`](./integrations/unplugin) for Vite, Rollup, webpack, Rspack, esbuild, and Farm
- Help: `ttc --help`, `ttc help <topic>`

## The language at a glance

- Model data with Rust-style `variant` declarations and unpack it with exhaustive `match` expressions, including guards, tuples, literals, or-patterns, nested patterns, and `is Error { message }` class patterns over open JavaScript hierarchies. TypeScript `enum` declarations remain ordinary TypeScript.
- Work with `TOption` and `TResult` through `try`, `let-else`, `if let`, and `result` blocks. Types come from `@tt/std`; tree-shakeable operations use `@tt/std/option` and `@tt/std/result`.
- Build value and function pipelines with `|>` and `flow`, including JavaScript-style optional postfix steps such as `value |> ?.name`.
- Mark bindings and parameters with `val` when mutation through them must be rejected.

The rest of the file is ordinary TypeScript. Existing TypeScript types, modules, tooling, and runtime behavior remain the foundation.

For the reasoning behind the language, read [Why I built tt](./docs/why-tt.md).

## Develop tt

The required tools are Rust 1.98, Node.js, and Bun.

```sh
git clone https://github.com/load28/tt.git
cd tt
npm ci
./scripts/setup
./scripts/ci
```

- Contribution workflow: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Architecture: [`docs/design`](./docs/design)

The compiler can also be embedded as a Rust library:

```rust
use ttc::{compile, Options};

let typescript = compile(source, &Options::default())?;
```

## License

[MIT](./LICENSE)
