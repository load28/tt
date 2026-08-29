# TASK-277: Diagnose ambiguous result tails without unsafe edits

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: `TASK-277: diagnose ambiguous result tails safely`

## Purpose

Report a missing `result` value without claiming that deleting a final
semicolon is always a safe repair.

## Scope

- Included: structural labeled-statement classification, diagnostic message and span, regression snapshots
- Excluded: changing `result` block semantics or the stable public diagnostic code

## Decisions

### Decision 1: Preserve the public diagnostic identity

- **Context**: `result-tail-semicolon` is exposed through CLI, server, explain, and content-mapper contracts.
- **Alternatives considered**: Rename it to `result-missing-value`; retain the stable code and correct its message and applicability.
- **Decision and rationale**: Keep `DiagnosticCode::ResultTailSemicolon` and `result-tail-semicolon`. The behavior can become more accurate without an unnecessary protocol break.

### Decision 2: Separate proof from ambiguity

- **Context**: A final labeled statement is structurally a statement, while `log(x);` may have been intended as either a statement or the block value.
- **Alternatives considered**: Continue issuing a machine-applicable deletion; suppress the diagnostic; emit non-applicable guidance only when intent is ambiguous.
- **Decision and rationale**: Diagnose both as a missing value. Give ambiguous expression statements prose help represented as a suggestion with no edit, and give labeled statements no deletion guidance.

## Work log

- 2026-08-29: Reviewed the proposal and amended it to preserve the wire code and distinguish non-applicable help from an empty suggestion list.
- 2026-08-29: Replaced the parser's value assertion with a structural missing-value attempt carrying an explicit ambiguity fact.
- 2026-08-29: Kept `result-tail-semicolon`, changed its message, removed automatic edits, and added labeled-statement and call-statement regressions.
- 2026-08-29: Regenerated the diagnostic snapshot. The reviewed diff keeps the semicolon span and stable code, changes the message, and changes the JSON suggestion edit to `null`.
- 2026-08-29: Re-ran `result { const x <- parse("1"); log(x); }`. Before, the diagnostic supplied a deletion edit; after, it supplies prose guidance with no edit. The labeled form now spans `label: doWork();` and supplies no deletion guidance.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Claimed `result` blocks now diagnose the actual missing-value contract without guessing user intent. Ambiguous expression statements receive non-applicable guidance, labeled statements receive none, and the stable diagnostic protocol remains unchanged.
