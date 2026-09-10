# TASK-357: Preserve contextual project inputs and package resolution

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-357: fix(compiler): preserve contextual project boundaries`

## Purpose

Verify TASK-356 and correct remaining error suppression and package shadowing in
standalone contextual typing.

## Scope

- Included: candidate input failures, toolchain availability, standard support
  package lookup, and regression tests.
- Excluded: unrelated language changes.

## Decisions

### Decision 1: A project snapshot must not silently lose readable dependencies

- **Context**: Ignoring a fail-fast directory walk yields a prefix of the sorted
  tree; skipping failed reads also hides potentially imported inputs.
- **Alternatives considered**: Skip unreadable entries; silently use partial facts.
- **Decision and rationale**: Report input failures with the input path. Only an
  absent toolchain removes refinement. Do not infer types over a partial snapshot.

### Decision 2: Compiler support must respect ancestor package resolution

- **Context**: A root-local synthetic package shadows a real package in an
  ancestor node_modules, including monorepo installations.
- **Alternatives considered**: Always overlay compiler sources; check only root.
- **Decision and rationale**: Supply embedded packages only when the ancestor
  node_modules chain contains no project package.

## Work log

- 2026-09-10: Doctor passed. Fast-forwarded the clean branch to PR head 5a298b1.
  Reviewed TASK-356 against both previous review findings.

- 2026-09-10: Confirmed both new regression tests fail at 5a298b1 and pass
  after the fixes. Strengthened the standard-library regression to inspect the
  inferred payload and fixed the analysis/output-path separation it exposed.
- 2026-09-10: Targeted CLI tests (74) and snapshots (4) passed without ambient
  standard-library packages. Converted the previous contribution-guide update
  to English and documented the contextual-input boundary.

## Issues and resolutions

### Input failure suppression

- Symptom: a dangling entry sorted before dependencies allowed a partial scan;
  invalid UTF-8 siblings were silently skipped.
- Cause: TASK-356 discarded directory-walk and read failures; the shared walker
  also discarded directory-iterator errors.
- Resolution: decide absent-toolchain behavior before scanning; propagate input
  failures through `StandaloneFailure::Input` and preserve backend failures.
  The shared walker now propagates iterator errors as well.

### Ancestor package shadowing

- Symptom: a custom ancestor `@tt/std/result` returning strings was typed using
  the embedded Result constructors instead.
- Cause: support-package presence was checked only at the source root.
- Resolution: respect the ancestor node_modules chain before creating overlays.

### Output paths used during analysis

- Symptom: the existing standard-library test passed without actually inferring
  any join type; strengthening it exposed an unannotated Result slot.
- Cause: analysis retained adapter rewrites pointing to not-yet-written output.
- Resolution: analyze authored support-module specifiers, then rewrite final
  annotations using the adapter's output mapping. The test now requires an
  inferred numeric payload, not only a zero exit code.

## Verification

- [x] Targeted regression tests and snapshots
- [x] cargo fmt --check
- [x] cargo clippy --all-targets -- -D warnings
- [x] cargo test

- `./scripts/ci agents rust` passed, including all Rust suites with
  `TTC_REQUIRE_TSGO=1` and the locked fuzz-target build. No ambient @tt/std
  installation was added to the checkout.

## Result

Corrected project-input propagation in `src/typescript/contextual.rs` and the
shared walker in `src/engine/project.rs`. Updated error rendering in
`src/lib/compile.rs`, package lookup and source/output module mapping, and CLI
regressions. Updated the contextual design, contribution instructions, task
index, and TASK-356 supersession notice. No unresolved findings remain in this
review's scope.
