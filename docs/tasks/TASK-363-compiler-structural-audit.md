# TASK-363: Repair compiler boundary defects found by a parallel audit

- **Status**: Complete
- **Started**: 2026-09-11
- **Completed**: 2026-09-11
- **Commit**: `TASK-363: fix(compiler): repair audited boundary defects`

## Purpose

Audit independent compiler layers in parallel and repair reproducible defects at
the layer that owns each contract.

## Scope

- Included: TypeScript pass-through ownership for `match` members, distinct case
  evidence in variant resolution, and `.mts`/`.cts` host overlays in typed engine
  snapshots
- Excluded: New language syntax, diagnostic suppression, and unrelated editor or
  release behavior

## Decisions

### Decision 1: Keep each repair in its owning compiler layer

- **Context**: The audit found three independent failures in parsing, resolution,
  and project source classification.
- **Alternatives considered**: Add input-specific parser exclusions, suppress the
  false diagnostic, or special-case additional extensions at each consumer.
- **Decision and rationale**: Repair candidate ownership in the parser, normalize
  resolution evidence before scoring, and make the engine's TypeScript extension
  set the single source of truth. Each choice applies to the structural class of
  inputs rather than the observed reproducer.

## Work log

- 2026-09-11: Fast-forwarded `main` to `fcaf2a0`, ran `./scripts/doctor`, and
  confirmed the baseline `cargo test` suite passes.
- 2026-09-11: Audited parser/codegen, semantic resolution, and engine/backend
  boundaries with three read-only task agents and reproduced one defect in each.
- 2026-09-11: Classified host-owned `match` member and function names before tt
  parsing while preserving tt matches inside methods, objects, and JSX hosts.
- 2026-09-11: Normalized each pattern position's resolution evidence to distinct
  case names before declaration scoring.
- 2026-09-11: Reused the engine's complete TypeScript extension set for project
  discovery and host-overlay classification.
- 2026-09-11: Added pass-through, compile, resolve, and native server regression
  coverage and ran the full Rust formatting, lint, test, and doctest gates.

## Issues and resolutions

### Issue 1: TypeScript `match` members can be claimed as tt matches

- **Symptom**: A valid class or object method named `match` whose body starts with
  an arrow expression fails with `malformed-match` or is lowered as tt syntax.
- **Cause**: Match intent uses a top-level `=>` in the body as ownership evidence
  without excluding host member-key positions.
- **Resolution**: The parser now recognizes host member-key and function-name
  positions from enclosing grammar and leaves those constructs verbatim. JSX
  expression containers remain tt expression hosts.

### Issue 2: Repeated pattern occurrences bias variant ownership

- **Symptom**: Repeating a guarded case can turn another variant's exact case into
  an `unknown-case` diagnostic.
- **Cause**: Resolution scores occurrence counts instead of distinct case names.
- **Resolution**: Resolution now scores each distinct case name once per pattern
  position, regardless of guarded or or-pattern repetition.

### Issue 3: `.mts` and `.cts` overlays disappear from typed snapshots

- **Symptom**: Unsaved `.mts` and `.cts` host modules are unresolved or stale in
  typed checks while equivalent `.ts` and `.tsx` overlays work.
- **Cause**: Project discovery recognizes four TypeScript extensions but the
  shared host-overlay predicate recognizes only two.
- **Resolution**: Project discovery and the shared host-overlay predicate now use
  the same `TS_EXTENSIONS` constant, including `.mts` and `.cts`.

### Issue 4: Initial member classification captured JSX expression containers

- **Symptom**: Three existing program-syntax tests failed because tt matches in
  JSX attributes and children remained in the projected TypeScript.
- **Cause**: The first object-literal classifier treated braces after `=` and `>`
  as object literals without recognizing JSX opening-tag context.
- **Resolution**: JSX expression-container ownership is now identified by a
  balanced reverse walk to the opening tag and excluded from member containers.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/parser/parse.rs`, `src/resolve/mod.rs`, `src/engine/mod.rs`, and
`src/engine/project.rs` at their owning boundaries. Added regressions in
`tests/passthrough.rs`, `tests/resolve.rs`, `tests/compile/cases_08.rs`, and
`tests/native/cases_01.rs`. Valid TypeScript host declarations pass through,
variant ownership is stable under repeated cases, and unsaved Node-format host
modules participate in typed snapshots.
