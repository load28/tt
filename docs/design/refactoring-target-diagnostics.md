# TASK-743 implementation brief: target placement diagnostics

Read [AGENTS.md](../../AGENTS.md) and the
[shared design](behavior-preserving-refactoring.md) before working. This is
roadmap PR 3 and follows the accepted server projection PRs. Parent owns
task bookkeeping, branch, independent review, full gates, and publication.
Implementation was authorized after PR #137 passed local/remote gates and
merged. The latest-main base is dea2ba2ab11e7813155ddfc449503e167b2753e8.

## Responsibility and boundary

`src/lib/compile.rs` exposes compile/analyze/report orchestration and also
contains the policy that projects a completed Evaluation IR lowering plan
into target-placement diagnostics. Extract that policy to a private child
`src/lib/compile/target_diagnostics.rs`. Preserve the compiler's existing
phase order, recovery, and public API.

Move these definitions with their exact bodies and documentation:

- `try_target_errors`, `match_target_errors`,
  `lexical_declaration_body_errors`;
- `LOGICAL_ASSIGNMENT_HELP`, `match_placement_message`,
  `try_placement_message`;
- `target_errors`, `nonredundant_target_errors`.

Only the latter two functions are `pub(super)`; the other helpers and
constant remain private to the child. Add `mod target_diagnostics` and
explicit imports for those two functions in the parent. Because compile is
declared with a path attribute in lib.rs, verify Rust resolves the new
child path correctly; do not change the public lib.rs re-exports.
Use explicit crate imports for `evaluation_ir`, `program_syntax`,
`TtError`, and `DiagnosticCode`; no `super::*` in the new child.

Keep `recovered_target_errors` in compile.rs. It owns retrying plan
construction after SourceNotTypeScript and is orchestration, not diagnostic
projection. It continues calling `nonredundant_target_errors` unchanged.
Keep `tt_errors`, `suppress_discarded_result_fallthrough`, every public
API, recovery helper, lowering call, and all compile/analyze/report call
sites unchanged apart from resolving their imported helper names.

## Exact invariants

- Preserve the ordered accumulation of try, match, and lexical-body errors.
- Preserve stable `sort_by_key`, `None` offsets sorting last, and equal-offset
  category order. Never replace with an unstable sort or a set.
- Preserve deduplication only for TryPlacement with a present offset and a
  prior TryCrossesValueRegion with equal start AND end. Preserve filtering
  order and every emitted span, code, help/message byte.
- Preserve match-arm order, particularly owner-priority cases in the try
  messages. Do not merge visually similar arms or normalize strings.
- Do not change which pipeline paths invoke a helper, compile's first-error
  policy, analyze/report's all-errors policy, or when recovery is attempted.
- No new allocations, traversals, dependency, public API, DTO, algorithm,
  renamed helper, reference-baseline update, or version change.

## Allowed files and evidence

Production: `src/lib/compile.rs`, new
`src/lib/compile/target_diagnostics.rs`. Parent owns docs/task/index.
Existing compile/snapshot/practical_diagnostics/case_baselines provide
public-boundary coverage. Read relevant placement tests and cases first,
including `tryPlacementReportsTheOwningReason*`,
`logicalAssignmentValueHoldingTtValueIsAPlacementError`, and loop/class
placement cases. Do not add tests that merely repeat the moved match arms.

Prepare structural comparison under `/tmp/tt-task-743-review`: show all
eight definitions retain exact bodies (only two visibility changes), and
the parent reconstructs to the base after reversing the moves and imports.
Inspect literal string bytes rather than whitespace-normalizing whole
functions, since diagnostic strings are observable. Parent independently
reviews this comparison and all call sites.

Run fmt and focused compile/snapshot/practical_diagnostics suites after
extraction; parent runs complete `./scripts/ci agents rust`. No
UPDATE_EXPECT. No concurrent full backend suites or editor output rebuilds.
Use the shared external toolchain variables from the preceding task:
RUSTUP_HOME=/workspace/tt-refactor-tools/rustup;
CARGO_HOME=/workspace/tt-refactor-tools/cargo;
prepend its bin and /workspace/tt-refactor-tools/bun/node_modules/.bin to PATH;
npm_config_cache=/workspace/tt-refactor-tools/npm-cache.

If a body or invocation cannot remain unchanged, stop and report the
specific dependency to parent before expanding scope. Do not commit, push,
merge, or start other agents. Return changed files, exact-body evidence,
commands/results, and any skipped coverage or unresolved question.
