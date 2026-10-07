# TASK-781: Fix the sixth audit's compiler findings

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-781: Fix the sixth audit's compiler findings`

## Purpose

The sixth audit of the compiler (a read-only agent against a release build
of `d8da7233`) found eight new defects: functions named by generated
storage, comments and directives lost from rewritten operands, a tt value
left as a discarded comma operand, a JSX tag evaluated after its
attributes, a storage annotation that loses TypeScript's object-literal
normalization, cubic time on left-deep chains, `super` method calls behind
a type wrapper rejected, and a build that skips storage refinement for a
`.tt` outside the configuration's globs. Fix each in the layer that owns
it.

## Scope

- Included: the eight findings (K1–K8 of this round), and K9, a variant
  field the editor agent of the round noticed.
- Excluded: the command-line and editor findings of the same round.

## Decisions

### Decision K3: A tt value that is a discarded comma operand leaves nothing behind

- **Context**: `match (o) {...}, log.push("second");` and `(try r(), try
  r())` wrote the value's slot as the comma's left operand (`$tt_v0,
  log.push(...)`), which TypeScript rejects (TS2695). TASK-769 decision 33
  removed a discarded operand that contains a value, not one that is a
  value.
- **Decision and rationale**: A planned value whose innermost step is a
  sequence element followed by its comma runs in the prelude for its
  effects, as before; its place and its comma are removed, as a discarded
  operand's are, and the comma is claimed by the plan.

### Decision K1: A function moved into generated storage keeps the name its position gives

- **Context**: Decision 20 of TASK-773 kept arm values and `Ok` wrappers
  from naming a function. Values evaluated before a tt value were still
  captured as `const $tt_v2 = (() => 1);`, and a conditional branch or a
  logical operand was written as `$tt_v3 = () => 1` and then wrapped for
  its contextual type as `({ value: () => 1 }).value`: ECMA-262
  NamedEvaluation named them `$tt_v2` and `value` where the source names
  nothing, and a class in `{ k: class {}, v: match ... }` was named
  `$tt_v8` instead of `k`.
- **Alternatives considered**: (a) Leave every function definition in
  place: an arrow or function expression is already inert, but a class
  expression may run code (static blocks, computed keys, `extends`), so it
  must still be evaluated in order. (b) Write the stored value so that
  NamedEvaluation gives it the name its own position gives.
- **Decision and rationale**: (b). The program syntax also records, for
  each anonymous function definition that is the value of a property with
  an identifier, string, or numeric key, that key; a capture, a
  conditional branch, and a logical operand write such a value as
  `({ "k": value })["k"]` (NamedEvaluation through the same key) and any
  other anonymous function definition as `(void 0, value)`, which names
  nothing and keeps the contextual type.

### Decision K4: A JSX element's component reference is evaluated before its attributes

- **Context**: `<lib.inner.Comp y={t()} x={match ...} />` captured the
  attributes before the match and left the tag in place, so the getter on
  the tag path ran after them, and `<Comp x={match (o) { A => (Comp =
  New, 1), ... }} />` created `New` where JavaScript creates the component
  `Comp` held before the attributes ran (TypeScript's `react` emit is
  `React.createElement(Comp, { x: ... })`, whose first argument is
  evaluated first).
- **Decision and rationale**: The program syntax lists a non-intrinsic tag
  as the element's first evaluation input (`EvaluationInputMode::JsxTagName`),
  so it is captured like any operand before a tt value; the plan renames
  the closing tag to the same capture. An intrinsic tag (`div`) is a
  string and evaluates nothing. In the typed path the capture's
  annotation is not taken from the tag: TypeScript's `getContextualType`
  answers any child of an opening element, the tag name included, with
  the element's attributes type, which is the props of the component, not
  the type of the tag (`src/typescript/host.mjs` skips such a use).

### Decision K7: A `super` method behind a type wrapper or an optional call keeps `this`

- **Context**: `super.m!(match ...)`, `(super.m as F)(match ...)`,
  `super[key()]!(match ...)` and `super.m?.(match ...)` were rejected as a
  reference position that loses `this`, while `super.m(match ...)` and
  `this.m!(match ...)` were lowered.
