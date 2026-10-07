use swc_ecma_visit::{AstParentKind, fields};

use super::{EvaluationFrequency, EvaluationOwner, HostContinuation, OwnerReach, ValueRole};
use crate::chain::Chain;

#[derive(Clone, Default)]
pub(super) struct ParentPath(Chain<ParentEdge>);

impl ParentPath {
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn kinds(&self) -> impl Iterator<Item = AstParentKind> + '_ {
        self.0.iter().map(|edge| edge.kind)
    }

    pub(super) fn push(&self, kind: AstParentKind, node: EdgeNode) -> ParentPath {
        crate::work::tick("parent path edges");
        let index = self.len();
        let facts = self
            .0
            .first()
            .map_or_else(PathFacts::default, |edge| edge.facts)
            .then(index, kind, node);
        ParentPath(Chain::cons(ParentEdge { kind, facts }, self.0.clone()))
    }

    pub(super) fn facts(&self) -> PathFacts {
        self.0
            .first()
            .map_or_else(PathFacts::default, |edge| edge.facts)
    }
}

impl std::fmt::Debug for ParentPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut kinds: Vec<_> = self.kinds().collect();
        kinds.reverse();
        formatter.debug_list().entries(kinds).finish()
    }
}

#[derive(Clone, Copy)]
struct ParentEdge {
    kind: AstParentKind,
    facts: PathFacts,
}

#[derive(Clone, Copy, Default)]
pub(super) struct EdgeNode {
    pub(super) ambient: bool,
    pub(super) decorated_class: bool,
    pub(super) decision_function: bool,
    pub(super) loop_head_reads: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChainBase {
    Call,
    Member,
}

#[derive(Clone, Copy)]
pub(super) struct PathFacts {
    owner: (EvaluationOwner, usize),
    ambient: bool,
    loop_head_reads: bool,
    last_loop: Option<usize>,
    last_unmodeled: Option<usize>,
    pending_chain: Option<(usize, Option<ChainBase>)>,
    local: LocalFacts,
}

impl Default for PathFacts {
    fn default() -> Self {
        PathFacts {
            owner: (EvaluationOwner::Module, 0),
            ambient: false,
            loop_head_reads: false,
            last_loop: None,
            last_unmodeled: None,
            pending_chain: None,
            local: LocalFacts::default(),
        }
    }
}

#[derive(Clone, Copy)]
struct LocalFacts {
    frequency: EvaluationFrequency,
    assignment_target: bool,
    pattern: bool,
    for_init: bool,
    declarator: Option<usize>,
    significant: Option<AstParentKind>,
    asserted: bool,
}

impl Default for LocalFacts {
    fn default() -> Self {
        LocalFacts {
            frequency: EvaluationFrequency::Once,
            assignment_target: false,
            pattern: false,
            for_init: false,
            declarator: None,
            significant: None,
            asserted: false,
        }
    }
}

impl PathFacts {
    fn then(mut self, index: usize, kind: AstParentKind, node: EdgeNode) -> PathFacts {
        self.resolve_chain(index, kind);
        if is_loop_edge(&kind) {
            self.last_loop = Some(index);
        }
        match kind {
            // Evaluated only when no earlier case matched — and always after
            // the discriminant, which hoisting would also reorder.
            AstParentKind::SwitchCase(fields::SwitchCaseField::Test)
            // A destructuring default: evaluated only when the matched
            // property or element is undefined.
            | AstParentKind::AssignPat(fields::AssignPatField::Right)
            | AstParentKind::AssignPatProp(fields::AssignPatPropField::Value) => {
                self.last_unmodeled = Some(index);
            }
            // Inside an optional chain, everything but the base object, the
            // callee, and the arguments of the chain's own optional call is
            // skipped when the chain short-circuits. The arguments are the
            // one position a protocol step models
            // ([`ConditionalBranch::OptionalCallArgument`]).
            AstParentKind::OptChainExpr(fields::OptChainExprField::Base) => {
                self.pending_chain = Some((index, None));
            }
            _ => {}
        }
        self.ambient |= node.ambient;
        if let Some(reads) = node.loop_head_reads {
            self.loop_head_reads = reads;
        }
        match (!node.decision_function)
            .then(|| evaluation_owner_edge(&kind, node.decorated_class))
            .flatten()
        {
            Some(owner) => {
                self.owner = (owner, index + 1);
                self.local = LocalFacts::default();
            }
            None => self.local = self.local.then(&kind),
        }
        self
    }

