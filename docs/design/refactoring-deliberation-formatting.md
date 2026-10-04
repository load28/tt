# Separate deliberation formatting from execution

Task: [TASK-755](../tasks/TASK-755-deliberation-formatting.md). Base: `87c69f9d76212a9f129f2cf9f10e8e75a86ad96b`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move serialize, untrusted, sharedRules, agentPrompt, mentionPrompt, moderatorPrompt, formatAgentComment and formatOutcome into deliberation-format.mjs. DeliberationEngine imports its five entry points; the two existing public formatters remain re-exported from deliberation.mjs. The class and all awaited operations stay unchanged.

## Invariants and exclusions

Preserve every prompt/comment byte including Korean strings, interpolation order, JSON indentation, round labels, empty-list fallbacks, and trust delimiters. Preserve agent order, moderation decisions, posting and state-save order, error propagation and process policy. Use fake services only; no external messages.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `tools/deliberation-bot/src/deliberation.mjs`, `tools/deliberation-bot/src/deliberation-format.mjs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/755`.

Run bot tests/check before and after and compare base/head fake-service event traces for consensus, objections, exhausted rounds, mentions and a request failure. Full npm gates remain required.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
