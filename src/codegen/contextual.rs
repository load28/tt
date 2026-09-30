//! Refine generated value storage with the facts the TypeScript backend
//! supplied about it (`docs/design/contextual-type-materialization.md`).

use std::collections::HashSet;

use swc_common::Spanned;
use swc_ecma_ast::{AssignOp, AssignTarget, Expr, SimpleAssignTarget, Stmt};
use swc_ecma_visit::{Visit, VisitWith};

use crate::host_input::HostInput;
use crate::{MappedEmit, SourceKind};

/// What the backend's rounds settled about one generated value slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SlotRefinement {
    /// The type written on the slot's declaration.
    pub annotation: Option<String>,
    /// The slot's source position has no contextual type. Every value
    /// written to the slot whose type TypeScript computes from its context
    /// is then first the property of an object literal a `const` of its own
    /// holds, where TypeScript types it without one, as at that position.
    pub detached: bool,
    pub provisional: bool,
    pub cleared: bool,
}

/// Where the refined emission declares its storage.
pub(crate) struct Refined {
    /// The end of each slot's declaration identifier, in the order of the
    /// refinements.
    pub declarations: Vec<usize>,
    /// The end of each `const` identifier that holds a value on its way to
    /// detached storage.
    pub locals: Vec<usize>,
}

struct Edit {
    start: usize,
    end: usize,
    text: String,
}

/// Applies `slots`, one per [`MappedEmit::contextual_slots`] entry, to an
/// emission no refinement has been applied to.
pub(crate) fn refine(
    emit: &mut MappedEmit,
    source_kind: SourceKind,
    slots: &[SlotRefinement],
) -> Refined {
    if slots.len() != emit.contextual_slots.len() {
        crate::ice::bug!("refinements do not match the emission's value slots")
    }
    let mut edits: Vec<Edit> = Vec::new();
    for (&end, slot) in emit.contextual_slots.iter().zip(slots) {
        let written = emit
            .asserted_slots
            .iter()
            .find(|(slot_end, _)| *slot_end == end)
            .map_or(end, |&(_, annotation_end)| annotation_end);
        let text = match &slot.annotation {
            Some(annotation) => format!(": {annotation}"),
            None if slot.cleared => String::new(),
            None => continue,
        };
        if text.is_empty() && written == end {
            continue;
        }
        edits.push(Edit {
            start: end,
            end: written,
            text,
        });
    }
    let mut locals = Vec::new();
    let detached: Vec<usize> = emit
        .contextual_slots
        .iter()
        .zip(slots)
        .filter(|(end, slot)| slot.detached && !emit.selector_slots.contains(end))
        .map(|(&end, _)| end)
        .collect();
    let writes = if detached.is_empty() {
        None
    } else {
        storage_writes(&emit.code, source_kind, &detached)
    };
    if let Some(writes) = writes {
        let mut occupied = crate::generated_names::source_names(&emit.code, source_kind);
        let newline = crate::line_ending(&emit.code);
        let (tested, carried): (Vec<_>, Vec<_>) = writes
            .into_iter()
            .filter(|write| write.typed_by_context)
            .partition(|write| write.tested);
        for write in tested {
            edits.push(Edit {
                start: write.value.0,
                end: write.value.0,
                text: "({ value: ".to_owned(),
            });
            edits.push(Edit {
                start: write.value.1,
                end: write.value.1,
                text: " }).value".to_owned(),
            });
        }
        for (index, write) in carried.into_iter().enumerate() {
            let local = crate::generated_names::allocate(&format!("$tt_a{index}"), &mut occupied)
                .unwrap_or_else(|| crate::ice::bug!("no free generated name remains for a value"));
            locals.push((write.target, format!("const {local}").len()));
            edits.push(Edit {
                start: write.target,
                end: write.value.0,
                text: format!("const {local} = {{ value: "),
            });
            // The write keeps the layout it was given: on a line of its own,
            // the assignment follows on the next line at its indentation.
            let line = crate::lines::line_start_before(&emit.code, write.target);
            let indent = &emit.code[line..write.target];
            let separator = if indent.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
                format!("{newline}{indent}")
            } else {
                " ".to_owned()
            };
            edits.push(Edit {
                start: write.value.1,
                end: write.value.1,
                text: format!(" }};{separator}{} = {local}.value", write.storage),
            });
            emit.generated_names.insert(local);
        }
    }
    edits.sort_by_key(|edit| (edit.start, edit.end));
    if edits
        .windows(2)
        .any(|pair| pair[0].end > pair[1].start || pair[0].start == pair[1].start)
    {
        crate::ice::bug!("value storage refinements overlap")
    }
    let declarations = emit
        .contextual_slots
        .iter()
        .map(|&end| shifted(&edits, end, false))
        .collect();
    let locals = locals
        .into_iter()
        .map(|(start, len)| shifted(&edits, start, false) + len)
        .collect();
    let unannotated: Vec<usize> = emit
        .contextual_slots
        .iter()
        .zip(slots)
        .filter(|(_, slot)| slot.annotation.is_none() && !slot.cleared)
        .map(|(&end, _)| end)
        .collect();
    emit.asserted_slots
        .retain(|(end, _)| unannotated.contains(end));
    emit.contextual_slots = unannotated;
    apply(emit, &edits);
    Refined {
        declarations,
        locals,
    }
}

