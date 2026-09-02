# Mixed-source composition matrix

This document defines the finite structural matrix used to validate projects
that combine `.tt`, `.ttx`, `.ts`, and `.tsx`. It does not attempt to enumerate
arbitrarily deep source strings. The compiler must instead cover every class it
uses to make a different parsing, ownership, evaluation, or module decision.

## Source-kind matrix

The four source kinds form a directed import graph. The mixed-source fixture
contains every edge, including same-kind imports, for sixteen edges total:

| Importer | Imported source kinds |
| --- | --- |
| `.ts` | `.ts`, `.tsx`, `.tt`, `.ttx` |
| `.tsx` | `.ts`, `.tsx`, `.tt`, `.ttx` |
| `.tt` | `.ts`, `.tsx`, `.tt`, `.ttx` |
| `.ttx` | `.ts`, `.tsx`, `.tt`, `.ttx` |

`.ts` and `.tsx` remain byte-preserved TypeScript inputs except for the
documented relative `.tt`/`.ttx` specifier rewrite. `.tt` emits `.ts`; `.ttx`
emits `.tsx`. The matrix is exercised through untyped compilation, typed
project checking, declaration sidecars, and TypeScript checking of the emitted
tree.

## tt surface matrix

The fixture and compiler tests cover every compiler-owned surface:

- `variant`, including unit, payload, generic, exported, and nested payload
  types;
- tag, nested, literal, tuple, and `is` `match` families, with guards,
  or-patterns, aliases, expression arms, and block arms;
- statement and expression `try`, let-else, chained `if let`, value and `flow`
  pipelines, statement-bodied `result`, and every supported `val` binding
  position;
- tt expressions in `.ttx` JSX children, attributes, spreads, callbacks, and
  ordinary TypeScript expressions around JSX.

Directed nesting is measured over value-producing tt regions (`match`, tuple
match, pipeline/flow, `result`, and expression `try`). Statement-only surfaces
are measured inside ordinary functions, match block arms, result bodies, and
functions nested in JSX expression containers. Invalid crossings must produce
their named placement diagnostic and must never reach output verification.

## TypeScript host matrix

The authoritative host classes are the enums in `program_syntax` rather than a
hand-written list of JavaScript spellings. A self-auditing unit matrix covers:

- every eager, conditional, reference, suspension, and loop-test evaluation
  operation;
- module, function, constructor, generator, parameter, class-field, and static
  block owners;
- return, concise-arrow return, declaration initialization, `for`
  initialization, discard, and composed continuations;
- same, repeated, and unmodelled-conditional owner reachability.

Adding an enum variant makes the matrix classifier fail to compile until the
new structural class has a representative. Valid TypeScript and TSX outside tt
constructs remain covered independently by the byte-for-byte differential
corpus in `tests/corpus.rs`.

The executable cross-product uses forty canonical host surfaces: the thirty-
nine enum-derived protocol classes plus the distinct JSX child replacement
role beside the JSX attribute role. Each of the forty hosts crosses all forty-
two standalone and directed-nesting value cases, for 1,680 canonical cells.
Twenty-eight self-delimiting value cases also cross nine unparenthesized host
spellings. Those 252 syntax-boundary cells prove that grouping is not hiding a
claim or precedence defect. Parentheses remain in a canonical cell only when
they are required to keep a low-precedence value, such as a pipeline, in the
host operand represented by that cell. The complete matrix contains 1,932
cells.

## Green condition

Every accepted matrix cell must emit parseable TypeScript or TSX and pass the
typed project path where types are relevant. Every rejected cell must report a
specific tt diagnostic code. No cell may panic, silently pass an owned tt token
through, report `verify-failed`, or depend on parentheses that do not change the
host grammar's structural class.
