# PR brief: isolate server response projection

Implementation task: TASK-741. Parent program: TASK-740.

## Problem and intended structure

`src/server.rs` mixes transport, dispatch, project/session lifetime, and the
conversion of already-computed answers to wire JSON. Its six existing pure
projection helpers form a narrow boundary that can be extracted without
changing any engine decisions or request sequencing.

Keep `src/server.rs` as the caller. Add private `mod responses;` backed by
`src/server/responses.rs`. Import the six helpers explicitly. The child module
depends only on `std::path::Path`, engine answer types, `ProtocolPositions`,
and `serde_json`; it must not depend on its parent's state or request handlers.

## Exact production edit scope

Move these functions, including their existing documentation and bodies:

- `suggestions_json`
- `range_json`
- `symbol_json`
- `location_json`
- `pattern_items_json`
- `labels_json`

Use `pub(super)` for each helper, preserving names, argument types, return
types, iteration order, closures, allocation, and JSON expressions. Add a short
English module comment naming the projection responsibility. Move only the
imports that become unused in the parent. Do not correct unrelated comments
or introduce new wrappers, serialization derives, traits, or dependencies.

The parent still needs `ProtocolPositions` for its own diagnostic handling.
Check all uses before removing any import. Existing recursive `symbol_json`
calls should resolve inside the child module.

## Observable invariants

1. `range_json` preserves zero-based engine ranges as supplied; it does not
   convert their coordinates. Diagnostic suggestions and labels retain their
   existing one-based UTF-16 protocol conversion through `ProtocolPositions`.
2. An unavailable suggestion source or absent edit yields `"edit": null`.
   Empty collections stay empty arrays. Optional fields retain the exact
   distinction between absent keys and JSON null.
3. Symbols retain recursive child order and every field. Locations retain
   their existing PathBuf serialization. Pattern items retain kind strings,
   covered values, optional range, and input order.
4. Labels query the source at `label.path` when present, otherwise at
   `default_path`. Missing source retains the original positions. The `path`
   field is emitted exactly when `label.path` exists, even if paths compare equal.
5. Leave transport framing, request IDs, errors, panic containment, dispatch,
   project routing, open/update/close/reload, dependency cache, typed-check
   cleanup, and all inline response construction byte-for-byte unchanged
   except necessary imports/module declaration.

## Allowed files and ownership

- Implementation agent: `src/server.rs`, new `src/server/responses.rs`.
- Characterization, if a gap warrants it: new
  `tests/cli/server_responses.rs` and its module declaration in `tests/cli.rs`.
- Main agent: task index/records and design documents, branch/commit handling,
  independent review, authoritative verification report.

Do not touch reference baselines, fixtures, toolchain pins, generated files,
vendor sources, or any other agent's files. This is no bug fix.

## Implementation and verification sequence

1. Read AGENTS.md and this brief. Locate every helper call. Report ambiguities
   before broadening the scope.
2. Main builds the original checkout from the pinned source. Do not edit
   production files until main confirms that this baseline has been built.
3. Use existing external server tests first. Add only meaningful whole-response
   characterization where coverage is missing. Assert explicit complete JSON
   responses through `ttc --server`, including Unicode coordinates, collection
   order, null/absent keys, and a following valid request after an invalid one.
   Do not use the helper under test to compute expected JSON. If a corner is
   not reachable without unrelated setup, preserve it by body comparison and
   say so rather than expanding production scope.
4. Run any new characterization against the unchanged production source; only
   then move the functions. No snapshot updates or `UPDATE_EXPECT`.
5. Run relevant server CLI tests and formatting. Main independently compares
   old/new function bodies (normalizing only visibility), reviews all call sites,
   and runs the required Rust gates with the pinned TypeScript available.
6. Hand back changed files, exact commands/results, deviations, and any missing
   coverage. Do not self-declare the PR verified if a command skipped or failed.

Existing tests include `the_server_reports_positions_in_the_units_an_editor_counts`,
`the_server_resolves_tt_names_without_a_toolchain`,
`a_server_position_past_the_line_end_stays_on_its_line`,
`the_server_answers_a_failed_request_and_keeps_the_session`, and
`tests/cli/server_print.rs`. Run the full CLI suite, not just names containing
`server`, because several wire-contract tests use other names.

## Main-review rejection criteria

Reject body edits, changes to observable output, widened public APIs, new
engine/backend ownership, changed state lifetimes, changed error defaults,
reordered work, or baseline acceptance. If equivalence cannot be established,
keep the implementation task incomplete and record the blocker.
