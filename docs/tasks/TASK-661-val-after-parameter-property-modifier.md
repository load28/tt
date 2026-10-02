# TASK-661: Read `val` after a parameter property's modifiers the same way in both checks

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-661`

## Purpose

`ttc --check-types` stopped with an internal compiler error (exit 101, `no
entry found for key` at `src/val/checker.rs:34`) on
`class K { constructor(public val p: number) {} }`, and on the same shape
with `private`, `protected`, `readonly`, or `override`, while plain `ttc`
accepted it and erased the modifier. Once the error was gone, the typed
check still disagreed with the untyped one: a write or a built-in mutator
call through such a parameter was never reported.

## Scope

- Included: the `val` analysis's parameter signature (`src/val.rs`,
  `src/val/checker.rs`), the binding question the engine asks TypeScript
  (`src/engine/projection.rs`, `src/typescript/backend.rs`,
  `src/typescript/native.rs`, `src/typescript/host.mjs`), `docs/ai/tt.md`,
  and a case under `tests/cases/compiler/`.
- Excluded: the property the parameter property also declares. `val`
  modifies a binding, and `this.p` is not that binding (`docs/ai/tt.md`,
  "val is per-BINDING"), so `this.p.push(1)` stays unreported.

## Sources

- `docs/ai/tt.md`, "val", SYNTAX rule: `val` sits "as the parameter binding
  it modifies ... at the start of an entry that TypeScript parses as a
  formal parameter"; `src/parser/vals.rs` (`is_param_modifier`) already
  lets TypeScript's parameter-property modifiers precede it
  (`constructor(private val x: X)`).
- TypeScript `src/compiler/binder.ts`, `bindParameter`: a parameter property
  is declared twice, as a `FunctionScopedVariable` in the constructor's
  locals and as a `Property` in the class members; `addDeclarationToSymbol`
  leaves the node's `symbol` on the later one, the property.
- TypeScript `src/compiler/checker.ts`,
  `getSymbolsOfParameterPropertyDeclaration`: "the parameter symbol" is
  looked up in `constructorDeclaration.locals`, the property symbol in the
  class members. The services use it where a parameter property's two
  meanings must both be found (find-all-references, rename).
- TypeScript `src/compiler/checker.ts`, `resolveNameHelper`: a function's
  locals answer a name looked up from its parameter list ("parameters are
  visible only inside function body, parameter list and return type").

## Decisions

### Decision 1: `public val p` is valid tt, and the modifier is the parameter's

- **Context**: The typed and untyped paths disagreed on whether the form
  exists; one of them had to be the model.
- **Alternatives considered**: (a) Reject `val` after a parameter-property
  modifier with a located tt error in both paths. That removes a form the
  guide and the parser already document and accept, and a constructor
  parameter is a formal parameter like any other. (b) Accept it and make
  the typed path agree.
- **Decision and rationale**: (b). The parser already records the modifier
  in its table; the checker looked it up at the entry's first token
  (`public`) instead of at the modifier. `ParamSig` now carries the
  modifier's own offset (`val_at`) rather than a boolean, so the binding
  and the table name the same byte.

### Decision 2: A binding's symbol is the one its references resolve to

- **Context**: With the lookup fixed, the typed pairing still failed: it
  asks TypeScript for the symbol at the binding's name, which for a
  parameter property is the class property, while every reference in the
  constructor resolves to the parameter.
- **Alternatives considered**: (a) Pair by name on the ttc side for
  parameter properties. That is the approximation the typed pairing exists
  to replace. (b) Ask for the symbol at a reference instead of the
  declaration. A binding may have no reference. (c) Mark the question as a
  binding question and let the TypeScript seam answer it with the
  parameter, as TypeScript's own `getSymbolsOfParameterPropertyDeclaration`
  does.
- **Decision and rationale**: (c). `SymbolQuery` gains `binding`, set for
  `val` bindings only. The host keeps the symbol at the position unless it
  is a property declared by a `Parameter`, and then resolves the name as a
  `FunctionScopedVariable` at that position, which reaches the
  constructor's locals. The knowledge stays inside `src/typescript/`.

## Work log

- 2026-09-30: Reproduced the ICE with the probe case
  (`ttc --check-types`, exit 101 at `src/val/checker.rs:34:41`).
- 2026-09-30: Changed `ParamSig` to carry `val_at`; the ICE was gone, but
  `items.push(1)` and `items[0] = 2` through `protected val items` were
  unreported by `--check-types` while `--check` reported the write.
- 2026-09-30: Added `SymbolQuery::binding` and `declaredBinding` in the
  host; both checks now report the same mutations.
- 2026-09-30: Added `tests/cases/compiler/valAfterParameterPropertyModifier.tt`
  and its baselines; updated the `val` syntax rule in `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The typed check paired no mutation with a parameter-property binding

- **Symptom**: After the ICE fix, `--check-types` reported nothing for
  `constructor(protected val items: number[]) { items.push(1); items[0] = 2; }`.
- **Cause**: The symbol at a parameter property's name is the property
  (binder), not the parameter the body's references resolve to.
- **Resolution**: Decision 2.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/valAfterParameterPropertyModifier.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: `thread '<unnamed>' panicked at
  src/val/checker.rs:34:41: no entry found for key`, and the case failed.
  With only the checker half of the fix, the `--check-types` section of
  `.errors.txt` lacked the three typed `val-mutation` errors.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the case runner, `cargo test --lib val`, and
  `cargo test --test native val` here; the full gate over TASK-661 to
  TASK-666 is recorded in TASK-666
- [x] Baseline changes reviewed and committed with the change

## Result

`val` after `public`/`private`/`protected`/`readonly`/`override` is one
modifier in both checks, and the typed check pairs the parameter's
references with it. Changed: `src/val.rs`, `src/val/checker.rs`,
`src/engine/projection.rs`, `src/typescript/backend.rs`,
`src/typescript/native.rs`, `src/typescript/host.mjs`, `docs/ai/tt.md`,
`tests/cases/compiler/valAfterParameterPropertyModifier.tt`, and its
baselines.
