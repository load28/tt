# TASK-542: Place sidecar map segments from declaration syntax

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

The `.d.ts.map` that `--types` and `--sidecar` write sent "go to
definition" to the wrong place. For a module with an inner local
`const total = 1` inside a function and a later `export const total = 2`,
`total` mapped to the inner local; `port` mapped to a column inside
`export`, and `ex` to column 0. The generated side had the same fault:
`export declare let a` put the segment for `a` on the `a` in `declare`.
`build_sidecar` documents a segment at the column where each
declaration's name starts.

## Scope

- Included: How `build_sidecar` (`src/sidecar.rs`) finds each exported
  declaration's name in the declaration text and its declaration in the
  source; a declaration-file parser on `HostInput`.
- Excluded: The declaration text, which stays the compiler's; the map's
  file names and line handling (TASK-498).

## Decisions

### Decision 1: Both sides are read as syntax

- **Context**: Both positions came from text scans. The generated side
  stripped keywords off a line and took `line.find(name)`, the first
  textual occurrence. The source side took the first line of any
  indentation that started with a declaration keyword and named the name,
  then again `line.find(name)`.
- **Alternatives considered**: Tightening the scans (word boundaries,
  unindented lines only) keeps guessing at syntax from text and still
  mislocates a declaration that spans lines, has several declarators, or is
  indented at module level.
- **Decision and rationale**: The declaration file is parsed with SWC as a
  `.d.ts` (`HostInput::declaration_parser`), and every identifier its
  exported declarations bind is taken with its own span, several per line
  where a line declares several names. The source is tt, so it is read
  through the TypeScript ttc emits for it (`emit_mapped_with_kind`): the
  emitted module's module-level declarations are parsed, and each
  identifier is placed where the emission took it from — the copied chunk's
  source bytes (`EmitMapping`) or, for a name ttc declared in glue such as a
  variant's type and constructor, the tt name it was declared for
  (`DeclaredName`). A name declared more than once maps to its first
  declaration in the source. Only module-level declarations count, so a
  local in a function body can never be chosen.

### Decision 2: One segment per declared name

- **Context**: A declaration line such as `export declare const b = 1, c`
  declares two names; the map had one name segment per line.
- **Decision and rationale**: Each located name gets its own segment at its
  column, and the line keeps one column-0 segment for its first name, so
  the map still starts every mapped line at column 0.

## Work log

- 2026-09-29: Reproduced with `src/loc.tt` through `--types`: `total`
  pointed at line 2, `port` at a column inside `export`, `ex` at column 0.
- 2026-09-29: Rewrote the location half of `src/sidecar.rs`
  (`exported_declarations`, `module_declarations`, `source_byte`,
  `declared_identifiers`, `encode_mappings` over per-line hits) and added
  `HostInput::declaration_parser` in `src/host_input.rs`.
- 2026-09-29: Added
  `a_name_maps_to_the_module_level_declaration_that_exports_it` and
  `every_name_a_declaration_line_declares_gets_its_own_segment` to
  `tests/sidecar.rs`; both fail before the change. The existing sidecar
  cases, including every line terminator and a byte-order mark on both
  sides, pass unchanged. `--types` on the reproduction now maps `total`,
  `port`, `ex`, `a` and a variant to their names.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test sidecar --test cli_outputs --test cli --test native --test integration`

## Result

Changed `src/sidecar.rs`, `src/host_input.rs` and `tests/sidecar.rs`.
Sidecar maps place every exported name at its own declaration in the
source and at its own column in the declaration file.
