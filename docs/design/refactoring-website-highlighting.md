# Separate website highlighting from artifact writes

Task: [TASK-754](../tasks/TASK-754-website-highlighting.md). Base: `ef83f563c273c7085ae8c7100bb0217edb05e974`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move topicLanguage and the two topic/section highlighting expressions into content-highlighting.ts. The module accepts content and an existing highlighter. highlight.ts keeps grammar reads, highlighter creation, writes, essay generation and sitemap construction in exactly their current order.

## Invariants and exclusions

Preserve language selection, iteration order, theme/options, generated HTML and JSON whitespace, missing sections, write order, essay paths and sitemap bytes. Do not combine both computations before the first write; errors must leave the same completed writes.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `website/scripts/highlight.ts`, `website/scripts/content-highlighting.ts`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/754`.

The unchanged website typecheck/build passed before extraction; 67 artifact hashes are saved in /tmp/tt-refactor/website-base-artifacts.json. Repeat typecheck/build and compare all generated JSON/HTML/XML and distribution files byte for byte.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
