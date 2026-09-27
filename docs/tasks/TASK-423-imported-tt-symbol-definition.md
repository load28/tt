# TASK-423: Find an imported variant's declaration the way the analysis resolved it

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttSymbol` answered a case or field in a pattern with `definition: null` in
three import situations, although the analysis had resolved the name to an
imported variant: an aliased import (`import { Shape as S }`), an earlier
import whose target does not exist, and an importing buffer that differs
from the file on disk.

## Scope

- Included: `imported_declaration` in `src/engine/names.rs` and the import
  resolution it shares with the analysis in
  `src/engine/language/service.rs`.
- Excluded: Reading the *imported* file from an open buffer. `ttSymbol` is a
  text-only request with no project (TASK-105 Decision 3), so the imported
  file is still read as last saved; that limit is documented on
  `tt_symbol_at`.

## Decisions

### Decision 1: Locate the declaration with the resolution the analysis used

- **Context**: The analysis resolves a pattern name against
  `externs_from`, which walks the buffer's imports, skips an unresolvable
  target with `continue`, and renames each declaration to the local name the
  import gives it (`S` for `Shape as S`, `ns.Shape` for a namespace).
  `imported_declaration` re-implemented that walk with three differences,
  one per symptom:
  1. It compared the declaration's own name with the last segment of the
     local name. For `Shape as S` the local name is `S`, which is neither.
  2. `canonical(...).ok()?` returned `None` from the whole function at the
     first import that does not resolve, where `externs_from` continues.
  3. It re-read the importing file with `std::fs::read_to_string(path)`
     instead of the `source` it was given, so an unsaved buffer's imports
     were not the ones the analysis had just used.
- **Alternatives considered**: Patching each of the three differences
  in place would keep a second copy of the name rule that can drift from the
  analysis again.
- **Decision and rationale**: `externs_from` is now a projection of
  `imported_variants`, which returns each imported declaration together with
  the file that declares it. `imported_declaration` takes the buffer's
  `source`, runs `imported_variants` over the buffer's imports, and picks the
  entry whose local name equals the analysis' `DeclaredVariant::name`. The
  declaration file's text is kept from the same read the symbols came from,
  so the offsets and the text cannot disagree. One resolution rule now serves
  sema, the editor analysis and navigation, as the module documentation of
  `names.rs` already claimed.

## Work log

- 2026-09-27: Reproduced all three through `ttc --server` `ttSymbol`
  requests on `Circle(r)` in a match: each case and field answered with
  `definition: null`.
- 2026-09-27: Split `imported_variants` out of `externs_from`, rewrote
  `imported_declaration` on it, and passed the buffer text through
  `case_definition` and `field_definition`.
- 2026-09-27: Added
  `an_imported_case_is_found_under_the_name_the_buffer_imports_it_by`
  covering an alias, a missing earlier import and a namespace import, each
  in a buffer whose file on disk has no imports at all.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] The new test fails with the previous `imported_declaration`
  (no definition for the aliased import) and passes with the change.
- [x] `ttc --server`: each of the three cases now answers `shapes.tt` 0:23
  for the case and 0:30 for the field.

## Result

Changed `src/engine/language.rs`, `src/engine/language/service.rs` and
`src/engine/names.rs`.
