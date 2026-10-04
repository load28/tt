# Behavior-preserving refactoring program

Design task: [TASK-740](../tasks/TASK-740-behavior-preserving-refactoring-design.md).
Initial source: `b62f2b6e727724748747d2a76351034f4daa010e` (main, checked
2026-10-03). This is an incremental program, not a claim that the entire
repository has already been refactored.

## Goal and compatibility contract

Make the code's responsibilities, data flow, and state ownership easier to
follow. Every implementation PR must preserve the previous observable result.
Internal steps may change only when their externally visible consequences stay
the same. Behavior fixes and improvements need separate tasks and PRs, even
when the old behavior looks accidental.

The contract includes:

- Emitted TypeScript/TSX bytes, including comments, whitespace, generated names,
  import rewrites, declarations, source maps, and mappings to authored source.
- Acceptance and rejection of inputs; diagnostic codes, messages, severity,
  labels, suggestions, locations, ordering, and first-error precedence.
- Runtime evaluation order, number of evaluations, short circuiting, lexical
  scope, `this`, thrown values, stdout, stderr, and exit status.
- Public Rust APIs, CLI flags/defaults, JSON/LSP shapes, null versus omitted
  fields, collection ordering, and byte/code-point/UTF-16 coordinate rules.
- File contents and paths written or removed, request ordering, cleanup on
  failure, cancellation/retry behavior, document overlays, cache invalidation,
  snapshot identity, and process lifetime wherever callers can observe them.
- Packaging entry points, installed files, UI content, routes, and generated
  assets for changes outside the compiler.

Wall-clock time itself is not expected to be identical, but existing timeout,
resource, and scaling contracts remain gates. Do not add concurrency, pooling,
new caching, extra I/O, or new normalization as incidental cleanup.

No finite test suite proves equality for every possible input. Approval requires
both a structural argument about the actual change and unchanged observations
on the relevant existing cases. Missing coverage is stated explicitly; it is
not permission to weaken the contract.

## What current TypeScript 7 teaches

