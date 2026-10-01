# TASK-705: Convert the integration suite's runtime and type-check tests to case files

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-705`

## Purpose

Move every test of `tests/integration.rs` and `tests/integration/*.rs`
whose observation a case pins (what the emitted program prints, and
whether TypeScript accepts the emitted tree) into
`tests/cases/conformance/evaluation/`, with the tool and proof of
TASK-703.

## Scope

- Included: 158 case files (121 with `@run`) under
  `tests/cases/conformance/evaluation/`, their baselines, the 153 Rust
  tests they replace, `tests/integration/pr115.rs` (all four of its tests
  converted) and the two prelude constants no remaining test uses.
- Excluded: the 62 tests that stay in Rust, listed below with the reason;
  `@twin` counterparts (see Decision 2); compiler changes (none).

## Decisions

### Decision 1: A case program is the module the integration helper compiled

- **Context**: `run` and `typecheck` compile `as_module(src)`, the snippet
  with `export {};` appended so its names do not collide with DOM globals;
  `run_with_std` compiles the snippet as written.
- **Alternatives considered**: (a) Write the snippet alone: a script, whose
  `Option` or `Node` collides with the DOM library, which the test never
  saw. (b) Write what the helper compiled.
- **Decision and rationale**: (b). The case unit is byte for byte the text
  the library compiled in the test, so the recorded program and the case
  are the same input. `typecheck` answers from the `.errors.txt` baseline's
  `tsc` section (its exit status and report) followed by the emitted file,
  as the helper's own report was; `run` answers from `.stdout`, and fails
  unless the program exited 0, as the helper did.

### Decision 2: No `@twin` for the converted programs

- **Context**: A twin is a hand-written TypeScript program that must print
  the same. The converted tests already state the expected output in their
  assertions, and the proof shows the `.stdout` baseline satisfies them.
- **Alternatives considered**: writing 121 twins by hand: a second program
  per case, written now from ttc's own output rather than from the
  documented semantics, which is the opposite of what a twin is for
  (TASK-678).
- **Decision and rationale**: No twins in this task. A later case that pins
  one construct's semantics can add one written from `docs/ai/tt.md`.

## Work log

- 2026-10-01: `scripts/convert-inline-tests scan|harness|record|generate integration`:
  215 tests; 8 rejected by their text, 38 items dropped by the build;
  178 tests recorded 198 compilations (135 run, 63 type-checked); one test
  compiles 16 programs; 182 case files for the other 177.
- 2026-10-01: `UPDATE_EXPECT=1 cargo test --test case_baselines`, `prove
  integration`: 153 proven, 25 not (24 whose programs import the `./tt/`
  modules the helper writes, and the 16-program table). `apply integration`
  deleted the 153 tests and the 24 case files no deleted test needs.
- 2026-10-01: Removed the emptied `tests/integration/pr115.rs` and its
  `mod`, `PIPE_PRELUDE` and `RESULT_PRELUDE`, and section banners left
  without tests; `cargo clippy --test integration -- -D warnings` clean.

## Results

| | Before | After |
| --- | --- | --- |
| Rust tests in the integration suite | 215 | 62 |
| Case files under `tests/cases/conformance/evaluation/` | 7 | 165 (158 converted, 121 run) |

## Issues and resolutions

### Issue 1: A third of the std-library tests cannot become cases as written

- **Symptom**: 24 converted programs fail with `Cannot find module
  './tt/index.js'` in their `.errors.txt`, so their programs never run.
- **Cause**: These tests import the standard library from `./tt/*.js`,
  files `write_std` puts next to the program; a case has only its units.
- **Resolution**: Kept in Rust. Converting them would mean either rewriting
  their imports to `@tt/std` (a different program) or copying the
  standard library's source into each case (a second copy that goes
  stale).

### Issue 2: Baselines that look wrong

None found: no converted run case has a `.stderr` baseline, no
`.errors.txt` mentions a panic or an internal error, and no converted
case shows a TypeScript diagnostic in generated code or a disagreement
between `ttc --check-types` and `tsc`.

## Regression test (fails before the fix)

Not applicable: no compiler behaviour changed; each converted test's
assertions were run against the new baselines by `scripts/convert-inline-tests prove`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Tests kept in Rust (62)

### Its assertion does not hold against the baselines (24)

- `tests/integration/cases_01.rs::runtime_let_else_diverges_through_an_inline_if_let` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_let_else_diverges_through_every_statement_form` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_let_else_else_block_returns_an_object_literal` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_let_else_narrows_and_diverges` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_std_new_combinators` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_std_option_result_functional_pipeline` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_try_error_propagation` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_try_inside_a_closure_propagates_from_the_closure` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_01.rs::runtime_try_inside_an_if_let_body_propagates_from_the_function` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::runtime_result_and_then_chain_short_circuits_on_the_first_err` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_and_then_on_a_variant_typed_value_keeps_the_chained_error` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_and_then_p_accumulates_error_types_along_a_pipeline` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_and_then_p_composes_under_flow` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_and_then_p_takes_an_annotated_inline_callback` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_and_then_unions_the_two_error_types` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_block_output_pipes_into_and_then_p` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_combinators_keep_the_side_a_callback_never_returns` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_constructors_type_only_their_own_variant` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::std_result_map_p_keeps_the_error_type_it_was_given` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_02.rs::try_error_types_infer_as_a_union_without_an_annotation` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_04.rs::runtime_result_block_replaces_nested_combinator_callbacks` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_05.rs::runtime_a_later_declarator_value_runs_after_earlier_declarators_in_their_scope` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_05.rs::runtime_a_result_block_in_an_enum_member_reads_the_members_in_order` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly
- `tests/integration/cases_05.rs::runtime_sibling_conditional_trys_each_evaluate_their_operand_in_order` — imports the `./tt/*.js` std modules the integration helper writes beside the program; as a case that import does not resolve, so it does not compile cleanly

### It writes its own files or runs processes (the CLI, tsc) (17)

- `tests/integration/cases_02.rs::cli_checks_exhaustiveness_across_tt_imports`
- `tests/integration/cases_02.rs::cli_cross_file_match_runs_end_to_end`
- `tests/integration/cases_02.rs::cli_skips_unresolvable_imports_silently`
- `tests/integration/cases_02.rs::cross_file_tt_import_typechecks_and_runs`
- `tests/integration/cases_02.rs::untyped_cli_does_not_infer_a_generic_payload_owner`
- `tests/integration/cases_02.rs::untyped_cli_does_not_infer_a_single_imported_case_owner`
- `tests/integration/cases_02.rs::untyped_cli_does_not_infer_imported_field_ownership`
- `tests/integration/cases_03.rs::cli_refuses_to_overwrite_a_pass_through_input`
- `tests/integration/cases_03.rs::cli_types_reports_tt_type_errors_at_the_source_position`
- `tests/integration/cases_03.rs::cli_types_reports_type_errors_but_keeps_the_sidecars_fresh`
- `tests/integration/cases_03.rs::cli_types_without_typescript_says_so`
- `tests/integration/cases_03.rs::scripts_stay_scripts_and_their_generated_globals_never_collide`
- `tests/integration/cases_05.rs::a_frame_inside_generated_glue_names_the_construct_that_wrote_it`
- `tests/integration/cases_05.rs::a_node_stack_frame_on_a_copied_line_names_its_column`
- `tests/integration/cases_05.rs::a_node_stack_trace_points_at_the_tt_source`
- `tests/integration/contextual.rs::sibling_contextual_match_family_matrix`
- `tests/integration/contextual.rs::sibling_storage_is_hygienic_and_omits_unused_subjects`

### It compiles with non-default `Options` or another `SourceKind` (7)

- `tests/integration.rs::parameter_and_field_matches_require_a_statement_owner`
- `tests/integration.rs::ttx_output_typechecks_as_tsx`
- `tests/integration/cases_05.rs::an_asserted_value_type_checks_without_a_checker_at_compile_time`
- `tests/integration/contextual.rs::composed_match_values_preserve_typescript_contextual_typing`
- `tests/integration/contextual.rs::scoped_contextual_family_matrix_covers_hosts_and_nesting`
- `tests/integration/contextual.rs::scoped_host_call_completions_preserve_contextual_typing`
- `tests/integration/contextual.rs::scoped_sibling_final_arguments_keep_contextual_typing`

### Its body calls the library directly (6)

- `tests/integration.rs::mixed_source_fixture_emits_one_type_clean_typescript_tree`
- `tests/integration/cases_03.rs::cli_build_emits_a_complete_tree_that_runs`
- `tests/integration/cases_03.rs::cli_types_leaves_nothing_but_the_sidecars`
- `tests/integration/cases_03.rs::cli_types_sidecars_typecheck_the_source_tree`
- `tests/integration/cases_03.rs::cli_types_under_a_configuration_writes_the_std_declarations_it_maps`
- `tests/integration/cases_04.rs::runtime_nested_results_preserve_constructor_and_generator_protocols`

### It calls `run_with_tsc_flags`, a helper over a library API (3)

- `tests/integration/cases_04.rs::runtime_using_disposes_when_try_propagates_err`
- `tests/integration/cases_05.rs::optional_variant_fields_are_absent_when_their_argument_is`
- `tests/integration/contextual.rs::nested_scoped_context_preserves_disposal_and_call_order`

### It calls `typecheck_recovery`, a helper over a library API (1)

- `tests/integration.rs::recoverable_codegen_errors_do_not_create_tsc_errors`

### Its body does its own I/O or process work (1)

- `tests/integration/cases_02.rs::symbols_reports_imports_and_positions_as_valid_json`

### It calls `options_with_runtime`, a helper over a library API (1)

- `tests/integration/cases_03.rs::pipeline_files_import_one_shared_runtime`

### Its body reads a file (1)

- `tests/integration/contextual.rs::guarded_contextual_values_have_no_unused_generated_locals`

### It is a table of more than six programs over one rule (1)

- `tests/integration/contextual.rs::scoped_contextual_hosts_and_cleanup_preserve_typescript_context`
