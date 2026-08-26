# Install tt

You need [Bun](https://bun.sh/) to run the recommended setup command. TT
packages and the prebuilt `ttc` compiler are installed from npm.

During early development, build the toolchain required by `ttc` from the
current [typescript-go source](https://github.com/microsoft/typescript-go);
TT itself still comes from npm:

```sh
git clone https://github.com/microsoft/typescript-go.git
cd typescript-go
npm ci
mkdir -p built/local
go build -o built/local/tsgo ./cmd/tsgo
npx tsc -b _packages/native-preview
export TTC_TSGO_ROOT="$PWD"
```

Keep `TTC_TSGO_ROOT` in every shell that runs `ttc`. Launch VS Code from the
same shell when using TT editor services. The executable and API client must
come from the same checkout because their protocol is not version-negotiated.

## Automatic setup

Create a Vite + TypeScript project with a starter `.tt` module:

```sh
bunx @load28/create-tt@next my-app
cd my-app
bun run dev
```

Add tt to an existing TypeScript project:

```sh
cd existing-project
bunx @load28/create-tt@next init
bun run tt:check
```

`init` detects Vite, Rollup, Rolldown, webpack, Rspack, esbuild, and Farm from
`package.json`. The `@next` initializer adds the `dev` channel of
`@load28/tt-lang` and `@load28/unplugin-tt`, plus TT scripts. It does not add
an npm TypeScript package; `ttc` uses the built typescript-go checkout above.
For bundlers with a declarative config it writes an `tt.*.config.mjs` wrapper
that composes the existing config; it never rewrites the user's config source.
Run `bun run tt:dev` or `bun run tt:build` to use that wrapper. esbuild build
scripts are arbitrary JavaScript, so the command prints the one manual plugin
line that must be added instead.

Useful non-interactive options:

```sh
bunx @load28/create-tt@next init --bundler vite
bunx @load28/create-tt@next init --bundler none
bunx @load28/create-tt@next init --no-install
bunx @load28/create-tt@next init --package-manager bun
```

New projects always use Bun. An existing project keeps the package manager in
its `packageManager` field or lockfile unless `--package-manager` is passed.

## Repository development: install locally built packages through a registry

For compiler development, run a real npm-compatible registry instead of
replacing dependencies with `file:` paths. Start Verdaccio in one terminal:

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
```

Build `ttc`, assemble packages for the current OS and CPU, and publish
`@load28/tt-lang`, its platform binary, `@load28/unplugin-tt`, and `@load28/create-tt` to that registry:

```sh
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

The publisher prints the exact bootstrap command. It has this form:

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx @load28/create-tt@latest my-app --registry http://127.0.0.1:4873
```

`--registry` passes the registry to `bun install` and writes it to the new
project's `bunfig.toml`. Verdaccio serves the locally built TT packages and
proxies third-party packages such as Vite.

## Manual compiler setup

Before using this path, complete the typescript-go source build at the top of
this guide and keep `TTC_TSGO_ROOT` exported. Then install the compiler:

```sh
bun add -d @load28/tt-lang@next
```

Keep sources in `src/**/*.tt` or `src/**/*.ttx`, then add scripts like these:

```json
{
  "scripts": {
    "build:tt": "ttc -o .tt-build src",
    "check:tt": "ttc --check-types src"
  }
}
```

`bun run build:tt` produces ordinary `.ts`/`.tsx` files in `.tt-build`; point
an existing TypeScript build at that tree. Add `.tt-build/` and `.tt-types/`
to `.gitignore`. Do not edit generated files.

## Manual bundler setup

The same built typescript-go checkout and `TTC_TSGO_ROOT` are required for this
path. Install the direct-source plugin in addition to the compiler:

```sh
bun add -d @load28/tt-lang@next @load28/unplugin-tt@next
```

Put `tt()` first in the bundler's plugins array:

```ts
// Vite: vite.config.ts
import tt from "@load28/unplugin-tt/vite";
export default { plugins: [tt()] };

// Rollup: rollup.config.js
import tt from "@load28/unplugin-tt/rollup";
export default { plugins: [tt()] };

// Rolldown: rolldown.config.js
import tt from "@load28/unplugin-tt/rolldown";
export default { plugins: [tt()] };

// webpack: webpack.config.mjs
import tt from "@load28/unplugin-tt/webpack";
export default { plugins: [tt()] };

// Rspack: rspack.config.mjs
import tt from "@load28/unplugin-tt/rspack";
export default { plugins: [tt()] };

// Farm: farm.config.ts
import tt from "@load28/unplugin-tt/farm";
export default { plugins: [tt()] };
```

esbuild uses its JavaScript API:

```js
import { build } from "esbuild";
import tt from "@load28/unplugin-tt/esbuild";

await build({ entryPoints: ["src/main.tt"], bundle: true, plugins: [tt()] });
```

The plugin makes the bundler read `.tt` and `.ttx` directly. Keep
`ttc --check-types src` as a separate check because transpiling bundlers do not
replace TypeScript type checking.

## Migrating files

Start by renaming only files that use tt syntax from `.ts` to `.tt` or from
`.tsx` to `.ttx`. Keep explicit `.tt`/`.ttx` extensions in relative imports.
Ordinary TypeScript and TSX may remain unchanged and can be migrated gradually.

```ts
import { render } from "./notice.tt";
```

Run `bunx ttc --check-types src` before the normal build. For editor diagnostics
and navigation, download `tt-language-<version>.vsix` from the newest
[GitHub Releases](https://github.com/load28/tt/releases) pre-release. Install it
with **Extensions: Install from VSIX...** in the VS Code Command Palette, or:

```sh
code --install-extension ./tt-language-<version>.vsix
```

Open the project root from the shell that provides `TTC_TSGO_ROOT`.