    fn resolve_chain(&mut self, index: usize, kind: AstParentKind) {
        let Some((start, base)) = self.pending_chain else {
            return;
        };
        let next = match (index - start, base, kind) {
            (1, None, AstParentKind::OptChainBase(fields::OptChainBaseField::Call)) => {
                Some(Some(ChainBase::Call))
            }
            (1, None, AstParentKind::OptChainBase(fields::OptChainBaseField::Member)) => {
                Some(Some(ChainBase::Member))
            }
            (
                2,
                Some(ChainBase::Call),
                AstParentKind::OptCall(
                    fields::OptCallField::Args(_) | fields::OptCallField::Callee,
                ),
            )
            | (
                2,
                Some(ChainBase::Member),
                AstParentKind::MemberExpr(fields::MemberExprField::Obj),
            ) => None,
            _ => {
                self.last_unmodeled =
                    Some(self.last_unmodeled.map_or(start, |last| last.max(start)));
                None
            }
        };
        self.pending_chain = next.map(|base| (start, base));
    }

    pub(super) fn owner(&self) -> (EvaluationOwner, usize) {
        self.owner
    }

    pub(super) fn ambient(&self) -> bool {
        self.ambient
    }

    pub(super) fn frequency(&self) -> EvaluationFrequency {
        self.local.frequency
    }

    /// The evaluation regions between a value's host owner and the value.
    ///
    /// Only the *header* positions of a loop can make the reach `Repeated`: a
    /// loop body is a statement, and a statement is itself a host owner, so a
    /// value in a body never sees the loop edge from its own owner.
    ///
    /// Conditional edges split by who reproduces them. The ternary, logical
    /// right-hand sides, and optional call arguments become
    /// [`ConditionalBranch`](super::ConditionalBranch) steps whose target regenerates the condition, so
    /// they leave the reach `Same`. A `switch` case test, a destructuring
    /// default, and the tail of an optional chain have no protocol step — a
    /// value behind one of them cannot be hoisted to its owner at all.
    pub(super) fn owner_reach(&self, edge: usize) -> OwnerReach {
        let from = |index: Option<usize>| index.is_some_and(|index| index >= edge);
        if from(self.last_loop) {
            OwnerReach::Repeated
        } else if from(self.last_unmodeled) || from(self.pending_chain.map(|(start, _)| start)) {
            OwnerReach::UnmodeledConditional
        } else {
            OwnerReach::Same
        }
    }

    pub(super) fn value_role(&self) -> ValueRole {
        if self.local.assignment_target {
            ValueRole::AssignmentTarget
        } else if self.local.pattern {
            ValueRole::Pattern
        } else {
            ValueRole::Value
        }
    }

    pub(super) fn continuation(&self) -> HostContinuation {
        if self.local.for_init {
            return HostContinuation::ForInitialize;
        }
        match self.local.significant {
            Some(AstParentKind::ReturnStmt(fields::ReturnStmtField::Arg)) => {
                HostContinuation::Return
            }
            Some(AstParentKind::ArrowFunctionBody(fields::ArrowFunctionBodyField::Expr))
            | Some(AstParentKind::ArrowExpr(fields::ArrowExprField::Body)) => {
                HostContinuation::ArrowReturn
            }
            Some(AstParentKind::VarDeclarator(fields::VarDeclaratorField::Init)) => {
                HostContinuation::Initialize
            }
            Some(AstParentKind::ExprStmt(fields::ExprStmtField::Expr)) => HostContinuation::Discard,
            _ => HostContinuation::Compose,
        }
    }

    pub(super) fn asserted(&self) -> bool {
        self.local.asserted
    }

    pub(super) fn loop_head_declarator(&self) -> bool {
        self.local.declarator.is_some_and(|index| index > 0)
    }

