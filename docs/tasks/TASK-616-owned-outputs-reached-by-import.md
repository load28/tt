# TASK-616: Keep TypeScript's `exclude` semantics for an owned output reached by import

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-616`

## Purpose

After an in-place build, `src/h.ts` importing `"./m.js"` under `module:
nodenext` resolves to ttc's owned output `src/m.ts`, so `ttc --check-types
src` reports an error in `src/m.tt` twice: once at the source and once in
the generated `src/m.ts`. TASK-587 left owned outputs out of the program's
file listing; this task decides what an import of one should do.

## Scope

- Included: an investigation of the host's seams (`src/typescript/host.mjs`),
  the documented rule (`docs/ai/tt.md`), and a regression test that pins it
  (`tests/workflow_repairs.rs`).
- Excluded: changes to module resolution. None of the host's seams can
  redirect a relative import faithfully (Decision 1).

## Decisions

### Decision 1: An import of an owned output reaches the output, as `tsc` does

- **Context**: TASK-587's rule mirrors how `tsc` treats its own outputs:
  they are left out of `include` globbing. TypeScript's `exclude` works the
  same way: "`exclude` only changes which files are included as a result of
  the `include` setting. A file specified by `exclude` can still become part
  of your codebase due to an import statement, a `types` inclusion, a
  `/// <reference` directive, or being specified in the `files` list"
  (TypeScript tsconfig reference, `exclude`). The host's comment on the
  listing already says that imports still resolve to output files. Under
  `nodenext`, `"./m.js"` names the emitted file, whose TypeScript source is
  `m.ts` (TypeScript handbook, "Modules - Reference", relative import
  extension substitution), so `h.ts` really depends on the output.
- **Alternatives considered**: The report asked for a redirect to the
  owning `.tt` projection at the host's resolution or read seam; each was
  tried or checked.
  (a) `realpath`: the host logged every `realpath` call. TypeScript
  realpaths only non-relative (`node_modules`) resolutions, and
  `src/m.ts` was never asked, so a relative import cannot be redirected
  this way.
  (b) `fileExists`/`readFile` hiding the output: `"./m.js"` then fails
  with TS2307, which `tsc` does not report for this project.
  (c) Serving the owner's current lowering as the output's text: TypeScript
  identifies a source file by its path, so `m.ts` and `m.tt` stay two
  modules with two sets of declarations and the error is still reported
  twice. Removing one of them afterwards would be diagnostic suppression.
  (d) Serving a re-export facade (`export * from "./m.tt"`) at the output:
  it does not forward a default export, `export =`, or `import x =
  require()`, so it is not the same module. The TypeScript API
  (`dist/api/fs.d.ts`) exposes only file-system callbacks, no module
  resolution hook, so the project-reference style source redirect that
  TypeScript applies internally is not reachable either.
- **Decision and rationale**: No redirect. The import names the output, and
  TypeScript's semantics for an excluded file reached by import are kept.
  `docs/ai/tt.md` now states this and names the documented import form
  (`"./m.tt"`), which reaches the projection and reports once. A regression
  test pins both.

## Work log

- 2026-09-30: Reproduced `target/probe6-cli/p12`: TS2322 at `src/m.ts:48:21`
  and `src/m.tt:3:21`.
- 2026-09-30: Logged the host's `realpath` calls: only the project root,
  `src`, the mapper link, and library files; never `src/m.ts`. Hiding the
  output through `fileExists` gave TS2307. Importing `"./m.tt"` reported
  the error once, with and without `allowImportingTsExtensions`.
- 2026-09-30: Documented the rule in `docs/ai/tt.md`; added
  `an_import_decides_whether_an_owned_output_joins_the_program`.

## Issues and resolutions

None.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test workflow_repairs an_import_decides`
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `docs/ai/tt.md`, `tests/workflow_repairs.rs`,
`docs/tasks/INDEX.md`, and this record. No compiler change: a faithful
redirect is not available through TypeScript's host seams, and TypeScript's
own `exclude` semantics are what the current behaviour implements.
