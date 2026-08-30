//! Surface-independent IR for tt-owned semantics.
//!
//! Pattern-bearing syntax is normalized to [`Decision`]; Result propagation
//! is normalized to [`Propagate`]. The IR refers to HIR and resolution by ID,
//! never by reparsing source text. TypeScript-specific statement shapes are
//! introduced by the next lowering stage.

mod lower;

use crate::hir;
use crate::hir::ids::Idx;
use crate::hir::{ArmBodyKind, BindingMode, BindingText, BodyId, ExprId, FieldId, NodeId};
use crate::resolve::{FieldRef, VariantRef};

pub(crate) use lower::lower_semantic;

#[derive(Debug)]
pub(crate) struct CoreFile {
    pub root: BodyId,
    pub bodies: Vec<Body>,
    pub exprs: Vec<Expr>,
    pub temporary_count: u32,
}

impl CoreFile {
    /// Whether this file contains a Core primitive that needs a TypeScript
    /// execution owner. Source-only import edits do not require host lowering.
    pub(crate) fn requires_host_lowering(&self) -> bool {
        self.body_requires_host(self.root)
    }

    fn body_requires_host(&self, body: BodyId) -> bool {
        self.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Opaque(_) | Statement::Import(_) => false,
                Statement::Adt(_) | Statement::Propagate(_) | Statement::Decision(_) => true,
                Statement::Expr(expr) => self.expr_requires_host(*expr),
            })
    }

    /// Whether a value expression has a statement form: a lowering that
    /// writes its result to a slot through ordinary TypeScript control flow
    /// rather than through an expression boundary.
    ///
    /// This is a fact about the Core shape, so Core owns it. Both the
    /// Evaluation IR (deciding a value's target capability) and target
    /// lowering (structuring a nested value under its parent's
    /// continuation) read it from here instead of each deciding it again.
    pub(crate) fn has_statement_form(&self, expr: ExprId) -> bool {
        match &self.exprs[expr.index()] {
            // Every arm must be able to deliver a value to a continuation.
            Expr::Decision(decision) => decision
                .arms
                .iter()
                .all(|arm| matches!(arm.action, ArmAction::Yield { .. })),
            Expr::ResultRegion(_) => true,
            Expr::Propagate(_) => true,
            Expr::Sequence(body) => self
                .body_value_expr(*body)
                .is_some_and(|inner| self.has_statement_form(inner)),
            // An optional postfix owns the conditional reach of every value
            // in its tail. Those values must stay at an expression boundary
            // inside that tail instead of being lifted into the Apply region.
            Expr::Apply(apply) => apply.head.is_some_and(|head| {
                self.has_statement_form(head)
                    || apply.steps.iter().any(|step| {
                        !matches!(step.mode, ApplyMode::Postfix { optional: true })
                            && self.has_statement_form(step.value)
                    })
            }),
            Expr::Opaque(_) | Expr::Template(_) => false,
        }
    }

    /// The value a sequence body delivers: its last value statement, when
    /// nothing but source trivia follows it.
    pub(crate) fn body_value_expr(&self, body: BodyId) -> Option<ExprId> {
        sequence_value(&self.bodies[body.index()].statements).map(|(_, expr)| expr)
    }

    fn expr_requires_host(&self, expr: ExprId) -> bool {
        match &self.exprs[expr.index()] {
            Expr::Opaque(_) => false,
            Expr::Sequence(body) => self.body_requires_host(*body),
            Expr::Decision(_) | Expr::Propagate(_) | Expr::Apply(_) | Expr::ResultRegion(_) => true,
            Expr::Template(template) => template.parts.iter().any(|part| match part {
                TemplatePart::Raw(_) => false,
                TemplatePart::Interpolation(expr) => self.expr_requires_host(*expr),
            }),
        }
    }
}

/// The index and expression of the value a statement sequence delivers.
pub(crate) fn sequence_value(statements: &[Statement]) -> Option<(usize, ExprId)> {
    let (index, expr) = statements
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, statement)| match statement {
            Statement::Expr(expr) => Some((index, *expr)),
            _ => None,
        })?;
    statements[index + 1..]
        .iter()
        .all(|statement| matches!(statement, Statement::Opaque(_)))
        .then_some((index, expr))
}

