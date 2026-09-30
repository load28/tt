//! What an access path is, read from TypeScript syntax.
//!
//! One definition serves every mutation `val` recognizes: the root of a
//! method call's receiver ([`super::method_calls`]) and the root of an
//! assignment, update, or `delete` target ([`super::targets`]).

use swc_ecma_ast::{Expr, Ident, OptChainBase};

/// The operand a TypeScript expression wrapper evaluates to. Parentheses,
/// a non-null assertion (`x!`), a type assertion (`x as T`, `<T>x`,
/// `x as const`), and `satisfies` evaluate their operand and yield its
/// value unchanged, so a path through them is the operand's path.
pub(super) fn unwrapped(mut expr: &Expr) -> &Expr {
    loop {
        expr = match expr {
            Expr::Paren(inner) => &inner.expr,
            Expr::TsNonNull(inner) => &inner.expr,
            Expr::TsAs(inner) => &inner.expr,
            Expr::TsSatisfies(inner) => &inner.expr,
            Expr::TsTypeAssertion(inner) => &inner.expr,
            Expr::TsConstAssertion(inner) => &inner.expr,
            _ => return expr,
        };
    }
}

/// The identifier an access path is rooted at, and the number of member
/// steps (`.p`, `?.p`, `[k]`) from it, with wrappers read through at every
/// depth: `(x as T).a!.b` is `x` with two steps.
pub(super) fn access_path(expr: &Expr) -> Option<(&Ident, usize)> {
    crate::stack::grow(|| access_path_grown(expr))
}

fn access_path_grown(expr: &Expr) -> Option<(&Ident, usize)> {
    let object = match unwrapped(expr) {
        Expr::Ident(ident) => return Some((ident, 0)),
        Expr::Member(member) => &member.obj,
        Expr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(member) => &member.obj,
            OptChainBase::Call(_) => return None,
        },
        _ => return None,
    };
    access_path(object).map(|(root, steps)| (root, steps + 1))
}
