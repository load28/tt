# TASK-406: Encode sidecar map URLs and verify output before the contextual pass

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Two defects on paths TASK-395 and TASK-388 did not cover: the sidecar
declaration maps still wrote raw paths where Source Map v3 requires URLs, and
with TypeScript installed an invalid file was reported as an internal-sounding
`error[other]` instead of the located diagnostic the untyped path gives.

## Scope

- Included: The `sources` entry and the `sourceMappingURL` comment written by
  `ttc::build_sidecar` (used by `--sidecar` in `src/main/modes.rs` and by
  `--types` in `src/main/typed.rs`), and the order of the output self-check
  and the contextual annotation pass in `compile_report`
  (`src/lib/compile.rs`).
- Excluded: The `@tt/std` import specifiers that `relative_path`
  (`src/main/modes.rs`) also builds; they are module specifiers resolved as
  paths by TypeScript and stay unencoded. The `file` field and the
  `@generated from` banner keep the plain file name (TASK-395 Decision 2).

## Defects

1. `ttc --sidecar decl -o "out dir" "my src"` over `my src/a b#1%.tt`
   wrote `//# sourceMappingURL=a b#1%.tt.d.ts.map` and
   `"sources":["../my src/a b#1%.tt"]`. The comment URL ends at the space, and
   `#` and `%` start a fragment and an escape.
2. In a directory whose `node_modules` resolves TypeScript,
   `ttc --check res.tt` for a function containing
   `const q = (try result { if (b) { return 10; } return 1; }) * 2;` (a
   `result` block with no inner `try`, which tt.md does not claim, so the
   file is not TypeScript) printed
   `error[other]: contextual projection lost a successfully lowered module`
   with no position. Without TypeScript the same file reports
   `error[verify-failed]` at `res.tt:3:25`.

## Decisions

### Decision 1: `build_sidecar` serializes its path argument as a relative URL

- **Context**: ECMA-426 (Source Map), "Source map format": each `sources`
  entry is a (potentially relative) URL resolved against the map's URL, and
  the `sourceMappingURL` comment carries a URL resolved against the generated
  file's URL. The WHATWG URL Standard reads `#`, `?`, `%`, and `\` as syntax.
  `build_sidecar` documents its argument as a *path* relative to the sidecar,
  and it derives the map's file name from that path itself.
- **Alternatives considered**: Encoding in the CLI before the call would also
  encode the name `build_sidecar` writes into `file` and the banner, and the
  library would still write the `sourceMappingURL` from an unencoded name.
  Encoding inside `relative_path` would also encode the `@tt/std` import
  specifiers built from it.
- **Decision and rationale**: The one percent-encoding routine TASK-395 added
  (`url_path`, moved from `src/main/output.rs` to `ttc::source_map::url_path`
  so the library and the CLI share it) now serializes the `sources` entry and
  the map name in the comment. `relative_path` keeps producing a path, so the
  import specifiers are unchanged. Every name that was already a valid URL
  path is written unchanged.

### Decision 2: Verify the lowered module before the contextual pass

- **Context**: `compile_report` ran the contextual annotation pass
  (`typescript::contextual::standalone`) before the output self-check. The
  pass re-lowers the file with `compile_projection_report`, which does verify
  and so returns no module for invalid TypeScript; `standalone` then reported
  its internal invariant ("lost a successfully lowered module"), because the
  module it was given had not been checked yet. `compile_mapped` already
  verified first.
- **Alternatives considered**: Mapping that failure to a diagnostic inside
  `standalone` would hide the cause of a real invariant violation and would
  not have the verification position.
- **Decision and rationale**: `compile_report` verifies the lowered module
  first (the same check and the same rule for already-reported tt errors,
  now in `verified_emit`), and only a verified module reaches the contextual
  pass. When the pass inserted annotations, the annotated module is verified
  again, as before. The invariant message now only fires when the pass truly
  loses a verified module. The user gets the same located diagnostic with or
  without TypeScript, which is the error-layer contract in `AGENTS.md`.

## Work log

- 2026-09-27: Reproduced defect 1 with `--sidecar` and defect 2 with
  `ttc --check` in a scratch directory whose `node_modules` links the
  repository's TypeScript 7.1.0-dev.20260826.1, and in one without it.
- 2026-09-27: Moved `url_path` into `src/source_map.rs` and used it in
  `src/sidecar.rs`; split the self-check out of `compile_report` into
  `verified_emit` and ran it before the contextual pass.
- 2026-09-27: Added `sidecar_map_urls_percent_encode_file_names`
  (tests/cli_outputs.rs) and
  `an_invalid_file_reports_the_same_diagnostic_with_typescript_installed`
  (tests/cli.rs; a function-level case reporting `verify-failed` and a
  module-level case reporting `try-placement`, each compared byte for byte
  with the report from a directory without TypeScript). Both fail with the
  previous `src/` (the raw `sourceMappingURL`, and
  `error[other]: contextual projection lost a successfully lowered module`)
  and pass now.

## Issues and resolutions

### Issue 1: A public helper without documentation

- **Symptom**: Moving `url_path` into the library made it public, and the
  crate's `missing_docs` lint warned.
- **Cause**: The CLI is a separate crate and can only call public items.
- **Resolution**: The function is `#[doc(hidden)]`: it serves the `ttc`
  binary and is not part of the documented library API.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (no snapshot update needed)
- [x] `node scripts/check-task-index`

## Result

Changed files: `src/source_map.rs`, `src/sidecar.rs`, `src/main/output.rs`,
`src/lib/compile.rs`, `tests/cli.rs`, `tests/cli_outputs.rs`. Sidecar maps
name their source and themselves with URLs, and an invalid file reports the
same located diagnostic whether or not TypeScript is installed.
