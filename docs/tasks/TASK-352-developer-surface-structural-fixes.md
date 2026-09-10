# TASK-352: Repair the developer-facing surfaces of tt

> TASK-353 supersedes the deferral of issue 19 and corrects the output path
> and staging-file ownership decisions below.

- **Status**: Complete
- **Started**: 2026-09-09
- **Completed**: 2026-09-09
- **Commit**: `TASK-352: fix(compiler): lower a tuple arm over the subjects it has` … `TASK-352: feat(sema): reject a payload field that takes the case tag's property`

## Purpose

Audit what a developer actually runs — the documentation, the `ttc` command
line, the compiler, and the editor integration — and repair each confirmed
defect in the layer that owns the behavior.

## Scope

- Included: user-facing documents and their examples, every CLI mode and its
  edge inputs, compiler diagnostics and emitted output, and the VS Code
  extension with the engine surfaces it drives
- Excluded: release publication, and any change to what the language means

## Decisions

### Decision 1: Audit each surface against the tools, not against the tests

- **Context**: The suite is green, so a defect that survives it is one no
  test describes. Reading the code alone would reproduce its assumptions.
- **Alternatives considered**: Extend the existing suites and see what
  breaks; sample features by hand; drive every documented surface with the
  built compiler and compare what it does against what it promises.
- **Decision and rationale**: Exercise the shipped binary, the extension
  build, and every documented example, and treat only a reproduced
  difference as a finding. Each finding carries its command and output.

### Decision 2: A subject index is the Core IR's invariant, so lowering owns it

- **Context**: A tuple arm naming more positions than the match has
  scrutinees crashed emission — once by indexing past the subject list, once
  by handing the switch emitter an alternative with no constructor test.
- **Alternatives considered**: Add `match-tuple-arity` to
  `blocks_projection` (this suppressed the file's independent type errors,
  which TASK-117 requires to survive); leave the arity-mismatched match as
  source text in HIR (the same suppression, plus a projection that no
  longer parses).
- **Decision and rationale**: `Place::subject` indexes the decision's own
  subjects, so Core IR lowering keeps only the positions that have one. Sema
  still reports the arity, the emitted TypeScript still parses, and the
  file's other diagnostics still reach the user. A one-position conjunction
  collapses to that position's plan so a single-subject decision keeps the
  arm shape the switch emitter is documented to receive. The invariant is
  now asserted where it belongs, in `validate_decision`.

## Work log

- 2026-09-09: Ran `./scripts/doctor`, installed dependencies, built the
  release compiler, and recorded a green `cargo test` baseline.
- 2026-09-09: Audited the four surfaces against the built tools and
  reproduced each reported difference before recording it.
- 2026-09-09: Repaired the two tuple-arity compiler crashes in Core IR
  lowering and pinned the contract in `tests/compile/cases_06.rs`.
- 2026-09-09: Rebased onto `main` at `e3b89e2` and re-verified.
- 2026-09-09: Gave the block a concise arrow body is rewritten to one
  closing brace, written where the body ends, and covered the placement
  matrix in `tests/compile/cases_06.rs`.
- 2026-09-09: Narrowed the declared return type a `return` slot carries to
  the functions whose declaration names that value's type, and checked the
  emitted output of every shape with the repository's TypeScript.
- 2026-09-09: Repaired three command-line defects — a banner on a
  hand-written file, named file inputs losing their depth under `-o`, and a
  temporary directory left behind by every typed run — and covered the
  first two in `tests/cli.rs`.
- 2026-09-09: Gave the typed pass a declaration-based answer for files the
  checker holds none about, which is both the file a caller names outside
  the project's `include` and every file when no TypeScript is installed.
- 2026-09-09: Corrected the user-facing statements the tools contradict —
  two flag descriptions in `ttc --help`, three in the reference the compiler
  embeds, the website's `Option` pipeline, the TypeScript 7.1 requirement in
  the npm README, and the gate table and contract count in `CONTRIBUTING.md`
  — verifying each against the built tools before changing it.
- 2026-09-09: Made every file the compiler publishes appear whole, checked
  against a full filesystem and a rename that cannot succeed.
- 2026-09-09: Gave a hand-written file's type errors their position back,
  checked on a line containing Korean text and through the server protocol.
