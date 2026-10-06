# TASK-769: Fix defects found by the third audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
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
  pipeline that does not parse (R3E7); `try` in a setter (R3C8).
- Excluded: to be recorded as the task proceeds.

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

## Work log

- 2026-10-06: Started from the third audit's reports. Merged
  `fix/cli-audit` (TASK-764), which this branch's CLI work builds on, and
  resolved the index and reference conflicts.

## Issues and resolutions

### Issue 1: `--types` failed every sidecar on a collision

- **Symptom**: With `a/p.tt` and `b/p.tt`, `--types -o ty a b` wrote
  nothing and blamed the collision on every file, exiting 3.
- **Cause**: The two inputs mirrored onto one output (Decision 1's old
  per-directory roots).
- **Resolution**: With one root the two sidecars no longer collide; a CLI
  test pins that all three are written. The engine serves each canonical
  source once, so no remaining input layout reaches the collision branch.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
