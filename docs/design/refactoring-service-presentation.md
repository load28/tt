# Language service text presentation extraction

Task: [TASK-745](../tasks/TASK-745-service-presentation.md).
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).
HEAD: `85729d01a6cc169d26bce81b9ee25e7b7734f9ae`; the effective local base
includes the already verified, uncommitted TASK-744 recovery extraction.

## Boundary and call flow

Project hover requests call `split_hover` before mapping documentation links.
Completion resolve and signature help call `docs_text`; signature help also
calls `parameter_span` to expose UTF-16 label bounds. These helpers consume JSON
and text without accessing a project, document, backend, filesystem, or session.

Move `split_hover`, `split_markdown_hover`, `docs_text`, and `parameter_span`
unchanged to `src/engine/language/service/presentation.rs`. Declare the private
child in `service.rs` and re-export the three existing entry points using
`pub(super) use`. In the child, use `pub(in super::super)` for those definitions
to preserve their existing language-module visibility; keep the Markdown helper
private. The child depends only on the already-used `serde_json::Value` and
standard string operations. No new dependencies or public exports are needed.

Leave `source_links`, `source_link`, all callers, and request/restore ordering
unchanged. Moving a link involves session-owned document identity and coordinate
projection; it does not belong in this pure presentation boundary.

## Invariants

Preserve every accepted JSON shape, empty/default result, string trim, Markdown
fence search, closing-fence suffix treatment, prose ordering, and code/prose
distinction. Do not replace the existing fence parser with a Markdown library.
Preserve parameter-array precedence, length checks, unsigned conversion/defaults,
casts, first substring match, empty/missing label handling, and UTF-16 counting.
Formatting a label must not acquire document-coordinate or URI responsibilities.

## Scope and stop conditions

Production files are only the existing `service.rs` and the new presentation
child. Documentation may change this brief, TASK-745, the task index, and the
shared roadmap's progress note. TASK-744's production and individual documents
remain untouched. No algorithm cleanup, baseline regeneration, caller changes,
source-link extraction, or wider visibility is included. Unexpected behavior
differences stop progression and require separate investigation.

## Verification

Before extraction, run language unit tests and the native/editor suites against
the effective local base. The existing
`hover_markdown_separates_signature_from_documentation_and_tags` test fixes code,
prose, embedded code fences, tags, plaintext, and marked-string behavior. Native
hover/signature/documentation tests and editor baselines exercise the consumer
contracts. Reuse these tests rather than adding tests that mirror the moved code.
Rare malformed JSON/fence inputs and supplementary-character substring labels
are covered by exact structural equality rather than an exhaustive case claim.

Commands, with the existing Rust 1.98.0 and pinned TypeScript
7.1.0-dev.20260826.1 toolchain:

```sh
TTC_REQUIRE_TSGO=1 cargo test --offline --lib engine::language::tests
TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TYPESCRIPT_CASES=1 \
  cargo test --offline --test native --test editor_cases
./scripts/ci agents rust
```

The full head gate repeats the focused suites, checks baselines, and checks the
fuzz crate. Compare all four complete definitions to the saved base after
reversing only the three visibility changes and any rustfmt signature wrapping.
Remove the new declaration/re-export and restore moved blocks; the entire
service parent must then match the saved source byte for byte. Independently
inspect the production diff and all callers. Record actual exit statuses,
skip/ignore counts, coverage limitations, and any environment issues in the task.