- 2026-09-09: Audited the editor surfaces and made every position the
  server reports a UTF-16 one, verified by applying an offered fix to a
  line containing an emoji. `./scripts/ci extension` passes with 160 tests.
- 2026-09-09: Stopped a save from being read as an edit made elsewhere, and
  named the rule that decides so it could be tested on its own. The
  extension suite passes with 164 tests.
- 2026-09-09: Let an unsaved `.tt` buffer have the answers the engine reads
  from its text, checked over real LSP; the suite passes with 165 tests.
- 2026-09-09: Gave a block arm body its statement scopes in the grammar,
  checked with the real tokenizer; the suite passes with 166 tests.
- 2026-09-09: Made the compiler a per-folder answer so one folder's setting
  cannot take over the window; the suite passes with 167 tests.
- 2026-09-09: Stopped a JSX closing tag from reading as a regex in `.ttx`;
  the suite passes with 168 tests.
- 2026-09-09: Made two error messages name what actually failed — the entry
  the walk could not read, and the path a server request sent.
- 2026-09-09: Put the editor's hover and completion text into English, the
  language every other user-visible string in the server already uses.
- 2026-09-09: Added the rule that a payload field cannot take the property
  the case tag lives in, with its explanation, wire number and reference
  entry.

### Decision 3: A rewritten arrow body closes where the body ends

- **Context**: The brace that closes a concise arrow body's block was
  written from two places, each of which knew only part of the body.
- **Alternatives considered**: Widen the source walk's boundary alone (the
  early close from the value path remains); suppress the value path
  entirely for arrow owners (it also emits the value, so the output loses
  it).
- **Decision and rationale**: Keep both paths, and let each close only at
  the position that ends the body — the value path when the value *is* the
  body, the source walk when source follows it. A registry on the emitter
  makes the brace exactly one write, so neither path has to know whether
  the other already ran.

### Decision 4: The passthrough contract decides whether a banner is written

- **Context**: A hand-written `.ts` copied into the output tree arrived with
  `// @generated from plain.ts by ttc — do not edit directly.` on top —
  untrue of a file its author wrote, and a byte the passthrough contract
  does not allow. TASK-336 settled where a banner goes and explicitly left
  whether one is written out of its scope, so this is not a reversal of it.
- **Alternatives considered**: Keep the banner and reword it; let
  `--no-banner` be the answer (it is off by default, so the default output
  would still break the contract).
- **Decision and rationale**: `AGENTS.md` allows exactly one change to a
  hand-written file — its relative tt import specifiers. The banner is
  written for the surfaces ttc compiles, which is the same condition the
  source map beside it already applies.

### Decision 5: Named file inputs mirror under the directory they share

- **Context**: `-o` promises to mirror input paths, but a named file was
  written by file name alone, so `src/sub/helper.ts` landed at
  `build/helper.ts` and its rewritten `../shape.js` pointed outside the
  output tree.
- **Alternatives considered**: Mirror every input under one common root
  (this dissolves the two output-collision contracts TASK-321 and TASK-338
  established for directory inputs); keep the file name and rewrite
  specifiers to match (the output would no longer mirror the input).
- **Decision and rationale**: Named files mirror under the deepest
  directory they are all inside. A directory input keeps mirroring under
  itself, so both collision contracts answer exactly as before.

### Decision 6: Deferring to the checker needs a checker that holds the file

- **Context**: `ttc --check-types lib/outside.tt` reported nothing and
  exited 0 while `ttc --check` on the same file reported a coverage hole and
  exited 1. The typed pass defers exhaustiveness to the checker because its
  alphabet is narrower, but the configured program does not contain that
  file, so no question about it is ever asked.
- **Alternatives considered**: Add the named file to the TypeScript program
  (the project's own `include` decides its members, and overriding it makes
  ttc disagree with `tsc` about what the project is); report that the file
  is not part of the project and stop (truthful, but it still leaves the
  file unchecked when `--check` checks it fine).
- **Decision and rationale**: A file the checker holds no answer about is
  in the same position as a file checked with no backend at all, and the
  rule for that is already written down: the typed facts go, the tt layer
  reports in full. Its coverage is answered from the declarations the file
  can see — the same answer `ttc --check` gives — so naming a file never
  passes in silence.

