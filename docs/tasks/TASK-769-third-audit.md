# TASK-769: Fix defects found by the third audit

- **Status**: Complete
- **Started**: 2026-10-06
- **Completed**: 2026-10-06
- **Commit**: —

## Purpose

A third audit of the compiler, the command line and `ttc --server` found
defects in output layout, sidecar preservation, configuration lookup and
request validation. Fix each in the layer that owns it.

## Scope

- Included: the output layout of several inputs (R3L1); the standard
  library's place and specifiers in an in-place build (R3L2); a sidecar
  replaced by one typed from a placeholder (R3L3); the project a `.ttx`
  import's spelling is read from (R3L4); `--types` sidecars of two
  inputs (R3L5); the server's request and option validation (R3L6); the
  in-place refusal's advice (R3L7); the path `--check-types` shows for a
  file outside the working directory (R3L8); non-ASCII paths in the
  language service's URIs (R3E1); the outline's and navigation targets'
  position conversions (R3E2); an empty match body (R3E6); TypeScript in a
  pipeline that does not parse (R3E7); `try` in a setter (R3C8); a case
  declaring one field twice (R3C9); a parenthesized comma operand before a
  tt value (R3C2); a pattern subject with a plain template after a tt
  value (R3C3); a `super` pipeline head (a regression from Decision 11);
  an integer literal receiver of a postfix step (R3C15); an arrow step
  whose template body holds a `match` (R3C1); a callback holding an `if
  let` beside a hoisted argument (R3C5) or in a shadowed pipeline step
  (R3C6); an `if let` subject piping a `match` (R3C7); a `val` binding
  passed through a wrapped callee (R3C10); an outer `try` around an inner
  `try` (R3C4, the pipeline form); an `await` in a function nested in a
  `result` block (R3C14); a pattern subject holding a tt value beside a
  pipeline, a `result` block, or a nested statement; a `flow` pipeline
  head before structured steps (R3C12); a spread before a pipeline; a
  parenthesized right operand of a short-circuit; an inert operand holding
  a tt statement; a captured operand holding a tt value; a `result`
  block returning a nested `match`; a discarded comma operand that is a
  tt value; an outer `try` around a `match` and an inner `try` (R3C4); a
  field an or-pattern binds that one of its cases lacks (R3E5); a parse
  failure on a `result` block's expression boundary (R3C14).
- Excluded: the editor's placement of a TypeScript assignability error at
  the value a structured mismatch names (R3E3; the CLI relocates it from
  the checker's structured facts, which the editor's pull diagnostics do
  not carry, so it needs one diagnostic producer for both surfaces and is
  a task of its own). References on a large file (R3E2) were measured
  after the outline fix: they grow linearly (0.23 s for 2,401 references
  to a constant in a 1,200-line file, 0.12 s for the same shape in a `.ts`
  file), so nothing was changed.

## Decisions

### Decision 1: Every input mirrors under the directory all inputs share

- **Context**: `ttc -o out src lib` wrote `out/a.ts` and `out/x.ts`, while
  `a.ts` still imported `../lib/x.js`; `--types -o` laid sidecars out the
  same way. TASK-352 Decision 5 kept each directory input mirroring under
  itself so the collision contracts of TASK-321 and TASK-338 answered as
  before.
- **Alternatives considered**: Keep per-directory roots and rewrite the
  specifiers (the output would no longer mirror the inputs); one root for
  all inputs.
- **Decision and rationale**: `tsc` writes every output relative to one
  common source directory, so an import between two inputs keeps its
  meaning in the output tree. ttc now does the same for builds and
  sidecars (`input_root`). Neither collision contract is weakened: two
  separate roots no longer share an output, and overlapping roots give a
  source one output. Their tests now pin that behaviour; TASK-352 records
  the reversal.

### Decision 2: The support root is a directory, not a spelling

- **Context**: `cd src && ttc deep ../lib` wrote the standard library to
  `src/tt` and `lib/b.ts` imported `../.././tt/option.js`, which does not
  exist; naming `./src/b.tt` beside `src/a.tt` moved it to `./tt`.
- **Decision and rationale**: The support root is documented as the
  deepest directory every output shares. It was computed over the outputs'
  path text, so `..`, `.` and absolute spellings counted as different
  directories, and specifiers to a `tt/` not written yet were computed from
  raw text. Both now work on normalized absolute paths; the root is shown
  relative to the working directory when it is inside it.

