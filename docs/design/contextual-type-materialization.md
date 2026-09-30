# Contextual types across statement lowering

Scoped tt values lower to statements without introducing a function boundary.
When storage separates an object literal or callback from its expected type,
the compiler asks TypeScript for that context and adds an ordinary annotation
to the generated declaration. No assertions or diagnostic suppression are used.

## Value identity and contextual facts

Codegen records declaration positions for both value slots and captured earlier
operands. The backend resolves each declaration's symbol and asks for contextual
types at references to that symbol. Shadowed identifiers are separate symbols.
An annotation is serialized in the declaration's scope. Conflicting contexts or
indefinite types do not produce a guessed annotation.

The declaration can enclose the scope the type was observed in: a class
declared in a match arm's block is out of scope at the storage, and an outer
declaration of the same name shadows it there (TASK-546). The declaration can
also sit inside a scope that shadows the declaration the type refers to: a
nested function's own type parameter `T`, or a local `interface Item`, while
the arm values have the outer `T` or `Item` (TASK-575). The node builder
writes both as the same name. The annotation's type node and the type are
therefore walked together, and every name the node references (each type
reference, type query and qualified name, resolved through its members) must
resolve, at the declaration, to the symbol of the part of the type it was
written for: the type parameter's own symbol, the alias or declaration a type
reference instantiates, the value a type query names. A part of the node that
uses a name and cannot be paired with a part of the type proves nothing, and
the node is not taken either. Otherwise the storage has no annotation and
TypeScript infers its type from its assignments.

An annotation is the whole type: the node builder is asked with
`NoTruncation` (TASK-553), since its default shortens a long type to
`... N more ...`, which is neither the type nor TypeScript. With it, the
node builder writes the cycle of a recursive anonymous type (the object
literal `{ k: 1, m() { return this; } }`) as `any` one level down, where its
default writes `...` (TASK-586). The walk that pairs the node's names with
the type's symbols therefore pairs each `any` keyword too, and takes the
node only when every `any` stands for the `any` type; otherwise the storage
is typed from its values, which TASK-570 types as at their source position.

Nor may an annotation name storage the lowering declared (TASK-552).
TypeScript names a class expression after the binding it is assigned to, so
the join of `class { q = 1 }` arms prints as `typeof $tt_v0`, the storage's
own type query (TS2502). Every round tells the backend the declarations of
all generated storage, the ones earlier rounds annotated included, and a
name that resolves to one of them rejects the annotation.

The same holds for a type TypeScript's node builder cannot write at the
declaration at all, such as the instance or constructor type of an anonymous
class (TASK-551): `typeToTypeNode` answers no node, and the storage is typed
from its assignments. One such type never stops the file's compilation.

Each round annotates previously unresolved declarations. An updated snapshot
then exposes those contexts to nested values. Rounds stop when no additional
facts are available; successful rounds strictly reduce the unresolved set.
Type errors are reported by the subsequent TypeScript check, not used to drive
this process. Context-only rounds do not compute diagnostics.

## Values with no contextual type

An assignment types its right operand by its target. A value written to
storage declared `let $tt_v0;` is contextually typed by the storage's
implicit `any`: a method of an object literal gets `this: any`, a function
literal's return expression is typed against `any`, and a reference to a type
parameter with a union constraint is read through its constraint. The source
position the storage stands for can have no contextual type at all
(`const b = match (n) { … }`), and there TypeScript types the value by itself
(TASK-570).

When contextual propagation reaches its first fixed point, every slot still
without a contextual type is detached. A value written to it whose type
TypeScript computes from its contextual type is first the `value` of an
object literal an arm-local `const` holds, and the storage reads it from
there:

```ts
const $tt_a0 = { value: ({ k: 1, m() { return this; } }) };
$tt_v0 = $tt_a0.value;
```

Every other value is written directly, as before: `$tt_v0 = 1;`,
`$tt_v0 = g();`.

An unannotated `const` initializer has no contextual type, and a property of
an object literal that has none has none either. The value is a property
rather than the initializer itself because TypeScript declares an unannotated
variable by rules of its own: an empty array literal initializer declares an
evolving array (TS7034 and TS7005 where it is read), and a `Symbol()`
initializer of a `const` declares a `unique symbol`. A property only widens a
fresh literal type, which storage with no contextual type widens anyway.

