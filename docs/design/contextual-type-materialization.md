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

Each round annotates previously unresolved declarations. An updated snapshot
then exposes those contexts to nested values. Rounds stop when no additional
facts are available; successful rounds strictly reduce the unresolved set.
Type errors are reported by the subsequent TypeScript check, not used to drive
this process. Context-only rounds do not compute diagnostics.

## Inferred joins

After contextual propagation reaches a fixed point, an uninitialized generated
slot without a contextual type uses its assignments
supply the incoming types. TypeScript computes and widens each right-hand side
in its branch scope. Its assignability relation removes subsumed constituents
before the remaining union is serialized at the declaration. For example,
`number[]` and `never[]` join as `number[]`, preserving empty-array expression
inference without evolving an implicit `any[]`. Unresolved, error, `any`, and
`unknown` inputs do not provide a definite annotation in that round.

## Project and output coordinates

Project snapshots include lowered tt files, TypeScript sources and unsaved host
overlays. Editor service projections use the same contextualized snapshot.
Standalone file compilation discovers its configuration and candidate tt files,
while the invoking working directory supplies the installed TypeScript client.
An unnamed buffer uses that working directory as its inferred project.

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
