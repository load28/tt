# @load28/create-tt

Create a ready-to-run Vite + TypeScript + tt project:

```sh
bunx @load28/create-tt@next my-app
```

Add tt to an existing TypeScript project. The initializer detects Vite,
Rollup, Rolldown, webpack, Rspack, esbuild, or Farm from `package.json`:

```sh
bunx @load28/create-tt@next init
```

The initializer updates `package.json` structurally. For bundlers with a
declarative config, it generates an `tt.*.config.mjs` wrapper instead of
rewriting arbitrary user code. Existing scripts and config files stay intact.
Use `--no-install` in CI or when dependencies will be installed later.
New projects use Bun for dependency installation and scripts. Existing projects
keep the package manager declared in `package.json` or selected by their lockfile.

For packages published to a private or local npm-compatible registry, pass the
registry through the entire bootstrap:

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx @load28/create-tt@next my-app --registry http://127.0.0.1:4873
```

The Nightly initializer installs the `next` channels of
`@load28/tt-lang` and `@load28/unplugin-tt`. The current TypeScript toolchain
prerequisite is documented in the repository
[installation guide](https://github.com/load28/tt/blob/main/docs/getting-started.md).
