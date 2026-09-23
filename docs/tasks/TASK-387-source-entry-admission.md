# TASK-387: Classify excluded source entries before reading metadata

- **Status**: Complete
- **Started**: 2026-09-23
- **Completed**: 2026-09-23
- **Commit**: —

## Purpose

The CLI collector reads metadata for entries it promises to exclude. A dangling `node_modules` or dot-directory link can therefore fail an otherwise valid source walk.

## Scope

- Included: Shared entry admission for project and CLI source discovery, and filesystem regression coverage.
- Excluded: Explicitly named files, non-excluded unreadable entries, and language compilation.

## Decisions

### Decision 1: Exclude by entry name before inspecting its target

- **Context**: The project scan already skips excluded names before target metadata, while the CLI scan does it afterward.
- **Alternatives considered**: Ignoring all metadata failures would silently omit real inputs. Special-casing dangling links would make exclusion depend on the link target's state.
- **Decision and rationale**: Share one name predicate and apply it before file-type or metadata access in both scanners. Keep errors for admitted entries.

## Work log

- 2026-09-23: Read the discovery contract and ran `./scripts/doctor`; Rust is pinned and present, while Bun and project TypeScript are absent. Created this task before modifying implementation.
- 2026-09-23: Added a Unix regression for dangling excluded directory aliases. Against the original collector it failed with `NotFound` on `.cache`; after sharing the admission predicate and checking before metadata, it passed.
- 2026-09-23: Installed repository TypeScript with `npm ci --ignore-scripts` and temporary TypeScript 6/rolldown executables outside the repository. An initial gate passed formatting and Clippy but encountered a `rust-lld` undefined-hidden-symbol error when linking the existing target's test binary. Re-ran `./scripts/ci rust` in a fresh target directory with incremental compilation disabled; all stages passed.

## Issues and resolutions

### Issue 1: An excluded dangling directory alias aborts CLI discovery

- **Symptom**: A source tree with a dangling `node_modules` or dot-directory link fails discovery even though those names are excluded.
- **Cause**: `collect_sources_in` calls `metadata` before checking the child name.
- **Resolution**: Both scanners now use `excluded_source_entry` before probing child metadata. Non-excluded unreadable entries still report their paths.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: 1,250 passed; zero failed or ignored.
- [x] `./scripts/ci rust` with a fresh `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, and temporary TypeScript 6/rolldown executables on PATH; fuzz target check passed.
- [x] Existing CLI unreadable-entry regression passed, alongside the new source-walk regression.

## Result

Changed `src/engine/project.rs` to share early entry exclusion between the project and CLI scans, added a filesystem regression in `tests/engine_cache.rs`, and updated this task and the index. The regression was observed failing before the fix and passing afterward.
