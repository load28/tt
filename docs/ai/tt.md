# tt — AI context

tt = TypeScript/TSX + 7 constructs + 1 binding modifier; `ttc` compiles `.tt` → `.ts`, `.ttx` → `.tsx`. Write normal TS/TSX everywhere; tt syntax only for: variant (tagged union), match, try, let-else, if let, `|>` (+ `flow` composition), `result` block, `val` (mutation-free binding).

CONTRACTS:
- Every valid TS file is a valid `.tt` file; every valid TSX file is a valid `.ttx` file. ttc transforms only text parsing COMPLETELY as a tt construct; all else passes through byte-for-byte. JSX tags/text stay opaque; tt constructs work in `{...}` expression containers. JSX transform/runtime selection remains the project's TypeScript `jsx` option.
- Output is plain TS (`kind`-tagged unions, switch/if chains), with no type tricks. Pipelines import one compiler-owned `@tt/runtime` module, written at the top of the output (after a shebang or a directive prologue such as `"use client"`); project builds materialize it once as `tt/runtime.ts`, and bundler adapters serve it virtually. Generated glue is laid out from the line the construct sits on (block structure indents like hand-written TS); your own code is copied byte-for-byte and never reformatted — a block arm's body keeps the line break and indentation you wrote after its `{`; parentheses around a lowered value are kept only where they group it (`$tt_v0 = w * h;`, but `$tt_v0 = (v, v + 1);` and `(await p).then(g)`). tt-level errors are rendered, not printed on one line: `error[<code>]: <msg>`, then ` --> file:line:col`, then the source line with a caret run under the construct, then one `= help:` line per suggestion. The position is the construct's start and the underline covers it as written (`try <expr>`, `match (scrutinee)`). The message says WHAT is wrong and nothing else — the rule's identity is the code, the fix is a suggestion, and a TypeScript diagnostic's `ts<code>` is likewise the code rather than a prefix in the sentence. `ttc explain <code>` prints the rule at length (`ttc explain` lists them). A suggestion that names a replacement carries it as an edit (span + text), so the CLI shows it after `help:` and an editor offers it as a quick fix. One run reports **every** diagnostic in file/source order (each tt rule has a stable code, e.g. `match-not-exhaustive`); identical position/range/message duplicates are merged, and a recoverable tt error doesn't hide the file's type diagnostics on the typed path. Type errors in pass-through code: tsc's job. A failure that is ttc's own fault is reported as one: `error: internal compiler error: ...` names what it was compiling, says the code it was given is not at fault, and points at the issue tracker; the CLI exits 101 and `--server` answers that one request with an error and keeps the session.
- Assignability errors are rendered from checker facts, not by rewriting TypeScript prose: ``type mismatch: expected `InputError`, found `RangeError` `` plus ``required type: `TResult<number, InputError>` `` when the surrounding obligation helps. The same syntax-neutral rule covers return values, annotations, call arguments, and lowered TT constructs. A checker span maps exactly only when one verbatim source mapping owns the whole span; spans crossing generated glue use the lowering anchor's primary source range (`match (subject)`, `binding <- expression`). Anchors separately retain the complete syntax owner, so one proven TT/type cause suppresses only checker consequences owned by the same lowering. The editor computes its provisional checker path and authoritative compiler path in one document generation, applies the same TT-cause ownership, then publishes the complete diagnostic set once with that document version; an untouched slower-layer error never disappears between partial updates. TT variant cases use declaration names when uniquely identifiable; otherwise checker structural names remain. CLI, server, and the editor consume this same range, message, and `ts<code>` identity. Non-assignability diagnostics still use the construct-specific translation table or raw checker fallback.
- Semantic pattern errors carry the parser AST's complete primary span, not a start offset whose width the editor guesses. A mixed-pattern error covers the first pattern of the other kind; tuple arity covers the complete parenthesized tuple pattern. `Arm` and `TupleArm` keep `pattern_span` as their stage contract, so future pattern diagnostics inherit the same source-range path.
- TRAP: once `match`/`variant` is distinguishable from valid TS, malformed syntax is a located ``tt `<construct>` could not be parsed`` error. `if let`, `|>`, and claimed `result` blocks likewise error at their construct. Unclaimed lookalikes remain byte-exact TypeScript passthrough.
- A file that CONTAINS a tt construct must be TypeScript around it: lowering models the file's TypeScript, so TS that does not parse inside a claimed construct (a match arm body, a `result` block) is a located ``the TypeScript here does not parse`` error (`source-not-typescript`) at the failing byte, and nothing is emitted (`--no-verify` does not skip it — it is not the output self-check). A file with NO tt construct keeps reporting through that self-check (``generated TypeScript failed to parse``).
- Identifiers inside tt constructs: ASCII `[A-Za-z_$][A-Za-z0-9_$]*` only. TS reserved words (new, default, if, in, of, static, class, ...) can't be tags/fields/bindings — construct silently passes through.

