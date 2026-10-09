# TASK-780: Fix the sixth audit's command-line findings

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-780: Fix the sixth audit's command-line findings`

## Purpose

The sixth audit of `ttc`'s command line (a read-only agent against a
release build of `d8da7233`) found ten new defects: a typed check that
hangs on a circular `extends`, `--types` writing nothing under `outDir`,
CommonJS support modules in an ES-module package, an internal error when
Node writes to stdout, inputs that abort on a dangling symlink or on
non-UTF-8 TypeScript, diagnostics reported twice across projects,
configurations `tsc` reads but `ttc` refuses, `-w` that exits without
sources, and dot-files compiled but never checked. Fix each in the layer
that owns it.

## Scope

- Included: the ten findings (C1–C10) and the three observations the
  audit listed after them, each fixed or recorded with a reason.
- Excluded: the compiler and editor findings of the same audit round.

## Decisions

### Decision C8: An empty, comment-only, or BOM-prefixed configuration is read

- **Context**: `ttc` read the `jsx` option of a `tsconfig.json` that was
  empty, held only a comment, or began with a byte order mark as a JSON
  error; `tsc -p . --showConfig` reads the first two as `{}` and the third
  with its options.
- **Decision and rationale**: The configuration reader drops a leading
  byte order mark, as TypeScript's `sys.readFile` does, and reads a text
  with no value as the empty object, as `parseConfigFileTextToJson` does.

### Decision C5: A symlink whose target cannot be read is skipped by the walk

- **Context**: A dangling symlink in a directory input (`src/README ->
  nowhere`, an Emacs lock file `.#a.tt`) stopped `--check`, the build,
  `--dependencies`, and the typed modes with `No such file or directory`.
  TASK-387 decision 1 and TASK-773 decision 2 kept this error for the
  command line's own walk; `tsc -p` builds the same tree.
- **Alternatives considered**: (a) Keep naming the entry: a lock file an
  editor holds would stop every build and every watch round. (b) Skip every
  entry the walk cannot read: a permission error on a real source would
  drop it without a word. (c) Skip what TypeScript skips.
- **Decision and rationale**: (c). TypeScript's directory listing
  (`getAccessibleFileSystemEntries` in `sys.ts`) asks `stat` only of a
  symbolic link and leaves out a link whose `stat` fails; the walk now does
  the same, and still names any other entry it cannot read. Both earlier
  records say so at the top. `contextual_input_walk_cannot_silently_stop_before_dependencies`
  (TASK-353) required `-p` to fail at a dangling link, so that a walk
  stopped at an error could not refine against part of the project; it is
  now `contextual_input_walk_reads_past_a_dangling_link`, which requires
  the walk to read a dependency sorted after the link.

### Decision C9: `-w` without sources keeps watching

- **Context**: `ttc -w -o out src` over an empty `src` printed `no sources
  found` and exited 1; `--check-types -w` exited 2. `tsc -w` reports TS18003
  and keeps watching, and `ttc -w` already kept watching when every source
  was deleted during a session.
- **Decision and rationale**: Both watch modes say there are no sources and
  wait: the build enters its watch loop with no jobs, and the typed watch
  retries opening the project each interval, printing each distinct error
  once, as its loop already does when a reopen fails. The one-shot modes,
  `-p`, `--symbols`, `--emit-map`, and `--sidecar` still exit 1.

### Decision C7: One run prints a diagnostic once across project groups

- **Context**: Inputs in two projects that both reach one file printed its
  diagnostics once per project (`--check-types a b`, `--tt-only` too), and
  `--json-report` counted both. `docs/ai/tt.md` says one run merges
  identical diagnostics; TASK-770 decision 2's per-project passes did not.
- **Decision and rationale**: The passes of one run share the set of
  rendered diagnostics they printed; a diagnostic rendered identically
  (same file, position, range, code, and message) is printed and counted
  once.

### Decision C1: A circular configuration is reported, not opened

- **Context**: `{"extends": "./tsconfig.json"}` (or an `a -> b -> a`
  chain) made `--check-types` and `--dependencies` wait forever, and a
  killed `ttc` left `tsc --api` behind. A probe of the TypeScript API showed
  `parseConfigFile` returning TS18000 and `updateSnapshot({ openProjects })`
  never answering for the same file; `tsc -p` reports TS18000 and stops.
- **Decision and rationale**: Before opening the project, the host asks
  TypeScript to parse the configuration, as it already asked it to read
  one (TASK-764 decision 3); a TS18000 is reported as the project's
  diagnostic and no project is opened, as for an unreadable configuration.
  The check exits 1 with `ts18000`. The orphaned processes were the
  deadlocked `tsc --api`, which no longer starts that request.

### Decision C4: Only the host's own lines on stdout answer

- **Context**: A `NODE_OPTIONS` preload that logs (dotenv does) or a
  `--node` shim that echoes a line wrote into the host's stdout, and the
  first line was read as the host's answer: an internal compiler error
  (exit 101). With the host fixed, the content mapper the API starts with
  Node failed the same way (`jsonrpc: invalid header`).
