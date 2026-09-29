# TASK-576: Read imported variants from the open buffer in tt's name surfaces

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-576: Read imported variants from the open buffer in tt's name surfaces`

## Purpose

With `st.tt` saved as `export variant St { A, B }` and `C` added to its
open, unsaved buffer, `use.tt`'s diagnostics said `missing "B", "C"` and
hover on `St` showed `C`, but pattern completion offered `A B _`, and hover
and definition on a written `C =>` arm returned null until `st.tt` was
saved. Every surface must see the same text of a file.

## Scope

- Included: where the parse-only tt surfaces (`ttSymbol`, `ttCompletions`,
  `ttHints`, `declarations`) and the project's tt-name lookups inside
  references read an imported `.tt` file.
- Excluded: the typed pass and the project's semantic analyses, which
  already read the shared store (TASK-536).

## Decisions

### Decision 1: A session's parse-only questions read its open documents

- **Context**: `analyses_for` (`src/engine/language/service.rs`) collected
  imported declarations with `std::fs::read_to_string`, and so did
  `imported_declaration` (`src/engine/names.rs`, the definition of an
  imported case or field) and `tt_declarations`. These surfaces take the
  asked buffer's text as a parameter and need no project, so the server
  called them as free functions without its workspace; the `.tt` files the
  buffer imports were read from disk. TASK-536 made the engine's
  `Documents` the text of an open file for every project, but these
  surfaces never went through a project.
- **Alternatives considered**:
  - Have the extension send the imported buffers' text with each request.
    The server already holds them; a second copy per request is a second
    source of truth that can disagree with the one the diagnostics use.
  - Route these requests through the file's `Project`. They would open a
    project (and its TypeScript session) for a question that needs
    neither, and fail where no project can be opened.
- **Decision and rationale**: The surfaces take where to read a file they
  were not handed (`documents::Texts`): `Open(&Documents)` reads the
  session's buffer over the disk, `Disk` reads the disk. `Workspace` answers
  `tt_symbol_at`, `tt_completions_at`, `tt_hints` and `tt_declarations`
  through its engine's store, and the server's four requests ask the
  workspace. The public free functions stay the stand-alone question with
  no session and read the disk, as before. A project's own lookups (the tt
  declaration and pattern references behind Find All References) read its
  store handle, the same text its projections use.

## Work log

- 2026-09-29: Reproduced with `target/probe4-editor/s6.cjs` through the
  language server: after the unsaved edit, completion `A B _`, hover and
  definition on `C` null; after saving, `A B C _` and the hover.
- 2026-09-29: Added `Documents::text` and `Texts` (`src/engine/documents.rs`),
  threaded `Texts` through `analyses_for`, `names::symbol_at`,
  `tt_pattern_references`, `case_definition`/`field_definition`/
  `imported_declaration`, `completions::completions_at`, `hints::hints` and
  `declarations::declarations`; added the four `Workspace` methods; the
  server's `ttSymbol`, `ttCompletions`, `ttHints` and `declarations` ask
  the workspace. `Project::text_of` reads through `Documents::text`.
- 2026-09-29: The probe now answers `A B C _`, hover `St.C` and the
  definition at `st.tt` 0:26 before saving.
- 2026-09-29: Added
  `tt_names_read_an_imported_declaration_from_its_open_buffer`
  (`tests/native/cases_10.rs`): completion, `ttSymbol`, `declarations` and
  `ttHints` after `openDocument` of the edited `st.tt`, and `ttSymbol` after
  `closeDocument`. With the workspace reading the disk it fails
  (`["A", "B", "_"]`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib engine`, `cargo test --test native tt_names_read`
- [x] Full suites with TASK-579 (see that record).

## Result

Changed `src/engine/documents.rs`, `src/engine/language.rs`,
`src/engine/language/service.rs`, `src/engine/language/project.rs`,
`src/engine/language/tests.rs`, `src/engine/names.rs`,
`src/engine/completions.rs`, `src/engine/hints.rs`,
`src/engine/declarations.rs`, `src/engine/workspace.rs`, `src/server.rs`,
`tests/native/cases_10.rs` and `docs/design/engine-architecture.md`.
Pattern completion, tt hover and definition, hints and the declaration
table see an imported variant's unsaved cases as soon as the diagnostics
do.
