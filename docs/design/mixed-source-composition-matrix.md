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
tree. A separate runtime fixture calls every directed edge, bundles the emitted
tree, and executes it. Its oracle fixes the sixteen returned values, their
left-to-right evaluation order, shared module identity, and a `match` lowered
inside `.ttx` JSX.

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

Strict contextual typing is additionally checked by 136 composed-match cells
across TypeScript and TSX, each paired with a TypeScript conditional-expression
oracle. Binding-free switch matches with expression arms can select an arm in
the scheduled prelude and evaluate its value inside the authored contextual
host. This preserves callback inference and literal types without type
assertions or callback boundaries. This prelude target form requires one TT
value in the host owner. Total conditional dispatch with a terminal unconditional
wildcard retains guards beside arm values, preserving their narrowing.
Blocks containing only one value-returning statement (plus comments/trivia)
use the same contextual value path, based on a host-AST statement-list proof
linked to the Core body identity.
Multi-value owners whose matches all have expression-compatible arms retain
native expression evaluation with inline subject captures and conditional
dispatch. An additional 112 sibling cells cover family pairs in calls, arrays,
conditional arguments, and TSX props. Unmatched values call a hygienic `never`
throw helper; no authored code is moved into a callback.

Calls whose final non-spread argument is exactly the match can use a scoped
invocation continuation: the callee and every earlier argument are captured
before the match, and each arm supplies its value directly while payload/local
bindings remain in scope. A match in a non-final argument position keeps its
join slot, because the call would otherwise run the later arguments' subjects
too early. The
completion covers discarded and consumed results, identifier and member
callees (through the existing receiver-preserving capture), explicit type
arguments (instantiating the captured callee once), and single-argument
optional calls without type arguments. A consumed completion assigns each
arm's call result to the value's join slot, which stands at the authored call
position. A host-AST proof permits expression arms and never-completing block
arms whose statement tree is free of cleanup boundaries — no `try`, `with`,
or `using` outside nested functions — so conditional and multiple returns,
loops, and `switch` statements in an arm each carry the call at their own
authored exit. Cleanup-bearing arms keep the consumer outside the arm: moving
the call would place it inside the arm's handler and ahead of its finalizers
or disposal. An argument wider than the match — a cast, an operator, a
containing literal — keeps its authored call frame.

A host expression the schedule must evaluate before a tt value stays at its
authored position whenever evaluating it there is unobservable: a literal, a
function expression, and an object or array literal built from those. Such an
expression therefore keeps the contextual type its position supplies without
an annotation. The one exception is a completed call, which is re-emitted
inside the dispatch: it binds each such input to a reserved generated name
once, from mapped source, rather than repeating it in every arm. Everything
else is captured, and a tt value in a position no completion covers still
crosses the boundary through an unannotated join slot — the residue tracked
in [TASK-332](../tasks/TASK-332-wrapped-argument-contextual-values.md).

Every accepted matrix cell must emit parseable TypeScript or TSX and pass the
typed project path where types are relevant. Every rejected cell must report a
specific tt diagnostic code. No cell may panic, silently pass an owned tt token
through, report `verify-failed`, or depend on parentheses that do not change the
host grammar's structural class.
