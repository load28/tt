# TASK-772: Fix the deferred findings of earlier audits

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

Earlier audit tasks left findings for a later round. This task fixes the
ones that can be fixed in this repository, before the next audit round.

## Scope

- Included: the quadratic copy of a nested template's pieces
  (TASK-765 result); hover and definition of a nested tag under a generic
  payload (TASK-771 decision 8); completion inside a variant whose body is
  not closed, and tokens and the unused-parameter hint around a declaration
  whose name is missing (TASK-767 scope); the editor's placement and
  wording of checker errors a structured mismatch or glue explains
  (TASK-769 R3E3, TASK-767 scope).
- Excluded: tsgo's content-mapper diagnostic cost (TASK-771 decision 6),
  which is upstream.

## Decisions

### Decision 1: A deep template around a tt value does linear work

- **Context**: TASK-765 left a nested template's emission quadratic. A
  template nested 20,000 deep around one `match` took 19.0 s for `--check`
  (4,000: 0.27 s; 8,000: 1.21 s). Stack samples showed four separate costs
  that each grew with depth.
- **Decision and rationale**:
  - `Rope::append` moved every piece of the child into the parent at each
    level. Pieces are now a `VecDeque`, and the shorter side moves, so a
    rope that wraps its child level by level moves only the wrapper.
  - `FunctionTargets::new` recurses into each template interpolation and
    asked `match_owned_tokens`, which walked every expression of the file,
    once per interpolation. `HirFile::match_owned` now collects the offsets
    a match's `{` and an arm's `=>` follow once, sorted, and each token
    stream reads only the offsets inside its own extent. An offset outside
    the extent named no token of that stream before either.
  - `validate_order` asked, for each step, whether any later step is
    conditional by scanning the rest. The reverse walk now carries that
    fact.
  - `lex_region` reserved `(end - start) / 3` token slots for every region.
    A braced region (an interpolation) ends at its `}`, not at `end`, so each
    nested region reserved the rest of the file again. Only the open region
    is sized up front.
  After the change, 20,000 levels take 0.51 s and 40,000 take 1.09 s.

### Decision 2: Deep match nesting loses its cubic term; the per-overlay path stays

- **Context**: While measuring decision 1, `match` nested inside `match`
  grew cubically: depth 1,000 took 1.46 s, 2,000 took 9.8 s, and 4,000 took
  79 s. `evaluation_owner` walks a value's ancestor path and, at each
  ancestor, searched the list of decision functions on that path.
- **Decision and rationale**: The decision-function and decorated-class
  indices are collected in path order, so they are binary-searched. Depth
  4,000 now takes 23 s and 1,000 takes 0.64 s.
- **Not changed**: each overlay still records its full ancestor path
  (`record_overlay`), which is quadratic in depth. Sharing path prefixes
  would change every consumer of `FoundOverlay::parents`. Depth in the
  thousands is not written by hand; it is recorded here rather than
  restructured.

### Decision 3: A nested tag under a generic payload is answered by a typed request

- **Context**: Hover and definition on `Circle` in `Has(item: Circle(r))`
  over `Opt<Shape>` showed nothing (TASK-771 decision 8). `ttSymbol` is
  parse-only by its protocol contract, and the field's declared type `T`
  names no variant.
- **Alternatives**: Make `ttSymbol` consult the project, which would spawn
  the checker for a parse-only request on every hover; or let TypeScript's
  hover answer on the emitted `"Circle"` literal, which TypeScript does not
  resolve to a declaration.
- **Decision and rationale**: A new project-backed request, `patternSymbol`
  (`Project::pattern_symbol`), answers the same `TtSymbol` shape for a
  nested tag whose position the parse-only answer leaves empty. It asks
  TypeScript which tags the payload admits at that arm (the path of TASK-771
  decision 8), takes the one visible variant that has all of them
  (`completions::sole_owner`, shared with completion), and renders that
  variant's case as `ttSymbol` would. The VS Code server asks it only when
  `ttSymbol` is null, for hover and definition. The editor case harness
  asks it on every hover and prints it only when it answers, as the client
  shows it.

## Work log

- 2026-10-06: Started from the deferred findings of TASK-765, TASK-767,
  TASK-769, and TASK-771.
- 2026-10-06: Profiled deep templates with gdb stack samples of a
  symbol-carrying release build; fixed the four costs of decision 1, then
  the cubic term found beside them (decision 2).
- 2026-10-06: Added `patternSymbol` to the engine, the JSON-lines server,
  the editor case harness, and the VS Code server (decision 3); ran
  `npm test` in `editors/vscode` (243 passed).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