### Decision 3: A recovery in an exported statement withholds declarations

- **Context**: `export const a = match (...) { ... ;` (a malformed match)
  replaced the sidecar's `a: number` with `a: any`. TASK-590 withheld
  declarations only for a recovery standing for a declaration, so that a
  recovered expression in unrelated code still refreshes the sidecar (the
  extension's contract).
- **Alternatives considered**: Withhold for every recovery (breaks that
  contract); mark the placeholder with a type the declaration emit would
  print (changes hover and diagnostics on the projection); withhold for a
  recovery inside an exported statement.
- **Decision and rationale**: A placeholder decides the declared type of
  the statement it sits in when that statement is exported (its
  initializer or body is what the type is inferred from). The lexer's
  statement model finds the top-level statement around the recovery; when
  it begins with `export`, the recovery counts among
  `recovered_declarations`, so the previous sidecar stands, as the
  reference states. A recovery in a statement that is not exported keeps
  refreshing. Remaining debt: a non-exported declaration that an exported
  one names (`export type T = typeof broken`) or exports later
  (`export { broken }`) is not detected.

### Decision 4: Each file's `.ttx` imports follow its own project

- **Context**: `ttc -o out a b/i.tt`, with `a/tsconfig.json` choosing
  `"jsx": "preserve"`, wrote `./v.js` for `a/i.tt`'s import; built alone,
  `a` wrote `./v.jsx`. The build looked for one configuration above the
  common directory of all inputs, while `--check-types` gives each file
  its own project.
- **Decision and rationale**: `tsc` names a `.tsx` output by the options
  of the project compiling it, and a file belongs to the nearest
  configuration above it. The build now reads the `jsx` option per file
  (`--project` still names one configuration for all), and a watch round
  rebuilds everything only when a file's answer changes, not when a file
  is added.

### Decision 5: Server options are checked for their type

- **Context**: `print` with `"banner": "no"` printed the banner and
  `"sourceMap": true` fell back to `off`, while a wrong string value was
  refused; a line with no `method` (`{"id":1}`, `[1,2]`, `"str"`) was
  answered `unknown method ""`.
- **Decision and rationale**: JSON-RPC 2.0 answers a request that is not
  an object with a `method` string as an invalid request; the session now
  answers it `malformed request`, the wording it already uses for a line
  it cannot read. An option of the wrong JSON type is refused, naming the
  option and the expected type, as a wrong string value already was
  (`print`'s `banner`, `verify`, `sourceMap`, `rewriteImports`, `check`'s
  `verify`, `completion`'s `member`, `signatureHelp`'s `isRetrigger`,
  `typedCheck`'s `includeTypes`).

### Decision 6: The refusal's advice holds with and without `-o`

- **Context**: An in-place build refusing an edited output advised
  "choose an empty output directory" though no `-o` was given.
- **Decision and rationale**: The ownership check does not know whether a
  build has an output directory, so its advice now names both ways out:
  remove the file, or write the outputs to another directory with `-o`.

### Decision 7: A file outside the working directory is shown relatively

- **Context**: From a sibling directory, `--check` showed
  `../ex2/src/bad.tt` and `--check-types` the absolute path.
- **Decision and rationale**: `tsc` shows every file relative to the
  working directory, `..` included. The typed pass strips the working
  directory and, for a path outside it on the same root, now writes the
  relative path with `..`.

### Decision 8: A file URI percent-encodes a path's UTF-8 bytes

- **Context**: In a directory named `ö`, the language service reported
  TS2307 for `./h.ts` and hover returned nothing; definitions came back
  under `Ã¶`.
- **Decision and rationale**: `file_uri` wrote each UTF-8 byte of the
  path as a character, so `ö` reached the server as two Latin-1
  characters. typescript-go's `FileNameToDocumentURI`
  (`internal/ls/lsconv/converters.go`) escapes a path segment with
  `url.PathEscape`, which percent-encodes every non-ASCII byte, and
  `uri_path` already decodes `%XX` back to bytes. Non-ASCII bytes are now
  written as `%XX`.

### Decision 9: A served document measures its lines once

- **Context**: `documentSymbols` took 0.97 s, 0.95 s and 3.79 s for 150, 300
  and 600 matches, against 0.13-1.15 s for the same file without tt syntax.
- **Decision and rationale**: Every entry of the outline, and every
  navigation target, converted its positions through helpers that measure
  the whole text's lines and count UTF-16 units from the start, so the
  work grew with the number of entries times the text's length. The
  served document now keeps its source's and its code's line measurements
  (computed once, `LineIndex`), and the outline and navigation targets
  convert positions to bytes and back through them, mapping bytes to the
  source directly. The outline now takes 0.09 s, 0.20 s and 0.33 s for 300,
  600 and 1200 matches.

### Decision 10: An empty match body is an unfinished arm list

- **Context**: `match (s) {}`, what an editor leaves after closing the
  braces, was not claimed, so the file failed as TypeScript and the
  service reported `Cannot find name 'match'`.
- **Decision and rationale**: TASK-768 made a body of patterns alone an
  arm-list candidate; an empty body is the same list with no element
  written yet. It is now a candidate too, and the host's parser still
  keeps `class D extends match (1) {}` TypeScript.

### Decision 11: A pipeline's hidden parts are parsed as TypeScript too

- **Context**: `r |> ((q) => q.)` failed the output self-check
  (`verify-failed`, which names a possible ttc bug), while the same text
  outside the pipeline is `source-not-typescript`.
- **Alternatives considered**: Project every head and step beside the
  pipeline's placeholder (changed the evaluation protocol of 23 cases and
  broke `flow` and missing steps); parse the hidden parts on their own.
- **Decision and rationale**: The reference makes TypeScript that does not
  parse inside a claimed construct `source-not-typescript`, decided from
  the projection. A pipeline is one placeholder in the projection, and a
  head or step holding no tt value stayed hidden behind it. The projection
  now records those parts, and each is parsed as an expression (a postfix
  step after a stand-in receiver) in an async generator method, where
  `yield`, `await` and `super` are as valid as in the surrounding code; a
  failure maps back to its source byte.

### Decision 12: A setter is not a `try` target

- **Context**: `set x(n) { const a = try r(n); ... }` compiled without a
  tt diagnostic; the output returned the `Err` from the setter, which
  TypeScript rejects (TS2408) and JavaScript discards, so the failure was
  lost.
- **Decision and rationale**: The reference already rejects targets whose
  return cannot carry the `Err` (a constructor, a generator, a static
  block). A setter's return value is discarded by the assignment that
  calls it (ECMA-262 §10.2.1, the `[[Set]]` result is the assigned value),
  so it joins them: the lexer marks class and object-literal setter bodies,
  the checker's function targets and the planner's evaluation owners
  report `try-placement` for a statement or value `try` there. The
  reference lists the setter.

### Decision 13: A case declaring one field twice is `variant-duplicate-field`

- **Context**: `variant V { A(n: number, n: string) }` passed `ttc --check`
  and emitted a constructor with a duplicate parameter, which does not
  load in strict JavaScript.
- **Decision and rationale**: The duplicate is a fact of the declaration
  as written, like a duplicate case tag (`variant-duplicate-case`), so it
  is a tt rule with its own code (tt53, appended so no number changes
  meaning), explanation, reference sentence and generated diagnostic cases;
  the checker reports the second occurrence with the help to rename it.

### Decision 14: A comma operand is the whole operand, parentheses included

- **Context**: `((o), match (o) { ... })` and `((g()), try r(1))` stopped
  the compiler with "a discarded comma operand is not followed by its
  comma".
- **Decision and rationale**: The comma expression's operands were read
  with the capture rule that looks through parentheses to the value inside
  them, so the operand `(o)` ended before its `)` and the comma after it
  could not be found. An operand evaluated only for its effects is the
  whole operand ECMA-262 names (`Expression , AssignmentExpression`),
  parentheses included; the lowering now runs it as written and removes it
  with its comma.

### Decision 15: A template without tt values is source to a sequence's anchor

- **Context**: `const A(n) = pick(match (k) { ... }, \`k=${k}\`) else { ... };`
  and the same subject in an `if let` stopped the compiler with "match
  reached expression emission without a host rewrite"; a string in place
  of the template compiled.
- **Decision and rationale**: Every template literal is a Core template
  expression, so the sequence's scheduling anchor, the last tt child
  followed only by source, was the template, which has no statement form,
  and the subject was emitted as one expression with the `match` still
  inside. A template none of whose interpolations needs a host is
  TypeScript source like the text around it; the anchor now passes over
  any expression that requires no host, so the `match` is the anchor and
  the subject is lowered as statements.

### Decision 16: A `super` pipeline head is checked as the receiver it is

- **Context**: Decision 11's separate parse of an opaque pipeline head
  rejected `super |> .value`, which the lowering writes as `super.value`,
  and reported `super |> f` as a lowering failure at the file's first byte.
- **Decision and rationale**: TypeScript parses `super` only before an
  argument list or a member access. A head whose first step is a postfix
  step and whose text the lowering keeps bare (a member receiver) is
  parsed with a member access after it, as it is emitted; any other head
  is parsed alone, as the lowering parenthesizes it. A failure of the
  separate parse is located at the part it parses, since the wrapper
  around it is the compiler's.

### Decision 17: A decimal integer literal is not a member receiver

- **Context**: `5 |> .toFixed(1)` emitted `5.toFixed(1)`, which does not
  parse.
- **Decision and rationale**: The `.` after a decimal integer literal is
  its decimal point. TypeScript's emitter writes `5..toFixed` for the
  same case (`mayNeedDotDotForPropertyAccess`: a literal with no radix
  specifier, `.`, or exponent). The receiver rule this compiler already
  uses parenthesizes anything that is not a member receiver, so such a
  literal is no longer one and is written `(5).toFixed(1)`.

### Decision 18: A sequence enters a host region where its source does

- **Context**: `v |> (w => \`value: ${match (w) { ... }}\`)` emitted the
  arrow's block before `(w =>`, which does not parse.
- **Decision and rationale**: A sequence's structured span is the span of
  its first tt child, so the step `(w => ...)` claimed the rewrite of the
  arrow body that begins at the template. Expression emission opens a
  host region before an expression only when the expression's emission
  starts there; a sequence whose source begins before its first tt child
  now leaves the region to the walk over its own source, which reaches it
  at the arrow body.

### Decision 19: Captured source is lowered, not copied

- **Context**: In `show(k |> String, (v) => { if let A(n) = v { ... } },
  match (k) { ... })` the callback was captured as raw text with its `if
  let` unlowered, and the `if let` was emitted a second time at the
  `return`; the same callback in a `match` arm beside `try` was copied
  unlowered too.
- **Decision and rationale**: A source capture can contain a nested
  function whose body holds tt statements; those are separate Core
  statements, so copying the bytes skips them. Every capture of a call
  completion, an optional call's operands, and a ternary's source branch
  now goes through the capture composer, which lowers nested statements,
  and the operand composer lowers the statements its span contains. A
  statement inside a completed call's claimed frame is emitted by that
  frame only, as an expression inside it already was.

### Decision 20: A shadowed pipeline step projects its statements

- **Context**: `vs |> pick((v) => { if let A(n) = v { ... } }, match ...)`
  failed with "a tt node's source span 0..0 is invalid".
- **Decision and rationale**: A step projected beside the pipeline
  placeholder rejected any statement-shaped decision. Its `if let` sits
  in a function body the step contains, where the main projection emits
  the same decision; the shadow now emits it through the same projection
  (bindings included).

### Decision 21: An `if let` subject opens a `match` body

- **Context**: `if let A(n) = match (x) { ... } |> id { ... }` reported
  a stray `|>`.
- **Decision and rationale**: The lexer's facts ended the subject at the
  `match` body's `{`, so the `|>` after it started a statement. The
  parser already skips a `match (...) {` body in that subject; the facts
  machine now does the same, except in a class heritage clause, where the
  `{` after `match(...)` is the class body in TypeScript.

### Decision 22: A capture closes no owner it does not contain

- **Context**: With Decision 19, a ternary's source branch at the end of a
  concise arrow body closed the arrow's block inside the branch
  (`LayoutScopeMissing`).
- **Decision and rationale**: The source walk closes an arrow block where
  the arrow's body ends, even from a span that starts after the arrow,
  because the source after a trailing tt value is the only span that can
  carry the brace. A capture is source copied to another place, so while
  a capture is being written the walk closes only arrows that began
  inside it.

### Decision 23: A `val` callee is read through grouping and `!`

- **Context**: `(bad)(x)`, `bad!(x)`, `bad?.(x)`, `x |> (bad)` and
  `x |> bad!` passed a `val` binding to a mutating parameter without the
  `val-pass` error that `bad(x)` reports.
- **Decision and rationale**: The callee was recognized only as an
  identifier directly before `(` or as a bare identifier step.
  TypeScript resolves a call through the outer expressions that keep the
  callee's value (`skipOuterExpressions`: parentheses and non-null
  assertions), and an optional call is still a call. The check now walks
  out of grouping parentheses (not a call's own argument list) and `!`,
  accepts `?.(`, and reads a pipeline step the same way.

### Decision 24: A propagation statement shadows an operand holding a `try`

- **Context**: `const x = try half((k |> abs) + (try half(k)));` called
  `half(k)` before `abs(k)` and left the pipeline unlowered.
- **Decision and rationale**: The statement form projected its operand as
  an opaque placeholder unless the operand held a `match` or `result`, so
  the inner `try` had no call or operator frames to order its siblings by.
  The expression form already shadowed an operand holding a propagation;
  the statement form now does too, and the inner `try` is scheduled after
  the operands written before it.

### Decision 25: A sequence has a statement form when any tt value in it has

- **Context**: `const A(n) = [match (k) { ... }, k |> String] else { ... };`
  emitted the pipeline unlowered, and `if let A(n) = w(try r(k) + (() => {
  const A(m) = c else { ... }; ... })())` stopped the compiler with
  "unscheduled expression try reached inline emission".
- **Decision and rationale**: A sequence's statement form was its last tt
  value's, so a sequence ending in a pipeline, or in a nested statement,
  had none and was emitted as one expression with its `match` or `try`
  inside. The operand lowering that a statement form selects schedules
  every tt value of the sequence, so the sequence has a statement form
  when any of them has one. That lowering now also emits the tt
  expressions it copies around its values (a pipeline, a `result` block)
  and the statements inside its span, instead of their source text.

### Decision 26: An `await` belongs to the function it is written in

- **Context**: `(k = result { const z = try r(async () => await o.z); ... })
  => k` made the `result` block an awaited async boundary inside a
  non-async arrow, which does not parse.
- **Decision and rationale**: Whether a block awaits was read by a token
  scan that skipped `function` and `class` bodies but not arrows. The same
  function-boundary model the compiler uses for a generator's `yield` and
  a `try`'s target (`FunctionTargets`) now decides it: an `await` counts
  only when its innermost function is the block's. The token scan and its
  unit test are removed; a case pins the arrow, function, and block-bodied
  arrow forms.

### Decision 27: Only a value with a statement form fills a slot

- **Context**: `(flow |> g |> String) |> (q => result { ... }) |> ...`
  stopped the compiler with "structured apply head was not emitted".
- **Decision and rationale**: A `flow` composition has no head, so it has
  no statement form, yet it was handed a slot to fill. A nested value's
  slot is now offered only to a value with a statement form; any other
  head is emitted as an operand.

### Decision 28: A spread is not part of a pipeline head

- **Context**: `[...xs |> f]` was lowered as `$tt_ap(...xs, f)`, spreading
  `xs` into the helper, and after Decision 11 it was reported as source
  that does not parse.
- **Decision and rationale**: A spread element is `...` followed by an
  `AssignmentExpression` (ECMA-262 §13.2.4), and a pipeline is one, so
  the head starts after the spread. The parser's expression tracker now
  starts an expression after the third dot of a spread.

### Decision 29: A short-circuit keeps the parentheses its right operand had

- **Context**: `g((o.y ?? (o.x += try r(1))))` emitted
  `$tt_v4 = $tt_v2 ?? o.x = ...`, which does not parse.
- **Decision and rationale**: The branch that runs the right operand
  writes the operation over the stored left operand, but it copied the
  right operand without the parentheses around it, which an assignment,
  a conditional, or an operator that cannot mix with `??` needs there.
  TypeScript's emitter keeps a parenthesized expression the author wrote;
  the branch now keeps the operand's authored parentheses.

### Decision 30: An inert input holding a tt construct is captured

- **Context**: `((x) => { if let A(n) = v { ... } }) ? match ... : ...`
  copied the arrow into the condition with its `if let` unlowered.
- **Decision and rationale**: An input whose evaluation is unobservable
  (a function, a literal) is left in place rather than captured, and its
  text was copied as written. Its text needs lowering when a tt construct
  is inside it, so such an input is now planned as a capture, which the
  capture composer lowers. Evaluating a function expression once into a
  slot is as unobservable as evaluating it in place.

### Decision 31: A capture writes the values inside it

- **Context**: In `apply((flow |> ((w) => (result { ... }))), result { ...
  })` the first argument is captured before the second runs, and the
  arrow's `result` block was written as `return ();`.
- **Decision and rationale**: A value inside a capture's span was treated
  as carried by that capture and printed nothing, also while the capture
  itself was being written. A capture now carries a value only for the
  writes outside it; inside it, the value is written as it would be
  anywhere else, as the other capture checks already require.

### Decision 32: A value being written structurally is active

- **Context**: `result { const z = try r(1); return match (a) { A(n) => 1,
  B => match (b) { ... } }; }` wrote the inner `match` with an empty
  subject and empty arm values.
- **Decision and rationale**: The inner `match` has a slot, and the source
  walk substitutes that slot for the `match`'s bytes unless the `match` is
  the value being written. The structured return wrote it through the
  continuation entry point without marking it, so its own source was
  taken for its slot. The continuation entry point, which every
  structural write goes through, now marks the value it writes.

### Decision 33: A discarded comma operand is removed even when it is a tt value

- **Context**: `match ((try r(1), match (v) { ... }) as number) { ... }`
  wrote `($tt_v3 $tt_v1) as number`.
- **Decision and rationale**: The comma's left operand runs as a statement
  and is removed with its comma. When that operand is itself a tt value,
  its slot replacement took precedence over the removal and the comma
  went alone. A value inside a discarded operand is no longer substituted;
  the operand is removed whole.

### Decision 34: An inner `try` is planned with the values beside it

- **Context**: `const x = try half(match (k) { ... } + (try half(k)));`
  wrote `half(` twice (`SourceEmittedTwice`); `1 + (try half(match ... +
  (try half(k))))` did the same.
- **Decision and rationale**: The `match` was planned with the
  declaration that holds the outer `try`, as it is anywhere else (its
  arms then complete the call, which keeps each arm's argument typed on
  its own), while the inner `try` was nested under the outer `try` with a
  schedule of its own; both plans captured the callee. An inner `try`
  whose parent is a `try` is now planned with its host like the `match`,
  so one schedule orders the callee, the `match`, and the inner `try`, as
  in `const x = half(match ... + (try half(k)));`. A first attempt nested
  the `match` under the outer `try` instead; it dropped the arms' call
  completion and changed three baselines, so it was not kept.

### Decision 35: A diagnostic on a shared binding names the case that lacks it

- **Context**: `match (v) { A(n) | B(n) => n }` over `variant V { A(n:
  number), B(m: string) }` was reported at the match head as "match on a
  tag pattern needs a value with a `kind` discriminant", in the CLI and
  the editor alike.
- **Decision and rationale**: An or-pattern's bindings share one
  destructuring, written as glue that records every source occurrence of
  the binding. A TypeScript diagnostic on that glue fell back to the
  construct's anchor and was translated as a missing discriminant. The
  shared binding is now a diagnostic origin of its own, as a destructured
  list already is: the diagnostic is placed at the occurrence whose field
  the declaration did not resolve, or at the first occurrence, and keeps
  TypeScript's message, as a single pattern's missing field does.

### Decision 36: A `result` block's expression boundary is its glue

- **Context**: `const { a = result { const z = try await g(); ... } } = p;`
  in a non-async function failed the output self-check with no position.
- **Decision and rationale**: The boundary call written around a `result`
  block in an expression-only owner (`(await $tt_expr(async () => {`)
  carried no anchor, so a parse failure on it mapped nowhere. It is now
  anchored to the block like every other construct's glue, and the
  failure is reported at the block.

## Work log

- 2026-10-06: Started from the third audit's reports. Merged
  `fix/cli-audit` (TASK-764), which this branch's CLI work builds on, and
  resolved the index and reference conflicts.
- 2026-10-06: Fixed the CLI findings (Decisions 1-8), the editor findings
  R3E1, R3E2 (outline), R3E6, and R3E7 (Decisions 9-11), and the compiler
  findings R3C8, R3C9, and R3C2 (Decisions 12-14).
- 2026-10-06: Fixed R3C3, R3C15, and the `super` head regression
  (Decisions 15-17), then R3C1, R3C5, R3C6, and R3C7 (Decisions 18-21).
- 2026-10-06: A full run found three regressions from the work above
  (Issues 2-4); fixed them (Decision 22) and continued with R3C10, R3C4's
  pipeline form, and R3C14 (Decisions 23-27).
- 2026-10-06: Re-ran the 51 round-3 fuzz failures; reduced the remaining
  ones and fixed them (Decisions 28-34). All 51 now compile and run.
- 2026-10-06: Fixed R3E5 and the unlocated boundary failure (Decisions
  35-36); re-ran 490 freshly generated differential fuzz programs: the
  remaining reports were rejections the language documents (a ternary
  branch holding an unparenthesized pipeline, a destructuring default
  holding a `match`), the documented looser binding of `|>` the generator
  did not model, or generator input with `await` in a non-async function.

## Issues and resolutions

### Issue 1: `--types` failed every sidecar on a collision

- **Symptom**: With `a/p.tt` and `b/p.tt`, `--types -o ty a b` wrote
  nothing and blamed the collision on every file, exiting 3.
- **Cause**: The two inputs mirrored onto one output (Decision 1's old
  per-directory roots).
- **Resolution**: With one root the two sidecars no longer collide; a CLI
  test pins that all three are written. The engine serves each canonical
  source once, so no remaining input layout reaches the collision branch.

### Issue 2: Shadowing a `try` operand overflowed the stack

- **Symptom**: `stack::tests::every_nested_tt_construct_compiles_without_the_callers_stack`
  aborted with a stack overflow on 300 nested `try h(...)`.
- **Cause**: Decision 24 projects a statement `try`'s operand through the
  shadow projection, whose recursion did not run on the compiler stack.
- **Resolution**: The shadow projection grows the stack as the main
  projection does.

### Issue 3: A ternary branch closed a concise arrow's block

- **Symptom**: `a_tt_value_anywhere_in_a_concise_arrow_body_keeps_the_block_balanced`
  stopped with `LayoutScopeMissing` for `x ? M : 0`.
- **Cause**: Decision 19 writes a source branch through the source walk,
  which closed the enclosing arrow's block at the branch's end.
- **Resolution**: Decision 22.

### Issue 4: Two tests pinned behaviour this task changed on purpose

- **Symptom**: `a_position_only_diagnostic_keeps_a_zero_end_over_the_protocol`
  and `types_names_each_file_of_a_declaration_collision_once` failed.
- **Cause**: The first used `match (A.X) { }`, which Decision 10 now
  reports as a located malformed match; the second pinned the collision
  of two directory inputs, which Decision 1 removed.
- **Resolution**: The first now uses another position-only diagnostic;
  the second pins that both inputs' sidecars are written apart.

### Issue 5: The first plan for Decision 34 dropped call completion

- **Symptom**: Three baselines lost a call pushed into each `match` arm.
- **Cause**: Nesting the `match` under the outer `try` moved it out of
  the plan that completes the call in each arm.
- **Resolution**: The inner `try` joins the host plan instead (Decision
  34); the three baselines are unchanged.

## Regression test (fails before the fix)

Each case below was run against the code before its fix and failed as
stated (a crash, output that does not parse, a wrong runtime result, or a
missing or misplaced diagnostic).

- **Path**: `tests/cases/compiler/aPatternSubjectWithAPlainTemplateAfterAMatchIsLowered.tt`
- **Observed failure**: internal compiler error "match reached
  expression emission without a host rewrite".
- **Path**: `tests/cases/compiler/anArrowStepWhoseTemplateBodyHoldsAMatchIsLowered.tt`
- **Observed failure**: verify-failed, "unbalanced TypeScript delimiter".
- **Path**: `tests/cases/compiler/aCallbackWithAnIfLetBesideAHoistedArgumentIsLowered.tt`
- **Observed failure**: verify-failed, "Expected '(', got 'let'".
- **Path**: `tests/cases/compiler/aPipelineStepWithAnIfLetCallbackAndAMatchIsLowered.tt`
- **Observed failure**: lowering-plan-failed, "a tt node's source span
  0..0 is invalid".
- **Path**: `tests/cases/compiler/anIfLetSubjectPipingAMatchIsOnePipeline.tt`
- **Observed failure**: stray-pipe at the `|>`.
- **Path**: `tests/cases/compiler/anOuterTryRunsAMatchAndAnInnerTryInOrder.tt`
- **Observed failure**: internal compiler error `SourceEmittedTwice`.
- **Path**: `tests/cases/compiler/anOuterTryRunsItsOperandsInOrderAroundAnInnerTry.tt`
- **Observed failure**: verify-failed, "Expression expected", with the
  inner `try` run before the operand written before it.
- **Path**: `tests/cases/compiler/aValBindingPassedThroughAWrappedCalleeIsReported.tt`
- **Observed failure**: no `val-pass` error for the five wrapped calls.
- **Path**: `tests/cases/compiler/anAwaitInANestedArrowDoesNotMakeAResultBlockAsync.tt`
- **Observed failure**: verify-failed, "await isn't allowed in non-async
  function".
- **Path**: `tests/cases/compiler/aSpreadAppliesToTheWholePipeline.tt`
- **Observed failure**: source-not-typescript, "Parenthesized expression
  cannot contain spread operator".
- **Path**: `tests/cases/compiler/aResultBlockReturningANestedMatchWritesTheInnerMatch.tt`
- **Observed failure**: verify-failed, "Expression expected" (the inner
  subject was written empty).
- **Path**: `tests/cases/compiler/anOrPatternFieldMissingFromOneCaseIsReportedAtThatCase.tt`
- **Observed failure**: the error was reported at the match head as a
  missing `kind` discriminant.
- **Path**: `tests/cases/compiler/anAwaitOutsideAnAsyncFunctionInAResultBoundaryIsLocated.tt`
- **Observed failure**: verify-failed with no position.
- The other cases added under `tests/cases/compiler/` by this task, and the
  CLI, server, and language tests named in the decisions, were checked the
  same way when their fix was made.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`--no-fail-fast`): every target passed except two
  editor baselines whose diagnostics Decision 35 moves from the match head
  to the or-pattern's binding; after reading the diff they were accepted
  and `cargo test --test editor_cases` passes.
