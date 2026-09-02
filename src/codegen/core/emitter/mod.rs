//! Shared emitter state and continuation protocol.

mod expression;
mod helpers;
mod host;
mod pattern;
mod result;
mod source;

use super::*;
use helpers::*;

pub(super) struct Emitter<'a> {
    pub(super) semantic: &'a SemanticFile,
    pub(super) core: &'a CoreFile,
    pub(super) source: &'a str,
    pub(super) source_kind: SourceKind,
    pub(super) direct_apply_inputs: HashSet<ExprId>,
    pub(super) rewrite_imports: ImportRewrite,
    pub(super) std_imports: StdImports<'a>,
    pub(super) owner_slot_rewrites: Vec<OwnerSlotRewrite>,
    pub(super) for_initializer_propagations: Vec<ForInitializerPropagationRewrite>,
    pub(super) compose_rewrites: Vec<ComposeRewrite>,
    pub(super) loop_test_rewrites: Vec<LoopTestRewrite>,
    pub(super) source_replacements: Vec<SourceReplacement>,
    pub(super) consumed_exprs: HashSet<ExprId>,
    pub(super) arrow_return_rewrites: Vec<ArrowReturnRewrite>,
    pub(super) slot_exprs: HashMap<ExprId, String>,
    pub(super) value_slots: HashMap<ExprId, String>,
    pub(super) scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    pub(super) value_exits: HashMap<ExprId, Vec<HostExit>>,
    pub(super) nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    pub(super) nested_values: HashSet<ExprId>,
    pub(super) structurally_nested_values: HashSet<ExprId>,
    pub(super) recovered_propagations: HashSet<ExprId>,
    pub(super) expression_boundary_name: String,
    /// How many conditional-operation regions are being emitted right now.
    /// Inside one, the operation's own host replacement does not apply —
    /// the region re-emits the operation's fragments itself.
    pub(super) conditional_region_depth: Cell<u32>,
    /// Structured owner-slot emission reads the value's authored children.
    /// Suppress only that value's own replacement; nested structured values
    /// must still replace their host occurrences compositionally.
    pub(super) active_structured_exprs: ActiveExprStack,
    pub(super) active_scheduled_exprs: ActiveExprStack,
    /// Owner preludes can be reached either through an opaque source prefix
    /// or through the Core expression entry. Record which path emitted the
    /// prelude so the other path contributes only the join-slot occurrence.
    pub(super) emitted_owner_rewrites: EmittedOwnerRewrites,
    /// Loop-test actions emit their tt values before the rebuilt source test.
    /// Host replacements apply only to that source test, not while the
    /// actions recursively emit their own source fragments.
    pub(super) loop_region_depth: Cell<u32>,
    pub(super) used_expression_boundary: Cell<bool>,
    pub(super) used_pipe: Cell<bool>,
    pub(super) used_flow: Cell<bool>,
}

/// A recursion stack whose guard never holds a `RefCell` borrow while target
/// emission calls back into itself. This state belongs to the emitter rather
/// than a single Core walk because opaque source scanning and structured
/// expression emission can enter one another in either direction.
#[derive(Default)]
pub(super) struct ActiveExprStack {
    exprs: RefCell<Vec<ExprId>>,
}

impl ActiveExprStack {
    fn contains(&self, expr: ExprId) -> bool {
        self.exprs.borrow().contains(&expr)
    }

    fn enter(&self, expr: ExprId) -> ActiveExprGuard<'_> {
        self.exprs.borrow_mut().push(expr);
        ActiveExprGuard { stack: self, expr }
    }
}

struct ActiveExprGuard<'stack> {
    stack: &'stack ActiveExprStack,
    expr: ExprId,
}

impl Drop for ActiveExprGuard<'_> {
    fn drop(&mut self) {
        let popped = self.stack.exprs.borrow_mut().pop();
        debug_assert_eq!(popped, Some(self.expr));
    }
}

/// The source scanner and Core walker share owner emission. Marking is one
/// atomic operation so neither path can retain a mutable borrow across the
/// recursive emission it triggers.
#[derive(Default)]
pub(super) struct EmittedOwnerRewrites {
    exprs: RefCell<HashSet<ExprId>>,
}

impl EmittedOwnerRewrites {
    fn contains(&self, expr: ExprId) -> bool {
        self.exprs.borrow().contains(&expr)
    }

    fn mark(&self, expr: ExprId) {
        self.exprs.borrow_mut().insert(expr);
    }
}

