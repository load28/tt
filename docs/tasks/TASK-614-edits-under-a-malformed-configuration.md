# TASK-614: Check edits made while the configuration was malformed

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-614`

## Purpose

`ttc --check-types -w src` kept checking an old copy of a `.tt` module that
was edited while `tsconfig.json` could not be parsed: break the
configuration, add a type error to `src/a.tt`, restore the configuration,
and the watch reported `0 reported` until `a.tt` was edited again. The same
happened when the fix and the edit landed in one pass. A one-shot run
reports the error, so the watch must too.

## Scope

- Included: the host's served arrangement (`src/typescript/host.mjs`), the
  design note (`docs/design/tsgo-native-backend.md`), and a regression test
  (`tests/workflow_repairs.rs`).
- Excluded: the engine's `Project::update` and the watch loop in
  `src/main/typed.rs`, which were suspected but deliver the edit correctly
  (Issue 1).

## Decisions

### Decision 1: The served arrangement includes whether the configuration carries the mapper

- **Context**: A configured project is opened through an identity content
  mapper that the host injects into the configuration it serves (TASK-472,
  `docs/design/tsgo-native-backend.md`). The injection happens only for a
  configuration the host can read: a malformed `tsconfig.json` is served as
  written, so during those passes `.tt` is not a mapped extension. The host
  reported the edited `src/a.tt` as changed in that pass, and again as part
  of the file map afterwards, yet once the mapper returned, the program held
  the text TypeScript had mapped before the configuration broke (observed
  through `program.getSourceFile`, Issue 1). TypeScript's own watch mode
  re-reads the configuration and builds the program from its result
  (TypeScript handbook, "Configuring Watch"; tsconfig reference), so a
  fresh run's answer is the expected one.
- **Alternatives considered**: (a) Re-announce every served module as
  changed whenever a configuration changes: the "together" case already
  sent the edit and the configuration change in one `updateSnapshot` and
  still came back stale, so a second notification in the same batch does
  not reach the mapped content. (b) Split the update into two
  `updateSnapshot` calls (configuration first, modules second): depends on
  an ordering inside the compiler that nothing documents. (c) Treat the
  mapper's presence in the served configuration as part of the arrangement
  and open the project on a fresh compiler when it changes, exactly as
  `reconnect()` already does when the mapped/unmapped choice flips.
- **Decision and rationale**: (c). Whether `.tt` content passes through the
  mapper is a property of the whole session, not of one file, and the host
  already models a change of that property as "a fresh compiler, the
  project not yet open". The served arrangement is now `mapped` and the
  served root configuration carrying the mapper (`configFiles` holds the
  rewritten root only when it could be read); the host reconnects when that
  pair changes after the project was opened. The cost is one cold open per
  transition, which a watch pays only when the configuration breaks or is
  repaired.

## Work log

- 2026-09-30: Reproduced `target/probe6-cli/p16` with a script in both
  orders (edit in a later pass; edit and fix together): `0 reported`.
- 2026-09-30: Instrumented the host to print each `updateSnapshot`'s
  parameters, its file map, and `program.getSourceFile(...).text`: the
  edit was in `fileChanges.changed` and in the served file map, but the
  program returned `export const a = 1;` after the repair. A plain `.ts`
  file edited the same way was re-read correctly, which isolated the mapped
  content.
- 2026-09-30: Added the `carried` arrangement flag to `host.mjs`. Both
  orders report the TS2322; an extended configuration that breaks (the root
  still readable and carrying the mapper) was already correct and stays so.
- 2026-09-30: Added
  `typed_watch_checks_edits_made_while_the_configuration_was_malformed`
  (`tests/workflow_repairs.rs`); it fails without the host change.

## Issues and resolutions

### Issue 1: The suspected layer delivered the edit correctly

- **Symptom**: The report pointed at `Project::update` or the watch loop
  consuming the edit during the failed configuration load.
- **Cause**: The engine re-projects the file from its text each pass, and
  the host's `serve()` reported it as changed; the stale text was the
  compiler's mapped content for a module that was edited while no mapper
  was configured.
- **Resolution**: Fixed at the host's arrangement seam instead (Decision
  1); the engine is unchanged.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test workflow_repairs typed_watch`
  (3 passed; the new test fails without the fix)
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `src/typescript/host.mjs`, `docs/design/tsgo-native-backend.md`,
`tests/workflow_repairs.rs`, `docs/tasks/INDEX.md`, and this record.
