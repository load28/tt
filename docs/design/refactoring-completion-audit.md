# Refactoring completion audit

Task: [TASK-757](../tasks/TASK-757-refactoring-final-audit.md).
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).
Implementation base: `85729d01a6cc169d26bce81b9ee25e7b7734f9ae`.
Review date: 2026-10-04.

## Scope and method

The continuation implements roadmap slices PR 4 through PR 16 as separate local
commits, TASK-744 through TASK-756. Earlier design/server/diagnostic slices
already reached main. The continuation preserves the algorithms and ownership
of existing state; new modules name existing responsibilities. The only package
manifest change adds the extracted unplugin helper to its published file list.
No dependency version, lockfile, reference expectation, or generated source is
changed.

Review combined the original responsibility inventory, module declarations and
callers, inspection of dense ownership boundaries, complete production diffs,
and base/head comparisons for the moved definitions. The final inventory lists
417 tracked implementation, test, and tooling files across `src` (172),
`tests` (87), editors (46), npm (25), integrations (21), website (20), scripts
(16), bot tools (14), workflows (8), initializer (4), fuzz (3), and benches (1).
It excludes vendored code, generated syntax grammars, case inputs, fixture and
baseline trees, build outputs, and dependencies. Inventory membership is an
ownership assessment, not a claim of a new line-by-line correctness audit of
all 417 files. The inventory and raw check logs are local execution evidence
under `/tmp/tt-refactor`; durable decisions and results are recorded here and
in the individual tasks.

## Implemented boundaries

| Tasks | Existing caller and new responsibility | Structural evidence |
| --- | --- | --- |
| 744–745 | Compilation recovery; service hover, documentation and parameter labels | Existing helper bodies moved intact, with narrow parent visibility; source-link lookup and session ownership stay in the caller. |
| 746 | Project completion operations | Existing `impl Project` methods moved together; probes, candidates and mutation order unchanged. |
| 747 | Service target coordinates | Existing target classification, source edits and shared-binding projections moved; document acquisition remains under the existing service owner. |
| 748 | Match arm lists | Existing recognition and recovery helpers moved together; pattern parsing and lexer advancement algorithms unchanged. |
| 749 | Projection segments | Existing segment records and indexed lookup implementation moved; projection construction unchanged. |
| 750 | Source rewrite records | Existing records and local edit helpers moved; `TargetRewritePlan::build`, scheduling and emitter logic unchanged. |
| 751 | LSP result projections | Four existing function bodies moved; URI callback and workspace-edit capability passed synchronously at existing call sites. Validation generations and timers unchanged. |
| 752 | Engine wire types | All 24 exported type declarations moved and re-exported; runtime transport, queue and retries unchanged. |
| 753 | Bundler module IDs | Existing constants, filesystem-based filename recognition and query helpers moved; compiler process policy and public package exports unchanged. New helper included in package contents. |
| 754 | Website highlighting | Existing topic and section transformations extracted; computation and write order unchanged. |
| 755 | Deliberation formatting | Eight existing formatter/prompt definitions moved; engine class and awaited execution sequence unchanged. Existing public formatters re-exported. |
| 756 | Baseline run selection | Pure `filtered_by` predicate moved byte-for-byte; environment reads, comparison and tracking I/O unchanged. |

## Retained ownership boundaries

