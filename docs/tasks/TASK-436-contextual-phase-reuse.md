# TASK-436: Reuse contextual projections and checker answers across a project's files

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: this commit (`TASK-436: Reuse contextual projections and checker answers across files`)

## Purpose

The contextual phase (`typescript::contextual::standalone`) was quadratic
across a project: every file with a contextual value slot re-projected every
`.tt` file of the project and re-ran the whole checker materialization. Through
the content mapper (`tsc --runExternalCode`) 30 files took 5.0 s, 60 took
16.1 s, 120 took 63.6 s, and 300 took 8 min 31 s.

## Scope

- Included: reuse of candidate projections and of the checker materialization
  inside one process (the content mapper, `ttc` builds, `ttc --watch`), with an
  exact invalidation rule; a disk-generation probe in the TypeScript host; a
  deterministic regression guard.
- Excluded: the project set itself. Every file `collect_sources` finds is still
  projected and served, and one-shot processes (`ttc -p`, the unplugin adapter,
  which spawns `ttc -p` per file) still project the project once per process
  (Decision 1).

## Decisions

### Decision 1: Keep the whole project in every materialization

- **Context**: The report suggested serving only the files the program reaches
  through imports.
- **Alternatives considered**: Serve the import closure of the requested file.
- **Decision and rationale**: The host's layered file system lists served
  modules in directory listings, so a `tsconfig.json` glob admits every `.tt`
  file as a program root, and an unimported file can still change the
  requested file's types (`declare global`, module augmentation). Serving only
  the import closure would change annotations, and TASK-357 (Decision 1)
  forbids inferring over a partial snapshot. The set stays complete; the work
  per file is what changed. A one-shot process therefore remains linear in the
  project size (300 files: 1.07 s against 0.28 s for 10, debug); removing that
  would need a cache that outlives the process, which is a separate decision.

### Decision 2: Reuse projections by path and exact source text

- **Context**: Each request compiled every sibling from scratch.
- **Alternatives considered**: Key by modification time; share one cache
  across threads.
- **Decision and rationale**: The pass still walks and reads every candidate
  on every request (the mapper outlives edits), and reuses a projection only
  when the text read now equals the text it was projected from. The cache is
  thread-local beside the existing backend, is rebuilt from the current walk on
  every request (so deleted files leave it), and is dropped when the backend's
  root changes. A projection is a function of its text and fixed options, so
  reuse returns what recompilation would.

### Decision 3: Reuse a materialization only for the same question over an unchanged disk

- **Context**: `materialize` already annotates every served module's slots,
  and the next file's request asks the same question again.
- **Alternatives considered**: Key only on the served modules (stale when a
  hand-written `.ts` dependency changes under `--watch` or the mapper);
  fingerprint dependency files from Rust (misses changes the host observes
  between rounds).
- **Decision and rationale**: Modules are served in path order, so every
  request of an unchanged project asks the literally identical question (the
  requested file used to be first; the host keys served files by path and the
  program's roots come from the configuration, so order carries no meaning;
  the inferred configuration still names the requested file). The host now
  counts a disk generation: it increments whenever its existing dependency and
  directory-listing check finds a change, reports the generation with every
  answer, and answers a `{ "diskGeneration": true }` probe by running the same
  check without touching the snapshot (found changes are held and applied by
  the next `ask`). A materialization is kept only when every one of its rounds
  saw the same host session and generation, and it is reused only when the
  configuration, root, support modules, and served modules are equal and a
  probe reports that same session and generation.

## Work log

- 2026-09-27: Reproduced with generated projects (`src/fN.tt` importing
  `f(N-1)`, one `const v = match …` each) through `tsc --runExternalCode` and
  `ttc -p`; timed the walk-and-project and checker parts separately.
- 2026-09-27: Added the projection and materialization reuse
  (`src/typescript/contextual.rs`), the generation probe
  (`src/typescript/host.mjs`, `src/typescript/native.rs`,
  `src/typescript/backend.rs`), and the guard in `src/lib/scaling_tests.rs`.
- 2026-09-27: Compared `ttc -p` of every file and `ttc -j 1`/`-j N` builds of a
  60-file project and of a project with variants, cross-directory imports,
  object and array unions, a hand-written `.ts` dependency, `declare global`,
  and `await`, before and after: byte-identical.

## Issues and resolutions

### Issue 1: Reuse keyed on served modules alone kept stale types

- **Symptom**: After editing a hand-written `.ts` file an annotation depends
  on, a reused materialization still carried the old type.
- **Cause**: The checker also reads disk files that are not served modules.
- **Resolution**: Decision 3's host generation; the regression test edits
  `dep.ts` and requires the same output as a fresh process. Removing the
  generation check makes that test fail.

### Issue 2: The first probe failed with a temporal-dead-zone error

- **Symptom**: `Cannot access 'diskGeneration' before initialization`.
- **Cause**: The request loop runs before the later `let` declarations in
  `main` are initialized.
- **Resolution**: Declared the generation state beside the other host maps.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

`project_files_share_one_projection_each_and_one_checker_materialization`
compiles six project files in one thread: the first request projects the five
siblings and asks the checker; the next five requests project one file each
(the previous request's own file) and ask the checker nothing; every output
equals a fresh thread's; after a dependency edit the checker is asked again and
the output again equals a fresh thread's.

`tsc --runExternalCode` through the content mapper (debug `ttc`):

| Files | Before | After |
| --- | --- | --- |
| 30 | 2.94 s | 0.59 s |
| 60 | 9.85 s | 0.91 s |
| 120 | 38.6 s | 1.69 s |
| 300 | — (8 min 31 s reported) | 4.59 s |

`ttc -p` of one file (debug): 10-file project 0.47 s → 0.28 s, 300-file project
1.23 s → 1.07 s (Decision 1).

## Result

Within one process, each project file is projected once per distinct text and
the checker materializes once per distinct project state; annotations are
unchanged. One-shot processes remain linear in the project size by design
(Decision 1).
