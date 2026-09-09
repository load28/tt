use super::*;

fn evaluation(source: &str) -> (EvaluationFile, CoreFile) {
    evaluation_kind(source, crate::SourceKind::TypeScript)
}

fn evaluation_kind(source: &str, source_kind: crate::SourceKind) -> (EvaluationFile, CoreFile) {
    let program = crate::parser::parse(source);
    let semantic = crate::analysis::coverage_semantics(source, &program, &[]);
    let core = crate::core_ir::lower_semantic(&semantic, source);
    let syntax =
        ProgramSyntax::build(&semantic, &core, source, source_kind).expect("program syntax");
    let file = EvaluationFile::build(&syntax, &core).expect("evaluation ir");
    (file, core)
}

fn plan(file: &EvaluationFile, core: &CoreFile) -> LoweringPlan {
    let plan = file.lowering_plan(core).expect("lowering plan");
    file.validate_order(&plan).expect("validate_order");
    file.validate_reference(&plan).expect("validate_reference");
    plan
}

#[test]
fn every_core_primitive_gets_one_region() {
    let (file, _core) = evaluation(
        "variant E { A(value: number), B }\n\
             import { load } from \"./load.tt\";\n\
             function f(e: E) {\n\
               try load();\n\
               return result { const x = try load(); return match (e) { A(value) => x + value, B => 0 } |> done; };\n\
             }\n",
    );
    let operations: Vec<_> = file.regions.iter().map(|region| region.operation).collect();
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::Adt(_))),
        "{operations:?}"
    );
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::Import(_))),
        "{operations:?}"
    );
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::Decision(_))),
        "{operations:?}"
    );
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::Propagate(_))),
        "{operations:?}"
    );
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::Apply(_))),
        "{operations:?}"
    );
    assert!(
        operations
            .iter()
            .any(|operation| matches!(operation, OperationId::ResultRegion(_))),
        "{operations:?}"
    );
}

#[test]
fn a_decision_region_has_switch_arms_and_a_join() {
    let (file, core) = evaluation(
        "variant E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
    );
    let region = file
        .regions
        .iter()
        .find(|region| matches!(region.operation, OperationId::Decision(_)))
        .expect("decision region");
    assert!(matches!(
        &region.blocks[0].terminator,
        EvalTerminator::Switch { arms, .. } if arms.len() == 2
    ));
    assert!(region.result.is_some());
    let plan = plan(&file, &core);
    let owner = plan.owners().next().expect("host rewrite");
    assert_eq!(owner.values.len(), 1);
    assert!(matches!(owner.values[0].target, ValueTarget::Slot(_)));
}

#[test]
fn nested_decisions_share_the_outer_host_plan() {
    let (file, core) = evaluation(
        "variant E { A, B }\nconst value = match (outer) { A => match (inner) { A => 1, B => 2 }, B => 0 };\n",
    );
    let plan = plan(&file, &core);
    let values = plan
        .owners()
        .flat_map(|owner| &owner.values)
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 1, "{values:#?}");
    assert!(
        values.iter().any(|value| {
            value.context.continuation == HostContinuation::Initialize
                && value.schedule.steps().is_empty()
        }),
        "{values:#?}"
    );
}

#[test]
fn expression_only_and_statement_values_keep_independent_capabilities() {
    let (file, core) = evaluation_kind(
        "variant E { A(value: string), B }\nconst view = (node: E) => { if let A(value) = node { return <section data-kind={match (node) { A => \"a\", B => \"b\" }}>{value |> .trim()}</section>; } else { return null; } };\n",
        crate::SourceKind::Tsx,
    );
    let plan = plan(&file, &core);
    let values = &plan.owners().next().expect("JSX return owner").values;
    assert_eq!(values.len(), 2, "{values:#?}");
    assert_eq!(values[0].capability, TargetCapability::StatementRegion);
    assert_eq!(
        values[1].capability,
        TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ValueHasNoStatementForm)
    );
}

