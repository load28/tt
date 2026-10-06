//! Target-placement diagnostics projected from an Evaluation IR lowering plan.

use crate::{DiagnosticCode, TtError, evaluation_ir, program_syntax};

fn try_target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    plan.unsupported_expression_propagations()
        .into_iter()
        .map(|failure| {
            let (message, help) = try_placement_message(failure.owner, failure.reason);
            TtError::span(
                failure.source.start,
                failure.source.end,
                message.to_string(),
            )
            .code(DiagnosticCode::TryPlacement)
            .help(help)
        })
        .collect()
}

fn match_target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    plan.unsupported_matches()
        .into_iter()
        .map(|failure| {
            let (message, help) = match_placement_message(failure.owner, failure.reason);
            TtError::span(
                failure.source.start,
                failure.source.end,
                message.to_string(),
            )
            .code(DiagnosticCode::MatchPlacement)
            .help(help)
        })
        .collect()
}

fn lexical_declaration_body_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    plan.lexical_declaration_bodies()
        .iter()
        .map(|body| {
            let (construct, code) = match body.statement {
                evaluation_ir::BindingStatement::LetElse => {
                    ("a let-else", DiagnosticCode::LetElsePlacement)
                }
                evaluation_ir::BindingStatement::Try => {
                    ("a `try` statement", DiagnosticCode::TryPlacement)
                }
            };
            TtError::span(
                body.source.start,
                body.source.end,
                format!(
                    "{construct} declaring `const` or `let` cannot be the unbraced body of an \
                     `if`, loop, or label — TypeScript allows no lexical declaration there"
                ),
            )
            .code(code)
            .help("wrap the statement in braces to give its binding a block")
        })
        .collect()
}

const LOGICAL_ASSIGNMENT_HELP: &str = "write the logical assignment as a statement of its own \
     (`target ??= value;`), then read the target where its value was used";

fn match_placement_message(
    owner: program_syntax::EvaluationOwner,
    reason: evaluation_ir::ExpressionBoundaryReason,
) -> (&'static str, &'static str) {
    use evaluation_ir::ExpressionBoundaryReason as Reason;
    use program_syntax::EvaluationOwner;
    let help =
        "move the match into a function-body statement that can own its generated control flow";
    match (owner, reason) {
        (EvaluationOwner::ParameterInitializer, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in a parameter initializer — this TypeScript boundary has no statement position",
            help,
        ),
        (EvaluationOwner::ClassInitializer, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in a class field initializer — this TypeScript boundary has no statement position",
            help,
        ),
        (EvaluationOwner::ClassDefinition, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in a decorator, a computed member name, or a decorated class's heritage — the class definition evaluates it with no statement position",
            help,
        ),
        (EvaluationOwner::EnumInitializer, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in an enum member initializer — the enum evaluates its members in order, in a scope where member names denote members, with no statement position",
            help,
        ),
        (_, Reason::RepeatedInOwner) => (
            "`match` cannot be lowered from this repeated loop position without changing how often it evaluates",
            help,
        ),
        (_, Reason::LoopHeadDeclarator) => (
            "`match` cannot be lowered from a later declarator of a `for` loop head — its statements would run before the earlier declarators and outside the bindings the head declares",
            "move the declaration before the loop, or make this declarator the first one",
        ),
        (_, Reason::LoopHeadBinding) => (
            "`match` cannot be lowered from a `for` loop head whose initializer refers to a binding the head declares — its statements would run before the loop, where that name does not denote the head's binding",
            "declare the binding before the loop, or compute the value in the loop body",
        ),
        (_, Reason::ConditionalInOwner | Reason::ConditionalOperationNotStructurable) => (
            "`match` cannot be lowered from this conditional expression position without evaluating a skipped branch",
            help,
        ),
        (_, Reason::LogicalAssignmentValue) => (
            "`match` cannot be lowered in the right operand of a logical assignment whose value is used — the operand runs only when the target's value does not decide the result, and no statement form reads the target once, skips the operand, and keeps TypeScript's narrowing of the target",
            LOGICAL_ASSIGNMENT_HELP,
        ),
        (_, Reason::ReferenceNotPreservable) => (
            "`match` cannot be lowered from this reference position while preserving its receiver and `this`",
            help,
        ),
        (_, Reason::CaptureOverlapsValue) => (
            "`match` cannot be lowered from this expression because its ordered source captures overlap",
            help,
        ),
        (_, Reason::ValueHasNoStatementForm | Reason::OwnerTakesNoStatements) => (
            "`match` cannot be lowered in this TypeScript host because it has no sound statement region",
            help,
        ),
    }
}