## variant

```tt
export variant Shape { Circle(radius: number), Rect(width: number, height: number), Point }
variant Status { Active, Inactive }
variant Tree<T> { Leaf(value: T), Node(left: Tree<T>, right: Tree<T>) }
```
→ emits type alias `Shape` = union of `{ kind: "Tag"; ...fields }` + constructor object `Shape` (both exported if `export`).
- Use: `Shape.Circle(1)`; unit case is a VALUE not fn: `Shape.Point`. Empty-paren cases remain zero-argument constructor functions when explicitly written.
- Discriminant always `kind`. Plain `{ kind: "Circle", radius: 1 }` is assignable; match works on ANY `kind`-string-discriminated union.
- Every `variant` is a tt tagged union, including unit-only declarations. Every `enum` (`enum Color { Red }`, `const enum`, `declare enum`) belongs to TypeScript and passes through.
- Duplicate case tag = error.

## match

```tt
const area = match (shape) {
  Circle(radius) => Math.PI * radius ** 2,
  Rect(width: w, height) => w * height,   // bind by FIELD NAME; alias via `field: alias`
  Point => 0,
};
```
- Expression: use after `=`, in `return`, in `${...}`. The compiler uses owner-scoped slots and `switch`/`if`. A value under `&&`/`||`/`??`/`? :`/`f?.()` lowers the WHOLE operation as one region (condition or callee evaluated once, every path assigning the result slot), so short-circuit, evaluation counts and the operation's type are unchanged. Where statements cannot reach the value — parameter defaults, class fields, a loop header (`while (f(match ...))`), a `switch` case test, a destructuring default, or a conditional operation that cannot be owned whole — one hygiene-safe named expression-boundary helper runs it in place. Scrutinee parens mandatory, non-empty.
- Bindings by field name, NEVER position; subset ok, any order.
- Arm body: expr, or block `{ ... return v; }` (no return → undefined; a block whose every path returns or throws gets no fall-through in the output). Object literal body needs parens: `Tag => ({a: 1})`.
- `_` arm must be LAST.
- Literal patterns: string/number/boolean literals match the scrutinee VALUE (`===`), e.g. `match (dir) { "north" => "N", _ => "?" }`. NEVER mix tag and literal patterns in one match (compile error); `_` works in both. See "literal match" below.
- or-pattern: `A | B => body` (never `||`); all alternatives must bind same (field,name) set.
- guard: `Some(v) if v > 0 => v`; guard false → falls to next arm; guarded arms may repeat a tag; re-matching a tag already covered by an unguarded arm = duplicate-arm error. A dead arm the duplicate rule misses (nested pattern or tuple combination already covered) is NOT an error — it compiles, and the editor dims it (engine `ttHints`).
- nested: `Ok(value: Some(v)) => v`; inner UNIT case needs parens `field: None()` (`field: name` = alias); no combining with or-patterns; same binding name twice in a pattern = error (alias one); inner mismatch falls through.
- Name resolution: pattern tags and field names are checked against the declaration — but ONLY when ttc can name what you meant (case-insensitive or near-miss; a transposition counts as one edit), because tag patterns also match hand-written `kind` unions whose tags are in no table. `Circel(r)` → `variant Shape has no case \`Circel\`` plus `help: a case with a similar name exists: \`Circle\``; `Circle(radiuz)` → `case \`Circle\` has no field \`radiuz\`` plus the field's replacement. The replacement is an applicable edit, not just prose. Same rule in let-else / `if let` (a single-tag site needs a ONE-edit match to report the tag; an or-pattern's several tags are match-grade evidence and use the match rule; fields are checked once the tag resolves) and in nested patterns (resolved against the outer field's declared type). A wrong-but-not-typo name is NOT reported (needs types). A reported typo suppresses that match's exhaustiveness error.
- Exhaustiveness: match without `_` is checked; missing case = compile error. Variant resolution: local decl > direct (1-hop) relative-`.tt`-import > built-in Option/Result. GUARDED arms NEVER count as covering (add an unguarded arm or `_`); NESTED patterns DO — the check descends into payloads, so `Ok(value: Some(v))` + `Ok(value: None())` + `Err(e)` is exhaustive, and a hole is reported as a PATTERN you can paste back (`missing "Ok(value: None)"`). An inner position's variant comes from the field's declared type, else from the patterns written there (so generic `T` payloads still work); if neither names a variant, only `_` covers that position. With `_`: unchecked. Unknown union: compiles unchecked, runtime default throws on unexpected kind.
- await allowed in scrutinee/guards/bodies → remains in the surrounding async owner; an expression-only boundary uses an awaited async callback. Detection is token-level: await inside a nested callback also triggers async — avoid in non-async contexts.

Literal match (`switch ($tt_m)` instead of `$tt_m.kind`):
```tt
match (code) { 200 | 201 => "success", 404 => "not found", _ => "other" }
match (flag) { true => "yes", false => "no" }
```
- Literals: string (`"a"`/`'a'`), number (`404`, `-1`, `0xff`, `1_000`, `1.5e2`, `10n`), `true`/`false`. No bindings. or-pattern alternatives must all be the SAME kind (`"a" | 1` = error). Guards allowed, same rules as tags.
- Duplicates compared BY VALUE: `200` and `0xc8` are the same arm → duplicate-arm error. `1n` ≠ `1`.
- NOT allowed inside tuple patterns (v1) — tuple elements are tag patterns or `_`.
- Exhaustiveness: the DEFAULT compile path does NOT check it (ttc has no TS types) — `_`-less literal match just gets a runtime `throw` guard. `ttc --check-types`/`--types` DO check it via the TypeScript checker, but only when the scrutinee type is a finite literal union (`"a" | "b"`, `1 | 2`, `boolean`, `typeof arr[number]`); `string`/`number`/`unknown`/`any`/`T`/`"a" | string` are never diagnosed. Reported at the `.tt` `match` keyword.

Tuple match (product exhaustiveness — missing COMBINATIONS are errors):
```tt
match (conn, mode) { (Online(latency), Auto) if latency < 50 => 10, (Online, _) => 5, (Offline, _) => 0 }
```
Every arm = tuple pattern (or final bare `_` covering all); element count = scrutinee count (an arity-one side is still claimed when the other side proves tuple intent, so ttc reports the exact mismatch); no `(A,B)|(C,D)` — use element-level or `(A, B|D)`; parenthesize scrutinees containing top-level `<`/`>` comparisons.
- Exhaustiveness is the product of the positions. `ttc --check-types` asks the checker for each position's alphabet, so narrowed types count, and reports combinations unquoted: `match is not exhaustive: missing (North, Slow)`. A position no arm tags stays `_`.

## try (Rust `?`)

```tt
const parsed = try parseNum(cfg);   // in fn returning Result: Err → returned from fn now
try validateRange(parsed);          // propagate-only; `try await f();` ok
```
- Statement position in a function body ONLY; trailing `;` MANDATORY (else passthrough).
- Result only (Ok unwraps `.value`; Err returned from enclosing fn). Option unsupported → `Option.okOr(o, err)` first.
- Enclosing fn return type must be Result compatible with expr's Err type; no auto conversion.
- UNANNOTATED fn: tsc infers the union of the return paths, so several `try`s with different Err types give `TOk<T> | TErr<E1> | TErr<E2>` = `TResult<T, E1 | E2>`. ttc never collects/unions error types — leave inference to tsc.
- FORBIDDEN (compile error): expression positions such as `return try f()` (bind first with `const value = try f();`), module top level / namespace body (no function for the emitted `return` to exit), and statement positions directly inside match (scrutinee/arm), template interpolation, `result` block, another try (the propagation would leave the construct's isolated value region). ALLOWED inside a function you write there — `run(() => { try g(); ... })` in a guard/step/arm is Rust's `?` in a closure — and inside if-let bodies / let-else else blocks whose statement sits in a function (inline contexts inherit the function). Placement is a control-flow fact, not a nesting rule.
- Expr can't start with `(` or `<`: `try f(x);` not `try (f(x));`.

## let-else

```tt
const Some(value: user) = findUser(id) else { return "who?"; };
```
- Pattern parens AND trailing `;` mandatory (else passthrough).
- else block must diverge — a CONTROL-FLOW check on a real CFG: every path leaves via return/throw/break/continue. Every TypeScript statement form is modeled. Accepts a diverging final statement, an `if`/`else` (chains too) whose branches ALL diverge (`if (c) return a; else return b;` is fine), a diverging bare block, unreachable code after a diverge, a `switch` with a `default` where no clause falls out (clauses fall through; an unlabeled `break` targets the switch, so it does NOT diverge), a loop with no normal exit (`while (true)`, `for (;;)` — only if no `break` leaves it), a `do`/`while` whose body diverges, `try`/`catch` when BOTH halves diverge (the handler may run in place of the guarded block), `try` without a handler when the guarded block diverges, any `try` whose `finally` diverges, and a labeled `break` whose landing site diverges. A `break`/`continue` naming a loop or switch inside the block does NOT leave the block. Statement boundaries do not need `;` (ASI-aware). tt's own constructs are answered exactly, not approximated: an `if let` counts when BOTH its body and its `else` diverge (chained `else if let` walked; no `else` = falls through), because both halves are INLINE — an exit written there leaves the enclosing function. A match arm, a `result` block and every other value region are ISOLATED, so an exit written in one belongs to the construct's value and can never leave the block; a `try` statement's early return is conditional. None of those can ever make the block diverge. A nested function's `return` doesn't count either — it leaves that function. An object literal's / arrow body's `}` ends no statement, so `else { return { kind: "Err", error: e }; };` is one diverging `return`.
- Or-patterns OK (`const Circle(r) | Square(r) = s else {...};` — first alternative needs parens, all alternatives must bind the same (field,name) set, shared destructuring); no guard/nested; no `= try expr else`. Position limits same as try (module top level allowed — no `return` of its own).

## if let

```tt
if let Some(value: user) = findUser(id) { greet(user); }
else if let Some(value: c) = cache.get(id) { greet(c); }
else { prompt(); }
```
- Statement position only (incl. match block-arm bodies); in expression regions only inside a function you write there (same flow rule as try).
- Pattern parens mandatory (first alternative); nested ok (`if let Ok(value: Some(value: v)) = r {}`); or-patterns ok (`if let Circle(r) | Square(r) = s {}` — same-binding-set rule, no nested inside or); no guards.
- else = block or another if-let ONLY; plain `else if (cond)` must go inside an else block.
- Malformed if let = located compile error (not passthrough).

## |>

```tt
const label = half(4) |> Option.mapP(x => x + 1) |> Option.unwrapOrP(0) |> .toFixed(1);
```
- `x |> f` = `f(x)`; step starting `.` = postfix chain on piped value (`x |> .trim().split(",")`). A step starting `?.` is one JavaScript optional postfix tail: `?.name`, `?.[key]`, or `?.(args)`, followed by ordinary/optional member, index, or call operations.
- Multi-arg: std `*P` curried variants or parenthesized arrow `x |> (n => add(n, 2))`.
- PARENTHESIZE ternaries & arrows at head/step top level: `(c ? a : b) |> f`, `x |> (n => n+1)` — else compile error.
- An optional step short-circuits only its own tail; a following `|> f` still calls `f(undefined)`. Optional keys and arguments are evaluated only when JavaScript would reach them. No empty step or incomplete/unsupported optional tail; no try STATEMENT inside head/step (pipeline inside a try expr is fine: `const a = try readCfg() |> normalize;`).
- Malformed `|>` = located compile error. Ambiguous head (no-semicolon style, `in`/`instanceof`) → parenthesize head.
- `flow` head = compose FUNCTIONS instead of piping a value: `const label = flow |> half |> Option.mapP(x => x + 1) |> .toFixed(1);` then `label(4)`. Same step rules; nothing runs until the composed fn is called.
- `flow` is contextual — only a head that is exactly `flow`; a `flow` VARIABLE pipes when parenthesized (`(flow) |> f`). `flow |> f` (one step) = `f`.
- flow's FIRST step fixes the input type and cannot be a method step (compile error). Generic/curried first step → `unknown`; give type args (`flow |> wrap<number> |> ...`, `flow |> Option.mapP((x: number) => x + 1) |> ...`). Later steps infer from the previous step.

## result block

```tt
const data = result {
  const user <- getUser(id);              // Ok → bind value; Err → whole block IS that Err
  const name = user.name |> .trim();      // ordinary TS/tt statements between bindings
  const company <- getCompany(user.companyId);
  { user, company, name }                 // LAST expr, NO `;` — wrapped in Ok
};
```
- Flat replacement for nested `Result.andThen(r, user => ... )` callbacks; every earlier binding stays in scope.
- `result` is contextual: block is claimed ONLY if it has ≥1 `<-` binding, else plain identifier + block statement (passthrough). Write `<-` with no space.
- Binding = `const|let|var <name|destructuring|: type> <- expr;` — `;` MANDATORY on bindings, FORBIDDEN on the final value expr (else located compile error). A top-level `>` in the bound expr needs parens (generic-type-argument ambiguity). Forgetting the keyword (`y <- g();`) is a located error (`` `result` binding is missing its declaration keyword ``) wherever the text cannot be TS — which is any claimed block; for an actual `y < -g()` comparison, put a space between `<` and `-`.
- Result only (no Option/Promise do-notation, no `<-` outside a result block). Bindings must be TOP-LEVEL statements of the block — `<-` inside an `if`/loop/function within the block is a located error (it cannot early-return the block); hoist it or `match`.
- Block is an EXPRESSION: usable anywhere, incl. pipeline head. Statement-capable owners use a result slot and explicit failure/success edges; expression-only owners use the shared named boundary. `await` stays in the surrounding async owner or is awaited at that boundary.
- Error types UNION automatically: bindings of `TResult<_, E1>` + `TResult<_, E2>` → block assignable to `TResult<T, E1 | E2>`. ttc infers NO types; tsc narrows each step.
- `return` inside the block returns from the BLOCK. So `try`/let-else directly in the block's statements are FORBIDDEN (located error) — use `<-`; inside a function written in the block they are fine. `if let` is fine anywhere here.
- Final expr already a Result → nested `TResult<TResult<...>>`; bind it with `<-` instead.

## @tt/std

```tt
import type { TOption, TResult } from "@tt/std";
import * as Option from "@tt/std/option";
import * as Result from "@tt/std/result";
```
- `TOption<T>` = `Some(value: T) | None`; `TResult<T, E>` = `TOk<T> | TErr<E>` (`TOk<T>` = `{kind:"Ok";value:T}`, `TErr<E>` = `{kind:"Err";error:E}`, both exported as TYPES). Field names: `value` (Some/Ok), `error` (Err) → arms `Some(value)`, `Ok(value)`, `Err(error)`, alias `Some(value: v)`.
- Constructors take only their own variant's type: `Result.Ok(1)` → `TOk<number>`, `Result.Err("bad")` → `TErr<string>` (NOT `Result.Ok<number, string>(1)` — one type arg each). Both fit any `TResult<T, E>` slot; annotate the variable/return type when you need the full Result. Combinators take/return `TResult<T, E>`.
- `andThen`/`andThenP` UNION the error types: `TResult<T, E>` + `(T) => TResult<U, F>` → `TResult<U, E | F>`, so a pipeline of steps that each fail differently ends up with every error type (also works on the scattered `TOk<T> | TErr<E1> | TErr<E2>` a `try`/`result` value infers as; `TErrorOf<R>` is exported for reading the error side out). `map`/`mapP` add no failure → error type unchanged.
- `andThenP` reads its input type off the function you pass: named function → nothing to write; inline arrow → ANNOTATE the parameter (`Result.andThenP((u: User) => f(u))`), else it is `unknown`.
- Both are BUILT-IN variants: `_`-less match on their tags is exhaustiveness-checked even without import. Built-ins give checking only — import (or declare) to construct values.
- Combinators = data-first static fns; `*P` = data-last curried for pipelines.
  - Option: map andThen orElse filter unwrapOr unwrapOrElse expect okOr fromNullable toNullable isSome isNone zip flatten transpose collect (+P: map andThen orElse filter unwrapOr unwrapOrElse expect okOr)
  - Result: map mapErr andThen orElse unwrapOr unwrapOrElse expect ok err fromThrowable fromPromise isOk isErr flatten transpose collect (+P: map mapErr andThen orElse unwrapOr unwrapOrElse expect)
- Bridges: `Option.fromNullable(x)` (T|null|undefined), `Result.fromThrowable(() => JSON.parse(s))`, `Result.fromPromise(p)`, `Result.collect(arr)` / `Option.collect(arr)`.

## val

```tt
val const config = load();          // binding + every path from it is read-only
val let state = { count: 0 };       // still rebindable: state = {...state}
function read(val user: User) {}    // param the function cannot mutate
const f = (val u: U) => u.name;     // arrows, methods, catch (val e), for (val const x of xs)
```
- No modifier = plain TS = mutable. There is no `mut`.
- ERRORS on a val-rooted path, at ANY depth (`s.a.b.name = v`): `x.a = v` (all compound forms), `x[i] = v`, `x.a++`/`++x.a`, `delete x.a`.
- Method calls are NOT judged by name: `query.set("k")` on a user-defined `set` is fine. `ttc --check-types`/`--types` (only) report a call they resolve to a built-in mutator — Array push/pop/shift/unshift/splice/sort/reverse/fill/copyWithin, Map set/delete/clear, Set add/delete/clear, WeakMap set/delete, WeakSet add/delete, TypedArray set/sort/reverse/fill/copyWithin — so `val const items: number[] = []; items.push(1)` fails under `--check-types` and passes plain `ttc`. Unresolvable receiver (`any`, type param) = not reported. The VSCode extension shows these while editing (it runs the same mode over the buffer), so you do not have to save and run the CLI to see them.
- NOT an error: `x = v` (that is const/let's axis), reads, comparisons, spread `{...x}`.
- Call check: a val binding may only be passed to a `val` parameter of a same-file named function (`function f`, `const f = (...) =>`, `const f = function`). Plain path args only.
- val is per-BINDING, not per-object: `val const view = original;` still lets `original.x = 1`. Inner declarations shadow an outer val.
- Compile-time only: keyword (and its trailing spaces) erased, no runtime, no `readonly`.
- SYNTAX rule: `val` must sit on the same line as `const|let|var` or as the parameter binding it modifies; anywhere else `val` is an ordinary identifier and passes through. Not usable in match patterns (`Ok(val u)` → the match won't parse).

## Modules

- Import `.tt`/`.ttx` files by relative path WITH extension: `./token.tt` → `./token.js`, `./view.ttx` → `./view.jsx` by default (`--rewrite-imports ts` emits `.ts`/`.tsx`; `off` preserves source specifiers).
- Exhaustiveness sees exported variants from DIRECT (1-hop) relative `.tt`/`.ttx` imports (named/aliased/`* as ns`); re-export chains & package paths NOT collected → those matches compile unchecked.
- Dynamic `import()` specifiers not rewritten.

## Install

- `bunx @load28/create-tt@next app` → new Bun + Vite project; `bunx @load28/create-tt@next init` → structurally adds tt to an existing TS project, detects Vite/Rollup/Rolldown/webpack/Rspack/esbuild/Farm, and preserves existing config through a generated wrapper where possible. The initializer does not install an npm TypeScript package. New projects use Bun; existing projects retain their declared/locked package manager unless overridden.
- Local built packages through a real registry: run Verdaccio with `scripts/verdaccio.local.yaml`, then `bun scripts/publish-local-registry.mjs http://127.0.0.1:4873`; use the printed `BUN_CONFIG_REGISTRY=... bunx @load28/create-tt@next ... --registry ...` command. This publishes the current platform binary and TT packages rather than substituting `file:` dependencies.
- Current Nightly: `bun add -d @load28/tt-lang@next` → officially published prebuilt `ttc` binary (linux-x64/arm64, darwin-x64/arm64, win32-x64), run via `bunx ttc`. Direct bundler imports additionally need `@load28/unplugin-tt@next`. ttc drives TypeScript 7 and resolves it from the project's own `node_modules` and nowhere else, so add `typescript@7` as a dev dependency. `ttc --types` (declaration sidecars) additionally needs the 7.1 declaration-emit API.
- Other platforms / no npm: `cargo install --git https://github.com/load28/tt`; to keep using the npm launcher, set env `TTC_BINARY=/path/to/ttc`.
- Update: `npm i -D @load28/tt-lang@next` (binary follows package version); verify `npx ttc -v`; then re-run `npx ttc --types src` and rebuild.
- Editor: download the VSIX from the newest GitHub Releases pre-release. Everything TypeScript answers comes from the compiler's own language server (`tsgo --lsp`), driven by the project's installed TypeScript — the same package the build uses, with no environment variable or editor setting able to name another.

## Setup

New project: `bunx @load28/create-tt@next app`; sources in `src/**/*.tt` or React/JSX sources in `src/**/*.ttx` (hand-written `.ts`/`.tsx` alongside is fine); gitignore `.tt-types/` and the out dir. Full manual paths: `docs/getting-started.md` / `docs/getting-started.ko.md`.
```jsonc
// package.json
"scripts": { "build": "ttc -o build src && tsc", "types": "ttc --types src", "check": "ttc --check-types src" }
// tsconfig.json — resolve "./x.tt" and "@tt/std":
"compilerOptions": { "rootDirs": ["./src", "./.tt-types"], "paths": { "@tt/std": ["./.tt-types/tt/index.d.ts"], "@tt/std/*": ["./.tt-types/tt/*.d.ts"] } }
```
Bundler alternative: `@load28/unplugin-tt` (`import tt from "@load28/unplugin-tt/vite"`, also `/rollup` `/webpack` `/esbuild`) — bundler reads `.tt`/`.ttx` directly, no ttc build step; types still via `ttc --types`.

## Workflow

- Edit loop: change `.tt` → `npx ttc --check src` (fast tt-level, no TypeScript) → `npx ttc --check-types src` (types, exhaustiveness by narrowed type, `val`).
- `.ts` importing `.tt` (TypeScript 7.1+): declare the content mapper once in tsconfig — `"contentMappers": [{ "package": "@load28/tt-lang", "extensions": [".tt", ".ttx"] }]` — and run `tsc` with `--runExternalCode`. TypeScript holds the transform virtually (no files on disk, no `rootDirs`/`paths`); diagnostics land at original `.tt` positions, tt-level rules under the `tt` source. The VS Code extension auto-registers the mapper with the TypeScript extension. Editor prerequisites until TypeScript 7.1 is officially released: install BOTH VSIXes from the newest nightly GitHub Release (`tt-language-*` and `tt-typescript-preview-*-<platform>`, the 7.1-API TypeScript build the Marketplace extension does not ship yet) and set `"js/ts.experimental.useTsgo": true` + `"typescript.experimental.useTsgo": true`. After the official 7.1 release: official extension route, no useTsgo needed.
- `.ts` importing `.tt` (classic tsserver, LEGACY): keep `npx ttc --types -w src` running so editor/tsc resolve `./x.tt` + `@tt/std` via sidecars; if not watching, re-run `--types` after variant changes.
- Build: `npm run build` (ttc emits TS tree then tsc) or bundler build. CI: `ttc --check src && tsc --noEmit` + tests.
- `ttc <dir>`: `.tt`→`.ts`, `.ttx`→`.tsx`, hand-written `.ts`/`.tsx` passthrough; `-o <dir>` separate tree (in-place overwrite refused); `@tt/std` auto-materialized when imported. `ttc -w` watches and also recompiles importers of changed files (cross-file exhaustiveness). Files compile in parallel (one per core) with identical output/diagnostics either way; `-j <n>` sets the count, `-j 1` = sequential.
- Emitted `.ts`/`.tsx` starts with `// @generated` — NEVER edit output or `.tt-types/`; edit the `.tt`/`.ttx` source.
- Source maps: `--source-map file` writes `<output>.map` beside each compiled file and appends `//# sourceMappingURL=`; `inline` puts the map in the output as a `data:` URL (what `-p` and bundler plugins use). Default `off` — a map appends a line, and hand-written `.ts` passes through byte for byte, so only compiled `.tt`/`.ttx` ever get one. With `node --enable-source-maps` a stack frame names the `.tt` line and column; a frame in generated glue names the construct that wrote it (the `match`, `try`, pipeline or `variant`). `@load28/unplugin-tt` asks for a map by default (`sourcemap: false` opts out) and hands it to the bundler.
- Offline docs: `npx ttc help` lists topics; `npx ttc help <topic>` (e.g. `match`, `try`, `install`) prints that section of this guide; `npx ttc help all` prints it whole. `npx ttc -h` = CLI options.

## Errors

- `ttc: file:line:col: msg` — e.g. `match on variant X is not exhaustive: missing "Y"` (add arms or `_`), `duplicate arm`, `or-pattern alternatives must bind the same names — <which binding differs>`, `cannot mix tag patterns and literal patterns`, else-block-must-diverge, try-position-restriction (extract helper), `cannot mutate through val binding `x``, `cannot pass val binding `x` to mutable parameter `p` of `f``. In typed modes, an unlowerable tt construct becomes a parser-owned error placeholder only for projection; its cascade is suppressed, while independent type errors elsewhere in the same file remain visible.
- `ttc --check-types` also prints `match on literal union is not exhaustive: missing "..."` for `_`-less literal matches over finite literal unions, and ``cannot call mutating method `set` through val binding `m` `` for val paths it proves land on a built-in mutator. Its variant exhaustiveness message has no variant name (`match is not exhaustive`) — the answer comes from the narrowed type, not from a declaration table, which is also why it is the more accurate of the two.
- Type judgments come from TypeScript and are reported at the **`.tt` source** position. Assignability failures use the common ``type mismatch: expected `<type>`, found `<type>` `` renderer; other checker diagnostics keep their translated or raw message. `--check-types` exits 1, `--types` still writes sidecars, and VSCode shows the identical compiler message inline (`source: ts`, stable `ts<code>`, `tt.typeDiagnostics` to disable). Applies inside `match` arms and `|>` pipelines too.
- A `|>` step's combinator inferring `unknown` (e.g. `Result.mapP((n) => n)` with `n: unknown`) means the pipeline **head** has no usable type — it is not a `Result`, or the head is an unannotated parameter. Fix the head, not the step.
- tsc errors on output containing literal `match`/`try` → silent passthrough; recheck semicolons/parens/reserved words.
- `generated TypeScript failed to parse` → pass-through source was invalid TS, or ttc bug.

## Checklist

`val` only in front of `const|let|var` or a parameter, same line; match parens + `_` last + object arms `({...})`; bind by field name not position; literal patterns are values (never mixed with tags, none in tuple elements); `_`-less match covers all (guards/nested don't count); `try`/`let-else` need `;` and diverging else, and a function to sit in when inside match/`${}`/`result` (try also at top level); pipelines parenthesize ternaries/arrows, use `*P`; `result` blocks need ≥1 `<-` binding, `;` on bindings and none on the final expr; relative imports keep `.tt`; verify with `npx ttc --check-types src`, re-run `npx ttc --types src` after variant changes; never edit generated `.ts`.
