# Install tt

You need [Bun](https://bun.sh/) to run the recommended setup command. The
generated project installs the prebuilt `ttc` compiler and TypeScript 7, so
Rust, Go, and a separate typescript-go checkout are not required.

## Automatic setup

Create a Vite + TypeScript project with a starter `.tt` module:

```sh
bunx @load28/create-tt@latest my-app
cd my-app
bun run dev
```

Add tt to an existing TypeScript project:

```sh
cd existing-project
bunx @load28/create-tt@latest init
bun run tt:check
```

`init` detects Vite, Rollup, Rolldown, webpack, Rspack, esbuild, and Farm from
`package.json`. It adds `@load28/tt-lang`, TypeScript 7, `@load28/unplugin-tt`, and TT scripts.
For bundlers with a declarative config it writes an `tt.*.config.mjs` wrapper
that composes the existing config; it never rewrites the user's config source.
Run `bun run tt:dev` or `bun run tt:build` to use that wrapper. esbuild build
scripts are arbitrary JavaScript, so the command prints the one manual plugin
line that must be added instead.

Useful non-interactive options:

```sh
bunx @load28/create-tt@latest init --bundler vite
bunx @load28/create-tt@latest init --bundler none
bunx @load28/create-tt@latest init --no-install
bunx @load28/create-tt@latest init --package-manager bun
```

New projects always use Bun. An existing project keeps the package manager in
its `packageManager` field or lockfile unless `--package-manager` is passed.

### Install locally built packages through a registry

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
proxies third-party packages such as Vite and TypeScript.

## Manual compiler setup

Install the compiler and the TypeScript version it drives:

```sh
bun add -d @load28/tt-lang typescript@7
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

Install the direct-source plugin in addition to the compiler:

```sh
bun add -d @load28/tt-lang typescript@7 @load28/unplugin-tt
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
and navigation, install the TT VS Code extension and open the project root.