    pub(super) fn loop_head_binding(&self) -> bool {
        self.loop_head_reads && self.local.for_init
    }
}

impl LocalFacts {
    fn then(mut self, kind: &AstParentKind) -> LocalFacts {
        if self.frequency != EvaluationFrequency::Repeated {
            if is_loop_edge(kind) {
                self.frequency = EvaluationFrequency::Repeated;
            } else {
                if matches!(kind, AstParentKind::BinExpr(fields::BinExprField::Right)) {
                    self.frequency = EvaluationFrequency::Indeterminate;
                }
                if matches!(
                    kind,
                    AstParentKind::CondExpr(
                        fields::CondExprField::Cons | fields::CondExprField::Alt
                    ) | AstParentKind::IfStmt(fields::IfStmtField::Cons | fields::IfStmtField::Alt)
                        | AstParentKind::SwitchCase(fields::SwitchCaseField::Cons(_))
                ) {
                    self.frequency = EvaluationFrequency::Conditional;
                }
            }
        }
        self.assignment_target |= matches!(
            kind,
            AstParentKind::AssignExpr(fields::AssignExprField::Left)
                | AstParentKind::AssignTarget(_)
                | AstParentKind::SimpleAssignTarget(_)
        );
        self.pattern |= matches!(
            kind,
            AstParentKind::Pat(_)
                | AstParentKind::ArrayPat(_)
                | AstParentKind::ObjectPat(_)
                | AstParentKind::AssignPat(fields::AssignPatField::Left)
        );
        let for_init = matches!(kind, AstParentKind::ForStmt(fields::ForStmtField::Init));
        if for_init && !self.for_init {
            self.for_init = true;
        } else if self.for_init
            && self.declarator.is_none()
            && let AstParentKind::VarDecl(fields::VarDeclField::Decls(index))
            | AstParentKind::UsingDecl(fields::UsingDeclField::Decls(index)) = kind
        {
            self.declarator = Some(*index);
        }
        if is_transparent_expression_edge(kind) {
            self.asserted |= matches!(
                kind,
                AstParentKind::TsAsExpr(fields::TsAsExprField::Expr)
                    | AstParentKind::TsSatisfiesExpr(fields::TsSatisfiesExprField::Expr)
                    | AstParentKind::TsTypeAssertion(fields::TsTypeAssertionField::Expr)
            );
        } else {
            self.significant = Some(*kind);
            self.asserted = false;
        }
        self
    }
}

fn evaluation_owner_edge(kind: &AstParentKind, decorated_class: bool) -> Option<EvaluationOwner> {
    match kind {
        AstParentKind::Class(fields::ClassField::Decorators(_) | fields::ClassField::Body(_)) => {
            Some(EvaluationOwner::ClassDefinition)
        }
        AstParentKind::Class(fields::ClassField::SuperClass) if decorated_class => {
            Some(EvaluationOwner::ClassDefinition)
        }
        AstParentKind::Function(fields::FunctionField::Params(_))
        | AstParentKind::ArrowExpr(fields::ArrowExprField::Params(_))
        | AstParentKind::Constructor(fields::ConstructorField::Params(_)) => {
            Some(EvaluationOwner::ParameterInitializer)
        }
        AstParentKind::Function(fields::FunctionField::Body)
        | AstParentKind::ArrowExpr(fields::ArrowExprField::Body)
        | AstParentKind::Constructor(fields::ConstructorField::Body) => {
            Some(EvaluationOwner::FunctionBody)
        }
        AstParentKind::ClassProp(fields::ClassPropField::Value)
        | AstParentKind::PrivateProp(fields::PrivatePropField::Value)
        | AstParentKind::AutoAccessor(fields::AutoAccessorField::Value) => {
            Some(EvaluationOwner::ClassInitializer)
        }
        AstParentKind::StaticBlock(fields::StaticBlockField::Body) => {
            Some(EvaluationOwner::StaticBlock)
        }
        AstParentKind::TsEnumMember(fields::TsEnumMemberField::Init) => {
            Some(EvaluationOwner::EnumInitializer)
        }
        _ => None,
    }
}

fn is_loop_edge(kind: &AstParentKind) -> bool {
    matches!(
        kind,
        AstParentKind::ForStmt(
            fields::ForStmtField::Test | fields::ForStmtField::Update | fields::ForStmtField::Body
        ) | AstParentKind::ForInStmt(fields::ForInStmtField::Body)
            | AstParentKind::ForOfStmt(fields::ForOfStmtField::Body)
            | AstParentKind::WhileStmt(fields::WhileStmtField::Test | fields::WhileStmtField::Body)
            | AstParentKind::DoWhileStmt(
                fields::DoWhileStmtField::Test | fields::DoWhileStmtField::Body
            )
    )
}

fn is_transparent_expression_edge(parent: &AstParentKind) -> bool {
    matches!(
        parent,
        AstParentKind::Expr(_)
            | AstParentKind::ExprOrSpread(fields::ExprOrSpreadField::Expr)
            | AstParentKind::ParenExpr(fields::ParenExprField::Expr)
            | AstParentKind::TsAsExpr(fields::TsAsExprField::Expr)
            | AstParentKind::TsSatisfiesExpr(fields::TsSatisfiesExprField::Expr)
            | AstParentKind::TsNonNullExpr(fields::TsNonNullExprField::Expr)
            | AstParentKind::TsTypeAssertion(fields::TsTypeAssertionField::Expr)
            | AstParentKind::TsInstantiation(fields::TsInstantiationField::Expr)
    )
}