/// Where `p` lands once `edits` are applied. A position at an insertion
/// moves past it when `inclusive` (a start of what follows), and a position
/// at the start of a replacement stays in front of the new text.
fn shifted(edits: &[Edit], p: usize, inclusive: bool) -> usize {
    let mut at = p;
    for edit in edits {
        if edit.start == edit.end {
            if edit.start < p || inclusive && edit.start == p {
                at += edit.text.len();
            }
        } else if edit.end <= p {
            at = at + edit.text.len() - (edit.end - edit.start);
        } else if edit.start < p {
            crate::ice::bug!("an emitted position lies inside refined glue")
        }
    }
    at
}

fn apply(emit: &mut MappedEmit, edits: &[Edit]) {
    for mapping in &mut emit.mappings {
        let end = mapping.out + mapping.len;
        if edits.iter().any(|edit| {
            if edit.start == edit.end {
                mapping.out < edit.start && edit.start < end
            } else {
                edit.start < end && mapping.out < edit.end
            }
        }) {
            crate::ice::bug!("value storage refinement splits copied source")
        }
        mapping.out = shifted(edits, mapping.out, true);
    }
    for mark in &mut emit.scrutinee_temps {
        mark.out = shifted(edits, mark.out, true);
    }
    for mark in &mut emit.payload_temps {
        mark.out = shifted(edits, mark.out, true);
    }
    for mark in &mut emit.result_return_temps {
        mark.out = shifted(edits, mark.out, true);
        mark.out_end = shifted(edits, mark.out_end, false);
    }
    for name in &mut emit.declared_names {
        name.out = shifted(edits, name.out, true);
        name.out_end = shifted(edits, name.out_end, false);
    }
    for binding in &mut emit.shared_bindings {
        binding.out = shifted(edits, binding.out, true);
        binding.out_end = shifted(edits, binding.out_end, false);
    }
    for list in &mut emit.destructured_lists {
        list.out = shifted(edits, list.out, true);
        list.out_end = shifted(edits, list.out_end, false);
    }
    for glue in &mut emit.inserted {
        glue.out = shifted(edits, glue.out, true);
        glue.out_end = shifted(edits, glue.out_end, false);
    }
    for anchor in &mut emit.anchors {
        anchor.out = shifted(edits, anchor.out, true);
        anchor.end = shifted(edits, anchor.end, false);
    }
    for (slot, annotation) in &mut emit.asserted_slots {
        *slot = shifted(edits, *slot, true);
        *annotation = shifted(edits, *annotation, false);
    }
    for position in emit
        .contextual_slots
        .iter_mut()
        .chain(&mut emit.selector_slots)
        .chain(&mut emit.operand_slots)
    {
        *position = shifted(edits, *position, true);
    }
    for edit in edits.iter().rev() {
        emit.code.replace_range(edit.start..edit.end, &edit.text);
    }
}

/// One statement that assigns a value to generated storage.
struct StorageWrite {
    storage: String,
    /// The start of the assignment's target identifier.
    target: usize,
    /// The assigned value.
    value: (usize, usize),
    /// TypeScript computes the value's type from its contextual type
    /// ([`typed_by_context`]).
    typed_by_context: bool,
    tested: bool,
}

/// Whether TypeScript computes the type of `value` from the contextual type
/// it is given, so the `any` of undeclared storage would change it.
///
/// The checker consults the contextual type in typing an object literal
/// (its properties, and `this` in its methods), an array literal (its
/// elements) and a function or arrow function (its parameters, and its
/// return expressions); `getContextualType` passes a position's contextual
/// type on to the operand of parentheses, `as const`, a non-null assertion
/// and `await`, to both branches of a conditional, to both operands of `||`
/// and `??`, and to the right operand of `&&` and of the comma operator.
/// Every other operand gets a contextual type of its own (a call argument
/// its parameter's, `as T` and `satisfies T` their `T`) or none, and the
/// remaining expressions type themselves: a literal is not kept literal by
/// `any`, and a call infers nothing from a contextual `any` return type.
fn typed_by_context(value: &Expr) -> bool {
    use swc_ecma_ast::BinaryOp;
    match value {
        Expr::Object(_) | Expr::Array(_) | Expr::Fn(_) | Expr::Arrow(_) => true,
        Expr::Paren(inner) => typed_by_context(&inner.expr),
        Expr::TsConstAssertion(inner) => typed_by_context(&inner.expr),
        Expr::TsNonNull(inner) => typed_by_context(&inner.expr),
        Expr::Await(inner) => typed_by_context(&inner.arg),
        Expr::Cond(inner) => typed_by_context(&inner.cons) || typed_by_context(&inner.alt),
        Expr::Bin(inner) => match inner.op {
            BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => {
                typed_by_context(&inner.left) || typed_by_context(&inner.right)
            }
            BinaryOp::LogicalAnd => typed_by_context(&inner.right),
            _ => false,
        },
        Expr::Seq(inner) => inner
            .exprs
            .last()
            .is_some_and(|last| typed_by_context(last)),
        _ => false,
    }
}

