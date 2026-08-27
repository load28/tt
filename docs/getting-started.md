# Install tt

You need [Bun](https://bun.sh/) to run the recommended setup command. TT
packages and the prebuilt `ttc` compiler are installed from npm.

`ttc` drives TypeScript 7, which it takes from your project's own
`node_modules` — the same package your build uses, and the only place it
looks. Install it alongside TT:

```sh
bun add -d typescript@7.1.0-dev.20260826.1
```

There is nothing to export and no editor-specific configuration: the
extension runs your project's `ttc`, which resolves your project's
TypeScript. Declaration output (`ttc --types` and the editor's `.tt.d.ts`
sidecars) uses an API that arrived in TypeScript 7.1; everything else works
on 7.0.

## Automatic setup

Create a Vite + TypeScript project with a starter `.tt` module:

```sh
bunx @load28/create-tt@0.3.0 my-app
cd my-app
bun run dev
```

Add tt to an existing TypeScript project:

```sh
cd existing-project
bunx @load28/create-tt@0.3.0 init
bun run tt:check
```

`init` detects Vite, Rollup, Rolldown, webpack, Rspack, esbuild, and Farm from
`package.json`. The `0.3.0` initializer adds the stable channels of
`@load28/tt-lang` and `@load28/unplugin-tt`, plus TT scripts. It does not add
an npm TypeScript package; add `typescript@7.1.0-dev.20260826.1` yourself as shown above.
For bundlers with a declarative config it writes an `tt.*.config.mjs` wrapper
that composes the existing config; it never rewrites the user's config source.
Run `bun run tt:dev` or `bun run tt:build` to use that wrapper. esbuild build
scripts are arbitrary JavaScript, so the command prints the one manual plugin
line that must be added instead.

Useful non-interactive options:

```sh
bunx @load28/create-tt@0.3.0 init --bundler vite
bunx @load28/create-tt@0.3.0 init --bundler none
bunx @load28/create-tt@0.3.0 init --no-install
bunx @load28/create-tt@0.3.0 init --package-manager bun
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

Install the compiler and the TypeScript it drives:

```sh
bun add -d @load28/tt-lang@0.3.0 typescript@7.1.0-dev.20260826.1
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
bun add -d @load28/tt-lang@0.3.0 @load28/unplugin-tt@0.1.0
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

Open the project root. The extension runs the `ttc` your project installed,
which uses the TypeScript your project installed — no environment variables,
and no way for the editor and the build to disagree.