- **Decision and rationale**: TypeScript erases `!`, `as`, `<T>`,
  `satisfies` and instantiation expressions in its emit, so `super.m!(v)`
  is the call `super.m(v)`; the callee is read again at the call through
  those wrappers as it is through parentheses. An optional call has to test
  the method before the arguments, so its callee is captured and called
  through `.call(receiver, ...)`, as `o.m?.(v)` is; the receiver of a
  `super` reference is `this` (ECMA-262 `MakeSuperPropertyReference` uses
  the environment's `this` value), recorded as a receiver of its own kind
  (`PlannedReceiver::ThisOfSuper`) and written `this`.

### Decision K5: A join of object literals has the type TypeScript widens their union to

- **Context**: `const x = match (o) { A(n) => ({ a: n }), B => ({ b: "x" }) }`
  annotated its storage `{ a: number } | { b: string }`, and `x.a` was
  TS2339, while `c ? { a: n } : { b: "x" }` and the hand-written
  assignments pass. Two causes: TASK-570 carried every object literal
  through `const $tt_a0 = { value: ... }`, and widening the holder's
  property (`getWidenedTypeOfObjectLiteral`) makes a type that is no longer
  an object literal type, so a union of them loses the rule that reads a
  property missing from an object-literal constituent as `undefined`
  (`createUnionOrIntersectionProperty`); and the join was widened one
  assigned type at a time, which loses the sibling context TypeScript
  widens a union of object literals with (`getWidenedTypeWithContext`
  adds `b?: undefined`).
- **Alternatives considered**: (a) Emulate the sibling widening on the
  printed annotation: a second implementation of the checker's rule,
  including its nested contexts. (b) Ask the checker for the widened type
  of the union the storage holds where all its writes reach it.
- **Decision and rationale**: (b), with the object literal written as it
  is. An object literal is typed by its contextual type only through the
  properties whose values are (functions, arrays, nested such literals)
  and `this` in its methods and accessors; any other object literal is
  written to the storage directly, so it stays an object literal type
  (TASK-570 decision 3 is narrowed accordingly). When two or more of the
  values written are object literal types, the join is
  `getWidenedType` of the storage's type at a read whose constituents are
  exactly the written types, which is the type TypeScript gives a
  declaration initialized with their union, nested literals included.

### Decision K8: A build refines a named source the configuration leaves out

- **Context**: With `"include": ["*.ts"]` (an in-place build), `ttc c.tt`
  wrote `const $tt_v2 = (x => x.length);` with no annotation and `tsc -p .`
  then failed with TS7006, while `ttc --check-types c.tt` passed: the check
  makes a file named on the command line a root by request
  (`Project::roots`), and the build's refinement (`contextual::standalone`)
  asked with no roots, so the checker had no program position for the
  file's storage.
- **Decision and rationale**: The build's refinement makes the named file's
  module a root, as the check does, so both type the file in the same
  program.

### Decision K6: A capture asks about claimed frames only for the parts it writes

- **Context**: `"" + (try r()) + (try r()) + ...` and `q.m(match ...).m(match
  ...)...` took 10 s at 1000 terms and 80 s at 2000. Each capture of a
  left-deep operand collected every value, statement and replacement
  inside it and asked, for each, whether a claimed frame covers it; that
  question listed every replacement covering the part, which in a
  left-deep chain is every enclosing capture. Every capture also parsed
  its own text to decide whether it is an entity name (`typeof` query),
  and that text is the whole prefix.
- **Decision and rationale**: The emitter writes parts in source order and
  skips any part inside one it has written, so a part's admission (not
  inside a claimed frame, its slot captured) is now decided only for a
  part it would write; the parts it skips are never asked about. Claimed
  frames are indexed by start with their ends sorted, so the question is a
  range and a binary search over the frames between the part and its
  bound. Entity names are recorded once by the program syntax, bottom-up
  over the parsed module, and a capture looks its span up. At 1000 terms
  the two shapes now take 0.27 s and 0.48 s, and 0.94 s at 2000; what
  remains grows quadratically with a small constant (each capture still
  lists the value slots and the replacements covering it), and the number
  of claimed-frame questions and checks is linear
  (`compiling_does_linear_work_in_a_left_deep_chain_of_tt_values`).