### Decision 7: The protocol's coordinate is converted at the protocol

- **Context**: `docs/design/lsp-architecture.md` §C fixes UTF-16 as the
  editor protocol's coordinate. The compiler measures a column in code
  points, which is what its own rendered caret lines up with, and reports
  declaration spans in bytes. The JSON-lines server passed both through.
- **Alternatives considered**: Make the compiler measure in UTF-16
  everywhere (the CLI's caret then misaligns for anyone whose source has
  astral characters); convert in the extension (every other client would
  have to repeat it, and the design doc puts the conversion in the engine).
- **Decision and rationale**: One helper at the server converts each
  position it emits, and the library exposes the two conversions so no
  surface counts its own way. The compiler keeps code points for the
  terminal; the protocol gets what the protocol means.

## Issues and resolutions

### Issue 1: A tuple arm wider than its match crashed the compiler

- **Symptom**: `match (x) { (A, B) => 1 }` exited 101 with `internal
  compiler error: switch variant alternative tests no constructor`, and
  `match (x, y) { (A, B, C) => 1, _ => 2 }` with `index out of bounds: the
  len is 2 but the index is 2`. Both crashed `--check` and `--emit-map`, so
  an editor keystroke could kill the compiler.
- **Cause**: `Lowering::pattern_at` turned every tuple element into a
  `Place` whose `subject` was the element's index, without consulting the
  decision's subject count. The parser keeps an arity-disagreeing tuple
  match on purpose so sema can name the mismatch, so the two phases
  disagreed about what a position was.
- **Resolution**: Lowering takes only the elements that have a subject and
  collapses a one-position conjunction to that position's plan.
  `validate_decision` now asserts that every place an arm tests names one of
  the decision's own subjects.

### Issue 2: A concise arrow body containing a tt value lost its brace

- **Symptom**: `[1].map(x => x + match (s) { ... })` emitted
  `return $tt_v1 + $tt_v0);` with no closing brace, and
  `[1].map(x => match (s) { ... } + x)` closed the block before `+ x`,
  leaving it outside the arrow. Both failed the output self-check with a
  message that names no position the user can act on.
- **Cause**: The block's closing brace was written either by the
  structured-value path, which fired whenever the value *started* the body,
  or by the source walk, whose boundary excluded a following span that
  begins exactly where the body ends. A body whose value sits at the tail
  matched neither.
- **Resolution**: Each path now closes only at the end of the body, and a
  registry on the emitter keeps the brace to one write.

### Issue 3: A generator's declared type was copied onto its return slot

- **Symptom**: `function* gen(): Generator<number, string, void>` emitted
  `let $tt_v0: Generator<number, string, void>;` and its async counterpart
  `let $tt_v1: Awaited< AsyncGenerator<...>>;` — both rejected by tsc. A
  type predicate emitted `let $tt_v2: x is number;`, which does not parse,
  so the user saw `verify-failed` with no position.
- **Cause**: The host projection captured `return_type` for every function
  and lowering copied it onto the slot the `return` assigns through. That
  is sound only when the declared return type *is* the returned value's
  type; a generator declares the iterator it produces, and a predicate is
  not a type.
- **Resolution**: One predicate in the projection decides what the
  annotation describes, and answers `None` for generators and predicates,
  which leaves the slot to be inferred from the value assigned to it.

### Issue 4: A hand-written file was published as generated

- **Symptom**: `ttc -o build src` wrote `// @generated from helper.ts by
  ttc — do not edit directly.` into the copy of a hand-written `.ts`.
- **Cause**: The banner was written for every job; only the source map
  beside it asked whether the file was one ttc compiles.
- **Resolution**: The banner asks the same question. A passthrough file is
  now byte-identical to its source apart from its tt specifiers.

### Issue 5: Named file inputs lost their depth under `-o`

- **Symptom**: `ttc -o build src/shape.tt src/sub/helper.ts` wrote
  `build/helper.ts`, whose `../shape.js` resolves above `build`.
- **Cause**: A named input's output path was its file name.
- **Resolution**: Named files mirror under the deepest directory they share.

