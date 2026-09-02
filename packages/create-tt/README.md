# @openload28/create-tt

Create a ready-to-run Vite + TypeScript + tt project:

```sh
bunx @openload28/create-tt@next my-app
```

Generated projects declare `@openload28/tt-lang` as a TypeScript 7.1 content
mapper and run `tsc` with `--runExternalCode`. `.ts` and `.tsx` files can import
`.tt` and `.ttx` directly without `.tt-types` sidecars.

Add tt to an existing TypeScript project. The initializer detects Vite,
Rollup, Rolldown, webpack, Rspack, esbuild, or Farm from `package.json`:

```sh
bunx @openload28/create-tt@next init
```

The initializer updates `package.json` structurally. For bundlers with a
declarative config, it generates an `tt.*.config.mjs` wrapper instead of
rewriting arbitrary user code. It also creates `tsconfig.tt.json`, which extends
the existing TypeScript config and declares the content mapper. Existing scripts
and config files stay intact.
Use `--no-install` in CI or when dependencies will be installed later.
New projects use Bun for dependency installation and scripts. Existing projects
keep the package manager declared in `package.json` or selected by their lockfile.

For packages published to a private or local npm-compatible registry, pass the
registry through the entire bootstrap:

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx @openload28/create-tt@next my-app --registry http://127.0.0.1:4873
```

The Nightly initializer installs the `next` channels of
`@openload28/tt-lang` and `@openload28/unplugin-tt`. The current TypeScript toolchain
prerequisite is documented in the repository
[installation guide](https://github.com/load28/tt/blob/main/docs/getting-started.md).