- **Alternatives considered**: A separate pipe for the protocol needs a
  file descriptor the standard library cannot pass without `unsafe`.
- **Decision and rationale**: The host marks each protocol line with an
  ASCII record separator; ttc reads only marked lines and passes anything
  else to stderr, where the user sees it. The host and the mapper are ttc's
  own processes, so the host is started without `NODE_OPTIONS`; a shim's
  own output is still handled by the marks.

### Decision C2: A sidecar is matched to the module it declares

- **Context**: With `outDir` or `declarationDir` in the configuration,
  `--types` wrote no sidecar and still reported success. The engine
  matched each declaration TypeScript emitted to a source by the path the
  declaration would have been written to (`x.tt.ts` → `x.tt.d.ts`), which
  those options move.
- **Decision and rationale**: The host emits one module at a time and
  names the module each declaration came from; the engine matches that
  name to the source's lowered module. Where TypeScript would have written
  it plays no part, so `outDir`, `declarationDir`, and `rootDir` no longer
  change which sidecars are written.

### Decision C6: Hand-written TypeScript is read as `tsc` reads it

- **Context**: A `.ts` input that was UTF-16 (with its byte order mark) or
  held bytes that are not UTF-8 stopped the build and `--check` with
  `stream did not contain valid UTF-8`; `tsc` reads both. Under
  `--check-types` a UTF-16 file's diagnostic quoted its raw bytes.
- **Decision and rationale**: TypeScript's `readFile` decodes a file that
  starts with a UTF-16 byte order mark as UTF-16; `lines::typescript_text`
  now does so as well, so diagnostics quote the decoded text. The build
  reads such a file in its own encoding and reads each other byte that is
  not UTF-8 as one Private Use Area character, and it encodes its output
  back the same way: a pass-through is the file's bytes, and a rewritten
  specifier (only ASCII changes) is written in the file's own encoding.
  Output records hold bytes that are not UTF-8 as an array of byte
  values, so a rebuild recognizes its own output. A `.tt` source must
  still be UTF-8; a file that is not UTF-8 and already holds those private
  characters is reported, since it cannot be read back exactly.

### Decision C3: The CommonJS support modules carry their own module type

- **Context**: A CommonJS importer (`import O = require("@tt/std/option")`
  in a `.cts`) in a package whose `"type"` is `"module"` got `tt/cjs/*.ts`,
  which Node and TypeScript (`nodenext`) read as ES modules there:
  TS1203 in the support modules, and `require` returned `undefined`.
- **Alternatives considered**: (a) Write the CommonJS form as `.cts`, which
  is CommonJS whatever the package says: the support sources would need
  `.cjs` specifiers and `<T,>` generic arrows (TS7060 in `.cts`), so they
  would no longer be the modules the rest of the build writes. (b) A
  `package.json` saying `"type": "commonjs"` beside them: Node's module
  format of a `.js` and TypeScript's of a `.ts` is the nearest
  `package.json`'s `"type"`, and the published `@tt/std` package used the
  same manifest for its `cjs/` copies.
- **Decision and rationale**: (b). The build writes
  `tt/cjs/package.json` with the CommonJS form, owned like the support
  modules. In the audit's project `tsc -p` (`nodenext`,
  `verbatimModuleSyntax`) now passes and `require` returns the value; as
  for any `package.json`, a `tsc` that compiles the tree elsewhere does not
  copy it.

### Decision C3b: An `.mts` output in a CommonJS package imports an ES-module copy

- **Context**: The mirror of C3: an `.mts` importer in a package without
  `"type": "module"` got `tt/option.ts`, which is CommonJS there, so the
  support module was TS1287 on every export under `verbatimModuleSyntax`.
  A `tt/package.json` saying `"type": "module"` would turn the support
  modules into ES modules for the package's own `.ts` outputs too, which
  `nodenext` compiles to CommonJS and which could then no longer import
  them.
- **Decision and rationale**: Chosen by the user. Only where the package
  is not `"type": "module"` and an `.mts` output imports a support module,
  the build writes an ES-module copy under `tt/esm/` with a manifest that
  says `"type": "module"`, and those outputs import it; every other output
  keeps `tt/`, so no other output changes. `tsc -p` (`nodenext`,
  `verbatimModuleSyntax`) passes on the audit's shape.

### Decision C10: A directory walk leaves out dot-files, as TypeScript's include does

- **Context**: The build compiled `src/.t.tt` and passed `src/.e.ts`
  through, while `--check-types` never checked them (TypeScript's include
  patterns skip names that start with a dot) and `--dependencies` listed
  them. Dot-directories were already left out by both. TASK-387 decision 1
  had kept hidden source files in the command line's walk.
- **Decision and rationale**: Chosen by the user. The walk of a directory
  input leaves out every name that starts with a dot, file or directory,
  as the project scan and TypeScript already do; a dot-file named on the
  command line is still an input. `source_walk_skips_excluded_names_before_following_links`
  now pins this, and TASK-387 says so at its top.

### Decision C11: A named file that is not a tt source is a command-line error

