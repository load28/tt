# TASK-306: Close the Result design and user-surface documentation

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-302: repair Result completion defects`

## Purpose

Repairs defect **—** in the shipped Result completion model. All of these
defects first ship in `a308c64` (#88) and are independent of any future language
proposal — they are wrong behaviour in released code, not design questions.

Severity: Documentation.

## Symptom

After the four repairs land, the design record and the user-facing surface still describe the pre-repair tree. `docs/design/try-result-scopes.md` names an obsolete baseline, and the language guide's Rust comparison and defect-sensitive examples need checking against repaired behaviour.

## Scope

- Included: Audit each listed surface against the repaired compiler. Refresh the guide only where its Rust comparison or its defect-sensitive examples are inaccurate. Run `./scripts/ci` and every applicable extension and site gate.
- Excluded: any change to the Result language model. This task does not revisit
  what `return` means, the success channel, or placement rules. A change to those
  is a change to `docs/design/try-result-scopes.md` first.

Files and symbols: `docs/design/try-result-scopes.md`, `docs/ai/tt.md`, CLI explanations, the content mapper, the JSON-lines server, grammar, completions, snippets, source-map snapshots, and the try/Result fixtures.

## Green condition

No shipped surface describes pre-repair behaviour, and no defect repair is hidden inside this documentation change.

## Decisions

Record every decision this task makes, including any new public diagnostic code
and any wire-compatibility choice, with its alternatives.

### Decision 1: Document the repaired implementation as shipped behavior

- **Context**: The design and language guide still described the pre-repair baseline and proposal state.
- **Alternatives considered**: Preserve historical wording, add a second repair note, or update the existing sources of truth.
- **Decision and rationale**: Keep history explicitly historical and make the current design and language guide describe the repaired implementation.

## Work log

- 2026-08-30: Began auditing the Result design baseline and every user-facing Result example against the repaired compiler.
- 2026-08-30: Updated the design and language guide, refreshed fixtures, and ran the complete repository gate including extension coverage.

## Issues and resolutions

- The extension gate initially selected a stale local release compiler; rebuilding the branch release exposed and verified the plan-less editor recovery path.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

The design record, language guide, fixtures, typed surfaces, and editor path now
describe and verify the same repaired Result behavior.
