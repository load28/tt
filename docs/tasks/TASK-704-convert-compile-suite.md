# TASK-704: Convert the compile suite's inline tests to case files

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-704`

## Purpose

Move every test of `tests/compile.rs` and `tests/compile/*.rs` whose
observation a case pins into `tests/cases/compiler/`, with the tool and
proof of TASK-703, so that the emitted TypeScript and the diagnostics of
those programs are reviewed as whole baselines instead of asserted in part.

## Scope

- Included: 562 case files under `tests/cases/compiler/` (each with
  `// @origin`), their baselines, the 394 Rust tests they replace, the
  helper (`advice`) and the section banners left with no test, and the
  test-placement guidance in `AGENTS.md` and `CONTRIBUTING.md`.
- Excluded: compiler changes (none were made); the 186 tests that stay in
  Rust, listed below with the reason.

## Decisions

### Decision 1: Keep the baselines as the CLI writes them, including what an unchecked snippet reports

- **Context**: Most inline tests compile a fragment that uses names it
  never declares (`read()`, `items`), which the library's `compile` never
  type-checks. As a case, the fragment goes through `ttc --check-types` and
  `tsc`, so 387 of the 562 cases have an `.errors.txt` (251 of them only
  for TypeScript's `Cannot find name` and what follows from it, 136
  because the test expected a tt error).
- **Alternatives considered**: (a) Add declarations to the fragments so
  they type-check: the case would no longer be the program the test
  compiled, and the proof would no longer be about the same input.
  (b) Keep the fragments byte for byte and their baselines as they are.
- **Decision and rationale**: (b). This is a refactor of tests, and the
  proof is only sound for the same input. The TypeScript diagnostics are
  real behaviour too: they show where a cascade lands in generated code
  (`'$tt_t0' is of type 'unknown'`, reported "in code ttc generated for
  this construct") and where the checker-backed path reports what the
  untyped one does not (Issue 2).

### Decision 2: Prove against the CLI's emission, not the library's

- **Context**: The inline tests call `ttc::compile` with default
  `Options`; a case runs `ttc --out-dir`, which calls the same
  `compile_report` with the file name, the support-module specifiers of
  the output tree, and contextual storage typing on top.
- **Alternatives considered**: (a) Add a library-emission baseline kind:
  a second artifact for every case that only differs in the generated
  storage annotations and the support import. (b) Accept a conversion when
  the test's own assertions hold against the CLI's emission, and keep the
  tests whose assertions are about the library's defaults.
- **Decision and rationale**: (b). The CLI emission is what users get and
  goes through the same lowering; the eight tests whose assertions are
  about the library-default `@tt/runtime` and `@tt/std` specifiers fail
  their proof and stay in Rust, which is the evidence that the proof
  distinguishes the two.

## Work log

- 2026-10-01: `scripts/convert-inline-tests scan|harness|record|generate compile`:
  580 tests; 125 rejected by their text, 20 items dropped by the build
  (15 tests and 5 helpers); 436 tests recorded 1,051 compilations of 997
  distinct programs; 29 tests compile more than six programs; 579 case
  files for the other 404.
- 2026-10-01: `UPDATE_EXPECT=1 cargo test --test case_baselines`, then
  `prove compile`: 396 proven, 39 not (29 tables, 10 failed assertions).
  `apply compile` deleted the 394 proven tests that recorded a program
  (two proven tests compiled nothing through a helper and stay) and the
  17 case files no deleted test needs; their baselines were removed.
- 2026-10-01: Removed `advice` (its last caller moved) and six section
  banners left without tests; `cargo clippy --test compile -- -D warnings`
  and `cargo fmt --check` clean.

## Results

| | Before | After |
| --- | --- | --- |
| Rust tests in the compile suite | 580 | 186 |
| Case files under `tests/cases/compiler/` | 40 | 602 (562 converted) |

## Issues and resolutions

### Issue 1: One converted program makes the engine's semantic tokens time out

- **Symptom**: `verifyPreflightsUnbalancedDelimitersBeforeSwc` fails its
  `.types` baseline: "TypeScript language service request
  `textDocument/semanticTokens/full` timed out after 8s".
- **Cause**: Recorded in TASK-703 Issue 1; the program is a fuzz-shaped
  line of unbalanced type-argument brackets.
- **Resolution**: The case keeps `@baselines: ts, errors.txt, map.txt`, the
  observations the original test made. The timeout is a defect for a
  separate task: an editor asking for semantic tokens on this text waits
  eight seconds and gets an error.

### Issue 2: The checker-backed path reports tt errors the CLI build does not

- **Symptom**: Five converted cases build with `ttc --out-dir` but fail
  `ttc --check-types` with a tt rule: `matchOnUnknownTagsIsNotChecked`
  (`match-not-exhaustive` on a plain TypeScript union),
  `valNeverCallsAMethodAMutationFromItsName3`–`5` (`val-mutation` on
  `push`, `set`, and a method TypeScript knows mutates),
  `aTryInAConciseArrowInsideAResultBlockTargetsTheArrow`
  (`result-return-nested`).
- **Cause**: `Options::defer_to_checker` (src/lib/compile.rs): the build
  answers exhaustiveness and `val` pairing from ttc's own model, the
  checker-backed path asks TypeScript. The original tests pinned the
  untyped answer only.
- **Resolution**: Documented behaviour, not a defect; the baselines now
  show both answers side by side, which no inline test did.

### Issue 3: Baselines that look wrong

None found. Every converted `.errors.txt` was scanned for panics and
internal errors (none), for TypeScript diagnostics in generated code (ten,
all cascades from an undeclared name, a DOM global the script redeclares
such as `Node` or `Option`, or JSX without `JSX.IntrinsicElements`), and
for disagreement between `ttc --check-types` and `tsc` on the emitted tree
(three, where `--check-types` reports one diagnostic for a construct whose
generated code `tsc` reports twice).

## Regression test (fails before the fix)

Not applicable: no compiler behaviour changed; each converted test's
assertions were run against the new baselines by `scripts/convert-inline-tests prove`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (see TASK-705 for the full gate run over both suites)
- [x] Baseline changes reviewed and committed with the change

## Tests kept in Rust (186)


### Its body calls the library directly (122)

- `tests/compile.rs::every_value_region_crosses_every_host_protocol_class`
- `tests/compile/cases_01.rs::apply_partitions_structured_children_by_their_function_host`
- `tests/compile/cases_01.rs::duplicate_variant_cases_do_not_duplicate_the_semantic_alphabet`
- `tests/compile/cases_01.rs::expression_only_match_owners_are_rejected_without_a_closure_fallback`
- `tests/compile/cases_01.rs::is_call_syntax_points_to_property_pattern_braces`
- `tests/compile/cases_01.rs::is_constructor_identity_crosses_or_and_binding_wrappers`
- `tests/compile/cases_01.rs::is_patterns_require_open_hierarchy_and_binding_rules`
- `tests/compile/cases_01.rs::malformed_unit_variant_is_a_variant_diagnostic`
- `tests/compile/cases_01.rs::match_arm_return_try_propagates_from_the_concise_arrow`
- `tests/compile/cases_01.rs::semicolon_free_arrow_does_not_own_the_following_try`
- `tests/compile/cases_01.rs::ttx_rewrites_ttx_imports_for_each_target_surface`
- `tests/compile/cases_02.rs::match_arms_reject_only_control_transfers_that_cross_the_arm`
- `tests/compile/cases_02.rs::rejected_match_boundaries_do_not_emit_an_expression_helper`
- `tests/compile/cases_03.rs::a_module_level_inline_try_reports_the_cause_not_the_backstop`
- `tests/compile/cases_03.rs::expression_try_reports_a_typescript_control_flow_boundary`
- `tests/compile/cases_03.rs::placement_matrix_prerequisite_gate`
- `tests/compile/cases_03.rs::result_try_crossing_an_isolated_match_arm_is_a_placement_diagnostic`
- `tests/compile/cases_03.rs::try_in_constructor_or_generator_is_a_placement_error`
- `tests/compile/cases_03.rs::try_placement_claims_for_update_and_destructuring_edges`
- `tests/compile/cases_04.rs::discarded_result_reports_a_named_diagnostic_without_unwinding`
- `tests/compile/cases_04.rs::every_other_diagnostic_is_still_reported_with_it`
- `tests/compile/cases_04.rs::import_equals_require_reference_is_rewritten`
- `tests/compile/cases_04.rs::invalid_typescript_in_a_match_arm_body_reports_the_byte_not_the_construct`
- `tests/compile/cases_04.rs::literal_import_rewrite_matrix_preserves_surrounding_syntax`
- `tests/compile/cases_04.rs::off_mode_leaves_the_specifier_untouched`
- `tests/compile/cases_04.rs::the_std_specifier_is_not_a_project_module`
- `tests/compile/cases_04.rs::the_std_specifier_is_rewritten_when_the_caller_places_the_module`
- `tests/compile/cases_04.rs::try_in_a_for_test_is_a_repeated_loop_placement_at_the_try`
- `tests/compile/cases_04.rs::ts_mode_points_at_the_emitted_file`
- `tests/compile/cases_04.rs::ts_mode_preserves_the_quote_style_and_path`
- `tests/compile/cases_05.rs::a_missing_step_keeps_the_pipeline_written_before_it`
- `tests/compile/cases_05.rs::a_step_with_an_open_list_ends_where_typescript_ends_the_list`
- `tests/compile/cases_05.rs::a_stray_if_let_recovers_only_the_statement_typescript_reads`
- `tests/compile/cases_05.rs::a_stray_pipe_recovers_only_to_the_end_of_its_statement`
- `tests/compile/cases_05.rs::an_arm_whose_guard_is_not_written_yet_is_a_malformed_arm`
- `tests/compile/cases_05.rs::an_arm_with_no_body_keeps_its_pattern_and_guard`
- `tests/compile/cases_05.rs::an_emission_reports_the_support_modules_it_imports`
- `tests/compile/cases_05.rs::bare_super_is_not_an_optional_receiver`
- `tests/compile/cases_05.rs::empty_or_dangling_step_is_an_error`
- `tests/compile/cases_05.rs::exported_variants_names_a_variant_by_its_local_export_specifiers`
- `tests/compile/cases_05.rs::exported_variants_returns_exported_tt_enums_only`
- `tests/compile/cases_05.rs::extern_variant_shadows_builtin_of_same_name`
- `tests/compile/cases_05.rs::malformed_optional_postfix_is_one_owned_diagnostic`
- `tests/compile/cases_05.rs::scan_module_answers_both_questions_in_one_pass`
- `tests/compile/cases_05.rs::tt_imports_reports_specifiers_and_names`
- `tests/compile/cases_05.rs::variant_symbols_carries_positions_and_field_shapes`
- `tests/compile/cases_06.rs::a_payload_field_cannot_be_named_like_the_case_tag_s_property`
- `tests/compile/cases_06.rs::a_required_variant_field_cannot_follow_an_optional_one`
- `tests/compile/cases_06.rs::a_stray_else_of_an_if_let_chain_is_reported_once_where_the_chain_stops`
- `tests/compile/cases_06.rs::a_value_pattern_in_a_tuple_element_is_reported_at_that_element`
- `tests/compile/cases_06.rs::match_without_scrutinee_parentheses_is_a_malformed_tt_match`
- `tests/compile/cases_06.rs::tuple_match_arity_mismatch_is_an_error`
- `tests/compile/cases_06.rs::tuple_match_arity_mismatch_lowers_over_the_subjects_it_has`
- `tests/compile/cases_06.rs::val_writes_follow_assignment_targets_not_neighboring_tokens`
- `tests/compile/cases_07.rs::discarded_result_suppresses_the_redundant_missing_success_diagnostic`
- `tests/compile/cases_07.rs::result_rejects_control_transfers_to_an_outer_region`
- `tests/compile/cases_07.rs::result_tail_is_an_ordinary_semicolon_terminated_statement`
- `tests/compile/cases_08.rs::a_misspelled_case_carries_its_replacement_as_an_edit`
- `tests/compile/cases_08.rs::a_misspelled_field_carries_its_replacement_as_an_edit`
- `tests/compile/cases_08.rs::applying_a_suggested_edit_resolves_the_diagnostic_it_came_from`
- `tests/compile/cases_08.rs::applying_the_authored_arms_makes_the_match_exhaustive`
- `tests/compile/cases_08.rs::authored_arm_lines_end_with_the_line_ending_of_a_cr_file`
- `tests/compile/cases_08.rs::authored_arm_lines_end_with_the_line_ending_of_a_crlf_file`
- `tests/compile/cases_08.rs::every_authored_arm_edit_compiles_after_the_last_written_arm`
- `tests/compile/cases_08.rs::the_wildcard_arm_closes_the_hole_too`
- `tests/compile/cases_08.rs::val_probes_carry_the_callee_and_the_declarations_it_may_name`
- `tests/compile/cases_08.rs::val_probes_collect_every_method_call_for_the_verdict`
- `tests/compile/cases_09.rs::a_destructuring_default_value_stays_inside_the_default`
- `tests/compile/cases_09.rs::a_duplicate_arm_does_not_hide_the_files_other_diagnostics`
- `tests/compile/cases_09.rs::a_mixed_match_reports_the_cause_and_suppresses_its_coverage`
- `tests/compile/cases_09.rs::a_switch_case_test_value_stays_behind_its_case`
- `tests/compile/cases_09.rs::a_typo_suppresses_coverage_for_its_own_match_only`
- `tests/compile/cases_09.rs::an_unterminated_interpolation_is_an_open_expression`
- `tests/compile/cases_09.rs::analyze_reports_every_uncovered_match_in_source_order`
- `tests/compile/cases_09.rs::compile_report_still_emits_under_recoverable_errors`
- `tests/compile/cases_09.rs::compile_report_withholds_emission_when_the_output_cannot_be_typescript`
- `tests/compile/cases_09.rs::duplicate_nested_binding_is_renamed_across_destructuring_statements`
- `tests/compile/cases_09.rs::duplicate_pattern_binding_is_renamed_in_recovery_output`
- `tests/compile/cases_09.rs::duplicate_tuple_binding_is_renamed_across_tuple_elements`
- `tests/compile/cases_09.rs::duplicate_variant_case_emits_only_one_constructor_property`
- `tests/compile/cases_09.rs::every_stray_construct_is_reported_not_just_the_first`
- `tests/compile/cases_09.rs::malformed_match_blocks_codegen_even_beside_a_lowered_variant`
- `tests/compile/cases_09.rs::malformed_namespaced_jsx_member_is_reported_without_panicking`
- `tests/compile/cases_09.rs::merge_conflict_markers_report_errors_without_panicking`
- `tests/compile/cases_09.rs::sema_and_val_diagnostics_merge_in_source_order`
- `tests/compile/cases_10.rs::a_result_tail_expression_without_a_semicolon_reports_only_the_missing_value`
- `tests/compile/cases_11.rs::a_conditional_try_keeps_its_operand_when_a_later_sibling_captures_it`
- `tests/compile/cases_11.rs::a_construct_its_position_does_not_admit_is_reported_at_the_construct`
- `tests/compile/cases_11.rs::a_default_exported_variant_is_reported_at_default_with_a_named_export_fix`
- `tests/compile/cases_11.rs::a_default_exported_variant_that_does_not_parse_reports_the_variant`
- `tests/compile/cases_11.rs::a_jump_crossing_a_result_block_reports_only_the_crossing`
- `tests/compile/cases_11.rs::a_lexical_binding_statement_as_an_unbraced_body_is_a_placement_error`
- `tests/compile/cases_11.rs::a_malformed_variant_behind_modifiers_is_reported_once`
- `tests/compile/cases_11.rs::a_match_the_class_definition_evaluates_reports_its_placement`
- `tests/compile/cases_11.rs::a_nested_hole_under_a_written_constructor_is_reported_with_the_missing_case`
- `tests/compile/cases_11.rs::a_recoverable_syntax_error_still_reports_plan_diagnostics`
- `tests/compile/cases_11.rs::a_result_block_whose_try_sits_in_a_match_arm_reports_the_crossing`
- `tests/compile/cases_11.rs::a_tuple_hole_count_is_every_missing_combination`
- `tests/compile/cases_11.rs::a_tuple_hole_under_a_written_constructor_is_reported_and_fixed_in_one_step`
- `tests/compile/cases_11.rs::a_tuple_missing_arm_suggestion_pastes_back_without_duplicate_bindings`
- `tests/compile/cases_11.rs::a_tuple_too_wide_to_enumerate_states_a_lower_bound_and_offers_only_the_wildcard`
- `tests/compile/cases_11.rs::a_type_assertion_on_the_line_after_a_pipeline_is_rejected_as_typescript_rejects_it`
- `tests/compile/cases_11.rs::a_var_let_else_as_an_unbraced_body_is_lowered_inside_one_block`
- `tests/compile/cases_11.rs::a_wide_tuple_counts_its_holes_exactly_without_listing_them_all`
- `tests/compile/cases_11.rs::a_yield_crossing_a_result_block_reports_only_the_crossing`
- `tests/compile/cases_11.rs::an_exported_try_declaration_reports_only_its_placement`
- `tests/compile/cases_11.rs::an_if_let_as_an_unbraced_body_is_projected_as_one_statement`
- `tests/compile/cases_11.rs::an_if_let_in_any_expression_position_reports_only_its_placement`
- `tests/compile/cases_11.rs::explained_examples_behave_as_their_explanations_say`
- `tests/compile/cases_11.rs::numeric_literal_patterns_take_their_ecmascript_values`
- `tests/compile/cases_11.rs::val_on_a_rest_parameter_guards_its_elements`
- `tests/compile/cases_14.rs::a_for_head_initializer_that_reads_a_head_binding_is_a_placement_error`
- `tests/compile/cases_14.rs::a_hoisted_value_in_a_member_step_is_emitted_once_after_its_method`
- `tests/compile/cases_14.rs::a_hoisted_value_in_any_pipeline_operand_is_emitted_once`
- `tests/compile/cases_14.rs::a_statement_value_in_a_later_loop_head_declarator_is_a_placement_error`
- `tests/compile/cases_14.rs::a_statement_value_in_an_enum_member_initializer_is_a_placement_error`
- `tests/compile/cases_14.rs::a_try_in_a_template_in_a_pipeline_operand_keeps_its_callee_before_it`
- `tests/compile/cases_14.rs::a_try_in_a_template_interpolation_claims_its_result_block`
- `tests/compile/cases_14.rs::a_write_to_a_call_result_is_not_a_write_to_its_argument`
- `tests/compile/cases_14.rs::an_optional_call_tests_the_link_its_chain_is_skipped_at`
- `tests/compile/cases_14.rs::an_unstructurable_conditional_in_a_pipeline_operand_is_a_placement_diagnostic`
- `tests/compile/cases_14.rs::val_sees_a_mutation_through_every_typescript_wrapper`

### It is a table of more than six programs over one rule (29)

- `tests/compile/cases_02.rs::call_arguments_wider_than_the_match_keep_their_authored_frame`
- `tests/compile/cases_04.rs::let_else_divergence_covers_every_statement_form`
- `tests/compile/cases_04.rs::let_else_divergence_still_rejects_every_normal_exit`
- `tests/compile/cases_04.rs::let_else_divergence_stops_at_an_isolated_value_region`
- `tests/compile/cases_06.rs::a_tt_value_anywhere_in_a_concise_arrow_body_keeps_the_block_balanced`
- `tests/compile/cases_07.rs::val_array_pattern_parameter_is_erased_in_every_parameter_list`
- `tests/compile/cases_07.rs::val_binds_a_function_or_class_expression_name_only_inside_itself`
- `tests/compile/cases_07.rs::val_resolves_hoisted_declarations_to_their_scope`
- `tests/compile/cases_11.rs::a_script_declares_its_helpers_as_typed_vars_after_its_file_pragmas`
- `tests/compile/cases_11.rs::a_value_hoisted_out_of_an_unbraced_body_opens_its_own_block`
- `tests/compile/cases_11.rs::a_yield_in_a_match_subject_or_guard_suspends_the_enclosing_generator`
- `tests/compile/cases_11.rs::an_if_let_after_an_automatic_semicolon_boundary_starts_a_statement`
- `tests/compile/cases_11.rs::only_a_file_typescript_reads_as_a_module_keeps_the_module_form`
- `tests/compile/cases_12.rs::a_let_else_block_diverges_after_a_line_ending_in_a_type_or_a_name`
- `tests/compile/cases_12.rs::a_pipeline_head_starts_on_the_line_after_a_type`
- `tests/compile/cases_12.rs::a_statement_match_after_a_line_ending_in_a_type_keeps_both_statements`
- `tests/compile/cases_12.rs::a_try_after_a_line_ending_in_a_type_is_a_statement`
- `tests/compile/cases_12.rs::an_if_let_after_a_line_ending_in_a_type_starts_a_statement`
- `tests/compile/cases_13.rs::a_comma_inside_type_arguments_stays_inside_its_construct`
- `tests/compile/cases_13.rs::a_jsx_element_after_a_finished_statement_keeps_its_text`
- `tests/compile/cases_13.rs::a_lowered_statement_after_a_semicolon_free_line_starts_its_own_statement`
- `tests/compile/cases_13.rs::a_regex_after_a_finished_statement_keeps_its_text`
- `tests/compile/cases_14.rs::a_contextual_type_or_statement_word_is_a_name_unless_typescript_reads_a_prefix`
- `tests/compile/cases_14.rs::a_function_return_type_still_continues_to_its_own_arrow`
- `tests/compile/cases_14.rs::a_line_after_an_import_equals_declaration_starts_a_statement`
- `tests/compile/cases_14.rs::a_statement_after_an_arrow_with_a_parenthesized_return_type_keeps_its_text`
- `tests/compile/cases_14.rs::a_statement_match_that_ends_the_file_closes_the_block_it_hoists_into`
- `tests/compile/cases_14.rs::a_value_inside_a_let_else_or_if_let_subject_is_lowered_once`
- `tests/compile/cases_14.rs::an_arrow_body_after_a_parenthesized_return_type_is_a_body`

### It compiles with non-default `Options` or another `SourceKind` (10)

- `tests/compile/cases_01.rs::variant_invalid_field_type_passes_without_verify`
- `tests/compile/cases_04.rs::filename_appears_in_error_display`
- `tests/compile/cases_04.rs::no_verify_does_not_bypass_the_lowering_precondition`
- `tests/compile/cases_04.rs::no_verify_passes_invalid_typescript_through`
- `tests/compile/cases_07.rs::tuple_patterns_do_not_accept_literals`
- `tests/compile/cases_07.rs::val_forbids_every_mutating_operator`
- `tests/compile/cases_09.rs::conflict_marker_text_in_literals_and_comments_is_preserved`
- `tests/compile/cases_09.rs::pipeline_values_containing_double_slashes_do_not_become_comments`
- `tests/compile/cases_10.rs::the_runtime_import_precedes_generated_text_at_the_top_of_the_file`
- `tests/compile/cases_14.rs::an_assertion_operand_takes_no_storage_type_from_outside_the_assertion`

### Its assertion does not hold against the baselines (10)

- `tests/compile/cases_04.rs::the_std_specifier_is_left_alone_by_default` — asserts the library default leaves `@tt/std` bare; the CLI points it at the materialized support module
- `tests/compile/cases_05.rs::pipeline_emits_nested_apply_helper_calls` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_05.rs::pipeline_runtime_is_imported_once_per_file` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_06.rs::flow_emits_nested_composition_helper_calls` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_06.rs::flow_runtime_is_imported_once_per_file` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_09.rs::a_duplicate_arm_covers_the_tag_it_repeats` — asserts the error's end position, which a one-caret rendering does not distinguish from a position-only error
- `tests/compile/cases_09.rs::an_error_without_a_known_extent_reports_a_position_only` — asserts the error has no end position, which a one-caret rendering does not distinguish from a one-character range
- `tests/compile/cases_11.rs::a_module_declares_its_helpers_with_its_import_after_its_file_pragmas` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_11.rs::a_string_that_continues_into_an_expression_is_not_a_directive` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`
- `tests/compile/cases_11.rs::generated_names_are_allocated_around_the_files_identifiers` — asserts the bare `@tt/runtime` import of the library default; the CLI imports `./tt/runtime.js`

### It calls `token_extern`, a helper over a library API (5)

- `tests/compile/cases_04.rs::extern_variant_full_coverage_compiles`
- `tests/compile/cases_04.rs::extern_variant_makes_match_checked`
- `tests/compile/cases_05.rs::extern_variants_do_not_affect_unrelated_matches`
- `tests/compile/cases_05.rs::local_variant_shadows_extern_of_same_name`
- `tests/compile/cases_08.rs::a_misspelled_case_of_an_imported_variant_names_its_origin`

### It calls `hole`, a helper over a library API (4)

- `tests/compile/cases_08.rs::a_match_with_holes_carries_the_arms_that_close_them`
- `tests/compile/cases_08.rs::an_authored_arm_binds_the_payload_the_body_will_need`
- `tests/compile/cases_08.rs::an_authored_arm_keeps_a_one_line_match_on_one_line`
- `tests/compile/cases_08.rs::authored_arms_take_the_indentation_of_the_written_arms`

### Its body observes a panic (3)

- `tests/compile.rs::value_region_nesting_matrix_compiles_every_directed_pair`
- `tests/compile/cases_09.rs::malformed_pipeline_tail_never_reaches_codegen_as_owned_source`
- `tests/compile/cases_09.rs::unterminated_template_interpolation_never_overlaps_source_spans`

### It compiles no program through a case-observable helper (2)

- `tests/compile/cases_09.rs::diagnostic_codes_are_stable_strings`
- `tests/compile/cases_11.rs::if_let_placement_explanation_names_value_positions`

### It calls `generated_lines`, a helper over a library API (1)

- `tests/compile/cases_05.rs::every_construct_lays_its_glue_out_from_the_line_it_replaces`

