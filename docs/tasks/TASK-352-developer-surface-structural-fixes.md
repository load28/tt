# TASK-352: Repair the developer-facing surfaces of tt

- **Status**: In progress
- **Started**: 2026-09-09
- **Completed**: —
- **Commit**: —

## Purpose

Audit what a developer actually runs — the documentation, the `ttc` command
line, the compiler, and the editor integration — and repair each confirmed
defect in the layer that owns the behavior.

## Scope

- Included: user-facing documents and their examples, every CLI mode and its
  edge inputs, compiler diagnostics and emitted output, and the VS Code
  extension with the engine surfaces it drives
- Excluded: release publication, and any change to what the language means

## Decisions

### Decision 1: Audit each surface against the tools, not against the tests

- **Context**: The suite is green, so a defect that survives it is one no
  test describes. Reading the code alone would reproduce its assumptions.
- **Alternatives considered**: Extend the existing suites and see what
  breaks; sample features by hand; drive every documented surface with the
  built compiler and compare what it does against what it promises.
- **Decision and rationale**: Exercise the shipped binary, the extension
  build, and every documented example, and treat only a reproduced
  difference as a finding. Each finding carries its command and output.

### Decision 2: A subject index is the Core IR's invariant, so lowering owns it

- **Context**: A tuple arm naming more positions than the match has
  scrutinees crashed emission — once by indexing past the subject list, once
  by handing the switch emitter an alternative with no constructor test.
- **Alternatives considered**: Add `match-tuple-arity` to
  `blocks_projection` (this suppressed the file's independent type errors,
  which TASK-117 requires to survive); leave the arity-mismatched match as
  source text in HIR (the same suppression, plus a projection that no
  longer parses).
- **Decision and rationale**: `Place::subject` indexes the decision's own
  subjects, so Core IR lowering keeps only the positions that have one. Sema
  still reports the arity, the emitted TypeScript still parses, and the
  file's other diagnostics still reach the user. A one-position conjunction
  collapses to that position's plan so a single-subject decision keeps the
  arm shape the switch emitter is documented to receive. The invariant is
  now asserted where it belongs, in `validate_decision`.

## Work log

- 2026-09-09: Ran `./scripts/doctor`, installed dependencies, built the
  release compiler, and recorded a green `cargo test` baseline.
- 2026-09-09: Audited the four surfaces against the built tools and
  reproduced each reported difference before recording it.
- 2026-09-09: Repaired the two tuple-arity compiler crashes in Core IR
  lowering and pinned the contract in `tests/compile/cases_06.rs`.
- 2026-09-09: Rebased onto `main` at `e3b89e2` and re-verified.
- 2026-09-09: Gave the block a concise arrow body is rewritten to one
  closing brace, written where the body ends, and covered the placement
  matrix in `tests/compile/cases_06.rs`.
- 2026-09-09: Narrowed the declared return type a `return` slot carries to
  the functions whose declaration names that value's type, and checked the
  emitted output of every shape with the repository's TypeScript.
- 2026-09-09: Repaired three command-line defects — a banner on a
  hand-written file, named file inputs losing their depth under `-o`, and a
  temporary directory left behind by every typed run — and covered the
  first two in `tests/cli.rs`.

### Decision 3: A rewritten arrow body closes where the body ends

- **Context**: The brace that closes a concise arrow body's block was
  written from two places, each of which knew only part of the body.
- **Alternatives considered**: Widen the source walk's boundary alone (the
  early close from the value path remains); suppress the value path
  entirely for arrow owners (it also emits the value, so the output loses
  it).
- **Decision and rationale**: Keep both paths, and let each close only at
  the position that ends the body — the value path when the value *is* the
  body, the source walk when source follows it. A registry on the emitter
  makes the brace exactly one write, so neither path has to know whether
  the other already ran.

### Decision 4: The passthrough contract decides whether a banner is written

- **Context**: A hand-written `.ts` copied into the output tree arrived with
  `// @generated from plain.ts by ttc — do not edit directly.` on top —
  untrue of a file its author wrote, and a byte the passthrough contract
  does not allow. TASK-336 settled where a banner goes and explicitly left
  whether one is written out of its scope, so this is not a reversal of it.
- **Alternatives considered**: Keep the banner and reword it; let
  `--no-banner` be the answer (it is off by default, so the default output
  would still break the contract).
