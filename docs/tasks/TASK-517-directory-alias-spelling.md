# TASK-517: Walk a symlinked directory under its link-free spelling

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-517`

## Purpose

With `src/lib/x.tt`, `src/main.tt` importing `./lib/x.tt`, and a symlink
`src/@lib -> lib`, `ttc -o out src` wrote `out/@lib/x.ts` and no
`out/lib/x.ts`, so `out/main.ts` imported a file that did not exist. The
alias `zlib` produced `out/lib/x.ts`: the result depended on how the alias
sorted against the real directory.

## Scope

- Included: which spelling the CLI source walk (`collect_sources` in
  `src/engine/project.rs`) keeps for a directory it reaches more than once,
  and a CLI regression test.
- Excluded: the directory identity model of TASK-386 (each canonical
  directory is still walked once per walk, cycles still end), file symlinks
  (each file name is its own source), and the project candidate scan, which
  canonicalizes every file and has no spelling to choose.

## Decisions

### Decision 1: The link-free spelling of a directory wins; otherwise the first alias

- **Context**: TASK-386 Decision 1 admits each canonical directory once and
  keeps "logical CLI file paths and deterministic sorted child traversal".
  The first spelling in sorted order took the identity, so an alias sorting
  before the real name took the real directory's files. The CLI mirrors the
  collected spelling under `-o`, and relative imports name the real path.
- **Alternatives considered**:
  - Walk every alias as its own tree (the model for file symlinks). The
    build then sees one source under two spellings; `compile_jobs` rejects
    that as "one input claims two outputs", so any project with such a
    link would stop building. It also reopens the cycle problem TASK-386
    closed.
  - Sort symlinked entries after real ones in each listing. That only
    covers an alias and its target in the same directory; an alias at
    `src/@a -> b/c` still sorts before the walk reaches `src/b/c`.
  - Two passes that rank every spelling by the number of links it follows.
    Same result for these inputs, with a second listing of every directory.
- **Decision and rationale**: when the walk meets a directory that is a
  symlink, `alias_of_walked_directory` asks whether its target is reached
  from the walk root without following a link: the canonical target lies
  under the canonical root, and no component of the path from the root to
  it is a symlink or an excluded entry (dot-directory, `node_modules`). Such
  an alias is skipped, and the target is walked under its own name, wherever
  the alias sorts. A target outside that link-free walk is reached only
  through aliases, and the first one in sorted order still takes it, as
  before. The checks cost one `lstat` per symlinked directory and one per
  path component of its target. Sorted traversal order, error reporting and
  the visited set are unchanged.

## Work log

- 2026-09-29: Reproduced with `@lib` (wrong) and `zlib` (correct).
- 2026-09-29: Added `alias_of_walked_directory` and threaded the walk root
  through `collect_sources_in`. Added
  `a_directory_alias_does_not_move_the_real_directory_outputs` to
  `tests/cli.rs`: aliases sorting before and after `lib`, and an alias to a
  directory outside the input, which must still be collected. The test fails
  without the change. `tests/engine_cache.rs` (self, ancestor and external
  aliases from TASK-386) passes unchanged.

## Issues and resolutions

### Issue 1: A directory alias moved the real directory's outputs

- **Symptom**: `out/@lib/x.ts` instead of `out/lib/x.ts`; `out/main.ts`
  imports `./lib/x.js`.
- **Cause**: `SourceDirectories::enter` admits the first spelling it sees
  for a canonical directory, and sorted traversal visits `@lib` before `lib`.
- **Resolution**: an alias of a directory that the link-free walk reaches is
  never entered (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test cli`, `cargo test --test engine_cache`
- [x] `node scripts/check-task-index`

## Result

Changed `src/engine/project.rs` and `tests/cli.rs`.
