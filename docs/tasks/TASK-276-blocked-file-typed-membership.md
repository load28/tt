# TASK-276: Preserve typed membership for blocked configured files

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: `TASK-276: preserve blocked project membership`

## Purpose

Ensure that TypeScript's configured program remains the authority for whether a
projection-blocked `.tt` file belongs to a typed check.

## Scope

- Included: placeholder modules for blocked files, typed membership mapping, native CLI and server regressions
- Excluded: Rust-side tsconfig glob interpretation and unrelated checker-cascade policy

## Decisions

### Decision 1: Represent blocked files in the backend candidate set

- **Context**: A blocked file cannot enter `projectModules` because it currently has no served module.
- **Alternatives considered**: Reimplement tsconfig matching in Rust; add a separate backend membership protocol; serve a minimal placeholder.
- **Decision and rationale**: Serve `export {};` under the normal projected module path. TypeScript then applies its existing files/include/exclude and import-resolution rules without leaking backend concepts or duplicating configuration semantics in Rust.

### Decision 2: Keep projection and membership distinct

- **Context**: Blocked files have diagnostics but cannot provide probes or meaningful generated code.
- **Alternatives considered**: Fabricate a partial projection; omit blocked candidates.
- **Decision and rationale**: Placeholders participate only in configured-program membership. They generate no probes, and declaration matching remains limited to real projected documents.

## Work log

- 2026-08-29: Reviewed the verified reproduction and selected the placeholder-module seam as the structural ownership boundary.
- 2026-08-29: Added placeholder candidates in `src/engine/projection.rs`, connected blocked snapshots in `src/engine/project.rs`, and mapped configured blocked modules in `src/engine/semantics.rs`.
- 2026-08-29: Added native regressions for included independent files, excluded independent files, imported outside files, and the typed server path.
- 2026-08-29: Re-ran the reported single-file reproduction. Before this change it exited 0 without naming `src/orphan.tt`; after this change it exits 1 and reports `src/orphan.tt` in both CLI and typed-server results.

## Issues and resolutions

### Issue 1: Formatting gate rejected two new expressions

- **Symptom**: The first `./scripts/ci rust` stopped at `cargo fmt --check`.
- **Cause**: The new iterator formatting did not match rustfmt output.
- **Resolution**: Ran `cargo fmt`, reviewed the formatting-only diff, and reran the full Rust gate successfully.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Blocked files now participate in TypeScript's configured-program membership through side-effect-free placeholder modules. Included and import-reached failures are reported without admitting excluded files or fabricating Rust-side tsconfig rules. The accepted checker consequence for an importer may change from TS2307 to TS2305, while the originating tt diagnostic remains authoritative.
