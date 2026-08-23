# Install rl

You need [Bun](https://bun.sh/) to run the recommended setup command. The
generated project installs the prebuilt `rlc` compiler and TypeScript 7, so
Rust, Go, and a separate typescript-go checkout are not required.

## Automatic setup

Create a Vite + TypeScript project with a starter `.rl` module:

```sh
bun create rl@latest my-app
cd my-app
bun run dev
```

Add rl to an existing TypeScript project:

```sh
cd existing-project
bun create rl@latest init
bun run rl:check
```

`init` detects Vite, Rollup, Rolldown, webpack, Rspack, esbuild, and Farm from
`package.json`. It adds `rl-lang`, TypeScript 7, `unplugin-rl`, and RL scripts.
For bundlers with a declarative config it writes an `rl.*.config.mjs` wrapper
that composes the existing config; it never rewrites the user's config source.
Run `bun run rl:dev` or `bun run rl:build` to use that wrapper. esbuild build
scripts are arbitrary JavaScript, so the command prints the one manual plugin
line that must be added instead.

Useful non-interactive options:

```sh
bun create rl@latest init --bundler vite
bun create rl@latest init --bundler none
bun create rl@latest init --no-install
bun create rl@latest init --package-manager bun
```

New projects always use Bun. An existing project keeps the package manager in
its `packageManager` field or lockfile unless `--package-manager` is passed.

### Install locally built packages through a registry

For compiler development, run a real npm-compatible registry instead of
replacing dependencies with `file:` paths. Start Verdaccio in one terminal:

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
```

Build `rlc`, assemble packages for the current OS and CPU, and publish
`rl-lang`, its platform binary, `unplugin-rl`, and `create-rl` to that registry:

```sh
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

The publisher prints the exact bootstrap command. It has this form:

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx create-rl@latest my-app --registry http://127.0.0.1:4873
```

`--registry` passes the registry to `bun install` and writes it to the new
project's `bunfig.toml`. Verdaccio serves the locally built RL packages and
proxies third-party packages such as Vite and TypeScript.

## Manual compiler setup

Install the compiler and the TypeScript version it drives:

```sh
bun add -d rl-lang typescript@7
```

Keep sources in `src/**/*.rl` or `src/**/*.rlx`, then add scripts like these:

```json
{
  "scripts": {
    "build:rl": "rlc -o .rl-build src",
    "check:rl": "rlc --check-types src"
  }
}
```

`bun run build:rl` produces ordinary `.ts`/`.tsx` files in `.rl-build`; point
an existing TypeScript build at that tree. Add `.rl-build/` and `.rl-types/`
to `.gitignore`. Do not edit generated files.

## Manual bundler setup

Install the direct-source plugin in addition to the compiler:

```sh
bun add -d rl-lang typescript@7 unplugin-rl
```

Put `rl()` first in the bundler's plugins array:

```ts
// Vite: vite.config.ts
import rl from "unplugin-rl/vite";
export default { plugins: [rl()] };

// Rollup: rollup.config.js
import rl from "unplugin-rl/rollup";
export default { plugins: [rl()] };

// Rolldown: rolldown.config.js
import rl from "unplugin-rl/rolldown";
export default { plugins: [rl()] };

// webpack: webpack.config.mjs
import rl from "unplugin-rl/webpack";
export default { plugins: [rl()] };

// Rspack: rspack.config.mjs
import rl from "unplugin-rl/rspack";
export default { plugins: [rl()] };

// Farm: farm.config.ts
import rl from "unplugin-rl/farm";
export default { plugins: [rl()] };
```

esbuild uses its JavaScript API:

```js
import { build } from "esbuild";
import rl from "unplugin-rl/esbuild";

await build({ entryPoints: ["src/main.rl"], bundle: true, plugins: [rl()] });
```

The plugin makes the bundler read `.rl` and `.rlx` directly. Keep
`rlc --check-types src` as a separate check because transpiling bundlers do not
replace TypeScript type checking.

## Migrating files

Start by renaming only files that use rl syntax from `.ts` to `.rl` or from
`.tsx` to `.rlx`. Keep explicit `.rl`/`.rlx` extensions in relative imports.
Ordinary TypeScript and TSX may remain unchanged and can be migrated gradually.

```ts
import { render } from "./notice.rl";
```

Run `bunx rlc --check-types src` before the normal build. For editor diagnostics
and navigation, install the RL VS Code extension and open the project root.
