use super::*;
use std::collections::BTreeSet;

fn syntax(source: &str) -> ProgramSyntax {
    syntax_kind(source, crate::SourceKind::TypeScript)
}

fn syntax_kind(source: &str, source_kind: crate::SourceKind) -> ProgramSyntax {
    let program = crate::parser::parse(source);
    let semantic = crate::analysis::coverage_semantics(source, &program, &[]);
    let core = crate::core_ir::lower_semantic(&semantic, source);
    ProgramSyntax::build(&semantic, &core, source, source_kind).expect("projection should parse")
}

fn build_error(source: &str) -> ProgramSyntaxError {
    let program = crate::parser::parse(source);
    let semantic = crate::analysis::coverage_semantics(source, &program, &[]);
    let core = crate::core_ir::lower_semantic(&semantic, source);
    ProgramSyntax::build(&semantic, &core, source, crate::SourceKind::TypeScript)
        .expect_err("projection should not parse")
}

#[test]
fn a_parse_failure_in_copied_text_is_the_source_not_an_internal_error() {
    // The byte the parse stopped on was copied from the source, so the
    // cause is the user's TypeScript — carried with the source byte it
    // is reported at.
    let source = "const x = match (s) { A(v) => { const q = ; return q; }, _ => 0 };\n";
    let error = build_error(source);
    let ProgramSyntaxError::SourceNotTypeScript { source: at, .. } = error else {
        panic!("expected a source-caused failure, got {error:?}");
    };
    assert_eq!(&source[at..at + 1], ";");
}

#[test]
fn an_incomplete_source_expression_owns_the_generated_closing_boundary() {
    // SWC reports this at the generated `)` after `radius.`, not on the
    // copied dot. The owner projection records that fixed delimiter as
    // the boundary of the copied arm expression, so malformed user text
    // remains an input failure instead of becoming an ICE.
    let source = "variant Shape { Circle(radius: number), Point }\n\
                      declare const shape: Shape;\n\
                      const label = match (shape) {\n\
                        Circle(radius) => radius.,\n\
                        Point => 0,\n\
                      };\n";
    let error = build_error(source);
    let ProgramSyntaxError::SourceNotTypeScript { source: at, .. } = error else {
        panic!("expected a source-caused failure, got {error:?}");
    };
    assert_eq!(&source[at..at + 1], ".");
}

#[test]
fn a_projected_byte_maps_only_through_copied_segments() {
    // A placeholder's bytes are the compiler's own text and belong to no
    // source byte: that is what separates an input fact from a broken
    // invariant.
    let source = "const value = match (s) { A(v) => v, _ => 0 };\n";
    let program = crate::parser::parse(source);
    let semantic = crate::analysis::coverage_semantics(source, &program, &[]);
    let core = crate::core_ir::lower_semantic(&semantic, source);
    let projection = ProjectionBuilder::new(&semantic, &core, source)
        .build()
        .expect("projection");
    let segments = &projection.source_segments;
    let placeholder = segments
        .iter()
        .find(|segment| segment.kind == ProjectionSegmentKind::Placeholder)
        .expect("a placeholder segment");
    let inside = ProjectedByte(placeholder.projected.start.0 + 1);
    assert_eq!(source_byte_for_projection(segments, inside), None);

    let copied = segments
        .iter()
        .find(|segment| segment.kind == ProjectionSegmentKind::Copied)
        .expect("a copied segment");
    assert_eq!(
        source_byte_for_projection(segments, copied.projected.start),
        Some(copied.source.start)
    );
}

#[test]
fn plain_typescript_projection_is_byte_identical() {
    let source = "// 주석\nconst value: { readonly x: number } = { x: 1 };\n";
    let syntax = syntax(source);
    assert_eq!(syntax.projection, source);
    assert!(syntax.overlay.is_empty());
}

#[test]
fn a_match_argument_keeps_its_call_parent_path() {
    let syntax = syntax(
        "variant E { A, B(x: number) }\nconst out = render(1, match (e) { A => 0, B(x) => x });\n",
    );
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert!(
        entry
            .parents
            .iter()
            .any(|parent| matches!(parent, AstParentKind::CallExpr(_))),
        "{:?}",
        entry.parents
    );
    assert_eq!(entry.context.continuation, HostContinuation::Compose);
}