- **Decision and rationale**: `AGENTS.md` allows exactly one change to a
  hand-written file — its relative tt import specifiers. The banner is
  written for the surfaces ttc compiles, which is the same condition the
  source map beside it already applies.

### Decision 5: Named file inputs mirror under the directory they share

- **Context**: `-o` promises to mirror input paths, but a named file was
  written by file name alone, so `src/sub/helper.ts` landed at
  `build/helper.ts` and its rewritten `../shape.js` pointed outside the
  output tree.
- **Alternatives considered**: Mirror every input under one common root
  (this dissolves the two output-collision contracts TASK-321 and TASK-338
  established for directory inputs); keep the file name and rewrite
  specifiers to match (the output would no longer mirror the input).
- **Decision and rationale**: Named files mirror under the deepest
  directory they are all inside. A directory input keeps mirroring under
  itself, so both collision contracts answer exactly as before.

## Issues and resolutions

### Issue 1: A tuple arm wider than its match crashed the compiler

- **Symptom**: `match (x) { (A, B) => 1 }` exited 101 with `internal
  compiler error: switch variant alternative tests no constructor`, and
  `match (x, y) { (A, B, C) => 1, _ => 2 }` with `index out of bounds: the
  len is 2 but the index is 2`. Both crashed `--check` and `--emit-map`, so
  an editor keystroke could kill the compiler.
- **Cause**: `Lowering::pattern_at` turned every tuple element into a
  `Place` whose `subject` was the element's index, without consulting the
  decision's subject count. The parser keeps an arity-disagreeing tuple
  match on purpose so sema can name the mismatch, so the two phases
  disagreed about what a position was.
- **Resolution**: Lowering takes only the elements that have a subject and
  collapses a one-position conjunction to that position's plan.
  `validate_decision` now asserts that every place an arm tests names one of
  the decision's own subjects.

### Issue 2: A concise arrow body containing a tt value lost its brace

- **Symptom**: `[1].map(x => x + match (s) { ... })` emitted
  `return $tt_v1 + $tt_v0);` with no closing brace, and
  `[1].map(x => match (s) { ... } + x)` closed the block before `+ x`,
  leaving it outside the arrow. Both failed the output self-check with a
  message that names no position the user can act on.
- **Cause**: The block's closing brace was written either by the
  structured-value path, which fired whenever the value *started* the body,
  or by the source walk, whose boundary excluded a following span that
  begins exactly where the body ends. A body whose value sits at the tail
  matched neither.
- **Resolution**: Each path now closes only at the end of the body, and a
  registry on the emitter keeps the brace to one write.

### Issue 3: A generator's declared type was copied onto its return slot

- **Symptom**: `function* gen(): Generator<number, string, void>` emitted
  `let $tt_v0: Generator<number, string, void>;` and its async counterpart
  `let $tt_v1: Awaited< AsyncGenerator<...>>;` — both rejected by tsc. A
  type predicate emitted `let $tt_v2: x is number;`, which does not parse,
  so the user saw `verify-failed` with no position.
- **Cause**: The host projection captured `return_type` for every function
  and lowering copied it onto the slot the `return` assigns through. That
  is sound only when the declared return type *is* the returned value's
  type; a generator declares the iterator it produces, and a predicate is
  not a type.
- **Resolution**: One predicate in the projection decides what the
  annotation describes, and answers `None` for generators and predicates,
  which leaves the slot to be inferred from the value assigned to it.

### Issue 4: A hand-written file was published as generated

- **Symptom**: `ttc -o build src` wrote `// @generated from helper.ts by
  ttc — do not edit directly.` into the copy of a hand-written `.ts`.
- **Cause**: The banner was written for every job; only the source map
  beside it asked whether the file was one ttc compiles.
- **Resolution**: The banner asks the same question. A passthrough file is
  now byte-identical to its source apart from its tt specifiers.

### Issue 5: Named file inputs lost their depth under `-o`

- **Symptom**: `ttc -o build src/shape.tt src/sub/helper.ts` wrote
  `build/helper.ts`, whose `../shape.js` resolves above `build`.
- **Cause**: A named input's output path was its file name.
- **Resolution**: Named files mirror under the deepest directory they share.

### Issue 6: Every typed run left a temporary directory behind

- **Symptom**: `/tmp` held over a thousand empty `ttc-host-*` directories;
  each `--check-types` or `--types` run added one.
- **Cause**: The session removed the host script it wrote but not the
  directory it created to hold it.
- **Resolution**: The session owns that directory and removes it on drop.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

In progress.
