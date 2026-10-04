# TASK-742 implementation brief: isolate remaining service response projections

Parent design: [Behavior-preserving refactoring](behavior-preserving-refactoring.md).
Read [AGENTS.md](../../AGENTS.md) before working. This is roadmap PR 2. Parent
owns branch/task bookkeeping, independent review, complete gates, publication,
and merge. The user authorizes sequential intermediate merges after review;
the final roadmap merge stays with the user. This permission does not relax
the exact behavior-preservation contract or allow expected-output changes.

## Current flow and goal

`server::respond` decodes the request, routes through `semantic`, invokes the
engine, and constructs JSON inline. Move only the answer-to-JSON portions of
four method arms into the existing private `server::responses` module. Keep
request decoding, defaults, engine invocation, failures, and operation order
visible and unchanged in the dispatcher.

## Exact functions to introduce

1. `pub(super) fn completion_json(answer: CompletionAnswer) -> Value`:
   move the existing `CompletionAnswer { items, member, probe }` destructuring
   and complete `json!` expression unchanged. The caller reads `member` and
   `triggerCharacter` exactly as before, invokes `triggered_completion` once,
   propagates its error with `?`, and passes its answer to this function.
2. `pub(super) fn completion_detail_json(detail: Option<CompletionDetail>) -> Value`:
   move the entire current None/Some match from completionResolve. Preserve
   consuming additional_edits with into_iter and omit additionalEdits only
   when that vector is empty. Caller still decodes label/source/probe before
   invoking completion_resolve once and propagating its error.
3. `pub(super) fn signature_help_json(help: Option<SignatureHelp>) -> Value`:
   move the current None/Some projection. TriggerKind/triggerCharacter parsing,
   SignatureTrigger construction, isRetrigger default, and engine call remain
   in server.rs in their existing order.
4. `pub(super) fn service_diagnostic_json(d: ServiceDiagnostic) -> Value`:
   move the existing per-diagnostic map closure body unchanged, including all
   severity/tag matches, comments, related information, and conditional path.
   Caller retains `service_diagnostics(path)?.into_iter().map(...).collect()`
   BEFORE invoking `service_restates(path)?`. Keep restates collection and
   the outer diagnostics/restates envelope in the dispatcher. Do not move both
   engine calls into a helper or evaluate restates before JSON conversion.

Use explicit helper imports and the existing engine answer types. Keep
SignatureTrigger in the parent. Move CompletionAnswer, ServiceSeverity and
ServiceTag imports only if their parent uses disappear. Existing six helpers
remain unchanged. No shared "optional result" utility, new serializable DTOs,
traits, generics, clones, dependencies, blanket imports, or new public APIs.
English doc comments should name the new helpers' responsibilities.

## Contracts and important distinctions

- Field spelling, null versus absent, empty arrays, vector order, probe values,
  enum-to-wire values, ranges, and UTF-16 parameter offsets remain identical.
- labelDetails stays null unless label_detail OR description exists; inside
  a present labelDetails, each absent component still serializes as null.
- completion tags is always present as an array, while diagnostic tags is
  omitted when empty. Do not unify these differing contracts.
- Diagnostic related is omitted when empty; an individual related path is
  present only for Some(path). Existing PathBuf serialization is preserved.
- Preserve method parameter reads, defaults, error text/propagation, wrapping
  by semantic/spanning, session/probe lifetime, and the success/error envelope.
- No expected baselines, generated fixtures, backend logic, or TS pin changes.

## Allowed files

Production: `src/server.rs`, `src/server/responses.rs` only.
Meaningful public-protocol characterization if needed:
`tests/cli/server_response_shapes.rs` plus module declaration in `tests/cli.rs`.
Task index/design docs are owned by parent. Do not commit, push, merge, start
agents, or rebuild the editor while tests run.

## Staged implementation

Initially perform read-only inspection and prepare a fixed temporary corpus
under `/tmp/tt-task-742-review`. The preceding integration PR is awaiting CI;
DO NOT edit tracked files until parent gives GO and creates the task/branch.
The initial source tree equals the accepted PR-1 tree. The implementation
base is main 21793c6b2aefe6342d0261df712fb68a40210369, integrated through #135
after all applicable CI passed. Parent preserved its source-built baseline
executable at /tmp/tt-task-742-base-ttc before authorizing tracked edits.

Characterize real server replies for these four methods on the unchanged
implementation: member completion with plain and deprecated entries; empty or
null responses where reachable; completionResolve with no additional edits
and auto-import edits if feasible; nonempty signature help with parameter
labels; diagnostics with/without tags and related paths. Exercise default and
explicit trigger parameters and following requests after an error. Inspect
actual replies to distinguish coverage from merely sending requests.

Add at most a few durable public-protocol characterization tests where the
existing CLI/public_api/editor suites miss a relevant contract. Prefer complete
expected JSON for small deterministic responses. Do not compute expected
results using newly extracted helpers or add a mirror of the implementation.
Run any added tests against unchanged production first and report the command.
Temporary differential comparisons must use the same paths/configuration for
base and head and compare raw stdout/stderr/exit bytes without normalization.
Explicitly report uncommon branches only covered by structural comparison.

After GO: extract projections without redesign, run fmt and focused CLI/public
API checks, hand back the full diff, structural equivalence evidence, corpus,
commands/results, and coverage gaps. Parent reviews independently and runs
`./scripts/ci agents rust` (or the same required checks) before publishing and
waiting for remote CI. Never use UPDATE_EXPECT or bypass a failing gate.

## Runtime

Toolchain is installed outside the repository:

```sh
export RUSTUP_HOME=/workspace/tt-refactor-tools/rustup
export CARGO_HOME=/workspace/tt-refactor-tools/cargo
export PATH="$CARGO_HOME/bin:/workspace/tt-refactor-tools/bun/node_modules/.bin:$PATH"
export npm_config_cache=/workspace/tt-refactor-tools/npm-cache
```

The original TypeScript 7 research remains pinned in the shared design. This
slice applies its separation of execution dependencies from answer projection;
it does not upgrade or port to upstream code.
