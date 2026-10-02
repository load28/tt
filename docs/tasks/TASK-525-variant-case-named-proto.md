# TASK-525: Define a variant case named `__proto__` as an own constructor property

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

`variant V { __proto__(x: number), Other }` emitted
`const V = { __proto__: (x: number): V => ..., Other: ... }`. In an object
literal, a non-computed, non-shorthand `__proto__: value` sets the object's
`[[Prototype]]` instead of defining a property (ECMA-262 B.3.1, `__proto__`
Property Names in Object Initializers, and PropertyDefinitionEvaluation's
`isProtoSetter`). The constructor object got the function as its prototype,
`Object.keys(V)` lost the case, and a unit case `__proto__` made its value
object the prototype.

## Scope

- Included: every object literal the compiler writes keyed by a user tag or
  field name.
- Excluded: object literals the user writes, which pass through.

## Decisions

### Decision 1: Write the constructor's `__proto__` key as a computed key

- **Context**: The constructor object is the only emitted object literal
  keyed by a user-chosen name with `name: value`. A payload object writes
  each field as shorthand (`({ kind: "A", __proto__ })`, and
  `{ __proto__ }` inside an optional field's spread), and B.3.1 does not
  apply to shorthand, so it defines an own property. Match and let-else
  bindings destructure, and B.3.1 applies only to an object literal, not to
  an assignment or binding pattern. The union type and the ambient
  constructor declaration are type literals, where `__proto__` is an
  ordinary property name. Tags are ASCII identifiers, so `__proto__` is the
  only spelling that triggers the rule.
- **Alternatives considered**: A string-literal key `"__proto__": value`
  is the same PropertyDefinition and still sets the prototype. A computed
  key `["__proto__"]: value` is excluded by `IsComputedPropertyKey`, and
  `Object.defineProperty` would move the constructor out of the literal
  TypeScript types.
- **Decision and rationale**: The computed key. TypeScript gives an object
  literal with a string-literal computed key a property of that name, so
  `V.__proto__(1)` type-checks as before and no type-side change is needed.
  The declared-name mapping covers the whole `["__proto__"]`, which is the
  declaration name TypeScript's language service returns for a computed
  property (confirmed with the project's TypeScript by
  `a_case_named_like_the_prototype_setter_navigates_to_its_declaration`), so
  navigation still lands on the case name in the source.

## Work log

- 2026-09-29: Reproduced with Node: `Object.keys(V)` was `["Other"]` and
  `Object.getPrototypeOf(V)` was the constructor function. A payload field
  named `__proto__` was already an own property (shorthand).
- 2026-09-29: `emit_adt` (`src/codegen/core/emitter/helpers.rs`) writes the
  key through `PROTOTYPE_SETTER_NAME`. Added
  `a_case_named_like_the_prototype_setter_is_an_own_constructor_property`
  (`tests/compile/cases_14.rs`),
  `runtime_a_prototype_setter_name_is_an_own_property`
  (`tests/integration/cases_05.rs`), and the navigation test
  (`tests/native/cases_03.rs`). The first draft mapped only the quoted name
  inside the brackets, and the navigation test returned nothing.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/codegen/core/emitter/helpers.rs`, `tests/compile/cases_14.rs`,
`tests/integration/cases_05.rs`, and `tests/native/cases_03.rs`. A case
named `__proto__` is an own property of its constructor object.