- **Context**: `--check-types x.ts` and `--types x.ts` exited 2 ("the check
  could not run") with `not a tt source`; help says an invalid command line
  exits 1, as a missing input already does. TASK-773 decision 16 keeps the
  refusal itself.
- **Decision and rationale**: The typed modes refuse a named file without a
  tt extension where they refuse a missing one, before anything runs, with
  the same message and exit 1. `tt_only_modes_name_the_file_and_the_extensions_they_accept`
  now pins the exit code.
  `types_that_cannot_check_exits_2_and_leaves_earlier_output` named a
  `.js` file to make the check unable to run; it now makes a tt source
  unreadable (invalid UTF-8) instead, which is still a pass that cannot
  run (exit 2).

### Decision C12: Observations kept as they are

- **A copied tree refuses its own outputs**: after `cp -r proj proj2`, a
  rebuild in `proj2` refuses outputs whose record names `proj/src/a.tt`,
  which still exists. This is the documented ownership rule (an output
  belongs to the input its record names while that input exists), and
  taking the outputs over would let one tree overwrite another's.
- **Depth 6000 of nested `match` aborts on allocation**: each level of a
  nested arm is indented, so the output itself grows quadratically (1.8 MB
  at depth 200, TASK-776 issue 2); at depth 6000 it is about 27 GB, which
  no change to the compiler's own bookkeeping can avoid.

## Work log

- 2026-10-07: Ran the sixth audit's command-line agent; reproduced C5,
  C8, C9, and C10 by hand.
- 2026-10-07: Fixed C8 (`src/engine/config.rs`), C5
  (`src/engine/project.rs`), C9 (`src/main/command.rs`,
  `src/main/typed.rs`), C7 (`src/main/typed.rs`), C1
  (`src/typescript/host.mjs`), and C4 (`src/typescript/host.mjs`,
  `src/typescript/native.rs`), each with a test run before and after its
  fix. Killed the orphaned `tsc --api` processes the hang had left in the
  scratch directories.
- 2026-10-07: Fixed C2 (`src/typescript/{host.mjs,backend.rs,native.rs}`,
  `src/engine/semantics/declarations.rs`, `src/engine/projection.rs`) and
  C6 (`src/main/{loading,build,ownership,output,out}.rs`,
  `src/ownership.rs`, `src/lines.rs`), each with a test run before and
  after its fix.
- 2026-10-07: Fixed C3 (`src/main/{build,ownership}.rs`, `src/stdlib.rs`,
  `src/lib.rs`), first as `.cts` support modules, which `tsc` rejected
  (TS7060, unresolved `./option.js`), then as a manifest; and the exit code
  of a named non-tt input (`src/main/command.rs`).

## Issues and resolutions

None.

## Regression test (fails before the fix)

Each test below was run on `d8da7233` with this task's tests copied in.

- **Path**: `tests/cli.rs::a_dangling_symlink_in_a_directory_input_is_skipped` (C5)
- **Observed failure**: `ttc: .../src/.#ok.tt: No such file or directory (os error 2)`.
- **Path**: `tests/cli.rs::contextual_input_walk_reads_past_a_dangling_link` (C5)
- **Observed failure**: `error[other]: .../aaa-broken: No such file or directory (os error 2)`.
- **Path**: `tests/engine_cache.rs::source_walk_skips_excluded_names_before_following_links` (C10)
- **Observed failure**: the walk listed `.hidden.tt` beside `ok.tt`.
- **Path**: `tests/cli.rs::watch_started_without_sources_builds_the_first_one_written` (C9)
- **Observed failure**: the watch exited (`Disconnected`) before a source was written.
- **Path**: `tests/cli.rs::a_diagnostic_two_projects_reach_is_reported_once` (C7)
- **Observed failure**: the `match-not-exhaustive` diagnostic of `a/a.tt` was printed twice.
- **Path**: `tests/native/cases_10.rs::a_circular_configuration_is_reported_rather_than_waited_on` (C1)
- **Observed failure**: "the check never finished for {"extends": "./tsconfig.json"}".
- **Path**: `tests/native/cases_10.rs::output_a_preload_or_a_node_shim_writes_does_not_break_the_check` (C4)
- **Observed failure**: "internal compiler error: the TypeScript backend answered with malformed JSON".
- **Path**: `tests/native/cases_10.rs::types_writes_sidecars_under_an_out_dir_or_a_declaration_dir` (C2)
- **Observed failure**: "no sidecar under "outDir": "dist", "rootDir": "src"".
- **Path**: `tests/cli.rs::a_typescript_file_that_is_not_utf8_passes_through_as_its_bytes` (C6)
- **Observed failure**: `ttc: .../src/plain.ts: stream did not contain valid UTF-8`.
- **Path**: `tests/cli.rs::commonjs_support_modules_carry_their_own_module_type` (C3)
- **Observed failure**: no `tt/cjs/package.json` was written (`NotFound`).
- **Path**: `tests/cli.rs::an_mts_output_in_a_commonjs_package_imports_an_es_module_copy` (C3b)
- **Observed failure**: the `.mts` output imported `./tt/option.js`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Complete. Every command-line finding of the sixth audit is fixed with a regression test, and the full gate passes.