pub(super) fn target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    let mut errors = try_target_errors(plan);
    errors.extend(match_target_errors(plan));
    errors.extend(lexical_declaration_body_errors(plan));
    errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
    errors
}

pub(super) fn nonredundant_target_errors(
    plan: &evaluation_ir::LoweringPlan,
    existing: &[TtError],
) -> Vec<TtError> {
    target_errors(plan)
        .into_iter()
        .filter(|target| {
            !(target.code == DiagnosticCode::TryPlacement
                && target.offset.is_some()
                && existing.iter().any(|prior| {
                    prior.code == DiagnosticCode::TryCrossesValueRegion
                        && prior.offset == target.offset
                        && prior.end == target.end
                }))
        })
        .collect()
}

fn try_placement_message(
    owner: program_syntax::EvaluationOwner,
    reason: evaluation_ir::ExpressionBoundaryReason,
) -> (&'static str, &'static str) {
    use evaluation_ir::ExpressionBoundaryReason as Reason;
    use program_syntax::EvaluationOwner;

    let help = "move the propagation into the nearest function-body statement with \
                `const value = try <expression>;`";
    match (owner, reason) {
        (EvaluationOwner::Module, _) => crate::diagnostics::TRY_OUTSIDE_FUNCTION,
        (EvaluationOwner::StaticBlock, _) => (
            "`try` cannot be used in a class static block — it has no enclosing function \
             failure edge for its `Err` propagation",
            help,
        ),
        (_, Reason::RepeatedInOwner) => (
            "`try` cannot be used in a repeated loop position — propagating its `Err` \
             across this TypeScript control-flow boundary would run once per iteration",
            help,
        ),
        (_, Reason::LoopHeadDeclarator) => (
            "`try` cannot be used in a later declarator of a `for` loop head — its \
             propagation would run before the earlier declarators and outside the bindings \
             the head declares",
            "move the declaration before the loop, or make this declarator the first one",
        ),
        (_, Reason::LoopHeadBinding) => (
            "`try` cannot be used in a `for` loop head whose initializer refers to a binding \
             the head declares — its propagation would run before the loop, where that name \
             does not denote the head's binding",
            "declare the binding before the loop, or compute the value in the loop body",
        ),
        (EvaluationOwner::ParameterInitializer, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in a parameter initializer — this TypeScript control-flow \
             boundary has no statement position for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::ClassInitializer, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in a class field initializer — this TypeScript control-flow \
             boundary has no statement position for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::ClassDefinition, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in a decorator, a computed member name, or a decorated \
             class's heritage — the class definition evaluates it with no statement position \
             for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::EnumInitializer, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in an enum member initializer — the enum evaluates its \
             members with no statement position for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::Constructor, _) => (
            "`try` cannot be used in a constructor — its `Err` propagation requires an \
             ordinary function return",
            help,
        ),
        (EvaluationOwner::Setter, _) => (
            "`try` cannot be used in a setter — a setter's return value is discarded, so its \
             `Err` propagation could not reach the caller",
            help,
        ),
        (EvaluationOwner::Generator, _) => (
            "`try` cannot be used in a generator — its `Err` propagation requires an \
             ordinary function return",
            help,
        ),
        (_, Reason::ConditionalInOwner) => (
            "`try` cannot be used in a conditionally evaluated expression position — \
             propagating its `Err` across this TypeScript control-flow boundary would \
             evaluate it when the surrounding expression skips it",
            help,
        ),
        (_, Reason::ConditionalOperationNotStructurable) => (
            "`try` cannot be used in this conditional operation — its TypeScript control-flow \
             boundary cannot be rebuilt without changing evaluation order",
            help,
        ),
        (_, Reason::LogicalAssignmentValue) => (
            "`try` cannot be used in the right operand of a logical assignment whose value is \
             used — the operand runs only when the target's value does not decide the result, \
             and no statement form reads the target once, skips the operand, and keeps \
             TypeScript's narrowing of the target",
            LOGICAL_ASSIGNMENT_HELP,
        ),
        (_, Reason::CaptureOverlapsValue) => (
            "`try` cannot be used in this expression context — its TypeScript control-flow \
             boundary requires overlapping source captures",
            help,
        ),
        (_, Reason::ReferenceNotPreservable) => (
            "`try` cannot be used in this expression context — its TypeScript control-flow \
             boundary requires preserving a reference that statements cannot represent",
            help,
        ),
        (_, Reason::ValueHasNoStatementForm) | (_, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in this expression context — propagating its `Err` \
             would require moving an evaluation across its TypeScript control-flow boundary",
            help,
        ),
    }
}