- [x] Baseline changes reviewed and committed with the change
- [x] The 51 round-3 fuzz failures compile and run; 490 newly generated
  differential programs report only documented rejections or generator
  errors (see the work log).

## Changed files

- CLI: `src/main/build.rs`, `src/main/output.rs`, `src/main/modes.rs`,
  `src/main/typed.rs`, `src/main/ownership.rs`, `src/server.rs`.
- Parser and lexer: `src/parser/parse.rs`, `src/parser/iflets.rs`,
  `src/parser/matches.rs`, `src/lexer.rs`, `src/lexer/queries.rs`,
  `src/lexer/facts.rs`, `src/lexer/facts/expressions.rs`.
- Analysis: `src/sema/checker.rs`, `src/val.rs`, `src/val/checker.rs`,
  `src/diagnostics.rs`, `src/flow/syntax.rs`.
- Core, evaluation, and projection: `src/core_ir/mod.rs`,
  `src/core_ir/lower.rs`, `src/evaluation_ir/builder.rs`,
  `src/evaluation_ir/evaluation.rs`, `src/evaluation_ir/planning.rs`,
  `src/program_syntax/projection.rs`, `src/program_syntax/visit.rs`.
- Codegen: `src/codegen/core/emitter/host.rs`,
  `src/codegen/core/emitter/source.rs`,
  `src/codegen/core/emitter/expression.rs`,
  `src/codegen/core/emitter/result.rs`,
  `src/codegen/core/emitter/pattern.rs`, `src/codegen/rope.rs`,
  `src/codegen/rope/builder.rs`, `src/lib/mapped.rs`.
- Engine and editor: `src/lib/compile.rs`, `src/lib/api.rs`,
  `src/engine/projection.rs`, `src/engine/language/service.rs`,
  `src/engine/lines.rs`, `src/typescript/mapper.rs`,
  `src/typescript/service.rs`.
- Documentation: `docs/ai/tt.md`, `docs/tasks/TASK-352-*.md`,
  `docs/tasks/TASK-590-*.md` (reversal notes), this record, the index.
- Tests: the cases under `tests/cases/compiler/` and
  `tests/cases/conformance/diagnostics/variant-duplicate-field/` with
  their baselines, `tests/cli.rs`, `tests/compile/`, `tests/passthrough.rs`,
  `tests/native/cases_07.rs`, `tests/matrix/diagnostics.mjs`.

## Result

The third audit's CLI, compiler, and editor findings are fixed in the
layer that owns each of them, except R3E3, which needs one diagnostic
producer for the CLI and the editor and is left for its own task.