#[test]
fn a_direct_return_is_a_function_statement_continuation() {
    let source = "variant E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n";
    let syntax = syntax(source);
    let context = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay")
        .context;
    assert_eq!(context.owner, EvaluationOwner::FunctionBody);
    assert_eq!(context.frequency, EvaluationFrequency::Once);
    assert_eq!(context.value_role, ValueRole::Value);
    assert_eq!(context.continuation, HostContinuation::Return);
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    let owner = entry.host_owner.span;
    assert_eq!(&source[owner.start..owner.start + 6], "return");
    assert_eq!(&source[owner.end - 1..owner.end], ";");
}

#[test]
fn an_expression_bodied_arrow_has_an_arrow_return_continuation() {
    let syntax = syntax("variant E { A, B }\nconst f = (e: E) => match (e) { A => 1, B => 2 };\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    let context = entry.context;
    assert_eq!(context.owner, EvaluationOwner::FunctionBody);
    assert_eq!(
        context.continuation,
        HostContinuation::ArrowReturn,
        "{:?}",
        entry.parents
    );
    assert!(entry.protocol.steps().is_empty(), "{:?}", entry.protocol);
}

#[test]
fn semicolon_free_concise_arrow_ends_before_the_next_try_statement() {
    let source = "type R<T> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: string };\n\
            declare const flag: boolean; declare function load(): R<number>;\n\
            function* probe() {\n\
              const choose = () => flag ? 1 : 2\n\
              try load();\n\
              yield choose();\n\
            }\n";
    let syntax = syntax(source);
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| matches!(entry.core_root, CoreRoot::Propagate(_)))
        .unwrap_or_else(|| {
            panic!(
                "try overlay\nprojection:\n{}\noverlay: {:#?}",
                syntax.projection, syntax.overlay
            )
        });
    assert_eq!(
        entry.context.owner,
        EvaluationOwner::Generator,
        "projection:\n{}\nparents: {:#?}",
        syntax.projection,
        entry.parents
    );
}

#[test]
fn a_nested_decision_keeps_the_outer_initializer_owner() {
    let syntax = syntax(
        "variant E { A, B }\nconst value = match (outer) { A => match (inner) { A => 1, B => 2 }, B => 0 };\n",
    );
    let entries = syntax
        .overlay
        .iter()
        .filter(|entry| matches!(entry.core_root, CoreRoot::Expr(_)))
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 2, "{:#?}", syntax.overlay);
    assert_eq!(
        entries[0].context.continuation,
        HostContinuation::Initialize
    );
    assert_eq!(entries[1].context.continuation, HostContinuation::Discard);
}

#[test]
fn a_conditional_branch_stays_conditional() {
    let syntax =
        syntax("variant E { A, B }\nconst out = flag ? match (e) { A => 1, B => 2 } : 0;\n");
    let context = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay")
        .context;
    assert_eq!(context.frequency, EvaluationFrequency::Conditional);
    assert_eq!(context.continuation, HostContinuation::Compose);
}

#[test]
fn an_eager_binary_rhs_has_an_explicit_evaluation_step() {
    let syntax = syntax("variant E { A, B }\nconst out = left + match (e) { A => 1, B => 2 };\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert_eq!(entry.context.frequency, EvaluationFrequency::Indeterminate);
    assert_eq!(entry.context.continuation, HostContinuation::Compose);
    assert!(entry.protocol.steps().iter().any(|step| {
        step.operation == HostEvaluationOperation::Eager(EagerPosition::BinaryRight)
    }));
}

#[test]
fn a_whole_variable_initializer_has_an_initialize_continuation() {
    let syntax = syntax("variant E { A, B }\nconst out = match (e) { A => 1, B => 2 };\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert_eq!(entry.context.continuation, HostContinuation::Initialize);
    assert!(entry.protocol.steps().is_empty());
}

#[test]
fn a_short_circuit_rhs_is_a_conditional_protocol_branch() {
    let syntax = syntax("variant E { A, B }\nconst out = ready && match (e) { A => 1, B => 2 };\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert!(entry.protocol.steps().iter().any(|step| {
        step.operation == HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
    }));
}

