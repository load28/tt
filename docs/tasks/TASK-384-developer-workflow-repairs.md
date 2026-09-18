# TASK-384: Repair developer workflow contracts

- **Status**: In progress
- **Started**: 2026-09-18
- **Completed**: —
- **Commit**: —

## Purpose

Repair the structural compiler, project, CLI, editor, and bundler defects reproduced in TASK-383, with regression coverage and a pull request.

## Scope

- Included: F1–F17, shared invariants, targeted and full local gates, and PR review.
- Excluded: E1's installed native extension mismatch, already fixed in main's source patch; no extension installation or release publication.

## Decisions

### Decision 1: Repair the owning contract

- **Context**: The audit spans expression scheduling, source-kind handling, project membership, filesystem ownership, and dependency invalidation.
- **Alternatives considered**: special-casing reproducers, disabling diagnostics or invariants, or falling back to alternate compilation.
- **Decision and rationale**: Preserve the invariants and repair each authoritative model, with paired behavior tests and the production audit fixtures.

## Work log

- 2026-09-18: Doctor passed; origin/main remains at 069a5fb. Preserved the audit artifacts and unrelated .task-agent-disabled.

## Issues and resolutions

- F1: Expression-only pipelines were assigned host slots that no action produced. Slot resolution now distinguishes expression lowering from statement-region production.
- F2: Overlapping source captures re-evaluated shared operands. Evaluation IR now records dependencies between captures, and emission composes captures from prior values and lowered Core nodes.
- F3: Captured functions and short-circuit conditions retained raw tt syntax. Captures now use the structural emitter, including statement extents and conditional-operation ownership.
- F4: Declaration output paths flattened relative directory inputs. Paths now share normalized input roots and all declaration targets are checked for collisions before writing.
- F5: Explicit tt inputs could overwrite authored TypeScript. Exact-content ownership records now authorize replacement only for unchanged outputs from the same source, independently of banners.
- F6: Typed watch only tracked tt timestamps, and the persistent backend retained disk state. Project watch inputs now include host sources, configurations, resolved dependencies and directory membership; backend snapshots receive filesystem changes.
- F7: Declaration emission used a frozen requested-file set. Directory input roots now admit newly discovered files during updates.
- F8: In-place output re-entered the input scan. Directory discovery excludes unchanged compiler-owned outputs.
- F9: Editor filesystem events omitted `.mts` and `.cts`. The watcher now includes both host extensions.
- F10: Bundler bare imports were joined to importer directories. Resolver-capable hosts now resolve package exports and external decisions through their own resolver; esbuild uses its native resolve hook.
- F11: Type-only dependencies did not invalidate cached Vite consumers. The compiler exposes dependency paths, the plugin registers them, and Vite invalidates affected module graphs even without HMR.
- F12: JSX coverage analysis reparsed as TypeScript. Coverage and projection now carry the original file's SourceKind.
- F13: Generic variant constructors emitted ambiguous JSX arrows. TSX emission disambiguates generic arrow parameters with a trailing comma.
- F14: A return's lexical containment incorrectly captured values inside nested callbacks. Evaluation ownership now requires the host to own the whole return argument. Completed calls and larger captures preserve their result slots across enclosing structural emission.
- F15: External relative tt imports were not projected. Candidate discovery follows explicit relative tt imports beyond the project root; TypeScript still decides membership.
- F16: JSX closing tags were mistaken for regex syntax while locating container ends. The lexer now finds balanced expression containers in the file's lexical mode and reuses their token stream, including nested JSX and templates.
- F17: Custom-extension include/files patterns did not name projected paths. Configuration patterns are projected through the same extension mapping, including extended JSONC configuration and exclusions, without changing authored files.

Additional regressions found during the full gate were fixed in the same owning models: unsaved editor documents remain valid discovery candidates, and newly learned watch dependencies establish their baseline without a redundant startup check.

## Verification

- Targeted compile suite: 443 passed.
- Integration runtime checks cover exact effect order, nested callback returns, overlapping calls, computed keys, and short-circuit conditions.
- Workflow tests cover output ownership, declaration layout, custom-extension membership, external imports, watch invalidation, and JSX production shapes.
- Actual VS Code isolated-profile probe: all four `.mts`/`.cts` to `.tt`/`.ttx` disk-change cases passed.
- React cart and relocatable library: typed checking, Vite bundling, server rendering, and standalone output checking passed. The nested calculation renders 7 for quantity 2 and price 5.
- Vite package-exports and type-only invalidation probes passed; five unplugin tests passed.
- Final gate coverage: `./scripts/ci` passed agents, npm, website, native, and extension; after repairing four source-ownership regressions, `./scripts/ci rust` passed formatting, clippy, all Rust tests, doctests, and fuzz-target compilation. Initial npm/website attempts needed network/listen access unavailable in the sandbox.
- Composition matrix: 1,815 cases; 1,743 accepted, 72 existing unsupported-placement diagnostics, zero unexpected failures (baseline: 711 internal errors plus 256 projection failures).
- Production probes: 29 typed/build cases passed; all five runtime scenarios matched expected values and resource-disposal/effect traces.
- Runtime composition matrix is still running; its final trace counts will be attached before completing the PR.

## Result

Compiler invariants and diagnostics remain enabled. No installed extension or release artifact was replaced. The supported scope is the reproduced failure families and their structural regressions, not an exhaustive proof for all programs or platforms.

Changed areas: Evaluation/Core/HIR ownership, source-preserving emission, lexer and coverage, project/backend invalidation, CLI output ownership and declaration paths, editor filesystem events, bundler resolution/cache invalidation, regression tests, snapshots, and user-facing documentation.