### Decision K9: A variant constructor names a strict-mode-restricted field's parameter itself

- **Context**: Found by the editor agent of the same round. A field named
  `arguments`, `eval`, `implements`, `interface`, `package`, `private`,
  `protected` or `public` made the variant itself an error (TS1100,
  TS1212): the constructor took each field as a parameter of the same
  name, and strict-mode code, which every module is, cannot bind those
  names (ECMA-262 §13.1.1 and §12.7.2). As a property of the variant's
  object type each is an ordinary name.
- **Decision and rationale**: The constructor gives such a field's
  parameter a generated name (`$tt_arguments`) and writes the property
  explicitly (`arguments: $tt_arguments`); the type, the patterns and
  every other field are unchanged. Reserved words stay refused as field
  names, as before.

### Decision K2: Comments around a moved or rebuilt operand need a structural change (open)

- **Context**: A comment or a TypeScript directive written next to an
  operand that a lowering captures, or inside a call, conditional, logical
  operation or pipeline that a lowering rebuilds, is dropped
  (`g(\n// @ts-expect-error\n"x",\nmatch ...)` reports TS2345). The
  rebuilt forms are assembled from slot names (`invoke` strings, branch
  assignments, `$tt_ap(...)`), not copied from the source, so the text
  between the pieces is not carried, and a directive above a captured
  operand would govern the capture's line while the error is reported at
  the rebuilt call.
- **Alternatives considered**: (a) Patch each rebuilt form to copy its
  comments. (b) Build every rebuilt form from the authored text between
  its pieces, as the pass-through emission does, so comments and line
  breaks stay where they were written and a directive keeps governing the
  line its operand's error is reported on.
- **Decision and rationale**: (b) is the structural change the finding
  needs, and it changes every rebuilt form's output; it is put to the user
  rather than made form by form in this task.

## Work log

- 2026-10-07: Ran the sixth audit's compiler agent; reproduced K3 and
  fixed it in `src/codegen/core/planning.rs`, with
  `aTtValueAsADiscardedCommaOperandLeavesNothingBehind`. A first version
  replaced the value's whole span, which dropped a recovered match's arm
  text in the typed projection of a malformed file (an internal error
  under `--check-types`); the value's slot read is now emptied instead,
  pinned by `aMalformedMatchAsACommaOperandIsReportedNotLowered`.
- 2026-10-07: Fixed K1 (`src/program_syntax/projection.rs`,
  `src/evaluation_ir{.rs,/evaluation.rs}`, `src/codegen/core/{mod,planning}.rs`,
  `src/codegen/core/emitter/{mod,pattern,host}.rs`), with
  `aFunctionMovedBeforeATtValueKeepsTheNameItsPositionGives`.
- 2026-10-07: Fixed K4 (`src/program_syntax{.rs,/collector.rs,/protocol.rs}`,
  `src/codegen/core/planning{.rs,/rewrites.rs}`, `src/typescript/host.mjs`),
  with `aJsxTagIsReadBeforeTheAttributesALoweredValueFollows`. Its first
  typed run reported TS2322 and TS2604 on the capture: the annotation came
  from the tag's contextual type, which TypeScript answers with the props
  type.
- 2026-10-07: Fixed K7 (`src/program_syntax{.rs,/protocol.rs,/visit.rs,/collector.rs}`,
  `src/evaluation_ir{.rs,/planning.rs}`, `src/codegen/core/{planning.rs,emitter/host.rs}`),
  with `aSuperMethodBehindATypeWrapperOrOptionalCallKeepsThis`.
- 2026-10-07: Fixed K5 (`src/codegen/contextual.rs`, `src/typescript/host.mjs`),
  with `aJoinOfObjectLiteralsKeepsTypeScriptsNormalizedUnion`. The case
  baselines that carried a plain object literal (`{ kind: "Ok" as const,
  value }` among them) now write it directly; their join annotations print
  as one union.
- 2026-10-07: Fixed K8 (`src/typescript/contextual.rs`), with
  `a_build_refines_a_named_tt_source_the_configuration_does_not_include`
  in `tests/native/cases_10.rs`.