Which values TypeScript types from their context follows its checker, read
off the value's syntax tree (`typed_by_context` in `src/codegen/contextual.rs`).
The checker consults the contextual type in typing an object literal (its
properties, and `this` in its methods), an array literal (its elements), and
a function expression or arrow function (its parameters and return
expressions). `getContextualType` passes a position's contextual type on to
the operand of parentheses, `as const`, a non-null assertion and `await`, to
both branches of a conditional, to both operands of `||` and `??`, and to the
right operand of `&&` and of the comma operator. Any other operand has a
contextual type of its own (a call argument its parameter's, `as T` and
`satisfies T` their `T`) or none. A class expression's members are not
contextually typed. The remaining expressions type themselves under `any` as
they do with no contextual type: a literal is kept literal only by a literal
contextual type, and a generic call infers nothing from an `any` return
context. A Result block's success value (`{ kind: "Ok" as const, value: … }`)
is an object literal, so it is carried too.

Storage for the index of the arm a dispatch selected is never detached: it
holds no value of the source (`MarkKind::SelectorSlot`). The writes are the
assignment statements the emission's syntax tree has for the slot's
generated name, which is unique in its file; the lowering writes storage only
in its own blocks and `switch` cases, where a `const` can be declared.

The backend says whether an annotation is a contextual type or an inferred
join (`ContextualSlotType::inferred`). A detached slot that a later round
finds a contextual type for is written directly again, under that type. The
arm-local `const`s are listed to the backend as settled storage, so no
annotation names them. Without a TypeScript toolchain nothing is known about
contextual types, and the unrefined emission assigns every value directly.

## Inferred joins

After contextual propagation reaches a fixed point, an uninitialized generated
slot without a contextual type uses its assignments
supply the incoming types. TypeScript computes and widens each right-hand side
in its branch scope. Its assignability relation removes subsumed constituents
before the remaining union is serialized at the declaration. For example,
`number[]` and `never[]` join as `number[]`, preserving empty-array expression
inference without evolving an implicit `any[]`. Unresolved, error, `any`, and
`unknown` inputs do not provide a definite annotation in that round.

Nor does an input typed through storage no round has settled yet (TASK-584).
Such storage has no type of its own: without `noImplicitAny` it reads as
`any`, and so does an evolving variable read in a closure. In
`const g = match (flag) { true => [f()], false => [] }`, `f()` is then `any`
while `f`'s own join is inferred in the same round, and `g` would keep
`any[]` after `f` is annotated. A join whose annotation writes `any` and
one of whose incoming values reads unsettled storage (directly, or through
the initializer or body of an unannotated declaration in a lowered module,
or the statement that contextually types an unannotated parameter) waits
for a later round; the slot's own storage does not count. Joins therefore
settle in dependency order, and a join that never stops depending on
unsettled storage (a cycle) is left unannotated and typed from its
assignments. An `any` of the source, such as `JSON.parse`'s, is annotated
as soon as its inputs read only settled storage.

A slot therefore holds values of one type. A structured pipeline writes the
value piped into each step to a slot of its own and only its result to the
pipeline's value slot (TASK-505), so no annotation has to cover the values of
several steps.

## Project and output coordinates

Project snapshots include lowered tt files, TypeScript sources and unsaved host
overlays. Editor service projections use the same contextualized snapshot.
Standalone file compilation discovers its configuration and candidate tt files,
while the invoking working directory supplies the installed TypeScript client.
An unnamed buffer uses that working directory as its inferred project.

Toolchain absence is decided before collecting contextual project inputs.
When a checker is available, input enumeration/read failures are reported with
paths rather than producing types from a partial snapshot. Standalone input
failures and backend failures are separate error categories.

Compiler support packages are supplied in memory only when no package exists
in the project's ancestor `node_modules` chain. Analysis retains authored
support-module specifiers; output-adapter rewrites are applied only to the final
artifact, including import types synthesized in annotations. Output files do
not need to exist for their source to be analyzed.

Analysis retains relative tt import names and distinct virtual `.tt.ts` and
`.ttx.tsx` paths. This avoids collisions with authored `.ts` and `.tsx` files.
Value declaration identities transfer the resulting annotations to the requested
output mode. Synthesized import types use the existing import rewrite pipeline.

Annotation insertion shifts emitted coordinates for verbatim mappings,
construct anchors, scrutinee/payload probes and Result return ranges. Source
coordinates remain unchanged. Source-preservation checks remain enabled.

## Nested returns and cleanup

AST return records distinguish the complete argument from the value beneath
parentheses and TypeScript wrappers. A returned structured value emits its
region before completing the enclosing arm. Authored wrappers remain around the
resulting value. Finalizers and resource disposal therefore run before the
consumer is called, including nested synchronous and asynchronous resources.