#[test]
fn a_call_argument_is_composed_by_its_host_owner() {
    let (file, core) =
        evaluation("variant E { A, B }\nconst out = render(match (e) { A => 1, B => 2 });\n");
    let placement = &file
        .regions
        .iter()
        .find(|region| matches!(region.operation, OperationId::Decision(_)))
        .expect("decision region")
        .placement;
    assert!(matches!(
        placement,
        RegionPlacement::Host {
            context: EvaluationContext {
                continuation: HostContinuation::Compose,
                ..
            },
            ..
        }
    ));
    let plan = plan(&file, &core);
    let value = &plan.owners().next().expect("host rewrite").values[0];
    assert!(value.schedule.steps().iter().any(|step| matches!(
        step.operation,
        crate::program_syntax::HostEvaluationOperation::Eager(
            crate::program_syntax::EagerPosition::CallArgument(0)
        )
    )));
}

#[test]
fn a_decision_nested_in_a_propagation_keeps_its_host_schedule() {
    let (file, core) = evaluation(
        "variant E { A(x: number), B }\nfunction f() { const a = try wrap(match (e) { A(x) => x, B => 0 }); }\n",
    );
    let plan = plan(&file, &core);
    let values = plan
        .owners()
        .flat_map(|owner| &owner.values)
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].capability, TargetCapability::StatementRegion);
}

#[test]
fn a_result_binding_is_nested_under_the_result_region() {
    let (file, _core) = evaluation("const out = result { const x = try load(); return x; };\n");
    let result = file
        .regions
        .iter()
        .find(|region| matches!(region.operation, OperationId::ResultRegion(_)))
        .expect("result region");
    let propagation = file
        .regions
        .iter()
        .find(|region| matches!(region.operation, OperationId::Propagate(_)))
        .expect("propagation region");
    assert_eq!(
        propagation.placement,
        RegionPlacement::Nested {
            parent: result.id,
            source: Some(SourceSpan { start: 21, end: 42 }),
            exits: Vec::new(),
            protocol: HostEvaluationProtocol::default(),
        }
    );
}

#[test]
fn a_result_nested_in_a_pipeline_owns_its_returns() {
    let (file, core) = evaluation(
        "declare const g: () => unknown; declare const unwrap: (x: unknown) => number;\nconst f = () => { const n = result { const value = try g(); return value; } |> unwrap; return n; };\n",
    );
    let plan = plan(&file, &core);
    assert!(
        plan.nested_value_exits()
            .any(|(_, exits)| !exits.is_empty()),
        "{file:#?}"
    );
}

#[test]
fn values_in_one_host_statement_form_one_rewrite() {
    let (file, core) = evaluation(
        "const out = [match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 }];\n",
    );
    let plan = plan(&file, &core);
    let owners: Vec<_> = plan.owners().collect();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].values.len(), 2);
    let [ValueTarget::Slot(left), ValueTarget::Slot(right)] =
        [owners[0].values[0].target, owners[0].values[1].target];
    assert_ne!(left, right);
}

#[test]
fn a_later_tt_value_depends_on_the_prior_value_slot() {
    let (file, core) =
        evaluation("consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n");
    let plan = plan(&file, &core);
    let owner = plan.owners().next().expect("host rewrite");
    let ValueTarget::Slot(first) = owner.values[0].target;
    assert!(
        owner.values[1]
            .schedule
            .steps()
            .iter()
            .flat_map(|step| &step.inputs)
            .any(|input| matches!(
                input,
                PlannedEvaluationInput::Slot { slot, .. } if *slot == first
            ))
    );
}

#[test]
fn generated_slot_names_do_not_collide_with_typescript_identifiers() {
    let (file, core) =
        evaluation("const $tt_v0 = 1;\nconst out = match (value) { A => $tt_v0, _ => 0 };\n");
    let plan = plan(&file, &core);
    let ValueTarget::Slot(slot) = plan.owners().next().expect("host rewrite").values[0].target;
    assert_eq!(plan.slot_name(slot), "$tt_v0_1");
}