### Issue 6: Every typed run left a temporary directory behind

- **Symptom**: `/tmp` held over a thousand empty `ttc-host-*` directories;
  each `--check-types` or `--types` run added one.
- **Cause**: The session removed the host script it wrote but not the
  directory it created to hold it.
- **Resolution**: The session owns that directory and removes it on drop.

### Issue 7: A named file outside the project passed silently

- **Symptom**: `ttc --check-types lib/outside.tt` printed nothing and exited
  0 on a file whose match is not exhaustive; `ttc --check` on the same file
  reported it and exited 1. With no TypeScript installed, `--check-types`
  printed "only tt-level diagnostics are shown" and then showed none of the
  coverage ones.
- **Cause**: Exhaustiveness on the typed path is derived from the checker's
  answers about each scrutinee. The host asks only about files the
  configured program contains, so a file outside it produced no answer, and
  nothing filled the gap.
- **Resolution**: The report falls back to the declaration-based coverage
  for any file the checker holds no answers about, which covers both the
  excluded file and the missing toolchain.

### Issue 8: Documented behavior the tools do not have

- **Symptom**: `ttc --help` offered `--overlay` and `--tt-only` with
  `--types`, which the CLI rejects by design. The reference the compiler
  serves as `ttc help` said a reserved word makes a construct "silently
  pass through" (it is a located `malformed-variant`, or a self-check
  failure), that `-o` means in-place overwrites are refused (only an output
  landing on its own hand-written input is), and that emitted `.ts` starts
  with `@generated` (a passthrough file does not). The website's `Option`
  pipeline did not type-check, because `Number.isFinite` takes `unknown`
  and infers the element type away. `npm/tt-lang/README.md` named content
  mappers as the only TypeScript 7.1 requirement, though `--types` needs
  the declaration-emit API that arrived with it. `CONTRIBUTING.md` listed
  five of the six gate stages and counted two of the three contracts.
- **Cause**: Drift; each statement was true of an earlier behavior.
- **Resolution**: Every claim was re-run against the built tools and the
  document corrected to what they do. The website's generated highlight
  files were rebuilt from the corrected source.

### Issue 9: A failed write replaced a good output with a prefix

- **Symptom**: With the output filesystem full, `ttc -o <dir> src` reported
  the error and left the previously good `big.ts` truncated to the bytes
  that fit.
- **Cause**: Outputs were written in place. The open truncates, so any
  failure between the truncation and the last byte publishes a prefix —
  while `main` already promises that "every file this run wrote was written
  whole".
- **Resolution**: One helper stages the bytes beside the target and renames
  them onto it, so a reader sees the previous file or the new one. The
  `--types` sidecars go through the same helper, and a staging file that
  never became an output is removed whichever step failed.

### Issue 10: A hand-written file's type errors arrived without a position

- **Symptom**: `ttc --check-types src` rendered `--> src/plain.ts` with no
  line or column and no excerpt, and the server answered the same
  diagnostic with `line: 0, col: 0`, which an editor pins to the top of the
  file.
- **Cause**: The report dropped TypeScript's own coordinates for a file
  nothing was lowered from, though the comment beside the code said they
  were used as they are.
- **Resolution**: They are converted against the file's text — the buffer's
  when one is open, the disk's otherwise. A file that cannot be read keeps
  the path alone rather than a made-up position.

### Issue 11: An offered quick fix deleted the code beside the one it named

- **Symptom**: In a file with an emoji earlier on the line, the
  `match-not-exhaustive` quick fix replaced the arm body instead of
  inserting before the closing brace: `Circle(r) => r` became
  `Circle(r) => , Square(s) => undefined, `. The diagnostic's own underline
  was off by the same amount, and a declaration's outline entry pointed at
  unrelated text because those spans were bytes.
- **Cause**: The compiler counts a column in code points and a declaration
  span in bytes; the editor protocol counts UTF-16 code units. The server
  passed both through unconverted, so each astral character earlier on the
  line moved the reported span by one.
- **Resolution**: The server converts every position it emits, through two
  conversions the library now exposes. The compiler keeps code points for
  its own caret.

### Issue 12: Every save threw away the state the session exists to keep

