# Editor validation scheduling

Task: [TASK-763](../tasks/TASK-763-editor-validation-scheduling.md).

## Problem

Every keystroke in a `.tt`/`.ttx` buffer revalidates every open tt document
after a 300 ms pause. For each document the adapter asks the engine for the
text layer (`check`), the typed layer (`typedCheck`), the service layer
(`tsDiagnostics`) and hints (`ttHints`), one request after another on one
ordered pipe. Measured on a 30-module chain project (300 lines each, three
documents open, release build):

| Work per edit | Before |
|---|---|
| Contextual storage materialization over the whole project | 1.7-1.8 s |
| Whole-program semantic diagnostics, for each open document | 0.8 s (first), 60-160 ms (rest) |
| Requests of superseded document versions | computed, then dropped |
| Settle time after the last of ten keystrokes 350 ms apart | 5.9-7.4 s |

The typed check asked TypeScript for the diagnostics of the whole program and
the adapter kept those of one file. It also inherited the batch compiler's
stop rule: a syntax error in any other file suppressed this file's semantic
diagnostics, so an unrelated broken file hid type errors in the open one.

## TypeScript's structure

The reference is TypeScript's own language server (`src/server/session.ts`
and `src/compiler/builderState.ts`, v5.9.3):

- **Per-file checks.** `updateErrorCheck` checks one file at a time:
  `syntacticCheck(file)`, then `semanticCheck(file)` with
  `getSemanticDiagnostics(file)`. Another file's syntax does not decide this
  file's semantic diagnostics.
- **Supersession.** Every document change increments `changeSeq`; a pending
  error check whose sequence is stale stops before its next step.
- **Dependency closure.** `BuilderState.getReferencedFiles` names the files a
  file's types can depend on: the declarations of the symbols its module
  references resolve to (`getReferencedFilesFromImportLiteral`), its
  triple-slash references and module augmentations; a file that affects the
  global scope (`isFileAffectingGlobalScope`) is a dependency of every file.

## Design

### 1. Superseded requests are not computed

The adapter marks the requests of one validation generation
(`typedCheck`, `tsDiagnostics`) `"supersedable": true`. The server reads
request lines on a reader thread, so before computing a marked request it can
see the lines already queued behind it. When a document change
(`openDocument`, `updateDocument`, `closeDocument`, `reloadProjects`) is
among them, it answers `{ "id", "superseded": true }` without computing. The
adapter treats that answer as stale; every change schedules a new generation,
and an opened document's generation is rescheduled explicitly. Unmarked
requests keep their exact semantics, so other protocol consumers see no
change.

### 2. The typed check is a per-file check

`typedCheck` with `"scope": "file"` (the adapter's validation) runs
`Project::check_file`: the backend answers the syntactic, semantic and
declaration diagnostics of that module only, as a language service checks
the file an editor shows, and the tt report keeps that file's diagnostics.
Command-line checks and builds keep the whole-program check.

This is an intended behavior change: a syntax error in another file no
longer hides this file's type errors in the editor. The command line still
follows `tsc`.

### 3. Materialization follows the reference closure

Contextual storage materialization (`contextual-type-materialization.md`)
asks the checker for every generated slot of every lowered module, in
synchronized rounds. A module's settled annotations depend only on the
modules its types can depend on, so a request about one file
(`Project::update_scoped`) materializes only that file's closure:

1. The backend answers the closure on the program as it will be served
   (each module's unchanged materialization, else its lowered text), by
   TypeScript's
   referenced-file rules, plus the files that affect the global scope
   (non-modules, declaration files included, and conservatively every file
   with a module augmentation). Type reference directives are not followed:
   the native API does not expose their resolution, and a local target that
   declares globals is a root as a file affecting the global scope.
   The closure is kept while those texts, the sources, the roots and the
   disk generation are unchanged, so repeated questions about one file ask
   nothing.
2. Each module keeps its last materialization, scoped or whole-project,
   while it is current: the backend's disk generation is the same, every
   served file it read (a lowered module's text, a support module's text)
   is still served with the same text, and the module's closure now reads
   no served file it did not. A file no longer served, such as a closed
   overlay, is read from disk now, so a materialization that read it served
   is stale. A file that comes to affect the global scope joins every
   closure, so a closure that grew is stale; one that leaves it changed the
   text it was read with. The modules materialized together share one
   record of what they read, and a request checks each record once.
3. Otherwise the closure is materialized from its lowered text, exactly as
   the whole project would materialize those modules: their rounds read only
   the closure, and a round in which a closure module learns nothing leaves
   it unchanged.

A disk change during the rounds makes the request fall back to the whole
project. The snapshot's other modules keep a materialization whose served
inputs are unchanged, or their lowered text; no question about the target
reads them, and their own requests settle them. Whole-project requests
(command line, references, rename) materialize the whole project as before,
reusing nothing scoped, and record their results for later scoped requests.

## Verification

`typescript::native::contextual_tests::
a_reference_closure_follows_imports_and_global_declarations` pins the
closure. `engine::scoped_tests` compares, for the multi-file cases and a synthetic
chain of modules whose exported types flow through settled storage, every
target file's emission from a cold and a warm scoped update, before and
after an edit, with the whole-project materialization. The LSP benchmark
(`lsp.mjs`) measures settle time with the real adapter.