#[test]
fn validation_rejects_a_normal_path_without_its_result() {
    let (mut file, _core) = evaluation(
        "variant E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
    );
    let region = file
        .regions
        .iter_mut()
        .find(|region| matches!(region.operation, OperationId::Decision(_)))
        .expect("decision region");
    region.blocks[2].statements.clear();
    assert!(matches!(
        file.validate(),
        Err(EvaluationError::MissingResultDefinition { .. })
    ));
}

#[test]
fn validation_rejects_an_out_of_region_target() {
    let (mut file, _core) = evaluation("variant E { A(value: number), B }\n");
    file.regions[0].blocks[0].terminator = EvalTerminator::Goto(EvalBlockId(u32::MAX));
    assert!(matches!(
        file.validate(),
        Err(EvaluationError::InvalidTarget { .. })
    ));
}

fn sole_value(plan: &LoweringPlan) -> &PlannedValue {
    let values: Vec<_> = plan.owners().flat_map(|owner| &owner.values).collect();
    assert_eq!(values.len(), 1, "{values:#?}");
    values[0]
}

#[test]
fn a_while_test_value_becomes_a_repeated_owner_region() {
    let (file, core) = evaluation(
        "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion,
        "{:#?}",
        sole_value(&plan).schedule,
    );
}

#[test]
fn a_loop_body_value_still_becomes_owner_statements() {
    let (file, core) = evaluation(
        "let n = 0;\nwhile (n < 3) { const v = match (n) { 0 => 1, _ => 0 }; n = n + v; }\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion
    );
}

#[test]
fn a_switch_case_test_value_may_not_become_owner_statements() {
    let (file, core) = evaluation(
        "declare const n: number;\nswitch (n) { case match (n) { 1 => 1, _ => 0 }: break; }\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ConditionalInOwner),
    );
}

#[test]
fn a_destructuring_default_value_may_not_become_owner_statements() {
    let (file, core) = evaluation(
        "declare const source: { value?: number };\nconst { value = match (1) { 1 => 1, _ => 0 } } = source;\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ConditionalInOwner),
    );
}

#[test]
fn a_conditional_operation_owns_its_complete_active_branch() {
    // The `id` call sits between the `&&` and the value. The operation
    // owns that complete active branch so its captures stay inside the
    // conditional region.
    let (file, core) = evaluation(
        "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion,
    );
    let operation = &plan.owners[0].operations[0];
    assert!(operation.active_branch.is_some());
    assert_eq!(operation.active_steps.len(), 1);
}

#[test]
fn a_direct_conditional_branch_becomes_a_whole_operation() {
    let (file, core) = evaluation(
        "declare const flag: boolean;\nexport const short = flag && match (flag) { true => 1, _ => 0 };\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion
    );
    let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
    assert_eq!(operations.len(), 1, "{operations:#?}");
    assert_eq!(operations[0].kind, PlannedConditionalKind::LogicalAnd);
}

#[test]
fn both_ternary_branches_join_one_operation() {
    let (file, core) = evaluation(
        "declare const flag: boolean;\nexport const pick = flag ? match (1) { 1 => 1, _ => 0 } : match (2) { 2 => 2, _ => 0 };\n",
    );
    let plan = plan(&file, &core);
    let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
    assert_eq!(operations.len(), 1, "{operations:#?}");
    assert!(matches!(
        operations[0].kind,
        PlannedConditionalKind::Ternary {
            consequent: PlannedBranch::Value(_),
            alternate: PlannedBranch::Value(_),
        }
    ));
    assert_eq!(operations[0].values.len(), 2);
}

#[test]
fn a_later_value_depends_on_the_prior_conditional_operation_slot() {
    // The second value's prior argument contains the first tt value.
    // The complete conditional operation produces a slot, so the later
    // schedule never copies tt source.
    let (file, core) = evaluation(
        "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n",
    );
    let plan = plan(&file, &core);
    let values: Vec<_> = plan.owners().flat_map(|owner| &owner.values).collect();
    assert_eq!(values.len(), 2, "{values:#?}");
    assert_eq!(values[1].capability, TargetCapability::StatementRegion,);
    let operation = &plan.owners[0].operations[0];
    assert!(values[1].schedule.steps().iter().any(|step| {
            step.inputs.iter().any(|input| {
                matches!(input, PlannedEvaluationInput::Slot { slot, .. } if *slot == operation.result)
            })
        }));
}