#[derive(Clone)]
struct ValueContinuation<'name> {
    destination: ValueDestination<'name>,
    wrappers: Vec<ValueWrapper>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueDestination<'name> {
    Expression,
    Return,
    Assign(&'name str),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueWrapper {
    ResultOk,
}

struct ArmEmissionContext<'context, 'name> {
    /// Indentation depth of the arm's own line inside the lowering.
    depth: u16,
    chain: bool,
    continuation: &'context ValueContinuation<'name>,
    exits: &'context [HostExit],
    exit_label: Option<&'context str>,
    /// The single target that leaves a conditional dispatch after it has
    /// delivered a value. A fall-through block arm already requires
    /// `$tt_b`; in that case expression arms use the same label instead of
    /// adding a nested `do { ... } while (false)` target.
    chain_exit_label: Option<&'context str>,
}

struct ResultEmissionContext<'context, 'name> {
    failure: &'context ValueContinuation<'name>,
    success: &'context ValueContinuation<'name>,
    exit_label: Option<&'context str>,
}

impl<'name> ValueContinuation<'name> {
    fn expression() -> Self {
        Self {
            destination: ValueDestination::Expression,
            wrappers: Vec::new(),
        }
    }

    fn assign(target: &'name str) -> Self {
        Self {
            destination: ValueDestination::Assign(target),
            wrappers: Vec::new(),
        }
    }

    fn returning() -> Self {
        Self {
            destination: ValueDestination::Return,
            wrappers: Vec::new(),
        }
    }

    fn wrap_result_ok(&self) -> Self {
        let mut continuation = self.clone();
        continuation.wrappers.push(ValueWrapper::ResultOk);
        continuation
    }

    fn assigns(&self) -> bool {
        matches!(self.destination, ValueDestination::Assign(_))
    }

    fn is_expression(&self) -> bool {
        self.destination == ValueDestination::Expression
    }

    fn is_unwrapped_assignment_to(&self, target: &str) -> bool {
        self.wrappers.is_empty()
            && matches!(self.destination, ValueDestination::Assign(destination) if destination == target)
    }

    fn assignment_target(&self) -> Option<&str> {
        match self.destination {
            ValueDestination::Expression | ValueDestination::Return => None,
            ValueDestination::Assign(target) => Some(target),
        }
    }

    /// The text an early exit's `return ` becomes. `grouped` says whether
    /// the value it returns has to keep its parentheses ([`push_grouped`]).
    fn assignment_prefix(&self, grouped: bool) -> String {
        let mut prefix = match self.destination {
            ValueDestination::Return => "return ".to_owned(),
            ValueDestination::Assign(target) => format!("{target} = "),
            ValueDestination::Expression => {
                crate::ice::bug!("inline expression continuation cannot rewrite an exit")
            }
        };
        for wrapper in &self.wrappers {
            match wrapper {
                ValueWrapper::ResultOk => {
                    prefix.push_str("{ kind: \"Ok\" as const, value: ");
                }
            }
        }
        if grouped {
            prefix.push('(');
        }
        prefix
    }

    fn assignment_suffix(&self, grouped: bool) -> String {
        let mut suffix = String::new();
        if grouped {
            suffix.push(')');
        }
        for _ in self.wrappers.iter().rev() {
            suffix.push_str(" }");
        }
        suffix
    }
}

fn decision_has_block_arm(decision: &Decision) -> bool {
    decision.arms.iter().any(|arm| {
        matches!(
            arm.action,
            ArmAction::Yield {
                kind: ArmBodyKind::Block { .. },
                ..
            }
        )
    })
}

fn exit_label(target: &str) -> String {
    format!("$tt_y_{}", target.strip_prefix("$tt_").unwrap_or(target))
}

fn push_region_break(out: &mut Rope<'_>, label: Option<&str>) {
    match label {
        Some(label) => out.push_lit(format!(" break {label};")),
        None => out.push_lit(" break;"),
    }
}

fn push_control_break(out: &mut Rope<'_>, depth: u16, label: Option<&str>) {
    out.push_break(depth);
    match label {
        Some(label) => out.push_lit(format!("break {label};")),
        None => out.push_lit("break;"),
    }
}

fn result_failure_test(temp: &str, layout: ResultLayout) -> String {
    match layout.discriminator {
        ResultDiscriminator::SuccessFieldPresent(field) => {
            format!("!(\"{field}\" in {temp})")
        }
    }
}