- **Symptom**: The user's own Ctrl+S arrives as a watched-file change, and
  the handler rebuilt the engine's project graphs and every buffer's
  projection for it — the audit measured a typed check going from 7 ms warm
  back to 267 ms after each one. The same path re-armed standing notices,
  so "ttc compiler not found" popped up again on every save.
- **Cause**: The handler could not tell a save from an edit made outside
  the editor, though the server already holds the saved buffer and its text
  reached the engine as it was typed.
- **Resolution**: A named rule decides what is news. A change to a buffer
  the server holds is not; creation and deletion still are, because those
  change what the project contains whoever holds the file.

### Issue 13: An unsaved buffer lost every tt-specific answer

- **Symptom**: In an untitled `.tt` document the outline was empty, `Shape.`
  offered no cases, and hovering a variant name answered nothing — though
  the engine answers all three for a path that does not exist.
- **Cause**: The surfaces asked for a filesystem path and gave up without
  one, even though what they read is the buffer's text. Semantic tokens
  already named an untitled buffer to get its answer.
- **Resolution**: A named buffer path serves the text-only surfaces the way
  semantic tokens already did. The typed surfaces still require a real file,
  and go-to-definition resolves a declaration found in an unsaved buffer to
  that document rather than to a file of the synthetic name.

### Issue 14: A block arm body was highlighted as an object literal

- **Symptom**: In `match (v) { _ => { const q = 1; return q; } }` the
  grammar scoped the braces as `meta.objectliteral.ts`, so `const` and
  `return` lost their keyword colours and the binding read as an object
  member.
- **Cause**: The arm-body rule included only expressions, so the brace fell
  through to TypeScript's object-literal rule. An object-valued arm has to
  be parenthesized, so a bare brace after `=>` can only open a block.
- **Resolution**: The arm body takes a block alternative that includes
  statements, the way the `result` block rule already does. A parenthesized
  object arm keeps its object scopes.

### Issue 15: One folder's compiler took over the whole window

- **Symptom**: `tt.compilerPath` is declared resource-scoped, so folders in
  one window may name different compilers, but the server kept a single
  answer. Validating a file in one folder repointed every later request —
  for every folder — at that folder's compiler, and validation is scheduled
  for every open document.
- **Cause**: `currentCompiler()` read one window-global set by whichever
  document was validated last.
- **Resolution**: The compiler is recorded per workspace folder and looked
  up by the document being served; the window's default answers for a
  document in no folder. A session is handed only the buffers it serves,
  and a settings or folder change clears the recorded answers.

### Issue 16: A closing tag broke member completion in `.ttx`

- **Symptom**: With a closing tag earlier on the line, typing `t.` in a
  `.ttx` buffer offered `Option`, `Result` and the tt keyword snippets ahead
  of the object's own members — none of which can follow a dot.
- **Cause**: The cursor-context mask treated the slash of `</p>` as the
  start of a regex, because `<` is a position a regex may follow in
  TypeScript. The imagined literal swallowed the rest of the line, so the
  member access at the cursor was not recognised and completion fell
  through to the general branch.
- **Resolution**: The mask is told which surface it is reading. On the JSX
  one, a slash immediately after `<` closes an element; a comparison
  against a regex still reads as a regex on both.

### Issue 17: Two messages named something other than the cause

- **Symptom**: One dangling symlink under a directory input failed the
  whole build with `ttc: src: No such file or directory`, about a directory
  that plainly exists, never naming the link. The server answered a
  `typedCheck` for a missing file with `--overlay <path>: ...`, a command
  line the caller never wrote and a flag the protocol does not have.
- **Cause**: The walk propagated the entry's I/O error without its path,
  and the server borrowed the CLI's wording for a protocol error.
- **Resolution**: The walk names the entry it could not read, and the
  server names the path the request sent.

### Issue 18: The editor's own help text was Korean only

- **Symptom**: Hovering `match` and the six keyword-snippet completions
  described tt in Korean, as did two completion details, while every other
  user-visible string in the server — diagnostics, notifications, output
  channel — is English.
- **Cause**: The strings predate the documentation-language rule in
  `AGENTS.md`.
- **Resolution**: Translated in place, saying the same thing.

### Issue 19: A `return` inside an `if let` inside a block arm — diagnosed, not fixed

