# TASK-465: Address a host TypeScript source as itself in the language service

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

On the JSON-lines server (`ttc --server`), `definition`, `references`, and `rename` with `path: plain.ts` answered locations in `plain.ts.ts`, a file that does not exist. A client applying that rename would create a new file. LSP 3.17 locations and workspace edits name the document by its URI, and the server must answer with the document the client asked about.

## Scope

- Included: How the engine's language service serves and addresses a hand-written `.ts`/`.tsx`/`.mts`/`.cts` source (`src/engine/language/service.rs`).
- Excluded: The VS Code adapter, which does not send host documents to the engine's semantic requests. Import-alias rename semantics (a local rename of `import { helper }` stays local, as TypeScript does and `editors/vscode/README.md` documents).

## Decisions

### Decision 1: A host source is served unprojected, under its own URI, with an identity mapping

- **Context**: `Project::serve` treated every requested path as a tt source: `service_doc` projected it through the tt compiler, `served_uri` named it `module_path_of(path)` (`plain.ts` + `.ts`), and `serve_one` opened that name in the service. The answer's URIs were `plain.ts.ts`, which `map_target` does not recognize as a lowered tt module, so it returned them as they were. The session already opens every host overlay under its own URI (`host_served`), so the service had two copies of the file.
- **Alternatives considered**:
  - Strip a `.ts.ts` suffix in `map_target`. That repairs the output only, still serves a second, projected copy of the file (with `.tt` import specifiers left as written but other passthrough identical), and asks about the copy instead of the file.
  - Reject semantic requests for host sources. Hover already worked, and a JSON-lines client editing a mixed project needs navigation in its `.ts` files too.
- **Decision and rationale**: `service_doc` returns an identity document for a host source (source equals code, one mapping over the whole text, no glue metadata), `served_uri` names a host source by its own path, and `serve_one` does not open a host source: an open buffer is already served through `host_served`, and a closed one is read from disk by the service. Every request then asks at the file's own URI, and every location in it comes back as the file itself; locations in `.tt` modules still map back through their projections.

## Work log

- 2026-09-28: Reproduced with the `ttc --server` driver: `rename`, `references`, and `definition` on `plain.ts` answered `plain.ts.ts`.
- 2026-09-28: Changed `service_doc`, `serve_one`, and `served_uri` in `src/engine/language/service.rs`.
- 2026-09-28: Checked a host file importing a `.tt` module: definition lands in `m.tt`, references include the `.tt` declaration, and the same answers hold after `closeDocument` (served from disk).
- 2026-09-28: Added `a_host_source_is_addressed_as_itself` (`tests/native/cases_03.rs`): definition, references, and rename on an open and on a closed host source, and definition from it into a `.tt` module.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`

## Result

Changed `src/engine/language/service.rs`, `tests/native/cases_03.rs`, this record, and `docs/tasks/INDEX.md`. Semantic requests on a host source answer in that file's own path.
