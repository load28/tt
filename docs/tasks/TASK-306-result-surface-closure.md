# TASK-306: Close the Result design and user-surface documentation

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

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

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
