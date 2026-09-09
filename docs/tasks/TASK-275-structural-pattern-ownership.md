# TASK-275: Prove variant ownership and preserve typed project boundaries

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: —

## Purpose

Resolve the four findings from PR #86 without global spelling heuristics: pattern
diagnostics must follow authoritative variant ownership, typed diagnostics must stay
inside the configured project, and public declaration input must retain a supported
construction contract.

## Scope

- Included: match and nested-pattern ownership, generic substitution evidence,
  blocked-file project membership, `ExternVariant` compatibility, negative regression
  tests, and the PR review findings
- Excluded: unrelated diagnostic wording, new syntax, and automatic PR approval or merge

## Decisions

### Decision 1: Require declaration evidence before reporting variant names

- **Context**: The current single-name and nested-tag fallbacks can associate a valid
  hand-written TypeScript union with an unrelated visible tt variant.
- **Alternatives considered**: Broaden spelling heuristics / suppress the new false
  positives after the fact / carry authoritative subject and payload ownership through
  resolution
- **Decision and rationale**: Resolve only against a declaration identified by exact
  pattern evidence or by a payload field's declared concrete type. A lone near-miss and
  a generic type parameter carry no owner, so TypeScript owns those typed questions.
  This preserves the TypeScript passthrough contract instead of tuning a global
  heuristic.

### Decision 2: Test rejected inference before changing implementation

- **Context**: Existing tests cover intended imported-variant diagnostics but not valid
  hand-written unions or excluded malformed files.
- **Alternatives considered**: Patch implementation first / add focused negative tests
  that reproduce each review finding
- **Decision and rationale**: Add the negative contracts first so the responsible layer
  and the absence of collateral suppression are observable.

### Decision 3: Derive typed membership from the configured graph

- **Context**: A blocked source has no projected module path, so filtering only projected
  files lets its diagnostics bypass tsconfig membership.
- **Alternatives considered**: Drop every blocked diagnostic / scan tsconfig text in the
  diagnostic layer / seed membership from TypeScript's program and requested roots, then
  follow relative tt imports
- **Decision and rationale**: Treat TypeScript's module list and explicit inputs as roots,
  and extend membership through the tt import graph held by the snapshot. Both projected
  and blocked files are then filtered by one graph-derived set.

### Decision 4: Keep rich engine symbols internal

- **Context**: Adding generic and field shapes directly to public `ExternVariant` changed
  its supported struct-literal construction contract.
- **Alternatives considered**: Accept the breaking API / add optional public fields / keep
  `ExternVariant` tag-only and use `VariantSymbol` inside the engine
- **Decision and rationale**: Restore the public tag-only shape exactly. The engine already
  has `VariantSymbol` for rich declarations, so internal correctness does not require a
  public breaking change.

## Work log

- 2026-08-29: Registered the follow-up task from PR #86 review and recorded the
  structural ownership requirement.
- 2026-08-29: Added negative resolver, public API, and excluded-file regressions before
  changing ownership and membership behavior.
- 2026-08-29: Removed spelling-only single-pattern and generic nested-tag fallbacks;
  checker-owned TS2339 and TS2678 diagnostics now map back to source coordinates.
- 2026-08-29: Unified projected and blocked diagnostic membership over configured roots
  plus transitive relative tt imports.
- 2026-08-29: Restored the tag-only `ExternVariant` API and kept rich field declarations on
  the engine's `VariantSymbol` path.
- 2026-08-29: Ran the Rust gate after focused CLI, server, content-mapper, and resolver
  regressions passed.

## Issues and resolutions

### Issue 1: Spelling selected a declaration without subject evidence

- **Symptom**: A one-edit pattern could be diagnosed against an unrelated visible variant.
- **Cause**: The resolver granted single-pattern sites a global near-miss fallback.
- **Resolution**: Removed that fallback. Exact declaration evidence still resolves normal
  multi-arm matches; otherwise the typed checker decides compatibility.

### Issue 2: A nested generic payload selected a declaration by tag

- **Symptom**: An exact nested tag could select an unrelated unique visible variant.
- **Cause**: Generic substitution was approximated with `unique_variant_with_tag`.
- **Resolution**: Nested ownership now follows only a concrete declared field type. Generic
  substitution remains checker-owned.

### Issue 3: Blocked diagnostics escaped tsconfig membership

- **Symptom**: A malformed `.tt` outside `include` entered typed CLI and server reports.
- **Cause**: The project-module filter covered projected documents but not blocked files.
- **Resolution**: Both collections now use one membership set derived from configured
  modules, requested roots, and their transitive relative tt imports.

### Issue 4: Rich imports broke the public declaration literal

- **Symptom**: Existing callers constructing `ExternVariant { name, tags, from }` no longer
  compiled.
- **Cause**: Rich generic/case/field data replaced the public `tags` field.
- **Resolution**: Restored the original public shape and routed rich engine imports through
  `VariantSymbol` and internal `ExternDecl`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Pattern diagnostics no longer infer declaration ownership from spelling or a generic
payload tag. Typed diagnostics stay within the configured project graph, direct property
errors retain source spans and checker wording, and the public tag-only declaration API is
source-compatible again.

Changed files:

- `src/resolve/mod.rs`, `src/analysis/mod.rs`
- `src/engine/project.rs`, `src/engine/semantics.rs`, `src/engine/snapshot.rs`
- `src/lib.rs`, `src/main.rs`, `src/content_mapper.rs`
- `tests/compile.rs`, `tests/content_mapper.rs`, `tests/integration.rs`, `tests/native.rs`,
  `tests/resolve.rs`
- `docs/tasks/INDEX.md`, `docs/tasks/TASK-275-structural-pattern-ownership.md`
