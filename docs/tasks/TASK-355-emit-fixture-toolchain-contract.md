# TASK-355: Say which artifact an emit fixture pins

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-355: test(snapshot): hold emit fixtures to the configured artifact`

## Purpose

ttc annotates the storage it generates for a match or a `result` with the
type TypeScript infers for it, so the same program emits a slot with a type
where a project's TypeScript is installed and one without where it is not.
Fourteen of the twenty-three emit fixtures therefore had two possible
contents, and nothing said which one was the contract.

## Scope

- Included: what an emit fixture pins, how the suite behaves without a
  toolchain, and what the gate requires
- Excluded: whether the annotation should exist (TASK-353 decided that) and
  the diagnostic fixtures, which do not depend on a toolchain

## Decisions

### Decision 1: The fixture is the artifact a configured checkout produces

- **Context**: `./scripts/doctor` rejects a checkout with no TypeScript and
  names `npm ci` as the fix, CI installs one, and a user with a project has
  one. The annotated emit is what all three produce.
- **Alternatives considered**: Pin the unannotated form (it is nobody's
  output, and it would stop covering what TASK-353 added); keep both forms
  as two expectations per case (fourteen extra files, and one regeneration
  run can only write one of them); normalise the annotation away before
  comparing (a real change inside an annotation would stop being visible).
- **Decision and rationale**: Pin the annotated artifact and make the
  requirement explicit, rather than let the same command produce two
  different contracts depending on the machine.

### Decision 2: Regeneration refuses before it writes

- **Context**: Without a toolchain the suite reported "fixture is out of
  date", which sends a contributor to `UPDATE_EXPECT=1` — and that run
  would rewrite fourteen fixtures to the unannotated form, which looks like
  a successful update and commits the wrong artifact.
- **Alternatives considered**: Warn and write anyway.
- **Decision and rationale**: The failure a contributor cannot recover from
  is the one to prevent. Regeneration without a toolchain refuses and names
  `npm ci`; comparison skips and says why.

### Decision 3: One toolchain answer for both suites

- **Context**: The backend suite already had a `toolchain()` guard with the
  `TTC_REQUIRE_TSGO` escape. The emit fixtures needed the same question.
- **Decision and rationale**: Move it to `tests/common/`, so the two cannot
  disagree about whether a checkout is configured, and so the gate's escape
  hatch means one thing.

## Work log

- 2026-09-10: Measured the dependency: regenerated the fixtures with and
  without `node_modules` and compared. Fourteen of twenty-three differ, and
  every difference is one join-slot declaration gaining or losing its type
  annotation. The emitted programs are otherwise identical, and the
  diagnostic fixtures do not differ at all.
- 2026-09-10: Lifted the toolchain guard into `tests/common/`, guarded the
  emit cases with it, made regeneration refuse without one, and set
  `TTC_REQUIRE_TSGO=1` for the gate's `cargo test` so the contract is never
  skipped there.
- 2026-09-10: Documented the rule where the fixture instructions are, in
  `CONTRIBUTING.md`.

## Issues and resolutions

### Issue 1: The same command pinned two different artifacts

- **Symptom**: `cargo test` in a checkout that had not run `npm ci` failed
  with `tests/fixtures/emit/contextual-composed-match/expected.ts is out of
  date`, and `UPDATE_EXPECT=1` there rewrote fourteen fixtures to a form no
  configured checkout produces.
- **Cause**: The emit artifact carries checker-inferred slot types, and
  nothing in the suite said the fixtures pin the form a checker produces.
- **Resolution**: The fixtures pin the configured artifact. Without a
  toolchain the emit cases skip and say why, `TTC_REQUIRE_TSGO=1` turns
  that skip into a failure, and regeneration refuses outright.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — all six stages pass.

The four cases this task is about were each run with `node_modules` moved
aside and put back:

| condition | result |
| --- | --- |
| toolchain present | 4 passed |
| toolchain absent | 4 passed, emit cases skipped with their reason |
| absent, `TTC_REQUIRE_TSGO=1` | fails, naming `npm ci` |
| absent, `UPDATE_EXPECT=1` | refuses before writing; fixtures unchanged |

## Result

An emit fixture now states which artifact it is: the one a checkout that
has run `npm ci` produces, annotations included. A checkout without one
cannot silently verify a different contract, and cannot silently write one.
