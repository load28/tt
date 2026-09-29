# TASK-566: `--sidecar` names tt modules as the source does

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-566: Write sidecar imports with the source's specifiers`

## Purpose

After `ttc -o gen src` (default `--rewrite-imports js`) and
`tsc --emitDeclarationOnly --outDir decl`, `ttc --sidecar decl src` wrote
`src/u.tt.d.ts` with `import { K } from "./sub/m.js"`. Beside `src/u.tt`
there is no `sub/m.js` or `sub/m.ts`, only `sub/m.tt` and its sidecar, so a
`.ts` importing `"./u.tt"` got TS2307 inside the sidecar and lost `K`.
`--types` writes `"./sub/m.tt"` for the same module.

## Scope

- Included: the declaration text `--sidecar` writes (`src/sidecar.rs`,
  `src/main/modes.rs`) and the inverse of the import rewrite
  (`ImportRewrite::source_specifier`, `src/lib/api.rs`)
- Excluded: specifiers tsc synthesizes without an extension (for example
  `import("./sub/m")` under `bundler` resolution). ttc's rewrite never
  produces that spelling, so it has no inverse to apply; the sidecar keeps
  it as tsc wrote it.

## Decisions

### Decision 1: The documented workflow compiles with the default rewrite

- **Context**: The task asked whether the documentation requires
  `--rewrite-imports off` before `--sidecar`.
- **Findings**: It does not. The design record
  (`docs/design/ts-sidecar-declarations.md`) and TASK-029 describe
  `ttc x.tt → x.ts`, `tsc --emitDeclarationOnly`, then `ttc --sidecar`, with
  no rewrite option; `--sidecar` itself does not accept `--rewrite-imports`;
  `docs/ai/tt.md` names `js` as the default. A sidecar is the declaration
  TypeScript finds for `"./x.tt"` beside the source (TASK-029 Decision 2),
  so its imports have to resolve from there — the spelling the source uses
  and `--types` writes.

### Decision 2: Invert the rewrite, confirmed by the source tree

- **Context**: The rewrite only replaces a relative `.tt`/`.ttx`
  specifier's extension (`.js`/`.jsx` in `js` mode, `.ts`/`.tsx` in `ts`
  mode). `--sidecar` does not know which mode the build used, and `.js`
  can equally name hand-written TypeScript.
- **Alternatives considered**: Map back only the specifiers the source's own
  import list names (misses the `import("…")` types tsc synthesizes for
  inferred types); require `--rewrite-imports off` (contradicts the
  documented workflow); accept `--rewrite-imports` on `--sidecar` (a second
  place to repeat the build's choice, and still ambiguous for `.js`).
- **Decision and rationale**: `ttc::source_specifiers` parses the
  declarations, visits every module specifier (imports, re-exports, import
  types, `import = require`, ambient module names), and restores one when a
  rewrite produces it from a `.tt`/`.ttx` specifier that names an existing
  tt source relative to the source file. A tree `ttc` can build cannot hold
  both `m.tt` and a hand-written `m.ts` in one directory (both claim
  `m.ts`), so the source file decides unambiguously. Every other byte is
  kept, and the map is built from the text that is written.

## Work log

- 2026-09-29: Reproduced under `target/probe4-cli/p5` (`src/sub/m.tt`,
  `src/u.tt`): the sidecar held `"./sub/m.js"`, and `tsc --noEmit` over a
  `.ts` importing `"./u.tt"` reported TS2307 at `src/u.tt.d.ts(2,19)`.
- 2026-09-29: Added `ImportRewrite::source_specifier` (with a doctest),
  `ttc::source_specifiers` and its specifier visitor, and the call in
  `sidecar_mode`.
- 2026-09-29: Added `sidecars_name_tt_modules_as_the_source_does`
  (`tests/cli_outputs.rs`): `.js`, `.jsx`, and `.ts` spellings of tt
  modules and an `import("…")` type are restored, with and without `-o`;
  a hand-written `./h.js` and a package specifier are kept. It fails
  without the fix.
- 2026-09-29: After the fix the probe's sidecar holds `"./sub/m.tt"` and the
  consumer check resolves `K` (it then reports the deliberate TS2322 of the
  probe, `K` assigned to `number`).

## Issues and resolutions

### Issue 1: Sidecar imports named files that do not exist beside it

- **Symptom**: `import { K } from "./sub/m.js"` in `src/u.tt.d.ts`; TS2307
  for any consumer.
- **Cause**: `--sidecar` copied tsc's declarations for the compiled tree,
  whose specifiers ttc had rewritten to the compiled files' names.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test cli_outputs --test sidecar`, and the new doctest

## Result

Changed `src/lib/api.rs`, `src/sidecar.rs`, `src/lib.rs`,
`src/main/modes.rs`, and `tests/cli_outputs.rs`. `--sidecar` sidecars name
tt modules the way the source and `--types` do.
