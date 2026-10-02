# TASK-655: Answer a request under its id when its parameters do not decode, and settle a `null`-id answer in every client

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-655`

## Purpose

The unplugin integration hung forever on a module whose path held a lone
UTF-16 surrogate (possible in a Windows file name). `JSON.stringify` writes
such a code unit as a `\udc00` escape, which JSON's grammar allows and
UTF-8 cannot hold, so `ttc --server` could not decode the request and
answered `{"id": null, "error": "malformed request: ..."}`. The integration's
`Session.answer()` looked the answer up by id, found no request `null`,
and dropped it: the promise never settled and the session stayed ref'd, so
the build never finished. The VS Code client already settled such an answer
against its oldest pending request and sent well-formed strings.

## Scope

- Included: `src/server.rs` (reading the id), the protocol description in
  its module documentation, `integrations/unplugin/compiler-server.js`,
  tests in `tests/cli.rs` and `integrations/unplugin/test/server.test.mjs`.
- Excluded: the VS Code client (`editors/vscode/server/src/engine.ts`),
  which already follows the rule this task makes common.

## Sources

- JSON-RPC 2.0 specification, §5 "Response object", `id`: "If there was
  an error in detecting the id in the Request object (e.g. Parse
  error/Invalid Request), it MUST be Null."
- RFC 8259 §7 and §8.2: a string may escape any code unit, and a lone
  surrogate escape is grammatical but its behaviour is unpredictable; §8.1
  requires UTF-8 on the wire, which cannot encode a lone surrogate.
- ECMAScript `JSON.stringify` (well-formed since ES2019):
  `QuoteJSONString` escapes a lone surrogate as `\uXXXX`.
  `String.prototype.toWellFormed` replaces it with U+FFFD.
- serde_json `value::RawValue` (the `raw_value` feature): a member kept as
  its raw JSON text, not decoded.

## Decisions

### Decision 1: The server reads the id on its own

- **Context**: Most `id: null` answers came from a line that was a JSON
  object with a readable id whose `params` did not decode.
- **Alternatives considered**: (a) Keep answering `null` and fix the
  clients only. The id is right there; §5 reserves `null` for when it
  cannot be detected. (b) Decode the line as a map of raw members and
  decode only `id`.
- **Decision and rationale**: (b), in `request_id`, which the parse-error,
  non-UTF-8, and panic paths already use. Only a line that is not a JSON
  object, or whose id does not decode, is answered with `null`. This needs
  serde_json's `raw_value` feature, no new crate.

### Decision 2: One client rule: a `null`-id error answers the oldest request

- **Context**: A line that is not an object still has no id. The server
  answers every non-blank line with exactly one line, in order, so the
  oldest unanswered request is the one a `null` answer belongs to.
- **Alternatives considered**: (a) Reject every pending request on a
  `null` answer: it would fail requests the server will still answer.
  (b) Retire the session: a restart for one bad line. (c) Settle the
  oldest pending request with the error, as the VS Code client does.
- **Decision and rationale**: (c) in `compiler-server.js`, and the in-order
  guarantee is now written in the protocol description in `src/server.rs`,
  so both clients rely on a documented contract.

### Decision 3: The integration sends well-formed strings

- **Context**: With Decision 1 the request is answered, but the root is a
  client producing JSON the server cannot decode.
- **Alternatives considered**: (a) Leave the escape and rely on the
  server's error. (b) Replace lone surrogates with U+FFFD before writing,
  with `toWellFormed` where it exists, as the VS Code client does.
- **Decision and rationale**: (b). The request then reaches the compiler,
  which reports the replacement path it cannot find as an ordinary
  diagnostic, and both clients send the same bytes for the same path.

## Work log

- 2026-09-30: Reproduced the hang with `target/probe7-cli/cases/02`.
- 2026-09-30: Changed `request_id`; documented the ordering rule and the
  `null` answer in the server's protocol description.
- 2026-09-30: Changed `Session.answer()` and the request writer in
  `compiler-server.js`; added the tests.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `integrations/unplugin/test/server.test.mjs` ("an error the
  server could not correlate answers the oldest request", "a path the
  protocol cannot carry is answered, and the session goes on");
  `tests/cli.rs` (`a_request_whose_parameters_do_not_decode_is_answered_under_its_id`,
  `a_server_line_that_is_not_utf8_is_answered_and_the_session_continues`).
- **Observed failure**: With `src/server.rs`, `Cargo.toml`, and
  `compiler-server.js` reversed: both unplugin tests failed with "first:
  no answer within 10 s" and "lone surrogate: no answer within 10 s", and
  the test process then never exited (the ref'd session; `timeout`
  stopped it). `a_request_whose_parameters_do_not_decode_is_answered_under_its_id`
  failed with `left: "null"` for
  `{"error":"malformed request: lone leading surrogate in hex escape at line 1 column 51","id":null}`,
  and the updated non-UTF-8 test with `"id":null` where the line's id was 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`
- [x] Baseline changes reviewed and committed with the change (none)

## Result

A request is answered under its id whenever the id can be read, and both
clients settle a `null`-id error against the oldest pending request.
Changed files: `Cargo.toml`, `src/server.rs`,
`integrations/unplugin/compiler-server.js`, `tests/cli.rs`,
`integrations/unplugin/test/server.test.mjs`.
