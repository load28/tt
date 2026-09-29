# TASK-434: Make create-tt init check solution configs, require the verified TypeScript, and quote the printed directory

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`create-tt` left three projects in a state that looked configured but was not: `init` on a Vite-template solution `tsconfig.json` produced a `tt:check` that checked no files and exited 0; `init` kept an existing `typescript` such as `~5.8.0`, with which the generated `--runExternalCode` and `contentMappers` scripts cannot work; and `create` printed `Run: cd ct app 2 && bun run dev` for a directory with spaces, which a shell splits into three words.

## Scope

- Included: `packages/create-tt/src/installer.js` (`initializeExisting`, the generated type-check configs and scripts, the TypeScript dependency, `printResult`), the package's unit tests, the package README, and the `init` section of `docs/getting-started.md`.
- Excluded: The versions of other existing dependencies (`@openload28/tt-lang`, `@openload28/unplugin-tt`), which keep their current `??=` behaviour, and the `create` flow's configuration, which already writes a complete `tsconfig.json`.

## Decisions

### Decision 1: Mirror the project-reference graph with mapper-enabled counterparts and check it with `tsc -b`

- **Context**: The generated `tsconfig.tt.json` is `{ "extends": "./tsconfig.json", ... }`. The TypeScript tsconfig reference for `extends` states that `files`, `include`, and `exclude` are inherited and that `references` is the one top-level property excluded from inheritance. A solution `tsconfig.json` (`"files": []` plus `references`, the Vite template shape) therefore became a config with no root files and no references. Measured with the pinned `typescript@7.1.0-dev.20260826.1`: `tsc -p tsconfig.tt.json --runExternalCode` exits 0 with a `TS2322` in `src/main.ts`, and it still exits 0 even when `references` is copied into the generated config, because `-p` compiles only that one project. The Project References handbook page describes solution-style configs and `tsc --build` (`-b`) as the way to build a config and every project it references.
- **Alternatives considered**: (a) Extend only the "app" reference: guesses which reference matters and drops the rest (Vite's `tsconfig.node.json` has different `lib`/`types` and cannot be merged into the app config). (b) Copy `references` unchanged into `tsconfig.tt.json` and use `-b`: the referenced configs have no `contentMappers`, so `.tt` imports inside them fail with `TS2307`. (c) Chain one `tsc -p` per referenced config in the script: loses the reference semantics (build order, `.d.ts` redirects) for configs whose own files import a referenced project.
- **Decision and rationale**: For the project's `tsconfig.json` and, recursively, every config it references inside the project directory, `init` writes a counterpart `<name>.tt.json` next to it that extends the original, adds `noEmit` and the content mapper, and, when the original has `references`, lists the counterparts of those references (a reference to a directory resolves to its `tsconfig.json`, as TypeScript resolves reference paths). References outside the project, or to configs that do not exist, are kept as written: `init` writes only inside the project it initializes, and TypeScript reports a missing config itself. When the root has references, `tt:check` and the `tt:build` prefix use `tsc -b tsconfig.tt.json --runExternalCode`; otherwise they keep `tsc -p` and the generated file is byte-identical to before. Reading the graph needs the configs' content, and tsconfig files are JSON with comments and trailing commas (the Vite template's `tsconfig.app.json` has both), so the installer parses that form; an unreadable config stops `init` before any write. Measured: on a Vite-style project the generated `tt:check` now prints `src/main.ts(2,7): error TS2322` and exits 2, and with a `.tt` import in the app project it type-checks through the mapper. An inherited `tsBuildInfoFile` is shared with the user's own `tsc -b`; TypeScript detects the differing options and rebuilds (`... is out of date because buildinfo file ... indicates there is change in compilerOptions`), so correctness does not depend on separate files.

### Decision 2: `init` always sets `typescript` to the verified version, and reports a replacement

- **Context**: `docs/ai/tt.md` and `docs/getting-started.md` require the repository-pinned `typescript@7.1.0-dev.20260826.1` for the content-mapper setup (content mappers and `--runExternalCode` arrived in the 7.1 line), and `create` already writes exactly that version. `init` used `devDependencies.typescript ??= ...`, so any existing version, including a 5.x range, was kept.
- **Alternatives considered**: (a) Keep the existing version and only warn: the generated scripts would fail, which is the broken setup the task rules out. (b) Evaluate whether an existing semver range admits a compatible 7.1 build: npm ranges do not match prereleases unless the range names one (node-semver "Prerelease Tags"), and until 7.1 is released the documented requirement is one exact build, so there is nothing else to accept. (c) Refuse to run: forces a manual edit for a change the initializer can make.
- **Decision and rationale**: The manifest's `typescript` entry, in `dependencies` when it is declared there and otherwise in `devDependencies`, is set to the verified version. When a different version was named, the result records it and `init` prints `Updated typescript from ~5.8.0 to 7.1.0-dev.20260826.1: tt's content mapper needs this TypeScript 7.1 build.`

### Decision 3: Quote the printed directory as a POSIX shell word

- **Context**: `Run: cd ${location} && ...` is a command line to paste into a shell.
- **Alternatives considered**: Double quotes still expand `$`, `` ` ``, and `\` (POSIX Shell Command Language, 2.2.3 Double-Quotes).
- **Decision and rationale**: A word made only of characters that are never special to the shell is printed as is; any other word is wrapped in single quotes, which preserve every character literally (2.2.2 Single-Quotes), with an embedded `'` written as `'\''`. `ct app 2` prints `Run: cd 'ct app 2' && bun run dev`.

## Work log

- 2026-09-27: Reproduced (a) in a Vite-style project with the pinned TypeScript and the debug `ttc` as mapper: the old generated config exits 0 despite `TS2322`. Measured `-b` against `-p`, a copied `references` list, and buildinfo sharing.
- 2026-09-27: Implemented the counterpart graph, the `-b` script choice, the JSONC reader, the TypeScript replacement and report, and `shellQuote`. Updated the package README and `docs/getting-started.md`.
- 2026-09-27: Added five tests to `packages/create-tt/test/installer.test.mjs`: the generated counterpart graph and scripts, a run of the generated `tt:check` with the repository's TypeScript on a solution project (skipped only when no TypeScript is installed), the TypeScript replacement in both dependency sections with its report, the quoted `Run:` line, and the JSONC reader. With the installer change reverted all five fail (`tsc -p tsconfig.tt.json` instead of `-b`, exit status 0, `~5.8.0` kept, `Run: cd ct app 2 && bun run dev`, a JSON parse error); with it they pass.

## Issues and resolutions

### Issue 1: The generated check of a solution config checked nothing

- **Symptom**: `tsc -p tsconfig.tt.json --runExternalCode` exited 0 on a Vite template project with a type error.
- **Cause**: `references` is not inherited through `extends`, and `-p` does not build referenced projects.
- **Resolution**: Decision 1.

### Issue 2: An incompatible TypeScript was kept

- **Symptom**: `"typescript": "~5.8.0"` survived `init`.
- **Cause**: `??=` only fills a missing entry.
- **Resolution**: Decision 2.

### Issue 3: The printed `cd` command broke on spaces

- **Symptom**: `Run: cd ct app 2 && bun run dev`.
- **Cause**: The location was interpolated unquoted.
- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed (no Rust change in this task).
- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`: 64 passed.
- [x] `TTC_BINARY=<debug ttc> npm --prefix packages/create-tt run test:e2e`: 1 passed.
- [x] `node scripts/check-task-index`

## Result

Changed `packages/create-tt/src/installer.js`, `packages/create-tt/test/installer.test.mjs`, `packages/create-tt/README.md`, and `docs/getting-started.md`. `init` now produces a check that covers every project a solution config references, installs the TypeScript the generated scripts need, and `create` prints a directory that pastes into a shell unchanged.