- **Symptom**: `docs/ai/tt.md` line 49 says a block arm's direct `return`
  delivers the match value and only "returns inside nested functions remain
  JavaScript returns". An `if let` is control flow, not a nested function,
  but a `return` inside one still leaves the enclosing function:

  ```tt
  const v = match (o) {
    Some(value) => { if let Some(value: v2) = o { return v2; } return value; },
    None => 0,
  };
  return v + 100;
  ```

  With `{ kind: "Some", value: 5 }` this prints 5, not 105. The arm's own
  trailing `return value` lowers correctly to `$tt_v0 = value; break;`; the
  nested one is emitted as a plain `return`.
- **Cause**: A statement-level decision is projected as one placeholder
  (`ProjectionBuilder::emit_statement_decision`), so nothing written inside
  its bodies is visited by the exit collector. The shadow island beside it
  (`emit_shadow_body_island`) exists only to give a *nested decision* its
  overlay, is skipped when the body holds none, and is itself an arrow —
  which the collector reads as a function boundary.
- **Why it is not fixed here**: Making the exit analysis see into a
  statement decision's bodies changes how the owner model projects them,
  which is the lowering's central contract rather than a local repair. It
  wants its own task, with the projection and Evaluation IR contracts
  restated together. A speculative change was written, measured against the
  case, and reverted when it did not address the cause.

### Issue 20: A payload field named `kind` destroyed its own variant

- **Symptom**: `variant Token { Word(kind: string) }` passed every check and
  emitted `{ kind: "Word"; kind: string }` — a property named twice, which
  tsc rejects — and a constructor `({ kind: "Word", kind })` that writes the
  payload over the tag, so the value can no longer say which case it is.
  `ttc --check` exited 0.
- **Cause**: Nothing checked payload names against the property the lowering
  reserves for the tag, and that property was a literal spelled out at each
  emission site rather than a named part of the shape.
- **Resolution**: The tag's property is one named constant the emitter and
  the new rule both read, and sema reports a case that would take it.
  Documented in the reference the compiler serves, with an explanation
  under `ttc explain`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — all six stages pass: `agents rust npm website native
      extension`. The extension suite reports 168 tests, 0 failures, 0
      skips. The only warning is that this checkout has no VS Code package
      built, which `./scripts/setup` produces and no change here affects.

Each repair was also re-run against the case that exposed it: both
tuple-arity crashes, the nine concise-arrow body shapes, generator, async
generator, type-predicate, ordinary, async and arrow return slots (whose
emitted output type-checks under `--strict`), a hand-written file's
passthrough bytes, named file inputs under `-o`, the temporary directories
a typed run leaves, a named file outside the project's `include`, a build
that runs out of disk, a type error's position on a line holding Korean
text, an offered quick fix on a line holding an emoji, an untitled buffer
over real LSP, a `.ttx` member completion after a closing tag, a dangling
symlink, and a payload field named for the case tag.

## Result

Twenty findings from an audit of the documentation, the command line, the
compiler and the editor. Eighteen are repaired here, each in the layer that
owns the behavior and each pinned by a test at the closest public boundary.

The compiler stops crashing on a tuple arm wider than its match, closes the
block a concise arrow body is rewritten to, and annotates a return slot only
from a type that describes the returned value. Sema gained one rule: a
payload field may not take the property the case tag lives in.

The command line publishes an output whole or not at all, leaves a
hand-written file's bytes alone, mirrors named file inputs, cleans up after
a typed run, and names what actually failed. The engine answers for files
the checker does not hold, so naming a file no longer passes in silence,
and keeps a hand-written file's type-error positions.

The editor reports every position in the units an editor counts — which is
what made an offered quick fix delete the code beside the one it named —
serves each folder with the compiler it configures, answers for a buffer
that has no file yet, keeps its project state across a save, reads a JSX
closing tag as a tag, and scopes a block arm body as the statements it
holds.

Issue 19 is diagnosed and recorded rather than repaired: a `return` inside
an `if let` inside a match block arm still leaves the enclosing function,
because a statement-level decision is projected as one placeholder and
nothing inside it reaches the exit collector. Fixing it changes how the
owner model projects those bodies, which wants its own task.