The archived `microsoft/typescript-go` repository's
[README](https://github.com/microsoft/typescript-go/blob/89d5d5b2849a0db0957065889ca58536fa6d2e4a/README.md)
states that development returned to `microsoft/TypeScript`. The current official
main revision inspected on 2026-10-03 is
[`50d70a3f5f453a79a4323b263165da51f656a4e3`](https://github.com/microsoft/TypeScript/commit/50d70a3f5f453a79a4323b263165da51f656a4e3).
These references intentionally pin that revision; they do not require a
TypeScript dependency upgrade in tt.

| Observed design | Source at the inspected revision | Application to tt |
| --- | --- | --- |
| Separate configuration, hosts, and factories | [`ProgramConfig`, `ProgramHosts`, `ProgramFactories`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/compiler/program.go#L37-L61); [`TestProgramSharedData`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/compiler/program_test.go#L29-L78) | Keep immutable input, session state, and external dependencies distinct. Preserve current owners and lifetimes during extraction. |
| Express environment interaction at a narrow boundary | [`CompilerHost`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/compiler/host.go#L15-L68), [`vfs.FS`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/vfs/vfs.go#L12-L49) | Consumers orchestrate existing engine operations; answer formatting does not acquire projects or access the filesystem. Add an interface only for a real boundary. |
| Name phase inputs/outputs and borrowed-state lifetime | [`ParseSourceFile`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/parser/parser.go#L123-L147), [`CheckerPool`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/compiler/checkerpool.go#L16-L25) | Organize by meaningful compiler operation, not arbitrary line limits. Make mutation and cleanup ownership visible. Do not copy pooling policy. |
| Distinguish semantic identities from display strings | [`tspath` type contract](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/tspath/doc.go#L1-L28) | Preserve authored paths, canonical identities, and coordinate spaces. Introduce narrowly useful names/types without silently changing path rules. |
| Compare all output dimensions | [`runSingleConfigTest`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/testrunner/compiler_runner.go#L164-L182), [`baseline.writeComparison`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/testutil/baseline/baseline.go#L41-L83) | Use tt's existing emitted-code, diagnostics, mapping, type, runtime, and editor baselines. Refactoring does not authorize accepting changed expectations. |
| Test ordering explicitly | [`verifyUnionOrdering`](https://github.com/microsoft/TypeScript/blob/50d70a3f5f453a79a4323b263165da51f656a4e3/tsc/internal/testrunner/compiler_runner.go#L524-L547) | A different map, traversal, or sort can change observable order. Preserve current order, even if another order looks cleaner. |

TypeScript's large checker/parser files, Go syntax, arenas, pools, and parallel
compiler strategy are not target shapes for this project. Learn how boundaries
are expressed and checked. tt's existing Rust ownership, compiler stages,
Project/Snapshot model, and source-preserving emitter remain the architecture.

## Shared design principles

1. **One owner per responsibility.** Recognition belongs to lexer/parser;
   semantic judgments to resolve/analysis/sema/val; lowering and emission to
   their existing IR/codegen stages; backend details to `typescript`; transport
   and UI projection to their consumers. Avoid deriving the same fact again.
2. **Extract a coherent operation.** A reader should be able to name a module's
   input, output, and responsibility. Do not create a generic `utils` module or
   split tightly coupled state machines solely to shorten files.
3. **Keep orchestration readable.** Entry points should reveal the existing
   sequence. Leaf modules receive the answer/input they need; they do not reach
   back into a parent context or acquire hidden services.
4. **Keep state with its current owner.** `Engine -> Project -> Snapshot`,
   Workspace document ownership, caches, probes, backend sessions, and lifetime
   boundaries stay intact. A move does not imply a new abstraction or owner.
5. **Keep representations distinct.** Authored/emitted text, AST/HIR/Core/
   Evaluation IR, file identities, and coordinate units are existing semantic
   contracts. Document transitions rather than merging superficially similar
   data structures.
6. **Use the smallest visibility.** Prefer private modules and `pub(super)` to
   new public APIs. Do not introduce dependencies, traits, or general-purpose
   frameworks without a concrete need demonstrated by the current slice.
7. **Separate moves from decisions.** Mechanical extraction comes first.
   Renaming, signature redesign, algorithm changes, and optimizations require
   their own justified slice and equivalence evidence.
8. **Keep evidence outside the implementation's assumptions.** Tests observe
   public boundaries and fixed expected results. Do not calculate expected
   results with the new helper, regenerate baselines, or hide differences behind
   new normalizers. Preserve existing test-harness normalization exactly.

## Current responsibility map and proposed PR sequence

The audit found an existing architecture, with readability pressure where one
file combines orchestration, representation, and output projection. This table
covers the first-party source surfaces. Each row is a candidate PR, not an
instruction to execute several rows together. Split a row again if review
requires unrelated reasoning. Later rows require a fresh detailed brief before
delegation; this roadmap alone is not an implementation specification.

| Order | Responsibility and bounded change | Preserved observations and tests |
| --- | --- | --- |
| PR 0 | This design, audit, workflow, and [first implementation brief](refactoring-server-responses.md) | Documentation/task consistency; no production changes. |
| PR 1 | `src/server.rs`: move six existing JSON helpers to private `server/responses.rs` | Exact bodies and wire output; full `cli`, editor cases, Rust gates. |
| PR 2 | Server inline completion, completionResolve, signatureHelp, tsDiagnostics projections | Null/absent fields, ranges, order; characterize complete wire answers first; extension consumer tests. |
| PR 3 | `lib/compile.rs`: target-diagnostic helpers (`try_target_errors`, `match_target_errors`, placement helpers, `target_errors`, `nonredundant_target_errors`) | Diagnostic precedence, spans, deduplication; `case_baselines`, `snapshot`, `compile`, `practical_diagnostics`. |
| PR 4 | `lib/compile.rs`: recovery helpers (`recoverable_constructs`, `outermost_recoveries`, `recover_source`, `declare_recovered_variants`, `overwrite_recovery`) | Mask byte lengths, preserved spans, declarations, editor recovery; editor/native/compiler cases. |
| PR 5 | `engine/language/service.rs`: hover/documentation formatting (`split_hover`, `split_markdown_hover`, `docs_text`, `parameter_span`) | Exact text and UTF-16 labels; language unit tests, native hover/signature, editor cases. Keep stateful `source_links` separate. |
| PR 6 | `engine/language/project.rs`: completion operation module under the existing owner | Probe identity, triggers, candidate order, auto-import edits; native editor service and editor baselines. |
| PR 7 | `engine/language/service.rs`: target coordinate projection (`TargetUse`, `TargetCoordinates`, `target_coordinates`, `map_target`, `source_edit`, `map_shared_target`) | Authored/projected provenance, URI/path identity, shared bindings; content mapper, emit-map, engine-cache, native/editor tests. Higher risk. |
| PR 8 | `parser/matches.rs`: arm-list recognition/recovery (`ArmPart`, `ArmOutline`, `outline_arms` and associated helpers) | Cursor advancement, lookahead, opaque TS passthrough; parser tests, passthrough, fuzz regressions, match/editor cases. Pattern parsing gets a separate brief. |
| PR 9 | `program_syntax/projection.rs`: projection segment representation apart from builder mechanics | Overlay/span order and source ownership; program syntax tests, emit-map and editor baselines. No simultaneous traversal redesign. |
| PR 10 | `codegen/core/planning.rs`: source-rewrite records and local edit helpers before dividing `TargetRewritePlan::build` | Names, replacement priority, directives, evaluation order, emitted bytes; compiler/runtime cases, snapshots, mappings. No scheduling redesign. |
| PR 11 | VS Code `server/src/server.ts`: diagnostic/symbol/fix projection (`toDiagnostic`, `toDocumentSymbol`, `insertSymbol`, `suggestedFixes`) | LSP fields, nesting, actions; diagnostics/completion/server tests and editor cases. Preserve timers and validation generations. |
| PR 12 | VS Code `server/src/engine.ts`: wire answer types apart from session transport | Timeouts, queue head IDs, retries, replay barriers, Unicode conversion; session, engine, compiler, roots tests. Queue redesign is separate. |
| PR 13 | `integrations/unplugin/index.js`: module-ID/query parsing helpers | Literal `?`/`#` filenames, markers, Windows paths, package files; plugin/windows/server tests. Preserve compiler-process policy. |
| PR 14 | Website `scripts/highlight.ts`: content transformation apart from file writes/sitemap construction | Exact generated JSON/HTML/XML and route order; website typecheck/build plus artifact comparison. |
| PR 15 | Bot `src/deliberation.mjs`: prompt/comment formatting apart from execution | Exact prompts/comments, rounds, posting sequence, stored state; deliberation/review/state/github tests with fake services only. |
| PR 16 | `tests/common/baseline.rs`: selection classification apart from comparison/tracking I/O, if this improves ownership | Missing/unused baseline and filter detection; `baseline_tracking` and baseline-tools tests. Test-harness refactoring follows stable product evidence. |

Review rather than automatically rewrite these other surfaces:

- AST/HIR/resolve/analysis/sema/val and Evaluation IR: retain the current domain
  ownership. Revisit individual dense operations after the related frontend and
  projection slices; establish an exact symbol boundary before scheduling.
- TypeScript native/service/toolchain and `host.mjs`: retain backend process and
  API ownership. Any extraction needs characterization of startup, errors,
  cancellation, and source-serving behavior; do not spread tsgo details outward.
- CLI/build/watch, content mapper, scanner/lines/diagnostics/render: retain their
  public defaults, byte rules, and phase ordering. Schedule only a demonstrated
  readability problem, independently from the server extraction.
- npm launcher, initializer, packaging/release scripts, workflow automation,
  stdlib, benchmarks, and fuzz harness: include in inventory/review, but leave
  coherent modules unchanged. Release and installation results are contracts.
- Vendored SWC, generated sources, fixtures, baseline references, matrices, and
  lockfiles: inputs or outputs with their own ownership. Do not mechanically
  refactor generated/vendor files or upgrade them as part of this program.

Completeness means every owned source surface has been assessed and each
justified change reviewed; it does not mean every file must receive a diff.

## Subagent handoff and main review

The main agent owns the shared design, task numbering, integration sequence,
and final decision. Each implementation PR uses a fresh subagent context with
its brief and necessary source paths. Research/audit agents do not implement.
Do not rely on implicit conversation history for an implementation handoff.

Every brief must contain:

1. Problem, current call flow, intended responsibility, and exact symbols.
2. Before/after dependency direction, input/output contracts, state ownership,
   coordinate/identity rules, and minimum visibility.
3. Allowed files, forbidden changes, dependent earlier PRs, and stop conditions.
4. Observable invariants and existing coverage; missing coverage to characterize
   on the unchanged implementation before production edits.
5. Reproducible base/head commands, expected unchanged artifacts, and required
   environment/tool versions. A skipped test is not a passing contract.
6. Main-review acceptance criteria, including scope, readability, equivalence,
   error paths, and unexpected diffs. The subagent reports evidence; it does not
   approve its own implementation.

Use separate worktrees when multiple implementations overlap. Otherwise keep
implementations sequential: the main reviews one PR before assigning its
dependent successor. Do not run extension clean/build while editor tests use
its output, or several full backend-heavy suites concurrently.

The main reviews the whole diff, not only the agent's report. For a mechanical
move, compare function bodies against the base and inspect every call site.
For a changed implementation, state why the relevant observations are preserved
and run before/after cases. Request corrections from the same implementation
agent until scope and evidence meet the brief.

## Verification and progression

Record the base SHA, toolchain, commands, exit statuses, skip/ignore counts, and
pre-existing failures in each task. Build the base from source; an old target
binary is not evidence about a newer checkout. Preserve any differential binary
and inputs outside tracked sources. Compare base and head in the same workspace,
configuration, and environment, preserving output bytes rather than introducing
refactoring-specific normalization.

For Rust slices, AGENTS.md requires `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test`.
`./scripts/ci agents rust` additionally builds editor prerequisites, requires
the pinned TypeScript and extension, checks baseline tracking, and checks the
fuzz crate. It may report an unavailable upstream corpus; record that gap.
Non-Rust slices run their relevant `scripts/ci` stages and artifact comparisons.

Do not use `UPDATE_EXPECT=1`, edit reference baselines, accept changed output,
or suppress failing/unused baselines in a behavior-preserving PR. If the base
already fails, reproduce and record it separately. Distinguish unchanged
pre-existing failure from a new failure; neither authorizes claiming all gates
passed. Block merge while required checks remain unresolved.

Every PR must stand alone for review and rollback. Merge only through the
repository's normal PR workflow after local gates, independent main-agent
review, and applicable remote CI checks. On 2026-10-04 the user authorized
autonomous sequential publication and intermediate squash merges, superseding
the original preparation-only workflow. Each implementation starts from
latest main after its predecessor merges and receives a fresh detailed brief
and subagent context. Leave the final roadmap PR open for the user's review
and merge; this authorization does not relax any compatibility or test gate.

PR 0 and PR 1 reached main through [#135](https://github.com/load28/tt/pull/135)
(including the stacked [#136](https://github.com/load28/tt/pull/136)). PR 2 is
specified in [its detailed brief](refactoring-service-responses.md) and tracked
by [TASK-742](../tasks/TASK-742-service-response-projections.md); it reached
main through [#137](https://github.com/load28/tt/pull/137). PR 3's
[detailed brief](refactoring-target-diagnostics.md) and
[TASK-743](../tasks/TASK-743-target-placement-diagnostics.md) continue with
target placement diagnostics.

TASK-732 is already In progress and records outstanding performance/parity
work. This refactoring program does not complete it, relax its checks, or mix
its behavior fixes into readability changes.