#[derive(Debug)]
pub(crate) struct Body {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub(crate) enum Statement {
    Opaque(NodeId),
    Adt(Adt),
    Import(Import),
    Propagate(Propagate),
    Decision(Decision),
    Expr(ExprId),
}

#[derive(Debug)]
pub(crate) enum Expr {
    Opaque(NodeId),
    Sequence(BodyId),
    Decision(Decision),
    Propagate(Propagate),
    Apply(Apply),
    ResultRegion(ResultRegion),
    Template(Template),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TempId {
    /// File-unique `$tt_tN`, shared by `try` and statement decisions.
    Statement(u32),
    /// File-unique `$tt_rN`, sharing the same ordinal space.
    Result(u32),
    /// A single-subject decision's local (`$tt_m`).
    Decision,
    /// One position of a tuple decision (`$tt_mN`).
    DecisionElement(u32),
}

#[derive(Debug)]
pub(crate) struct Decision {
    pub subjects: Vec<Subject>,
    pub arms: Vec<DecisionArm>,
    pub miss: MissAction,
    pub head: NodeId,
    pub extent: NodeId,
    pub is_async: bool,
    pub kind: DecisionKind,
}

#[derive(Debug)]
pub(crate) enum DecisionKind {
    Match {
        dispatch: MatchDispatch,
        needs_label: bool,
    },
    IfLet,
    LetElse {
        binding_mode: BindingMode,
        direct_variants: Option<Vec<Constructor>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchDispatch {
    Conditional,
    VariantSwitch,
    LiteralSwitch,
}

#[derive(Debug)]
pub(crate) struct Subject {
    pub value: ExprId,
    pub temporary: TempId,
}

#[derive(Debug)]
pub(crate) struct DecisionArm {
    pub pattern: PatternPlan,
    pub guard: Option<ExprId>,
    pub action: ArmAction,
}

#[derive(Debug)]
pub(crate) enum PatternPlan {
    Any,
    Test(Test),
    Bind(Bind),
    AllOf(Vec<PatternPlan>),
    AnyOf(Vec<PatternPlan>),
}

#[derive(Debug)]
pub(crate) enum Test {
    Variant {
        place: Place,
        constructor: Constructor,
    },
    Literal {
        place: Place,
        pattern: crate::hir::PatternId,
    },
    /// JavaScript class identity test. The constructor is copied from its
    /// source node so namespaces and local bindings retain ordinary
    /// TypeScript name resolution.
    InstanceOf { place: Place, constructor: NodeId },
}

#[derive(Debug, Clone)]
pub(crate) enum Constructor {
    Resolved { reference: VariantRef, node: NodeId },
    Recovery { node: NodeId, name: String },
}

#[derive(Debug)]
pub(crate) struct Import {
    pub specifier: NodeId,
    pub kind: hir::ImportKind,
}

#[derive(Debug)]
pub(crate) struct Adt {
    pub node: NodeId,
    pub name: String,
    pub exported: bool,
    pub generics: String,
    pub variants: Vec<AdtVariant>,
}

#[derive(Debug)]
pub(crate) struct AdtVariant {
    pub name: String,
    pub fields: Option<Vec<AdtField>>,
    pub emit_constructor: bool,
}

#[derive(Debug)]
pub(crate) struct AdtField {
    pub name: String,
    pub optional: bool,
    pub ty_text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Place {
    pub subject: usize,
    pub fields: Vec<FieldAccess>,
}

#[derive(Debug, Clone)]
pub(crate) enum FieldAccess {
    Resolved { reference: FieldRef, node: NodeId },
    Recovery { node: NodeId, name: String },
}

#[derive(Debug)]
pub(crate) struct Bind {
    pub source: Place,
    pub source_field: Option<FieldId>,
    pub binding: NodeId,
}

#[derive(Debug)]
pub(crate) enum ArmAction {
    Yield { body: BodyId, kind: ArmBodyKind },
    Execute(BodyId),
    BindThrough(BindingMode),
}

#[derive(Debug)]
pub(crate) enum MissAction {
    ThrowUnexpected(UnexpectedKind),
    Execute(BodyId),
    Decision(Box<Decision>),
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnexpectedKind {
    Case,
    Literal,
    Tuple,
}

#[derive(Debug)]
pub(crate) struct Propagate {
    pub node: NodeId,
    pub owner: NodeId,
    pub value: ExprId,
    pub temporary: TempId,
    pub binding: Option<BindingText>,
    pub exit: ExitTarget,
    pub layout: ResultLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitTarget {
    EnclosingFunction,
    ResultRegion(ResultRegionId),
}

/// Stable identity for a lexical Result region. The HIR node is stable for
/// one snapshot, which is the lifetime of every Core and codegen plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ResultRegionId(pub(crate) NodeId);

/// The structural Result ABI. It is fixed once in semantic lowering rather
/// than rediscovered by each backend emission site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResultLayout {
    pub discriminator: ResultDiscriminator,
    pub payload_field: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultDiscriminator {
    SuccessFieldPresent(&'static str),
}

pub(crate) const RESULT_LAYOUT: ResultLayout = ResultLayout {
    discriminator: ResultDiscriminator::SuccessFieldPresent("value"),
    payload_field: "value",
};

#[derive(Debug)]
pub(crate) struct Apply {
    pub node: NodeId,
    pub head: Option<ExprId>,
    pub steps: Vec<ApplyStep>,
}

#[derive(Debug)]
pub(crate) struct ApplyStep {
    pub node: NodeId,
    pub value: ExprId,
    pub mode: ApplyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyMode {
    Call,
    Postfix { optional: bool },
}

#[derive(Debug)]
pub(crate) struct ResultRegion {
    pub id: ResultRegionId,
    pub node: NodeId,
    pub items: Vec<ResultRegionItem>,
    pub value: Option<ExprId>,
    pub is_async: bool,
}

#[derive(Debug)]
pub(crate) enum ResultRegionItem {
    Statements(BodyId),
}

#[derive(Debug)]
pub(crate) struct Template {
    pub node: NodeId,
    pub parts: Vec<TemplatePart>,
}

#[derive(Debug)]
pub(crate) enum TemplatePart {
    Raw(NodeId),
    Interpolation(ExprId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower(source: &str) -> CoreFile {
        let program = crate::parser::parse(source);
        let semantic = crate::analysis::coverage_semantics(&program, &[]);
        lower_semantic(&semantic, source)
    }

    #[test]
    fn every_pattern_surface_is_one_decision_ir() {
        let source = "variant E { A(value: number), B }\n\
            function f(e: E) {\n\
              if let A(value) = e { use(value); }\n\
              const A(value) = e else { return 0; };\n\
              return match (e) { A(value) => value, B => 0 };\n\
            }\n";
        let core = lower(source);
        let statement_decisions = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .filter(|statement| matches!(statement, Statement::Decision(_)))
            .count();
        let expression_decisions = core
            .exprs
            .iter()
            .filter(|expr| matches!(expr, Expr::Decision(_)))
            .count();
        assert_eq!((statement_decisions, expression_decisions), (2, 1));
    }

    #[test]
    fn return_try_in_a_result_body_targets_its_nearest_region() {
        let source = "const value = result { return try read(); };\n";
        let core = lower(source);
        let region = core
            .exprs
            .iter()
            .find_map(|expr| match expr {
                Expr::ResultRegion(region) => Some(region),
                _ => None,
            })
            .expect("result region");
        let body = region
            .items
            .iter()
            .map(|item| {
                let ResultRegionItem::Statements(body) = item;
                *body
            })
            .next()
            .expect("result statement body");
        let propagate = core.bodies[body.index()]
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Expr(expr) => match &core.exprs[expr.index()] {
                    Expr::Propagate(propagate) => Some(propagate),
                    _ => None,
                },
                _ => None,
            })
            .expect("return try propagation");
        assert_eq!(propagate.exit, ExitTarget::ResultRegion(region.id));
    }

    #[test]
    fn nested_function_try_keeps_its_function_target() {
        let source = "const value = result {\n  const inner = () => { return Result.Ok(try step()); };\n  return try inner();\n};\n";
        let core = lower(source);
        let exits: Vec<_> = core
            .exprs
            .iter()
            .filter_map(|expr| match expr {
                Expr::Propagate(propagate) => Some(propagate.exit),
                _ => None,
            })
            .collect();
        assert!(exits.contains(&ExitTarget::EnclosingFunction), "{core:?}");
        assert!(
            exits
                .iter()
                .any(|exit| matches!(exit, ExitTarget::ResultRegion(_))),
            "{core:?}"
        );
    }

    #[test]
    fn resolved_patterns_carry_definition_identity() {
        let source = "variant E { A(value: number), B }\n\
            const n = match (e) { A(value) => value, B => 0 };\n";
        let core = lower(source);
        let decision = core
            .exprs
            .iter()
            .find_map(|expr| match expr {
                Expr::Decision(decision) => Some(decision),
                _ => None,
            })
            .expect("decision");
        assert!(decision.arms.iter().all(|arm| {
            fn resolved(plan: &PatternPlan) -> bool {
                match plan {
                    PatternPlan::Any | PatternPlan::Bind(_) => true,
                    PatternPlan::Test(Test::Variant { constructor, .. }) => {
                        matches!(constructor, Constructor::Resolved { .. })
                    }
                    PatternPlan::Test(Test::Literal { .. } | Test::InstanceOf { .. }) => true,
                    PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
                        parts.iter().all(resolved)
                    }
                }
            }
            resolved(&arm.pattern)
        }));
    }

    #[test]
    fn execution_shape_is_fixed_before_target_lowering() {
        let source = "async function f(e: E) {\n\
            const x = match (e) { A => await read(), _ => 0 };\n\
            return result { const y = try await parse(x); return y; };\n\
        }\n";
        let core = lower(source);
        assert!(
            core.exprs
                .iter()
                .any(|expr| { matches!(expr, Expr::Decision(Decision { is_async: true, .. })) })
        );
        assert!(core.exprs.iter().any(|expr| {
            matches!(
                expr,
                Expr::ResultRegion(ResultRegion { is_async: true, .. })
            )
        }));
    }

    #[test]
    fn target_metadata_is_present_for_every_generated_surface() {
        let source = "const a = try read();\n\
            const p = value |> step;\n\
            const r = result { const b = try parse(a); return b; };\n";
        let core = lower(source);
        let propagate = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .find_map(|statement| match statement {
                Statement::Propagate(propagate) => Some(propagate),
                _ => None,
            })
            .expect("propagate");
        assert!(matches!(propagate.temporary, TempId::Statement(_)));
        assert!(core.exprs.iter().any(|expr| {
            matches!(expr, Expr::Apply(Apply { steps, .. }) if steps.iter().all(|step| step.node.0 > 0))
        }));
        assert!(
            core.exprs
                .iter()
                .any(|expr| matches!(expr, Expr::ResultRegion(_)))
        );
    }

    #[test]
    fn decision_tree_preserves_boolean_pattern_structure() {
        let source = "const n = match (left, right) {\n\
            (A | B, C | D) => 1,\n\
            _ => 0,\n\
        };\n";
        let core = lower(source);
        let decision = core
            .exprs
            .iter()
            .find_map(|expr| match expr {
                Expr::Decision(decision) => Some(decision),
                _ => None,
            })
            .expect("decision");
        let PatternPlan::AllOf(elements) = &decision.arms[0].pattern else {
            panic!("tuple pattern must remain a conjunction");
        };
        assert_eq!(elements.len(), 2);
        assert!(
            elements
                .iter()
                .all(|element| matches!(element, PatternPlan::AnyOf(alts) if alts.len() == 2))
        );
    }

    #[test]
    fn match_dispatch_is_fixed_before_target_lowering() {
        let source = "variant E { A, B }\n\
            const tagged = match (e) { A => 1, B => 2 };\n\
            const literal = match (n) { 0 => 1, _ => 2 };\n\
            const nested = match (e) { A => 1, B if ready => 2, _ => 3 };\n";
        let core = lower(source);
        let dispatches = core
            .exprs
            .iter()
            .filter_map(|expr| match expr {
                Expr::Decision(Decision {
                    kind: DecisionKind::Match { dispatch, .. },
                    ..
                }) => Some(*dispatch),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            dispatches,
            vec![
                MatchDispatch::VariantSwitch,
                MatchDispatch::LiteralSwitch,
                MatchDispatch::Conditional,
            ]
        );
    }

    #[test]
    fn statement_decision_kind_is_fixed_before_target_lowering() {
        let source = "variant E { A(value: number), B }\n\
            if let A(value) = e { use(value); }\n\
            const A(value) | B = e else { return; };\n";
        let core = lower(source);
        let kinds = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .filter_map(|statement| match statement {
                Statement::Decision(decision) => Some(&decision.kind),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(matches!(kinds[0], DecisionKind::IfLet));
        assert!(matches!(
            kinds[1],
            DecisionKind::LetElse {
                direct_variants: Some(variants),
                ..
            } if variants.len() == 2
        ));
    }
}