- 2026-10-07: Fixed K6 (`src/codegen/core/emitter/{host,mod}.rs`,
  `src/codegen/core/{mod,planning}.rs`, `src/program_syntax{.rs,/projection.rs}`,
  `src/evaluation_ir{.rs,/evaluation.rs}`), measured with callgrind on the
  audit's generated inputs, with the scaling test in
  `src/lib/scaling_tests.rs`.
- 2026-10-07: Fixed K9 (`src/codegen/core/emitter/helpers.rs`), with
  `aVariantFieldNamedArgumentsOrEvalIsAConstructorParameterStrictModeAllows`.
- 2026-10-07: K2 (comments around operands that a rewrite moves or
  rebuilds) needs every rebuilt form to carry its authored gaps; it is
  put to the user as a structural proposal rather than patched form by
  form.

## Issues and resolutions

### Issue 1: A capture of the piped value was annotated `typeof ` with no name

- **Symptom**: the full suite failed `aMemberStepCallsItsMethodOnThePipedValueAfterTheArgument`
  and four related tests with `verify-failed` ("Expected ident"): the
  output held `const $tt_v3: typeof  = ($tt_v4);`.
- **Cause**: K6 recorded entity names by their mapped source span, and a
  projection placeholder (the piped value) maps to an empty span, so the
  set held a span whose text is no name.
- **Resolution**: the set is a pre-filter; a capture in it is still checked
  as an entity name on its own text (`source_entity_name`), which is short
  for any name, so the chain stays linear.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aTtValueAsADiscardedCommaOperandLeavesNothingBehind.tt` (K3)
- **Observed failure**: an `.errors.txt` with TS2695, and different `.ts`
  and `.stdout`.
- **Path**: `tests/cases/compiler/aFunctionMovedBeforeATtValueKeepsTheNameItsPositionGives.tt` (K1, wrapper disabled)
- **Observed failure**: `.stdout` was
  `["$tt_v2:1","$tt_v5:1","$tt_v7","a-b","value","value"]` instead of
  `[":1",":1","k","a-b","",""]`.
- **Path**: `tests/cases/compiler/aJsxTagIsReadBeforeTheAttributesALoweredValueFollows.tt` (K4)
- **Observed failure**: on `d8da7233`, `.stdout` was `y, arm, get inner,
  create Old, y, arm, get inner, child, create Old, create New` instead of
  `get inner, y, arm, create Old, get inner, y, arm, child, create Old,
  create Old`.
- **Path**: `tests/cases/compiler/aSuperMethodBehindATypeWrapperOrOptionalCallKeepsThis.tt` (K7)
- **Observed failure**: on `d8da7233`, four `match-placement` errors ("cannot
  be lowered from this reference position while preserving its receiver
  and `this`") and no `.stdout`.
- **Path**: `tests/cases/compiler/aJoinOfObjectLiteralsKeepsTypeScriptsNormalizedUnion.tt` (K5)
- **Observed failure**: on `d8da7233`, TS2339 for `a`, `b`, `p` and `q`
  ("Property 'a' does not exist on type '{ a: number; } | { b: string; }'")
  and no `.stdout`.
- **Path**: `tests/native/cases_10.rs::a_build_refines_a_named_tt_source_the_configuration_does_not_include` (K8, roots emptied)
- **Observed failure**: the emitted `c.ts` held `const $tt_v2 = ((void 0,
  x => x.length));` with no annotation.

- **Path**: `src/lib/scaling_tests.rs::compiling_does_linear_work_in_a_left_deep_chain_of_tt_values` (K6)
- **Observed failure**: on `d8da7233` with only the two work counters added,
  "claimed frame queries: 19602 for n values but 79202 for 2n".

- **Path**: `tests/cases/compiler/aVariantFieldNamedArgumentsOrEvalIsAConstructorParameterStrictModeAllows.tt` (K9)
- **Observed failure**: on `d8da7233`, TS1100 for `arguments` and `eval` and
  TS1212 for `public`, and no `.stdout`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Complete for K1 and K3–K9, each with a regression test; the full gate passes. K2 is a structural change put to the user (decision K2) and stays open until they decide.
