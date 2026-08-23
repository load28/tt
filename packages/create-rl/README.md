# create-rl

Create a ready-to-run Vite + TypeScript + rl project:

```sh
bun create rl@latest my-app
```

Add rl to an existing TypeScript project. The initializer detects Vite,
Rollup, Rolldown, webpack, Rspack, esbuild, or Farm from `package.json`:

```sh
bun create rl@latest init
```

The initializer updates `package.json` structurally. For bundlers with a
declarative config, it generates an `rl.*.config.mjs` wrapper instead of
rewriting arbitrary user code. Existing scripts and config files stay intact.
Use `--no-install` in CI or when dependencies will be installed later.
New projects use Bun for dependency installation and scripts. Existing projects
keep the package manager declared in `package.json` or selected by their lockfile.

For packages published to a private or local npm-compatible registry, pass the
registry through the entire bootstrap:

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx create-rl@latest my-app --registry http://127.0.0.1:4873
```
