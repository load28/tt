//! Method-call probes read from the TypeScript the file emits.

use swc_common::Spanned;
use swc_ecma_ast::{
    CallExpr, Callee, Expr, Lit, MemberExpr, MemberProp, OptChainBase, OptChainExpr,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::host_input::HostInput;
use crate::typescript::mapper;
use crate::{MappedEmit, SourceKind};

use super::Mutation;
use super::reference::{access_path, unwrapped};

/// Every call in `emit` whose callee is a member access rooted at an
/// identifier and keyed by a name: `x.m(..)`, `x?.m(..)`, `x["m"](..)`,
/// `(x.m)(..)`, at any path depth. A computed key that is not a string
/// literal names no method and is not collected. Positions are source
/// bytes, and a call whose root or key is compiler-written glue is skipped.
pub(crate) fn method_calls(emit: &MappedEmit, source_kind: SourceKind) -> Vec<Mutation> {
    let input = HostInput::new(&emit.code);
    let mut parser = input.parser(source_kind);
    let Ok(program) = parser.parse_program() else {
        return Vec::new();
    };
    let mut calls = Calls {
        input: &input,
        emit,
        found: Vec::new(),
    };
    program.visit_with(&mut calls);
    calls.found
}

struct Calls<'a> {
    input: &'a HostInput,
    emit: &'a MappedEmit,
    found: Vec<Mutation>,
}

impl Calls<'_> {
    fn source(&self, position: swc_common::BytePos) -> Option<usize> {
        mapper::to_source(&self.emit.mappings, self.input.byte(position))
    }

    fn callee(&mut self, callee: &Expr) {
        let Some(member) = member_of(callee) else {
            return;
        };
        let Some((method, key)) = method_key(member) else {
            return;
        };
        let Some((root, _)) = access_path(&member.obj) else {
            return;
        };
        let (Some(root_at), Some(key_at)) = (self.source(root.span.lo), self.source(key)) else {
            return;
        };
        self.found.push(Mutation {
            root: root_at,
            name: root.sym.to_string(),
            method: Some((method, key_at)),
        });
    }
}

impl Visit for Calls<'_> {
    fn visit_call_expr(&mut self, node: &CallExpr) {
        if let Callee::Expr(callee) = &node.callee {
            self.callee(callee);
        }
        node.visit_children_with(self);
    }

    fn visit_opt_chain_expr(&mut self, node: &OptChainExpr) {
        if let OptChainBase::Call(call) = &*node.base {
            self.callee(&call.callee);
        }
        node.visit_children_with(self);
    }
}

fn member_of(callee: &Expr) -> Option<&MemberExpr> {
    match unwrapped(callee) {
        Expr::Member(member) => Some(member),
        Expr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(member) => Some(member),
            OptChainBase::Call(_) => None,
        },
        _ => None,
    }
}

fn method_key(member: &MemberExpr) -> Option<(String, swc_common::BytePos)> {
    match &member.prop {
        MemberProp::Ident(name) => Some((name.sym.to_string(), name.span.lo)),
        MemberProp::Computed(computed) => match unwrapped(&computed.expr) {
            Expr::Lit(Lit::Str(text)) => Some((text.value.as_str()?.to_string(), text.span.lo)),
            Expr::Tpl(template) if template.exprs.is_empty() => {
                let quasi = template.quasis.first()?;
                Some((
                    quasi.cooked.as_ref()?.as_str()?.to_string(),
                    template.span().lo,
                ))
            }
            _ => None,
        },
        MemberProp::PrivateName(_) => None,
    }
}
