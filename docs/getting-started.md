# Install tt

You need [Bun](https://bun.sh/) for the recommended setup.

## Automatic setup

Create a Vite + TypeScript project with a starter `.tt` module:

```sh
bunx @openload28/create-tt@next my-app
cd my-app
bun run dev
```

Add tt to an existing TypeScript project:

```sh
cd existing-project
bunx @openload28/create-tt@next init
bun run tt:check
```

`init` performs these steps:

- Detects Vite, Rollup, Rolldown, webpack, Rspack, esbuild, or Farm
- Adds `@openload28/tt-lang`, `@openload28/unplugin-tt`, TypeScript, and TT scripts
- Creates `tt.*.config.mjs` for declarative bundlers
- Prints the plugin code to add for esbuild

Useful non-interactive options:

```sh
bunx @openload28/create-tt@next init --bundler vite
bunx @openload28/create-tt@next init --bundler none
bunx @openload28/create-tt@next init --no-install
bunx @openload28/create-tt@next init --package-manager bun
```

New projects use Bun. Existing projects keep the package manager from their
`packageManager` field or lockfile.

## Manual compiler setup

Install the compiler and the TypeScript it drives:

```sh
bun add -d @openload28/tt-lang@next typescript@7.1.0-dev.20260826.1
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
bun add -d @openload28/tt-lang@next @openload28/unplugin-tt@next
```

Put `tt()` first in the bundler's plugins array:

```ts
// Vite: vite.config.ts
import tt from "@openload28/unplugin-tt/vite";
export default { plugins: [tt()] };

// Rollup: rollup.config.js
import tt from "@openload28/unplugin-tt/rollup";
export default { plugins: [tt()] };

// Rolldown: rolldown.config.js
import tt from "@openload28/unplugin-tt/rolldown";
export default { plugins: [tt()] };

// webpack: webpack.config.mjs
import tt from "@openload28/unplugin-tt/webpack";
export default { plugins: [tt()] };

// Rspack: rspack.config.mjs
import tt from "@openload28/unplugin-tt/rspack";
export default { plugins: [tt()] };

// Farm: farm.config.ts
import tt from "@openload28/unplugin-tt/farm";
export default { plugins: [tt()] };
```

esbuild uses its JavaScript API:

```js
import { build } from "esbuild";
import tt from "@openload28/unplugin-tt/esbuild";

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
code --install-extension ./tt-typescript-preview-<version>-<platform>.vsix
```

After installing both VSIX files, enable TypeScript 7 in the editor:

```jsonc
// .vscode/settings.json (or user settings)
"js/ts.experimental.useTsgo": true,
"typescript.experimental.useTsgo": true
```

Open the project root in VS Code.