#[test]
fn a_parameter_default_value_uses_the_expression_boundary_capability() {
    let (file, core) =
        evaluation("function f(x: number = match (1) { 1 => 1, _ => 0 }): number { return x; }\n");
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::OwnerTakesNoStatements),
    );
}

#[test]
fn a_direct_optional_call_argument_still_becomes_owner_statements() {
    let (file, core) = evaluation(
        "declare const f: ((v: number) => number) | undefined;\nf?.(match (1) { 1 => 1, _ => 0 });\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion
    );
}

#[test]
fn a_member_optional_call_becomes_a_whole_operation_through_its_receiver() {
    let (file, core) = evaluation(
        "declare const host: { f?: (v: number) => number };\nhost.f?.(match (1) { 1 => 1, _ => 0 });\n",
    );
    let plan = plan(&file, &core);
    assert_eq!(
        sole_value(&plan).capability,
        TargetCapability::StatementRegion
    );
    let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
    assert_eq!(operations.len(), 1, "{operations:#?}");
    assert!(matches!(
        operations[0].condition,
        PlannedEvaluationInput::Source {
            mode: EvaluationInputMode::MemberReference,
            receiver: Some(_),
            ..
        }
    ));
}

#[test]
fn an_inert_input_needs_no_capture_but_an_effectful_one_does() {
    // §9 capture elision: a literal's evaluation is unobservable, so it
    // stays in place; a call may do anything, so it is captured to
    // preserve its order against the hoisted region.
    let value_inputs = |source: &str| {
        let (file, core) = evaluation(source);
        let plan = plan(&file, &core);
        plan.owners()
            .flat_map(|owner| &owner.values)
            .flat_map(|value| value.schedule.steps())
            .flat_map(|step| &step.inputs)
            .filter(|input| {
                !matches!(
                    input,
                    PlannedEvaluationInput::Source {
                        mode: EvaluationInputMode::DirectReference
                            | EvaluationInputMode::MemberReference,
                        ..
                    }
                )
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    let elided = value_inputs("const a = [1, match (1) { 1 => 1, _ => 0 }];\n");
    assert!(
        elided
            .iter()
            .all(|input| matches!(input, PlannedEvaluationInput::Stable { .. })),
        "{elided:#?}"
    );

    let captured = value_inputs(
        "declare function eff(): number;\nconst a = [eff(), match (1) { 1 => 2, _ => 0 }];\n",
    );
    assert!(
        captured.iter().all(|input| matches!(
            input,
            PlannedEvaluationInput::Source {
                mode: EvaluationInputMode::Value,
                ..
            }
        )),
        "{captured:#?}"
    );

    // A completed call re-emits itself inside the dispatch, where the
    // authored position is gone, so its elided inputs also reserve a name
    // the completion can bind them to.
    let completed = value_inputs(
        "declare function g(a: number, b: number): void;\ng(1, match (1) { 1 => 1, _ => 0 });\n",
    );
    assert!(
        completed.iter().all(|input| matches!(
            input,
            PlannedEvaluationInput::Stable {
                reserved: Some(_),
                ..
            }
        )),
        "{completed:#?}"
    );
}

#[test]
fn validate_order_rejects_a_repeated_value_planned_into_its_owner() {
    let (file, core) = evaluation(
        "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
    );
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    // Break the plan the way a protocol bug would: keep the statement
    // capability but drop the loop operation that preserves repetition.
    plan.owners[0].values[0]
        .schedule
        .steps
        .retain(|step| step.operation != HostEvaluationOperation::LoopTest);
    let error = file.validate_order(&plan).expect_err("must be rejected");
    assert_eq!(error.invariant, crate::ice::Invariant::RepetitionRegionLeft);
    assert_eq!(error.stage, crate::ice::LoweringStage::EvaluationOrder);
}

#[test]
fn validate_order_rejects_a_conditional_region_capture() {
    let (file, core) = evaluation(
        "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n",
    );
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    // Break the plan by removing the operation that owns the active
    // branch while leaving its value statement-capable.
    plan.owners[0].operations.clear();
    let error = file.validate_order(&plan).expect_err("must be rejected");
    assert_eq!(
        error.invariant,
        crate::ice::Invariant::ConditionalRegionLeft
    );
}

#[test]
fn validate_order_rejects_a_capture_overlapping_a_tt_value() {
    let (file, core) = evaluation(
        "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n",
    );
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    // Break the dependency edge by turning the prior operation's slot
    // back into a raw source capture containing tt syntax.
    let operation = plan.owners[0].operations[0].clone();
    for step in &mut plan.owners[0].values[1].schedule.steps {
        for input in &mut step.inputs {
            let PlannedEvaluationInput::Slot { slot, mode } = *input else {
                continue;
            };
            if slot == operation.result {
                *input = PlannedEvaluationInput::Source {
                    source: operation.parent,
                    mode,
                    target: slot,
                    receiver: None,
                };
            }
        }
    }
    let error = file.validate_order(&plan).expect_err("must be rejected");
    assert_eq!(
        error.invariant,
        crate::ice::Invariant::EvaluationCountChanged
    );
}

#[test]
fn validate_order_rejects_values_out_of_source_order() {
    let (file, core) =
        evaluation("consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n");
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    plan.owners[0].values.swap(0, 1);
    // Isolate the ordinal contract: drop the slot dependency so only
    // the source-order inversion remains for the validator to see.
    for value in &mut plan.owners[0].values {
        for step in &mut value.schedule.steps {
            step.inputs
                .retain(|input| !matches!(input, PlannedEvaluationInput::Slot { .. }));
        }
    }
    let error = file.validate_order(&plan).expect_err("must be rejected");
    assert_eq!(
        error.invariant,
        crate::ice::Invariant::EvaluationOrderChanged,
        "{error}"
    );
}

#[test]
fn validate_order_rejects_a_slot_read_before_it_is_produced() {
    let (file, core) =
        evaluation("consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n");
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    // Point the second value's dependency at its own not-yet-produced slot.
    let ValueTarget::Slot(own) = plan.owners[0].values[1].target;
    for step in &mut plan.owners[0].values[1].schedule.steps {
        for input in &mut step.inputs {
            if let PlannedEvaluationInput::Slot { slot, .. } = input {
                *slot = own;
            }
        }
    }
    let error = file.validate_order(&plan).expect_err("must be rejected");
    assert_eq!(
        error.invariant,
        crate::ice::Invariant::ValueReadBeforeItIsProduced
    );
}

#[test]
fn validate_reference_rejects_a_receiverless_member_reference() {
    let (file, core) = evaluation(
        "declare const host: { f: (v: number) => number };\nhost.f(match (1) { 1 => 1, _ => 0 });\n",
    );
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    assert_eq!(
        plan.owners[0].values[0].capability,
        TargetCapability::StatementRegion
    );
    for step in &mut plan.owners[0].values[0].schedule.steps {
        for input in &mut step.inputs {
            if let PlannedEvaluationInput::Source {
                mode: EvaluationInputMode::MemberReference,
                receiver,
                ..
            } = input
            {
                *receiver = None;
            }
        }
    }
    let error = file
        .validate_reference(&plan)
        .expect_err("must be rejected");
    assert_eq!(error.invariant, crate::ice::Invariant::ReceiverLost);
    assert_eq!(error.stage, crate::ice::LoweringStage::EvaluationReference);
}

#[test]
fn validate_reference_rejects_a_reference_demoted_to_a_value_slot() {
    let (file, core) =
        evaluation("consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n");
    let mut plan = file.lowering_plan(&core).expect("lowering plan");
    for step in &mut plan.owners[0].values[1].schedule.steps {
        for input in &mut step.inputs {
            if let PlannedEvaluationInput::Slot { mode, .. } = input {
                *mode = EvaluationInputMode::MemberReference;
            }
        }
    }
    let error = file
        .validate_reference(&plan)
        .expect_err("must be rejected");
    assert_eq!(error.invariant, crate::ice::Invariant::ReferenceDemoted);
}
