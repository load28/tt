# TASK-476: `ttc explain` accepts the number `tsc` prints

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

Through the TypeScript content mapper, `tsc --runExternalCode` prints a tt diagnostic as `error tt27: match on variant A ... is not exhaustive`, because the mapper protocol carries a numeric code. `ttc explain tt27` and `ttc explain 27` answered "unknown diagnostic code", so the reader of a `tsc` log had no way from the code to the rule. `docs/ai/tt.md` §Contracts says each tt rule has a stable code that `ttc explain` prints.

## Scope

- Included: One numbered code table in the library (`src/diagnostics.rs`), used by the content mapper (`src/content_mapper.rs`) and by `ttc explain` (`src/main.rs`); tests; `docs/ai/tt.md`; the table name in `docs/design/content-mapper.md`.
- Excluded: The numbers themselves, which are published and do not change.

## Decisions

### Decision 1: Move the number table into `DiagnosticCode`

- **Context**: The numbers lived in `CODE_NUMBERS`, a private list of names in the binary's `content_mapper` module, looked up by string. `ttc explain` parsed names only. Two consumers of one fact need one owner.
- **Alternatives considered**:
  - Make `ttc explain` read `content_mapper::CODE_NUMBERS`. It would work, but the numbering is a property of a diagnostic code, not of one transport, and the library's `DiagnosticCode` is where every other consumer (server, engine, editor) gets code facts.
  - Add a `number` arm to `DiagnosticCode`'s exhaustive matches. The compiler would then force a number on every new code, but retired numbers have no variant, and the table's append-only order is the thing to protect.
- **Decision and rationale**: `NUMBERED_CODES` in `src/diagnostics.rs` is an append-only array of `Active(DiagnosticCode)` or `Retired(name)` slots, and the numbers are array positions from 1, as before. `DiagnosticCode::number` gives the number, `DiagnosticCode::lookup` accepts a name, `tt<number>`, or a bare number, and `DiagnosticCode::retired` names a retired slot. The mapper writes `diagnostic.code.number()`. Unit tests pin the published numbers (1, 27, 34, 35, 42, 50, retired 8 and 33) and require every active code to have a unique number.

### Decision 2: Say so for a retired number

- **Context**: Numbers 8, 9 and 33 belong to codes that are no longer reported but may still be in an old log.
- **Alternatives considered**: Report them as unknown. That hides that the number was once valid and suggests a typo.
- **Decision and rationale**: `ttc explain tt33` fails with `diagnostic code "tt33" (result-tail-semicolon) is retired and no longer reported`. `ttc explain` also accepts the whole `error tt27:` fragment, prints the number next to the name in its header (`error[match-not-exhaustive] (tt27)`), and lists each code with its number.

## Work log

- 2026-09-28: Reproduced `ttc explain tt27` and `ttc explain 27` failing, with the `tsc` output from a mapper project showing `error tt27`.
- 2026-09-28: Added `Numbered`, `NUMBERED_CODES`, `number`, `lookup` and `retired` to `src/diagnostics.rs`; removed `CODE_NUMBERS` and `code_number` from `src/content_mapper.rs`; switched `run_explain` in `src/main.rs` to `lookup` and `retired`.
- 2026-09-28: Tests: `code_numbers_are_stable_and_start_at_one` and `a_code_is_looked_up_by_name_or_number` (`src/diagnostics/tests.rs`), the mapper transform test now pins wire code 27 and its lookup (`src/content_mapper/tests.rs`), and `explain_accepts_the_number_tsc_prints` (`tests/cli/cases_01.rs`).
- 2026-09-28: Documented the numeric form in `docs/ai/tt.md` (Contracts and the content mapper setup) and renamed the table in `docs/design/content-mapper.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/diagnostics.rs`, `src/diagnostics/tests.rs`, `src/content_mapper.rs`, `src/content_mapper/tests.rs`, `src/main.rs`, `tests/cli/cases_01.rs`, `docs/ai/tt.md`, `docs/design/content-mapper.md`, this record, and `docs/tasks/INDEX.md`. A tt diagnostic's number from `tsc` now leads to its explanation through the same table the mapper numbers it with.
