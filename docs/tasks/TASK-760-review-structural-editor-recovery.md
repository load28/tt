# TASK-760: Review and correct structural editor recovery

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: `TASK-760: fix(editor): preserve recovery guards and member boundaries`

## Purpose

Review PR #140 and correct recovery regressions at their owning compiler layers.

## Scope

- Included: lexical parser protections, grammatical member recovery boundaries,
  projection regressions, editor cases and repository validation.
- Excluded: changes to complete-program semantics or toolchain versions.

## Decisions

### Decision 1: Preserve lexical protections independently of delimiter recovery

- **Context**: Editor parsing bypasses the combined host syntax check.
- **Alternatives considered**: Patch individual SWC panic sites, or preserve the
  existing lexical protection contract while allowing delimiter recovery.
- **Decision and rationale**: Separate lexical protection from delimiter
  validation in the shared lexer layer, preserving all existing protections.

### Decision 2: Recognize members in their grammatical list context

- **Context**: Object member recovery is also used for enum and type lists.
- **Alternatives considered**: Expand the shared object punctuation allowlist,
  or use distinct enum and type member rules.
- **Decision and rationale**: Use context-specific rules so valid enum
  initializers and implicitly typed properties do not close their containers.

## Work log

- 2026-10-05: Read AGENTS.md, inspected PR #140 at d38a90cc and its prior review.
  Ran doctor successfully using the installed pinned toolchain on PATH.
- 2026-10-05: Added projection API and parser regressions, ran them against
  unchanged PR production code, and recorded the panic and member-boundary
  failures below. Separated lexical guards from delimiter recovery and corrected
  enum/type member continuation recognition. Broader JSX variants required a
  grammar-level rejection of namespace member access.
- 2026-10-05: Added compiler cases, editor cases with TypeScript twins, and a real
  content-mapper regression. Reviewed generated baselines: keyword members retain
  both independent errors and editor answers, matching their TypeScript twins.
- 2026-10-05: A second review pass deleted each character and truncated at each
  delimiter in 20 source templates (calls, arrays, objects, enums, type members,
  functions, namespaces, classes, switches, regexes, templates, JSX, and tt
  constructs), with independent tt statements before and after the damaged text.
  The 1,003 inputs exposed ten computed-enum panics. Fixed their token expectation,
  then used a failing owner test and baseline review to also prevent consumption
  of the following declaration. Updated baselines now report `const later: number`
  and retain the original syntax error.
- 2026-10-05: Rebuilt the exploratory projection probe against the current Cargo
  library with `defer_to_checker: true` (the editor path). All 1,003 inputs then
  returned without panic or timeout (three seconds per input); six additional
  malformed computed-enum literal/escape inputs also returned normally. Restarted
  `./scripts/ci` after the final production change so all gates test the final code.
- 2026-10-05: Final `./scripts/ci` passed all six stages: agents, rust, npm,
  website, native, and extension. This includes all 420 library tests, 192
  compiler API tests, 25 content-mapper tests, 14 parser recovery tests, the
  compiler/editor baselines and incremental suites, and 241 extension tests with
  zero failures or skips. TypeScript, extension, and corpus requirements were
  enabled. Full log: `/tmp/pr140-ci-final.log`. No further defect was found in
  the final review and tested input set.

## Issues and resolutions

### Issue 1: Editor recovery bypassed lexical parser protections

- **Symptom**: A namespaced JSX member and a conflict marker followed by a
  malformed regexp panicked in projection instead of reporting source errors.
- **Cause**: Editor mode bypassed all of `host_syntax_error`, including checks
  protecting SWC from lexical inputs it cannot safely consume.
- **Resolution**: Separate lexical protection from delimiter validation and
  run lexical protection in both strict and editor host parsing.

### Issue 2: Valid enum and type members acquired inserted closing braces

- **Symptom**: Keyword enum initializers and unannotated type members caused
  recovery records in otherwise complete declarations.
- **Cause**: Type and enum lists reused object-literal recovery continuations.
- **Resolution**: Recognize enum initializers and type-member terminators,
  including automatic semicolon insertion, in their own list context.

### Issue 3: Complete namespaced JSX member tags still panicked

- **Symptom**: Extending the panic regression to `<G:U.m />` and a nested
  tag still reached `JSXNamespacedName -> JSXObject` after restoring guards.
- **Cause**: Complete JSX is lexically opaque to the existing guard. The JSX
  grammar assumed every name followed by a dot was a valid member object.
