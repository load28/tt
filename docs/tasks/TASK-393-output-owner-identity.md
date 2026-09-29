# TASK-393: Make output owner identity cwd-independent and symlink-canonical

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Output ownership records (TASK-383) identify the owner of each written file.
Two identities were wrong: the `@tt/std` support modules were owned by a
cwd-relative path, and source owners were compared lexically, so ttc refused
to overwrite its own untouched outputs.

## Scope

- Included: The owner identity written to and compared against
  `.name.ts.ttc-output.json` records (`src/main/ownership.rs`) and the callers
  in `src/main/build.rs`.
- Excluded: The record content check (edited outputs are still refused), the
  input-overwrite guard (`same_file`), and output placement.

## Defects

1. `ttc -o out src` run from `p/` and then `ttc -o p/out p/src` run from the
   parent refused `p/out/tt/index.ts`: "output is not owned by this input or
   has been edited". The support owner was `PathBuf::from("@tt/std/<file>")`,
   which `normalized_absolute` joins with the current directory, so the
   recorded identity changed with the cwd.
2. After `ttc real`, running `ttc link` (where `link -> real`) refused
   `link/a.ts`, because the recorded `real/a.tt` and the new `link/a.tt` were
   compared as lexical absolute paths although they name the same file.

## Decisions

### Decision 1: Model the owner as a typed identity, not a path

- **Context**: A support module is not a file in the user's tree. Encoding it
  as a relative path made it subject to path absolutization.
- **Alternatives considered**: Absolutizing against the output directory
  would still couple the identity to layout. Recognizing records whose source
  path ends in `@tt/std/<file>` would be string-shape matching.
- **Decision and rationale**: `OutputOwner::{Source(&Path), Support(StdModule)}`.
  Support records store `"support": "@tt/std/<file>"`, which is the
  compiler-owned module name and has no filesystem interpretation. The
  input-overwrite guard applies only to source owners, as before.

### Decision 2: Identify a source by its canonical path

- **Context**: POSIX pathname resolution (IEEE Std 1003.1, 4.13 "Pathname
  Resolution") resolves symbolic links, so `link/a.tt` and `real/a.tt` name the
  same file. `std::fs::canonicalize` (Rust std docs: "Returns the canonical,
  absolute form of a path with all intermediate components normalized and
  symbolic links resolved") is the platform's statement of that identity.
- **Alternatives considered**: Comparing device and inode numbers is
  Unix-only and cannot be stored across rebuilds that replace the file.
- **Decision and rationale**: Records store the canonical source path, and a
  comparison canonicalizes both the recorded and the current source, falling
  back to the lexical absolute path only when the path cannot be resolved.
  Canonicalizing the recorded side keeps records written by earlier versions
  (lexical paths) valid.

### Decision 3: Keep records written by earlier versions valid

- **Context**: Existing output trees hold `version: 1` records whose support
  owner is the old cwd-joined path.
- **Decision and rationale**: A support owner also accepts a record whose
  `source` equals the identity earlier versions computed under the current
  directory — exactly the old acceptance rule, so no previously accepted
  rebuild is refused. New writes replace the record with the `support` form.
  The record version stays 1 because readers (`owned_output`) only depend on
  `version` and `content`.

## Work log

- 2026-09-27: Reproduced both defects with the investigation material
  (`p2`, `p3`). Added `tests/cli_outputs.rs` with
  `support_module_ownership_does_not_depend_on_the_working_directory` and
  `a_symlinked_source_path_owns_the_outputs_of_its_target`; both failed
  against the original code with the "not owned by this input or has been
  edited" error.
- 2026-09-27: Introduced `OutputOwner`, canonical source identity, and support
  identity in `src/main/ownership.rs`; updated the five callers in
  `src/main/build.rs`. Both tests passed, and the symlink test still refuses an
  edited output reached through the link.
- 2026-09-27: Added `support_records_written_before_owner_identities_remain_owned_from_their_directory`
  to pin compatibility with earlier records. Updated the ownership bullet in
  `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: `./scripts/doctor` reports missing setup products

- **Symptom**: doctor reports a missing `target/release/ttc`, npm launcher
  marker, and VSIX in this worktree.
- **Cause**: The worktree was never set up; these are setup products, not
  prerequisites for the Rust gate.
- **Resolution**: Per AGENTS.md, setup was not run. `npm ci` installed the
  pinned TypeScript, and `cargo build`/`cargo test` were used directly.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/main/ownership.rs`, `src/main/build.rs`, `tests/cli_outputs.rs`,
and `docs/ai/tt.md`. Rebuilding into an existing output tree from any working
directory, or through a symlinked source path, now reuses ttc's own outputs
while edited or authored outputs remain refused.
