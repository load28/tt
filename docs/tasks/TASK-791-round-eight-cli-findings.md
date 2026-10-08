# TASK-791: Fix the round-eight CLI findings

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-791: Fix the round-eight CLI findings`

## Purpose

The eighth audit found ten CLI findings (L1–L10). This task checks each one
against TypeScript 7.1 (`7.1.0-dev.20260826.1`, the version the repository
pins) and the npm registry. It fixes the ones that reproduce and records
why the others need no change.

## Scope

- Included: `tsconfig.json` `jsx` reading, the verify-failed advice, the
  install advice, `--check` claim conflicts, and two refusal messages.
- Excluded: the round-eight editor findings (a later task).

## Decisions

### Decision 1: Read `jsx` as TypeScript reads it (L1, L10)

- **Context**: `"jsx": null` in a configuration that `extends` another
  kept the base's `preserve`, so `.ttx` imports were named `.jsx`. A value
  TypeScript rejects (`"bogus"`, `5`) was ignored instead.
- **Evidence**: `tsc --showConfig` (7.1) drops `jsx` when the extending
  configuration sets it to `null`, also through an `extends` array. It
  reports TS6046 for `"bogus"` and TS5024 for `5`.
- **Decision and rationale**: `jsx_option` distinguishes an option that is
  unset, reset by `null`, or set. A later base or the configuration itself
  overrides an earlier one, as with any option. A value TypeScript rejects
  is an error with TypeScript's message. The CLI reports it when the value
  decides an output name (a `.ttx` import under `--rewrite-imports js`).

### Decision 2: A solution-style configuration names no file's options (L2)

- **Context**: With a root `tsconfig.json` of `"files": []` and
  `references`, the build read `jsx` from the root, which has none. The
  referenced project that contains the file was ignored.
- **Evidence**: The TypeScript 3.9 release notes ("Support for 'Solution
  Style' tsconfig.json Files") say such a file belongs to one of the
  referenced projects. `--check-types` already gets that project from the
  compiler (`getDefaultProjectForFile`) and reported the errors in the
  reproduction.
- **Alternatives considered**: Re-implementing TypeScript's
  `include`/`exclude` matching for `.tt` files would guess at what the
  content mapper and the compiler decide.
- **Decision and rationale**: The build has no compiler to ask. When the
  owning project decides an output name, it reports that the configuration
  is solution-style, and the existing message asks for `--project`.

### Decision 3: The `--no-verify` advice is a suggestion where it applies (L3)

- **Context**: The verify-failed message ended with "use --no-verify to
  bypass", including in `--check-types`, `--types`, and the editor, which
  have no such option.
- **Decision and rationale**: The message says only what is wrong, as the
  diagnostic contract requires. The build-mode renderer, which serves the
  build, `--check`, and `--print` (the modes that accept `--no-verify`),
  adds the advice as a `help:` suggestion.

### Decision 4: Name the TypeScript that can be installed (L7)

- **Context**: The install advice said `npm i -D typescript@7.1`. The npm
  registry has no 7.1 release: `npm view typescript@7.1` returns 404, and the
  dist-tags are `latest: 7.0.2`, `next: 7.1.0-dev.20261007.1`.
- **Decision and rationale**: The advice names the version `package.json`
  pins, from one macro (`pinned_typescript!`). A test holds the macro and
  `package.json` to the same version.

### Decision 5: `--check` reports the build's claim conflicts (L8)

- **Context**: The build rejected `x.tt` beside `x.ts`, because both claim
  the output `x.ts`. `--check` on the same inputs passed.
- **Decision and rationale**: A conflict is a property of the input set,
  not of writing, so `--check` (including its watch) reports it too.
  `--check-types` checks the source tree as the editor sees it, where
  `x.tt` and `x.ts` are separate modules, so it is unchanged.

### Decision 6: Refusal messages name the real situation (L9)

- **Context**: "output would overwrite the input — pass -o <dir>" was
  printed even when `-o` was given. A directory at an output path was
  reported as an output that "has been edited".
- **Decision and rationale**: Both refusals now say "write the outputs to
  another directory with -o <dir>". A directory at the output path is named
  as a directory.

### Decision 7: Findings that need no change (L4, L5, L6)

- L4: The reproduction had a `tsconfig.json` above the directory. Without
  one, `--dependencies` lists every walked directory, including ones with
  no direct sources (checked in a fresh directory).
- L5: `x.tt` and `x.ttx` write different outputs (`x.ts`, `x.tsx`). That
  both emit `x.js` is TypeScript's own project error (TS5056), which
  TypeScript reports.
- L6: `foo.d.tt` → `foo.d.ts` is a declaration file by TypeScript's naming,
  which suits a file of ambient declarations (`declare variant`). TypeScript
  reports non-ambient code there.

## Work log

- 2026-10-08: Reproduced L1–L10 and checked each against `tsc` 7.1 and the
  npm registry.
- 2026-10-08: Changed `src/engine/config.rs`, `src/verify.rs`,
  `src/main/build.rs`, `src/main/output.rs`, `src/main/ownership.rs`,
  `src/typescript/toolchain.rs`, `src/typescript/native.rs`; added tests in
  `src/engine/config.rs`, `src/typescript/toolchain.rs`, and `tests/cli.rs`;
  regenerated the case baselines whose verify-failed message changed.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/engine/config.rs::tests::a_null_jsx_unsets_the_value_a_configuration_extends`
- **Observed failure**: `jsx_preserve` returned `Ok(true)` for `"jsx": null`.
- **Path**: `src/engine/config.rs::tests::a_jsx_value_typescript_rejects_is_an_error`
- **Observed failure**: `unwrap_err` on `Ok(false)`.
- **Path**: `src/engine/config.rs::tests::a_solution_style_configuration_names_no_jsx_option_for_a_file`
- **Observed failure**: `unwrap_err` on `Ok(false)`.
- **Path**: `src/typescript/toolchain.rs::tests::the_install_advice_names_the_typescript_the_repository_pins`
- **Observed failure**: with the version written out in the test, the advice did not contain `npm i -D typescript@7.1.0-dev.20260826.1` (it named `typescript@7.1`).
- **Path**: `tests/cli.rs::check_reports_a_hand_written_twin_as_the_build_does`
- **Observed failure**: `--check` exited 0.
- **Path**: `tests/cli.rs::a_directory_at_an_output_path_is_named_as_a_directory`
- **Observed failure**: the message said the output "has been edited".
- **Path**: `tests/cli.rs::verify_failed_offers_no_verify_only_where_it_is_accepted`
- **Observed failure**: the message line itself carried `--no-verify`, and no `help:` line did.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`

## Result

L1, L2, L3, L7, L8, L9, and L10 are fixed. L4, L5, and L6 need no change,
for the reasons in Decision 7.