/// Every assignment to the storage declared by the identifiers ending at
/// `declarations`, in output order. An emission that does not parse (an
/// editor buffer mid-edit) has no statements to rewrite: `None`.
///
/// The lowering writes storage only in statements of its own blocks and
/// `switch` cases, where a `const` can be declared. Generated names are
/// unique in their file, so the name identifies the storage.
fn storage_writes(
    code: &str,
    source_kind: SourceKind,
    declarations: &[usize],
) -> Option<Vec<StorageWrite>> {
    struct Collect<'a> {
        input: &'a HostInput,
        declarations: &'a [usize],
        names: HashSet<String>,
        statements: HashSet<(usize, usize)>,
        tests: HashSet<(usize, usize)>,
        /// Each assignment to a plain identifier, with its own extent.
        assignments: Vec<(StorageWrite, (usize, usize))>,
    }
    impl Collect<'_> {
        fn statements(&mut self, statements: &[Stmt]) {
            for statement in statements {
                if let Stmt::Expr(expression) = statement
                    && let Expr::Assign(assign) = &*expression.expr
                {
                    let span = assign.span();
                    self.statements
                        .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
                }
            }
        }
    }
    impl Visit for Collect<'_> {
        fn visit_var_declarator(&mut self, node: &swc_ecma_ast::VarDeclarator) {
            if let Some(name) = node.name.as_ident() {
                let end = self.input.byte(name.id.span.hi);
                if self.declarations.contains(&end) {
                    self.names.insert(name.id.sym.to_string());
                }
            }
            node.visit_children_with(self);
        }
        fn visit_block_stmt(&mut self, node: &swc_ecma_ast::BlockStmt) {
            self.statements(&node.stmts);
            node.visit_children_with(self);
        }
        fn visit_function_body(&mut self, node: &swc_ecma_ast::FunctionBody) {
            self.statements(&node.stmts);
            node.visit_children_with(self);
        }
        fn visit_switch_case(&mut self, node: &swc_ecma_ast::SwitchCase) {
            self.statements(&node.cons);
            node.visit_children_with(self);
        }
        fn visit_if_stmt(&mut self, node: &swc_ecma_ast::IfStmt) {
            let mut test = &*node.test;
            loop {
                match test {
                    Expr::Paren(inner) => test = &inner.expr,
                    Expr::Bin(inner) if inner.op == swc_ecma_ast::BinaryOp::EqEq => {
                        test = &inner.left;
                    }
                    Expr::Assign(assign) => {
                        let span = assign.span();
                        self.tests
                            .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
                        break;
                    }
                    _ => break,
                }
            }
            node.visit_children_with(self);
        }
        fn visit_assign_expr(&mut self, node: &swc_ecma_ast::AssignExpr) {
            if node.op == AssignOp::Assign
                && let AssignTarget::Simple(SimpleAssignTarget::Ident(target)) = &node.left
            {
                let span = node.span();
                let value = node.right.span();
                self.assignments.push((
                    StorageWrite {
                        storage: target.id.sym.to_string(),
                        target: self.input.byte(target.id.span.lo),
                        value: (self.input.byte(value.lo), self.input.byte(value.hi)),
                        typed_by_context: typed_by_context(&node.right),
                        tested: false,
                    },
                    (self.input.byte(span.lo), self.input.byte(span.hi)),
                ));
            }
            node.visit_children_with(self);
        }
    }

    let input = HostInput::new(code);
    let mut parser = input.parser(source_kind);
    let module = parser.parse_module().ok()?;
    if !parser.take_errors().is_empty() {
        return None;
    }
    let mut collect = Collect {
        input: &input,
        declarations,
        names: HashSet::new(),
        statements: HashSet::new(),
        tests: HashSet::new(),
        assignments: Vec::new(),
    };
    module.visit_with(&mut collect);
    if collect.names.len() != declarations.len() {
        crate::ice::bug!("detached value storage has no declaration")
    }
    let mut writes = Vec::new();
    for (mut write, span) in collect.assignments {
        if !collect.names.contains(&write.storage) {
            continue;
        }
        write.tested = collect.tests.contains(&span);
        if !write.tested && !collect.statements.contains(&span) {
            crate::ice::bug!(
                "a write to value storage is neither a statement of a block nor an `if` test"
            )
        }
        writes.push(write);
    }
    writes.sort_by_key(|write| write.target);
    Some(writes)
}