| Surface reviewed | Decision and reason |
| --- | --- |
| AST, HIR, resolve, semantic analysis and `val` | Retain phase ownership. `hir/lower.rs` assigns node identities and source maps in one traversal; `resolve/mod.rs` owns scope and stable definition identity. `sema` already separates checker and coverage, and `val` separates calls, checker, references and targets. A larger reorganization would need a new exact operation boundary and equivalence evidence. |
| Core and Evaluation IR | Retain the lowering/build contexts. `evaluation_ir/evaluation.rs` assembles and validates host bindings and regions with shared traversal state. Moving scheduling or identity assignment is outside a mechanical extraction. |
| Parser, lexer and program syntax | Extract arm lists and segment representation as planned; retain the statement machine in `lexer/facts/statements.rs`, whose transitions share lexical frames. Projection building and syntax visitation retain their current ordering. File length alone does not justify another boundary. |
| Code generation | Extract source-rewrite records and local edit helpers. Retain plan construction and the existing emitter split among host, source, pattern and evaluation responsibilities. `emitter/host.rs` shares owner and evaluation context; no new scheduling abstraction is justified by this review. |
| Engine and snapshots | Retain `engine/project.rs` as the long-lived cache, file graph, backend and snapshot owner. Completion request orchestration is now separate from `engine/completions.rs`, which owns semantic pattern candidates. No cache or lifetime redesign. |
| TypeScript backend | Retain native/service/toolchain seams and `host.mjs`. The host is embedded as a standalone process and owns native API loading and session protocol. Splitting it safely would require additional process startup, error, cancellation and source-serving characterization, plus an embedding design; no such change is part of this program. |
| CLI, content mapper, rendering and diagnostics | Retain existing command/build/loading/output/ownership/typed modules and the content-mapper entry point. `diagnostics.rs` is the code/message vocabulary; rendering, line conversion and source mapping already have separate owners. |
| Editor transport and consumers | Extract projections and protocol declarations. Retain workspace roots, session queue, replay, cancellation and validation state. Existing server and engine tests exercise these lifecycle contracts. |
| npm, initializer, release and workflow automation | Keep launcher, installer, package assembly and release jobs in their existing modules. Apart from the unplugin file allowlist, installation and publication behavior is unchanged. |
| Website and deliberation bot | Extract deterministic formatting from effectful execution. Keep routing, rendering, file writes, posting and persistence with existing owners. Bot comparisons use fake services only. |
| Standard library | Retain `src/stdlib` runtime/type modules and CommonJS declaration counterparts; tree-shaking and package resolution contracts remain covered by repository gates. |
| Tests, fuzz and benchmarks | Extract selection classification only. Retain baseline comparison, tracking, test matrices and fuzz/benchmark entry points as evidence independent of the implementation. |
| Vendor, generated sources, cases and reference artifacts | Preserve their own ownership and all tracked bytes; do not mechanically reorganize these inputs or outputs. |

## Validation

Final default CI is incomplete because this long-lived container exhausted
process/thread capacity. The first run passed agents, formatting, clippy,
807 Rust tests across nine completed suites, all 121 npm-stage tests
(84 tooling/initializer, 21 plugin, one generated-project end-to-end, 15 bot),
and website typecheck/build. The editor suite stalled; native/extension
subprocesses subsequently reported OS error 11 and Node `uv_thread_create`
startup failures. These are failed/incomplete gates, not passing results.

Over 32,000 unreaped processes were observed. A retry with an external child
subreaper, two-CPU affinity and `GOMAXPROCS=2` also stalled under exhausted
capacity and was stopped. No tracked implementation, expectation, test selection,
or timeout changed to accommodate the environment. Raw logs remain at
`/tmp/tt-refactor/757/first-ci-resource-exhaustion.log` and
`/tmp/tt-refactor/final-ci.log`.

Completion still requires all six stages (agents, Rust, npm, website, native,
and extension) to pass in a healthy local environment. Rust and TypeScript pins
remain 1.98.0 and 7.1.0-dev.20260826.1; Node is 24.19.0. On 2026-10-04 the user
explicitly requested PR creation after being informed of the incomplete CI.
Publish a draft with these limitations; keep TASK-746 through TASK-757 In progress
and do not merge or claim validation complete.

Focused checks are recorded per task. In addition:

- Website base/head builds with the same build-time input produced identical
  bytes for all 67 generated JSON, HTML, XML, JavaScript and stylesheet files,
  with no additional files. Ordinary builds differed only in framework-generated
  serialized timestamps. The comparison fixes the time input through an
  external test preload; it does not normalize output or modify production.
- Bot base/head runs compared exact prompts, comments, state, results and
  errors using fake services for consensus, objections, continued rounds,
  mentions and request failure (18, 23, 23 and 4 matching events).
- The unplugin package dry run includes `module-id.js` and leaves package
  exports unchanged. A missing `TSX_SUFFIX` import discovered by the focused
  plugin tests was corrected before committing TASK-753.
- Restricted subprocess/listener failures were reproduced as environment
  failures. Relevant tests/builds use network-enabled sandbox permissions and
  temporary writable package caches; no expectations were changed to pass them.

## Completion limits

The implementation covers the justified extraction roadmap and records decisions
for retained source surfaces. It does not prove equivalence for every possible
input, claim every source file needed editing, or complete TASK-732's separate
performance/parity work. Optional coverage and benchmark stages are not default
merge gates and are not substitutes for the complete behavioral checks. Remote
CI and final PR review remain distinct from local validation; no merge is
implied by a completed implementation task.
