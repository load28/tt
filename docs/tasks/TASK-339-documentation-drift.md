# TASK-339: Correct user-facing documentation that no longer matches the tools

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-339: correct documentation that drifted from the tools`

## Purpose

Several user-facing descriptions name diagnostics, formats, flags and setup
steps the tools do not actually produce, so following them leads nowhere.

## Scope

- Included: `docs/ai/tt.md` (which the compiler embeds and serves as
  `ttc help errors`), `docs/why-tt.md`, `docs/getting-started.md`, the
  `ttc --help` flag list, the watch summary line, and the `tt.sidecar`
  setting description.
- Excluded: Anything that would change what the tools do, other than the
  watch line's own wording.

## Decisions

### Decision 1: Name the codes the compiler answers to

- **Context**: `docs/ai/tt.md` listed `else-block-must-diverge` and
  `try-position-restriction`, and opened the section with
  `ttc: file:line:col: msg`. Neither code exists — `ttc explain` rejects
  both — and that message shape is now used only for usage errors.
- **Alternatives considered**: Add the missing codes to the compiler (they
  duplicate `let-else-not-diverging` and `try-placement`).
- **Decision and rationale**: Correct the document, and describe the
  rendered form the compiler actually prints. Every code named in the file
  was cross-checked against `ttc explain`, which also corrected
  `match-or-binding-mismatch` and `val-pass`.

### Decision 2: A count of rebuilt files is not a count of failures

- **Context**: Watch mode printed `ttc: 2 file(s) failed — watching` after
  rebuilding two files of which one failed.
- **Decision and rationale**: The number is what was rebuilt, so the words
  after it say how the round went rather than borrowing the count.

## Work log

- 2026-09-05: Verified each claim against the tools before changing it: the
  two absent codes through `ttc explain`, the diagnostic layout and the
  `why-tt.md` help line by compiling that document's own example, the
  `create-tt` steps by running `init --bundler none`, the two undocumented
  flags against `src/main/command.rs`, and the sidecar opt-in against
  `sidecar.ts`, which runs `--types`.

## Issues and resolutions

### Issue 1: Two named diagnostic codes do not exist

- **Symptom**: `ttc explain else-block-must-diverge` and
  `ttc explain try-position-restriction` both report an unknown code.
- **Cause**: Drift; the rules are `let-else-not-diverging` and
  `try-placement`.
- **Resolution**: Decision 1.

### Issue 2: The stated diagnostic format is not the one printed

- **Symptom**: `docs/ai/tt.md` opened with `ttc: file:line:col: msg`, and
  `docs/why-tt.md` showed a suggested arm on a continuation line.
- **Cause**: Both predate the rendered diagnostics.
- **Resolution**: Both now describe what the compiler prints.

### Issue 3: `create-tt init` is described inaccurately

- **Symptom**: The step list says init adds `@openload28/unplugin-tt`
  (it does not with `--bundler none`, which the same section documents) and
  "configures the TypeScript content mapper", without saying it writes a
  second config, `tsconfig.tt.json`, which is where a reader would look.
- **Resolution**: Both corrected.

### Issue 4: Two implemented flags were absent from `--help`

- **Symptom**: `--overlay` and `--tt-only` are accepted and have their own
  usage errors, but `ttc --help` did not list them.
- **Resolution**: Both listed.

### Issue 5: The watch summary reported the batch size as failures

- **Symptom**: `ttc: 2 file(s) failed — watching` after one failure.
- **Resolution**: Decision 2.

### Issue 6: The sidecar setting names a flag that does not do it

- **Symptom**: `tt.sidecar` told users to opt in with `ttc --sidecar`, while
  the extension runs `ttc --types`, which is also the flag that writes the
  sidecars.
- **Resolution**: The description names `--types`.

## Verification

- [x] Every code-shaped token in `docs/ai/tt.md` cross-checked against
  `ttc explain`; the only remaining one is `let-else`, a construct name
- [x] `ttc --help` lists `--overlay` and `--tt-only`
- [x] `editors/vscode/package.json` still parses
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `docs/ai/tt.md`, `docs/why-tt.md`, `docs/getting-started.md`,
`src/main.rs`, `src/main/output.rs`, `editors/vscode/package.json`, and this
record.
