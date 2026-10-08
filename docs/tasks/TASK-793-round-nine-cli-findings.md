# TASK-793: Fix the round-nine CLI findings

> **Superseded in part by TASK-796**: Decision 7 (L4) was decided there: the configuration keeps deciding, and the typed modes name each input file it leaves out.

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-793: Fix the round-nine CLI findings`

## Purpose

The ninth audit found ten CLI findings (L1–L10) on `c48ad20e`. This task
fixes the ones that contradict the documented behaviour. It leaves one
(L4) for a decision, because fixing it changes what a directory input
means to the typed check.

## Scope

- Included: `--check` against the build's refusals, declaration
  ownership in `--types` and `--sidecar`, the server's `typedCheck` on a
  TypeScript buffer, the TypeScript version message, `jsx` in `--check`,
  an explicit `--node`, and diagnostic order.
- Excluded: L4 (Decision 7).

## Decisions

### Decision 1: `--check` runs the build's pre-write refusals (L1, L2)

- **Context**: The build refuses an input that is, or claims the output
  of, a compiler support module, an edited output, and an input path that
  cannot be recorded (not UTF-8). `--check` passed all three. TASK-791
  Decision 5 made `--check` report the build's claim conflicts, but these
  are checked in other places.
- **Decision and rationale**:
  - `--check` runs the same owner checks the build runs before writing,
    which also cover the non-UTF-8 path.
  - It keeps each job's emitted outcome, so the support modules the
    outputs import are known.
  - It checks the support-module claims (`support_forms`,
    `support_claim_conflict`, shared with the write path).
  - Only `--print` writes code to stdout.

### Decision 2: Declarations are written as owned outputs (L3, L8)

- **Context**: `--types` and `--sidecar` replaced hand-written or edited
  declaration files, including the standard library's, without a word.
  They also wrote the file before its record, so a failed record write
  left a replaced file reported as not written.
- **Decision and rationale**: Both write through `write_owned_output`,
  as the build does: ownership is checked, the record is written first,
  then the file. The standard library's declarations get their own owner
  kind (`SupportDeclaration`), whose identity is the one their records
  already carried.

### Decision 3: Messages carry one `ttc:` prefix (L8)

- **Context**: The ownership and output helpers returned messages that
  began with `ttc: `. `--types` wrapped them in its own line, which
  printed `ttc: cannot write X: ttc: Y: …`.
- **Decision and rationale**: The helpers return `path: reason`, and each
  place that prints one adds the prefix once.

### Decision 4: The server's `typedCheck` lowers only tt buffers (L5)

- **Context**: A `.ts` buffer was added to the tt candidate list, so it
  entered the program twice (TS2451 against itself). The CLI's
  `--overlay` adds only tt sources.
- **Decision and rationale**: The server applies the same rule.

### Decision 5: Name an installed TypeScript that is too old (L6)

- **Context**: A TypeScript without the 7.1 API file (any 5.x or 7.0
  release) was reported as "no TypeScript compiler found".
- **Decision and rationale**: The installed package's version is read
  first, so an older line gets the existing "TypeScript X is installed,
  but ttc needs the 7.1 line" message.

### Decision 6: Advice that the mode accepts (L7, L9, L10)

- L7: `--check` writes nothing, so no output name depends on `jsx`; it
  no longer reads it. The advice moved out of `project_jsx_preserve`.
  The CLI names `--project` and `--rewrite-imports`, and the server's
  `print` names its `rewriteImports` parameter.
- L9: When `--node` names a binary that cannot run, the TypeScript host
  is unavailable. The build now reports this as an error instead of
  silently writing unrefined output. Without `--node`, a missing
  toolchain still leaves the output unrefined, as documented.
- L10: TypeScript orders diagnostics at one position by code, after
  file, start and length. The typed paths compare codes numerically
  before the message: a TypeScript code by its number, and a tt rule by
  the number `tsc` prints for it through the content mapper (`tt27`). A
  tt rule's number is below every TypeScript code, so at one position the
  tt diagnostic comes first, as `tsc` prints them. Six case baselines
  changed order for this; each change is two diagnostics at one position.

### Decision 7: L4 waits for a decision

- **Context**: `--check-types <dir>` type-checks only the files the
  tsconfig `include` covers. A `.tt` file under the directory that
  `include` leaves out is compiled by the build, but not type-checked.
- **Alternatives considered**: Making every file under a directory input
  a root by request would also type-check files a project excludes on
  purpose (fixtures, generated code) under `--check-types .`.
- **Decision and rationale**: The choice changes what a directory input
  means to the typed check, so it is left to the maintainer.

## Work log

- 2026-10-08: Reproduced L1–L10 with the audit binary (`c48ad20e`).
- 2026-10-08: Changed `src/main/{build,command,output,ownership,modes,typed}.rs`,
  `src/server.rs`, `src/typescript/{toolchain,contextual}.rs`, and
  `src/engine/semantics/translate.rs`. Added tests in `tests/cli.rs` and
  `src/typescript/toolchain.rs`.
- 2026-10-08: Regenerated the case baselines and reviewed the six
  reordered `.errors.txt` files.

## Issues and resolutions

### Issue 1: The first ordering put tt diagnostics by message

- **Symptom**: With only TypeScript codes compared, a tt diagnostic and a
  TypeScript one at one position compared as `None` against a number.
- **Cause**: The sort key had no number for a tt rule.
- **Resolution**: The key uses the tt rule's own number, the one `tsc`
  prints through the content mapper.

## Regression test (fails before the fix)

- **Path**: `tests/cli.rs::check_reports_an_input_that_claims_a_support_module`
- **Observed failure**: `--check src` exited 0 on `c48ad20e`.
- **Path**: `tests/cli.rs::check_refuses_an_edited_output_as_the_build_does`
- **Observed failure**: `--check src` exited 0 while the build refused.
- **Path**: `tests/cli.rs::check_does_not_read_the_jsx_option_it_names_no_output_with`
- **Observed failure**: exit 1, "cannot read the project's `jsx` option".
- **Path**: `tests/cli.rs::types_refuses_declarations_it_does_not_own`
- **Observed failure**: exit 0, with `mine` replaced.
- **Path**: `tests/cli.rs::a_server_typed_check_of_a_typescript_buffer_counts_it_once`
- **Observed failure**: TS2451 "Cannot redeclare block-scoped variable 'qq'".
- **Path**: `tests/cli.rs::a_named_node_that_cannot_run_is_reported_by_the_build`
- **Observed failure**: exit 0 with unrefined output.
- **Path**: `tests/cli.rs::diagnostics_at_one_position_follow_tsc_order`
- **Observed failure**: `ts6133` printed before `ts2322`.
- **Path**: `src/typescript/toolchain.rs::tests::a_typescript_without_the_api_is_named_by_its_version`
- **Observed failure**: "no TypeScript compiler found".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change

## Result

L1, L2, L3, L5, L6, L7, L8, L9, and L10 are fixed. L4 waits for a decision
(Decision 7).
