# TASK-360: Give unsaved files full project type services

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-360: fix(editor): type new files before save`

## Purpose

Audit the compiler/editor type boundary after TASK-359 and make a newly created
file participate in its project before its first disk write.

## Scope

- Included: document path identity, server project routing, typed overlays, and
  editor/engine regression coverage for new `.tt` and `.ttx` files
- Excluded: untitled buffers without a filesystem project identity and new
  language syntax

## Decisions

### Decision 1: Separate document identity from file existence

- **Context**: Editor buffers have a stable filesystem URI before the file is
  first saved, but the engine protocol requires that path to canonicalize.
- **Alternatives considered**: Keep typed features unavailable until save;
  create a temporary file; or canonicalize the existing parent and preserve the
  new leaf as the document identity.
- **Decision and rationale**: Canonicalize the parent directory and retain the
  leaf. This gives the overlay a stable project-relative identity without
  writing user data or weakening canonical identity for existing files.

## Work log

- 2026-09-10: Ran `./scripts/doctor`; the pinned environment was ready.
- 2026-09-10: Updated local `main` to `fd03025` and opened a fresh audit branch.
- 2026-09-10: Traced new-file requests from the VS Code document lifecycle
  through the JSON-lines server and found file-existence checks at both project
  identity and typed-check boundaries.
- 2026-09-10: Added one document-path normalizer at the engine boundary,
  routed registered documents by their stored project identity, and made every
  typed request include its overlay as an explicit program root.
- 2026-09-10: The first full Rust run exposed candidate/requested-set leakage;
  corrected document project construction and retained the existing tsconfig
  exclusion regression.
- 2026-09-10: Added native server and VS Code client regressions, updated the
  editor contract, and ran the complete local CI gate.
- 2026-09-11: Merged the latest `main`, preserved TASK-360 through TASK-362 in
  the task index, and reran the complete local CI gate.

## Issues and resolutions

### Issue 1: A new file has no typed project until its first save

- **Symptom**: A file URI whose parent exists but whose leaf is not yet on disk
  cannot register an overlay, so typed diagnostics and language-service answers
  are unavailable for the initial editor buffer.
- **Cause**: Document routing uses `Path::canonicalize` on the file itself, and
  the typed check includes the overlay only when the project scan is empty.
- **Resolution**: Canonicalize the existing parent, retain the unsaved leaf,
  route its requests through the registered project, and include the overlay in
  every typed snapshot without writing it to disk.

### Issue 2: Document project discovery reported excluded files

- **Symptom**: The first implementation made a typed editor request report a
  malformed `.tt` file outside the project's `tsconfig.json` include set.
- **Cause**: Document project opening used every scanned candidate as an
  explicitly requested file, collapsing two sets with different ownership.
- **Resolution**: Keep the document alone in the requested set and let the
  existing project scan populate candidates for TypeScript to filter.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Native TypeScript backend: 68 passed
- [x] VS Code server/client: 170 passed
- [x] `./scripts/ci`: agents, Rust, npm, website, native, and extension passed

## Result

Changed the engine's document identity and project-routing boundary, the server
overlay lifecycle, the VS Code typed-check client, native/editor regressions,
and the editor documentation. New `.tt` and `.ttx` files now receive project
type information before their first save while excluded project candidates
remain excluded.