- **Resolution**: Reject member access on a JSX namespace name in the JSX
  grammar before conversion, in strict and editor parsing alike.

### Issue 4: An incomplete computed enum name asserted its closing bracket

- **Symptom**: The second review pass found ten computed enum variants that
  panicked with `assertion failed: expected ], got ...`.
- **Cause**: After parsing the computed expression, the enum parser asserted
  that `]` must exist, although editor expression recovery can return without it.
- **Resolution**: Use the parser's checked token expectation, which reports
  strict errors and records editor delimiter recovery, instead of asserting.
  A subsequent baseline review showed that enum member initialization still
  consumed the following declaration. Its recovery now respects the enum list's
  terminator, allowing the enclosing statement list to own that declaration.

## Regression test (fails before the fix)

- **Path**: `tests/compile.rs`,
  `editor_projection_rejects_namespaced_jsx_members_without_panicking` and
  `editor_projection_rejects_conflict_markers_without_panicking`.
- **Observed failure**: Both failed on unmodified PR code: `parser/jsx.rs:130`
  panicked with `JSXNamespacedName -> JSXObject`; `lexer/mod.rs:1869` failed an
  assertion comparing `Some(61)` with `Some(47)`. After restoring lexical
  protections, the closed and nested JSX variants still panicked, proving
  the additional JSX grammar change was necessary.
- **Path**: `tests/swc_editor_recovery.rs`,
  `keyword_enum_members_are_not_recovery_boundaries` and
  `keyword_type_members_are_not_recovery_boundaries`.
- **Observed failure**: Both failed on unmodified PR code with unexpected
  `MissingToken` recovery records for `}` in `enum E { const = 1, ... }` and
  `type T = { const; ... }`. Commands: `cargo test --test compile
  editor_projection_rejects` and `cargo test --test swc_editor_recovery keyword_`.
- **Path**: `tests/compile.rs`,
  `editor_projection_recovers_incomplete_computed_enum_members`.
- **Observed failure**: Before replacing the enum closing-bracket assertion,
  `cargo test --test compile editor_projection_recovers_incomplete_computed_enum_members`
  failed at `parser/mod.rs:708` with `assertion failed: expected ], got export`.
  Compiler and editor cases `editorRecoveryComputedEnumName.tt` and
  `recoveryComputedEnumName.tt` also pin strict rejection and sibling queries.
- **Path**: `tests/swc_editor_recovery.rs`,
  `incomplete_computed_enum_members_leave_following_statements_to_their_owner`.
- **Observed failure**: After fixing the panic but before respecting enum-list
  terminators in member initialization, the test found `[]` instead of the
  expected declaration name `["later"]`. The editor baseline likewise showed
  `E.later` instead of the independent exported `const later: number`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed
- [x] All 14 `swc_editor_recovery` tests
- [x] Targeted projection and real content-mapper regressions
- [x] New compiler and editor case baselines, including TypeScript twin parity
- [x] Final 1,003-input mutation pass: no panic or timeout
- [x] Baseline ownership check: 5,515 compared, none unused; the repository's
  default matrix sample leaves 5,793 unsampled references unjudged
- [x] `./scripts/ci` (all six stages)

## Result

Corrected the four recovery issues above in the responsible lexical and parser
layers. No diagnostic suppression, parity exclusion, toolchain change, or
executable recovery output was introduced.

Changed files:

- `src/lexer.rs`, `src/lexer/validation.rs`, and
  `src/program_syntax/{collector,projection}.rs`: shared lexical protections.
- `vendor/swc_ecma_parser/src/parser/{jsx,typescript}.rs`: JSX namespace
  validation, list-specific member continuations, and computed enum recovery.
- `tests/{compile,content_mapper,swc_editor_recovery}.rs`: projection API,
  parser ownership, and actual mapper process regressions.
- `tests/cases/compiler/editorRecoveryLexicalGuards.ttx` and
  `editorRecoveryComputedEnumName.tt`, plus their eight reference baselines.
- `tests/cases/editor/recoveryKeywordEnumMembers.{tt,ts}`,
  `recoveryKeywordTypeMembers.{tt,ts}`, and `recoveryComputedEnumName.tt`, plus
  their three editor reference baselines.
- `docs/design/compiler-architecture.md`, this record, and `docs/tasks/INDEX.md`.

Final full-gate validation passed. These changes correct PR #140's structural
editor recovery implementation.