#[test]
fn a_member_call_preserves_the_reference_chain() {
    let syntax =
        syntax("variant E { A, B }\nconst out = (match (e) { A => left, B => right }).run();\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    let references: Vec<_> = entry
        .protocol
        .steps()
        .iter()
        .filter_map(|step| match step.operation {
            HostEvaluationOperation::Reference(reference) => Some(reference),
            _ => None,
        })
        .collect();
    assert_eq!(
        references,
        vec![
            ReferencePosition::MemberObject,
            ReferencePosition::CallCallee
        ]
    );
}

#[test]
fn a_call_argument_keeps_left_to_right_argument_position() {
    let syntax =
        syntax("variant E { A, B }\nconst out = render(first(), match (e) { A => 1, B => 2 });\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert!(entry.protocol.steps().iter().any(|step| {
        step.operation == HostEvaluationOperation::Eager(EagerPosition::CallArgument(1))
    }));
}

#[test]
fn an_await_operand_records_the_suspension_point() {
    let syntax = syntax(
        "variant E { A, B }\nasync function f(e: E) { return await match (e) { A => 1, B => 2 }; }\n",
    );
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert!(
        entry.protocol.steps().iter().any(|step| {
            step.operation == HostEvaluationOperation::Suspend(SuspensionKind::Await)
        })
    );
}

#[test]
fn a_parameter_initializer_is_a_composed_owner_value() {
    let syntax = syntax(
        "variant E { A, B }\nfunction f(e: E, x = match (e) { A => 1, B => 2 }) { return x; }\n",
    );
    let context = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay")
        .context;
    assert_eq!(context.owner, EvaluationOwner::ParameterInitializer);
    assert_eq!(context.continuation, HostContinuation::Compose);
}

#[test]
fn an_expression_in_a_loop_keeps_repeated_frequency() {
    let syntax =
        syntax("variant E { A, B }\nwhile (ready()) { consume(match (e) { A => 1, B => 2 }); }\n");
    let context = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay")
        .context;
    assert_eq!(context.frequency, EvaluationFrequency::Repeated);
}

#[test]
fn a_while_test_records_its_repeated_owner_protocol() {
    let syntax = syntax("variant E { A, B }\nwhile (match (e) { A => true, B => false }) {}\n");
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert_eq!(entry.context.owner_reach, OwnerReach::Repeated);
    assert!(entry.protocol.steps().iter().any(|step| {
        step.operation == HostEvaluationOperation::LoopTest && step.loop_test.is_some()
    }));
}

#[test]
fn every_core_surface_gets_a_typed_overlay() {
    let syntax = syntax(
        "variant E { A(x: number), B }\n\
             function f(e: E) {\n\
               if let A(x) = e { use(x); }\n\
               const A(y) = e else { return 0; };\n\
               const r = result { const z = try load(); return z; };\n\
               return match (e) { A(x) => x, B => r } |> done;\n\
             }\n",
    );
    assert!(
        syntax
            .overlay
            .iter()
            .any(|entry| entry.category == SyntaxCategory::Item)
    );
    assert!(
        syntax
            .overlay
            .iter()
            .any(|entry| entry.category == SyntaxCategory::Statement)
    );
    assert!(
        syntax
            .overlay
            .iter()
            .any(|entry| entry.category == SyntaxCategory::Expression)
    );
}

