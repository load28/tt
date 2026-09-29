# TASK-477: The typed `val` check sees every spelling of a member call

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttc --check-types` reported `items.push(1)` through a `val` binding but accepted `items["push"](3)`, `(items.push)(4)` and `m["delete"]("a")`, which call the same built-in mutators. `docs/ai/tt.md` §val says the typed modes report "a call they resolve to a built-in mutator"; the spelling of the member access does not change what the call resolves to.

## Scope

- Included: Collection of method-call probes (`src/val.rs`, `src/val/checker.rs`, new `src/val/calls.rs`), its callers (`src/lib/mapped.rs`, `src/engine/projection.rs`), `docs/ai/tt.md` §val, and tests.
- Excluded: The verdict (`src/engine/semantics/report.rs`), which already asks the checker whether the resolved method is a TypeScript built-in and applies tt's mutator policy. Assignment, increment and `delete` probes and the `val-pass` probes, which stay on the token walk.

## Decisions

### Decision 1: Read method calls from the SWC syntax tree of the emitted TypeScript

- **Context**: The token walk recognized a call only as `root(.name)* .name (`: `parse_path` dropped the method name at a `[..]` step, and a callee wrapped in parentheses put a `)` between the path and the `(`. Extending those token patterns would add a case per spelling and still miss `(items as number[]).push`, `items![k]` or an optional call.
- **Alternatives considered**:
  - Extend `parse_path` for string-literal indexes and look back over `(`. More token shapes, the same class of gap.
  - Parse the `.tt` source with SWC. It is not TypeScript while it contains tt constructs.
  - Build `ProgramSyntax`'s placeholder projection. It needs the semantic and Core files and replaces tt values, including the arm bodies whose calls must be seen.
- **Decision and rationale**: The emitted TypeScript is complete TypeScript in which the user's code is copied byte for byte, and `MappedEmit` maps each copied byte back to its source byte. `val::method_calls` parses it with the project's SWC parser (`HostInput`), visits every `CallExpr` and optional call, unwraps the callee through parentheses, non-null and type assertions, and collects a member callee whose key is an identifier, a string literal or a template literal without substitutions, rooted (through any path depth) at an identifier. The root and the key are mapped back to source bytes; a call whose root or key is compiler glue is skipped. The engine passes the emission it already has (`val_probes_with_emit`); the public `val_probes` builds one when the file declares a `val` binding. The token walk no longer collects calls, so there is one collection path.

### Decision 2: A computed non-literal key is not judged

- **Context**: `items[k](1)` may call `push` at run time, but the key names no method.
- **Alternatives considered**: Ask the checker for the key's type and judge each literal member of a union. That is a different question (which property a type can name) from the one the verdict asks (which declaration a name resolves to), and `k: string` would still have no answer.
- **Decision and rationale**: Not collected, like an unresolvable receiver. Documented in `docs/ai/tt.md` §val and in the `val_probes` documentation example.

## Work log

- 2026-09-28: Reproduced with the hunter project `p4`: of `items.push(1)`, `items?.push(2)`, `items["push"](3)`, `(items.push)(4)`, `m?.set`, `m["delete"]`, only the dot and optional-dot forms were reported.
- 2026-09-28: Added `src/val/calls.rs` and `val_probes_with_emit`; removed the method-call branch and the unused `last_prop_tok` from the token walk; the engine now reuses its emission.
- 2026-09-28: `ttc --check-types` on `p4` reports all ten mutator calls, including both string-literal keys and the parenthesized callee.
- 2026-09-28: Tests: `val_mutation_covers_every_spelling_of_a_member_call` (`tests/native/cases_02.rs`: string key, parenthesized callee, optional template key, `as` receiver, a call inside a `match` arm, a non-mutating `get`, and a non-literal key that is not reported) and a `val_probes` documentation example.
- 2026-09-28: Documented the accepted spellings in `docs/ai/tt.md` §val.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/val.rs`, `src/val/calls.rs`, `src/val/checker.rs`, `src/lib/mapped.rs`, `src/engine/projection.rs`, `tests/native/cases_02.rs`, `docs/ai/tt.md`, this record, and `docs/tasks/INDEX.md`. The typed `val` check now judges a member call by what it resolves to, whatever its spelling.
