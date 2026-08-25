# Why I built tt

[한국어](./why-tt.ko.md)

I've written TypeScript for years, and I still like it. So this isn't a
complaint about TypeScript. It's a note about one place where I kept getting
stuck, and what I ended up doing about it.

## The escape hatches are load-bearing

TypeScript's first job is to accept JavaScript that already exists. That isn't
a footnote in its design; it's the foundation. `any` and `as` are there because
a type system layered on top of a decade of running code needs somewhere to put
the parts it can't prove.

Most of us learn to avoid them. And the moment you decide to avoid them
seriously, you notice where the cost moves: you start writing types whose only
job is to make other types work. Conditional types, recursive tuple types,
overload tables. The type-level code grows until it feels like a second
language living inside the annotations.

The pipe operator is the example I keep coming back to. TypeScript has no `|>`,
so libraries approximate it with a variadic `pipe(...)`. Open any of them and
you'll find the same thing — a stack of hand-written overloads, one per arity,
usually stopping somewhere around ten or twenty:

```ts
export function pipe<A, B>(a: A, ab: (a: A) => B): B;
export function pipe<A, B, C>(a: A, ab: (a: A) => B, bc: (b: B) => C): C;
// ...and on, and on
```

It works right up until it doesn't. Go one step past the last overload and the
error you get is about the library's overload list, not about your program.
That's nobody's mistake. The authors did the best available thing. The limit is
structural: you're asking a type system to describe a syntax the language
doesn't have, and a description is only ever an approximation of the real
thing.

## The compiler knows, but doesn't say

The other thing that nagged at me was the silence.

```ts
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "rect"; width: number; height: number };

function area(shape: Shape) {
  switch (shape.kind) {
    case "circle":
      return Math.PI * shape.radius ** 2;
  }
}
```

TypeScript knows `shape.kind` is exactly `"circle" | "rect"`, and it knows
`"rect"` isn't handled here. The information is sitting right there in the
narrowing. But you won't hear about it unless you ask, and asking means writing
the ritual — the `default` branch that assigns the value to a `never`, or an
explicit return type plus the right compiler flags. Write the same `case` twice
and the compiler says nothing at all; catching that is left to a lint rule.

Given everything TypeScript has to be, none of this is unreasonable. It's just
that in most languages with tagged unions, this check is a feature of the
language, and here it's a pattern I have to remember to write — one more piece
of type machinery whose only purpose is to make the type system speak up.

## What I actually wanted

Products keep getting more demanding, and the code follows. When a function
grows past what fits in your head, the things that save you are pretty simple:
knowing exactly what goes in and what comes out, having as many mistakes as
possible caught before the code runs, and being able to say all that without
three screens of ceremony.

That's the whole motivation. I wanted those three properties inside
TypeScript — not by getting cleverer with types, but by moving the work one
step earlier, into a compile step.

## Why not one of the existing languages

Languages like ReScript already solved this, and solved it well. But they solve
it by compiling their own syntax down to JavaScript, and that's the hurdle. You
aren't adopting a feature; you're stepping outside the ecosystem, at least at
the boundary. Types for the packages you already use, editor tooling, the code
your team wrote last year, the next person you hire. For a codebase with a lot
of TypeScript in it, that trade is hard to justify no matter how good the
language is.

Meanwhile TypeScript stopped being an optional layer. Bun runs it natively;
Node has type stripping; type-first APIs are the norm. TypeScript is a superset
of JavaScript, and by now it's the part of the ecosystem you can't really opt
out of.

So I went the other way. tt is a superset of TypeScript that compiles *to*
TypeScript — not to JavaScript. TypeScript stays the target, the type checker,
and the thing everything else in your project already understands.

## The one rule everything else hangs off

**Every valid TypeScript file is a valid tt file.**

The compiler transforms only the syntax it owns and passes everything else
through byte for byte. What it emits is plain TypeScript — `kind`-tagged
unions, `switch` statements, ordinary functions. No runtime library, no type
tricks. If tt has something to say, it says it itself, with a file, line, and
column.

That rule buys two things. Learning tt is learning a handful of keywords, not a
language. And migration doesn't have to be a project: rename a file and it
compiles. Because `.tt` and `.ts` import each other in both directions, you can
also just... not migrate. Write the part that matters — a state machine, a
parser, a payment flow — in tt, and use it from the TypeScript you already
have.

Here's the `switch` from earlier:

```tt
enum Shape {
  Circle(radius: number),
  Rect(width: number, height: number),
}

const area = (shape: Shape): number =>
  match (shape) {
    Circle(radius) => Math.PI * radius ** 2,
  };
```

```text
error[match-not-exhaustive]: match on enum Shape is not exhaustive: missing "Rect"
 --> shape.tt:7:3
  |
7 |   match (shape) {
  |   ^^^^^^^^^^^^^
  |
  = help: add the missing arms
  |     Rect(width, height) => undefined,
  = help: or add a final `_` arm: `_ => undefined,`
```

Write `Circle` twice and you get `match: duplicate arm "Circle"`. Same
information the type system had all along — it just gets said out loud now.

## Errors you can read off a signature

The other half is failure. tt leans on returned errors rather than thrown ones,
and gives the ergonomics that usually make people give up on that style:

```tt
const loadProfile = (id: string): TResult<Profile, DbError | HttpError> =>
  result {
    const user <- findUser(id);
    const company <- fetchCompany(user.companyId);
    { user, company }
  };
```

Each `<-` unwraps a success and short-circuits a failure, and the error types
union themselves as you go. It reads like the happy path because it is the
happy path, but the failures are in the signature rather than in a comment or a
`catch` three frames up.

tt doesn't forbid throwing — it's still TypeScript, and `throw` is still there
when you want it. It just makes the other style comfortable enough to actually
use.

## The part I didn't expect to care about

A lot of code gets written with AI now, and what decides whether that goes well
is context. Not "enough" context — the *right* context, in the window the model
can actually see.

That turned out to be the same property I wanted for myself. A `match` is the
complete list of cases, right there. A signature returning
`TResult<Profile, DbError | HttpError>` is the complete list of failures, right
there. Neither answer lives in another file, or in a `throw` several frames
down, or in a convention someone explained in a meeting. A model reading forty
lines of tt can tell you what those forty lines do, and so can a reviewer at
11pm.

## Where this is

tt is early. The compiler is written in Rust, the language is seven constructs
and one binding modifier — `enum`, `match`, `try`, `let-else`, `if let`, `|>`
(with `flow`), `result` blocks, and `val` — and I wouldn't put it in production
yet.

But the shape of it is what I wanted: nothing to give up, a few things to
learn, and a compiler that tells you what your types already knew.