#[test]
fn mixed_syntax_matrix_covers_every_host_protocol_class() {
    fn operation_name(operation: HostEvaluationOperation) -> &'static str {
        match operation {
            HostEvaluationOperation::Eager(position) => match position {
                EagerPosition::BinaryLeft => "eager-binary-left",
                EagerPosition::BinaryRight => "eager-binary-right",
                EagerPosition::ArrayElement(_) => "eager-array-element",
                EagerPosition::ObjectEvaluation(_) => "eager-object-evaluation",
                EagerPosition::AssignmentRight => "eager-assignment-right",
                EagerPosition::SequenceElement(_) => "eager-sequence-element",
                EagerPosition::UnaryOperand => "eager-unary-operand",
                EagerPosition::CallArgument(_) => "eager-call-argument",
                EagerPosition::ConstructArgument(_) => "eager-construct-argument",
                EagerPosition::TemplateInterpolation(_) => "eager-template-interpolation",
                EagerPosition::JsxExpression(_) => "eager-jsx-expression",
            },
            HostEvaluationOperation::Conditional(branch) => match branch {
                ConditionalBranch::LogicalAndRight => "conditional-and-right",
                ConditionalBranch::LogicalOrRight => "conditional-or-right",
                ConditionalBranch::NullishRight => "conditional-nullish-right",
                ConditionalBranch::Consequent => "conditional-consequent",
                ConditionalBranch::Alternate => "conditional-alternate",
                ConditionalBranch::OptionalCallArgument(_) => "conditional-optional-call-argument",
            },
            HostEvaluationOperation::Reference(position) => match position {
                ReferencePosition::CallCallee => "reference-call-callee",
                ReferencePosition::OptionalCallCallee => "reference-optional-call-callee",
                ReferencePosition::MemberObject => "reference-member-object",
                ReferencePosition::MemberProperty => "reference-member-property",
                ReferencePosition::ConstructorCallee => "reference-constructor-callee",
                ReferencePosition::TaggedTemplateTag => "reference-tagged-template-tag",
            },
            HostEvaluationOperation::Suspend(kind) => match kind {
                SuspensionKind::Await => "suspend-await",
                SuspensionKind::Yield => "suspend-yield",
                SuspensionKind::YieldDelegate => "suspend-yield-delegate",
            },
            HostEvaluationOperation::LoopTest => "loop-test",
        }
    }

    fn owner_name(owner: EvaluationOwner) -> &'static str {
        match owner {
            EvaluationOwner::Module => "module",
            EvaluationOwner::FunctionBody => "function",
            EvaluationOwner::Constructor => "constructor",
            EvaluationOwner::Generator => "generator",
            EvaluationOwner::ParameterInitializer => "parameter",
            EvaluationOwner::ClassInitializer => "class-field",
            EvaluationOwner::StaticBlock => "static-block",
        }
    }

    fn continuation_name(continuation: HostContinuation) -> &'static str {
        match continuation {
            HostContinuation::Return => "return",
            HostContinuation::ArrowReturn => "arrow-return",
            HostContinuation::Initialize => "initialize",
            HostContinuation::ForInitialize => "for-initialize",
            HostContinuation::Discard => "discard",
            HostContinuation::Compose => "compose",
        }
    }

    fn reach_name(reach: OwnerReach) -> &'static str {
        match reach {
            OwnerReach::Same => "same",
            OwnerReach::Repeated => "repeated",
            OwnerReach::UnmodeledConditional => "unmodeled-conditional",
        }
    }

    let expression = "match (e) { A => 1, _ => 0 }";
    let cases = [
        (
            crate::SourceKind::TypeScript,
            format!("const x = ({expression}) + right();"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = left() + {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = [before(), {expression}];"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = {{ a: before(), b: {expression} }};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("target = {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = (before(), {expression});"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = !({expression});"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("call(before(), {expression});"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("new Box(before(), {expression});"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = `${{before()}}${{{expression}}}`;"),
        ),
        (
            crate::SourceKind::Tsx,
            format!("const x = <main data-x={{{expression}}}>{{{expression}}}</main>;"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ready && {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ready || {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ready ?? {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ready ? {expression} : 0;"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ready ? 0 : {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("fn?.(before(), {expression});"),
        ),
        (crate::SourceKind::TypeScript, format!("({expression})();")),
        (
            crate::SourceKind::TypeScript,
            format!("({expression})?.();"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ({expression}).value;"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = object[{expression}];"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = new ({expression})();"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = ({expression})`text`;"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("async function f() {{ return await {expression}; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("function* f() {{ yield {expression}; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("function* f() {{ yield* {expression}; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("while ({expression}) {{ break; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("function f() {{ return {expression}; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const f = () => {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("const x = {expression};"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("for (let x = {expression};;) {{ break; }}"),
        ),
        (crate::SourceKind::TypeScript, format!("{expression};")),
        (
            crate::SourceKind::TypeScript,
            format!("class C {{ constructor() {{ use({expression}); }} }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("function* f() {{ use({expression}); }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("function f(x = {expression}) {{}}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("class C {{ x = {expression}; }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("class C {{ static {{ use({expression}); }} }}"),
        ),
        (
            crate::SourceKind::TypeScript,
            format!("switch (value) {{ case {expression}: break; }}"),
        ),
    ];

    let mut operations = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let mut continuations = BTreeSet::new();
    let mut reaches = BTreeSet::new();
    for (kind, source) in cases {
        let syntax = syntax_kind(&source, kind);
        for entry in syntax.overlay.iter().filter(|entry| {
            entry.category == SyntaxCategory::Expression
                && matches!(entry.core_root, CoreRoot::Expr(_))
        }) {
            owners.insert(owner_name(entry.context.owner));
            continuations.insert(continuation_name(entry.context.continuation));
            reaches.insert(reach_name(entry.context.owner_reach));
            operations.extend(
                entry
                    .protocol
                    .steps()
                    .iter()
                    .map(|step| operation_name(step.operation)),
            );
        }
    }

    assert_eq!(
        operations,
        BTreeSet::from([
            "conditional-alternate",
            "conditional-and-right",
            "conditional-consequent",
            "conditional-nullish-right",
            "conditional-optional-call-argument",
            "conditional-or-right",
            "eager-array-element",
            "eager-assignment-right",
            "eager-binary-left",
            "eager-binary-right",
            "eager-call-argument",
            "eager-construct-argument",
            "eager-jsx-expression",
            "eager-object-evaluation",
            "eager-sequence-element",
            "eager-template-interpolation",
            "eager-unary-operand",
            "loop-test",
            "reference-call-callee",
            "reference-constructor-callee",
            "reference-member-object",
            "reference-member-property",
            "reference-optional-call-callee",
            "reference-tagged-template-tag",
            "suspend-await",
            "suspend-yield",
            "suspend-yield-delegate",
        ])
    );
    assert_eq!(
        owners,
        BTreeSet::from([
            "class-field",
            "constructor",
            "function",
            "generator",
            "module",
            "parameter",
            "static-block",
        ])
    );
    assert_eq!(
        continuations,
        BTreeSet::from([
            "arrow-return",
            "compose",
            "discard",
            "for-initialize",
            "initialize",
            "return",
        ])
    );
    assert_eq!(
        reaches,
        BTreeSet::from(["repeated", "same", "unmodeled-conditional"])
    );
}

#[test]
fn an_enclosing_tt_value_owns_the_outer_jsx_protocol_once() {
    let source = "import type { TResult } from \"@tt/std\";\ndeclare const outcome: TResult<string, string>;\nconst view = <aside>{match (result {\n  const value = try outcome;\n  return value;\n}) {\n  Ok(value) => <b>{value}</b>,\n  Err(error) => <code>{error}</code>,\n}}</aside>;\n";
    let syntax = syntax_kind(source, crate::SourceKind::Tsx);
    let expression_protocols: Vec<_> = syntax
        .overlay
        .iter()
        .filter(|entry| entry.category == SyntaxCategory::Expression)
        .map(|entry| entry.protocol.steps())
        .collect();
    assert_eq!(expression_protocols.len(), 2, "{:#?}", syntax.overlay);
    assert!(expression_protocols[0].iter().any(|step| {
        step.operation == HostEvaluationOperation::Eager(EagerPosition::JsxExpression(0))
    }));
    assert!(expression_protocols[1].is_empty(), "{:#?}", syntax.overlay);
}

#[test]
fn pipeline_head_and_match_share_structural_host_facts() {
    let source = "declare const flag: boolean, ready: boolean;\nfunction probe() { const x = ready && ((match (flag) { true => 1, false => 2 }) |> ((value) => value)); }\n";
    let parsed = syntax(source);
    assert_eq!(parsed.overlay.len(), 2, "{:#?}", parsed.overlay);
    assert_eq!(parsed.overlay[0].host_owner, parsed.overlay[1].host_owner);
    assert_eq!(parsed.overlay[0].protocol, parsed.overlay[1].protocol);
    assert!(
        parsed.overlay[0].source.start <= parsed.overlay[1].source.start
            && parsed.overlay[1].source.end <= parsed.overlay[0].source.end
    );

    let arrow_source = "declare const flag: boolean;\nconst probe = () => (match (flag) { true => 1, false => 2 }) |> ((value) => value);\n";
    let arrow = syntax(arrow_source);
    assert_eq!(arrow.overlay.len(), 2, "{:#?}", arrow.overlay);
    assert!(
        arrow
            .overlay
            .iter()
            .all(|entry| entry.host_owner.kind == HostOwnerKind::ArrowExpression)
    );
}

#[test]
fn a_try_declaration_gets_a_whole_propagation_overlay() {
    let syntax =
        syntax("function run(r: Result<number, string>) {\n  const n = try r;\n  return n;\n}\n");
    assert!(syntax.overlay.iter().any(|entry| {
        entry.category == SyntaxCategory::Propagation
            && matches!(entry.core_root, CoreRoot::Propagate(_))
    }));
}

#[test]
fn expressions_in_one_statement_share_one_stable_owner() {
    let syntax = syntax(
        "const out = [match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 }];\n",
    );
    let owners: Vec<_> = syntax
        .overlay
        .iter()
        .filter(|entry| entry.category == SyntaxCategory::Expression)
        .map(|entry| entry.host_owner.id)
        .collect();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0], owners[1]);
    let owner = syntax.owners().next().expect("statement owner");
    assert_eq!(owner.roots.len(), 2);
}

#[test]
fn a_nested_statement_is_a_distinct_owner() {
    let syntax = syntax(
        "const out = [match (left) { A => 1, _ => 0 }, () => { return match (right) { B => 2, _ => 0 }; }];\n",
    );
    let owners: Vec<_> = syntax
        .overlay
        .iter()
        .filter(|entry| entry.category == SyntaxCategory::Expression)
        .map(|entry| entry.host_owner.id)
        .collect();
    assert_eq!(owners.len(), 2);
    assert_ne!(owners[0], owners[1]);
}

#[test]
fn a_result_statement_island_exposes_its_nested_initializer_owner() {
    let syntax = syntax(
        "const outer = result {\n  const x = try f();\n  const inner = result { const y = try g(); return y; };\n  return inner;\n};\n",
    );
    let mut entries: Vec<_> = syntax
        .overlay
        .iter()
        .filter(|entry| matches!(entry.core_root, CoreRoot::Expr(_)))
        .collect();
    entries.sort_unstable_by_key(|entry| entry.source.start);
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].context.continuation,
        HostContinuation::Initialize
    );
    assert_eq!(
        entries[1].context.continuation,
        HostContinuation::Initialize
    );
    assert_ne!(entries[0].host_owner.id, entries[1].host_owner.id);
}

#[test]
fn multibyte_source_and_projection_coordinates_do_not_mix() {
    let source = "const 한글 = match (e) { A => \"안녕\", _ => \"끝\" };\n";
    let syntax = syntax(source);
    let entry = syntax
        .overlay
        .iter()
        .find(|entry| entry.category == SyntaxCategory::Expression)
        .expect("match overlay");
    assert_eq!(entry.source.start, source.find("match").expect("match"));
    let projected = &syntax.projection[entry.projected.start.0..entry.projected.end.0];
    assert!(projected.starts_with("(() => {"), "{projected}");
    assert!(projected.contains("\"안녕\""), "{projected}");
    assert!(projected.contains("\"끝\""), "{projected}");
}
