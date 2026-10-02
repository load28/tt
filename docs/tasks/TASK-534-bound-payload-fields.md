# TASK-534: Leave already-bound fields out of payload completion

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Completion after `Rect(w, ` offered `w` again. A TypeScript developer
completing `const { w, | } = rect` is offered only the properties the
pattern does not bind yet.

## Scope

- Included: The payload-field context of tt pattern completion
  (`src/engine/completions.rs`) and its unit tests.
- Excluded: The rule that a pattern may bind one field twice under an alias
  (`Rect(w, w: v)`), which compiles as the destructuring `{ w, w: v }` does
  and stays valid.

## Decisions

### Decision 1: The field context carries the fields its payload already writes

- **Context**: `Context::Field` named only the case, so every field of the
  case was offered at every entry.
- **Alternatives considered**: Filtering in the editor would need the
  pattern grammar the engine already owns.
- **Decision and rationale**: `Context::Field` carries `written`: each name
  at the payload list's own level that starts an entry (after the list's `(`
  or a `,`), skipping the name being typed and stopping at the list's `)`
  or, while it is unclosed, at the arm's `=>`. `written_fields` reads the
  same token stream the context is computed from. An alias (`w: width`)
  and a nested pattern (`w: Some(v)`) bind `w`, since the entry starts with
  it.

## Work log

- 2026-09-29: Reproduced with the editor probe's payload completion. Added
  `written`, `written_fields`, and
  `a_payload_position_leaves_out_the_fields_already_bound`.
- 2026-09-29: Five existing unit assertions probed the field context at
  `Rect(w, ` and expected `["w", "h"]`; TASK-375, which wrote them, verified
  that the context is recognized and recorded no decision to repeat bound
  fields. They now expect `["h"]`.

## Issues and resolutions

### Issue 1: Two LSP cases expected the bound field

- **Symptom**: "pattern positions complete cases and fields" expected
  `["w", "h"]` at `Rect(` in `Rect(w, h)`, and "pattern completion handles
  delimiter triggers and incomplete prefixes" expected `name` after
  `Admin(name,`.
- **Cause**: Both asserted the previous answer. At `Rect(` the name under
  the cursor (`w`) is the one being replaced and `h` is bound.
- **Resolution**: They expect `["w"]` and `["level"]`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests, 224 passed

## Result

Changed `src/engine/completions.rs` and
`editors/vscode/server/src/test/server.test.ts`. Payload completion offers
only the fields a pattern has not bound.
