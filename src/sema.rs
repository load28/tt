//! Semantic checks over the AST.
//!
//! Everything the parser deliberately does not do lives here: tt-level rules
//! whose violation is a compile error rather than a passthrough. The checker
//! walks the AST depth-first and **accumulates** every violation as an
//! [`TtError`] with a byte offset and a [`DiagnosticCode`], then reports
//! them in source order — one broken construct does not stop the next
//! independent one from being checked (TASK-117; the recovery boundary is
//! per construct: a match whose names did not resolve keeps its own
//! exhaustiveness question suppressed, nobody else's).
//!
//! Error layering: every rule here is an
//! tt-level rule, reported by ttc itself with an exact position. Nothing is
//! delegated to tsc — in particular exhaustiveness, which this module
//! *reports* but no longer computes: [`crate::analysis`] owns the subject
//! table and the coverage rule, and sema turns its answer into positioned
//! errors after the walk (so a match may precede the variant it matches on).
//! One rule, one implementation — see `docs/design/match-analysis.md` §5.
//!
//! Checks performed:
//! - `variant`: no duplicate case tags; with verification enabled, every field
//!   type parses as a TypeScript type fragment (via [`crate::verify`]).
//! - `match`: the wildcard `_` arm is last; no arm repeats a tag already
//!   covered by an unguarded arm (guarded arms may share tags with each
//!   other); or-pattern alternatives all bind the same (field, name) set.
//! - `match` literal patterns: tag and literal patterns never mix in one
//!   match (their emitted discriminants differ); or-pattern alternatives
//!   are all the same kind of literal; no unguarded arm repeats a literal
//!   *value* another already covers (`200` and `0xc8` are one case).
//!   Whether a literal match is exhaustive is a question about the
//!   scrutinee's TypeScript type and is deliberately left to the
//!   `--types` pipeline ([`crate::literal_matches`]).
//! - `try`: must sit inside a function written in its parse region
//!   ([`crate::flow::in_function_body`], carried on the statement by the
//!   parser) — its emitted `return` needs a user function to exit. That
//!   rules out the module's top level, and the statement regions of tt
//!   constructs (a match arm, a `result` block, a template interpolation)
//!   *except* where the user wrote a function there: a `try` inside an
//!   arrow inside an arm is Rust's `?` inside a closure, and is fine.
//! - let-else: the same flow fact decides placement, except the module's
//!   top level is fine (the lowering emits no `return` of its own; a
//!   `throw`-diverging `else` is valid anywhere) — only tt constructs'
//!   statement regions need a user function around it. And the `else`
//!   block must diverge on every path (`return`/`throw`/`break`/
//!   `continue`; a CFG answer) — otherwise the destructuring after the
//!   block would run with the case unproven.
//! - exhaustiveness: a wildcard-free match whose arm tags all belong to an
//!   variant declared in this file, an imported declaration
//!   ([`crate::Options::extern_variants`], collected by the CLI from direct
//!   relative `.tt` imports), or a built-in variant (`Option`, `Result`; the
//!   analysis' declaration table) — must cover every case of that
//!   variant with unguarded arms (a guard may be false, so guarded arms
//!   identify the variant but cover nothing). A tuple match must cover the
//!   cartesian product of its positions. Same-name shadowing runs
//!   local > imported > built-in. Matches whose tags belong to no known
//!   variant (hand-written unions, unresolved imports) are not checked — ttc
//!   has no type information for them. The whole computation lives in
//!   [`crate::analysis`]; what is here is the reporting.

mod checker;
mod coverage;

use crate::analysis::{CoveredVariant, NameKind, Origin, has_nested};
use crate::ast::*;
use crate::diagnostics::{
    DiagnosticCode, MatchSite, non_exhaustive_message, non_exhaustive_suggestions,
};
use crate::error::TtError;
use crate::verify;
use std::collections::HashMap;

use coverage::*;

/// Checks a whole program and returns **every** tt-level violation, in
/// source order. `verify` enables swc validation of field types; `externs`
/// are variant declarations collected from imported modules
/// ([`crate::Options::extern_variants`]). With `defer_to_checker` the two
/// exhaustiveness passes are skipped, because a TypeScript backend answers
/// the question better than this file's declaration table can
/// ([`crate::Options::defer_to_checker`]); every other tt-level rule is
/// checked either way.
pub(crate) fn check_all(
    source: &str,
    program: &Program,
    verify: bool,
    defer_to_checker: bool,
    semantic: &crate::analysis::SemanticFile,
) -> Vec<TtError> {
    let result_completions = semantic
        .hir
        .exprs
        .iter()
        .filter_map(|(_, expr)| {
            let crate::hir::Expr::ResultBlock {
                node, completes, ..
            } = expr
            else {
                return None;
            };
            let span = semantic.hir.source_map.node_span(*node)?;
            Some((span.start, *completes))
        })
        .collect();
    let mut checker = Checker {
        source: source.to_owned(),
        tokens: crate::lexer::lex(source, 0, source.len()),
        verify,
        errors: Vec::new(),
        coverage_suppressed: Vec::new(),
        result_completions,
    };
    checker.visit_program(program, Ctx::Top, Place::Module);
    // One analysis, two reports. Resolution comes first — a pattern whose
    // names do not resolve has no exhaustiveness question worth asking, and
    // answering both at once would bury the cause under its effect. With
    // accumulation the suppression is per match
    // ([`PatternAnalyses::match_has_resolution_error`]), not per file: match
    // B's coverage is not match A's typo's business.
    checker.errors.extend(resolution_errors(&semantic.patterns));
    if !defer_to_checker {
        report_coverage(
            source,
            &semantic.patterns,
            &checker.coverage_suppressed,
            &mut checker.errors,
        );
    }
    // Source order, whatever order the categories ran in — the reader fixes
    // a file top to bottom. Stable, so equal positions keep report order.
    checker
        .errors
        .sort_by_key(|e| e.offset.unwrap_or(usize::MAX));
    checker.errors
}

/// Turns [`crate::analysis`]'s resolution answer into positioned tt
/// errors.
///
/// Every entry the analysis produced is an error: the *decision* whether
/// an unresolved name is reportable belongs to the analysis (which is what
/// keeps one rule in one place), and it only produces entries it can name
/// a replacement for. This function is the wording.
pub(crate) fn resolution_errors(analyses: &crate::analysis::PatternAnalyses) -> Vec<TtError> {
    let mut errors = Vec::with_capacity(analyses.unresolved.len());
    for unresolved in &analyses.unresolved {
        let described = describe(&CoveredVariant {
            name: unresolved.variant_name.clone(),
            origin: unresolved.origin.clone(),
        });
        // The message states the problem; the replacement is carried as a
        // suggestion rather than spelled into the sentence. The analysis
        // only produces an entry when it can name a replacement, so every
        // one of these has exactly one — and it is the same datum the CLI
        // renders as `help:` and an editor offers as a code action
        // (TASK-213 decision 2).
        let (message, hint, code) = match (&unresolved.kind, &unresolved.tag) {
            (NameKind::Field, Some(tag)) => (
                format!(
                    "{described}: case `{tag}` has no field `{}`",
                    unresolved.name
                ),
                "a field with a similar name exists",
                DiagnosticCode::UnknownField,
            ),
            _ => (
                format!("{described} has no case `{}`", unresolved.name),
                "a case with a similar name exists",
                DiagnosticCode::UnknownCase,
            ),
        };
        errors.push(
            TtError::span(unresolved.start, unresolved.end, message)
                .code(code)
                .suggest(
                    hint,
                    unresolved.start,
                    unresolved.end,
                    unresolved.suggestion.clone(),
                ),
        );
    }
    errors
}

struct Checker {
    source: String,
    tokens: Vec<crate::lexer::Token>,
    verify: bool,
    /// Every violation found so far — the walk keeps going after each one.
    errors: Vec<TtError>,
    /// Keyword offsets of matches whose *structure* is broken (mixed tag
    /// and literal patterns). Their coverage answer would be an effect
    /// stacked on a cause, so [`report_coverage`] skips them — the same
    /// per-match recovery boundary resolution failures use.
    coverage_suppressed: Vec<usize>,
    /// Result completion is a HIR flow fact. Index it by the AST node's
    /// stable source start so this AST diagnostic walk consumes the same
    /// answer codegen will lower instead of running a second CFG query.
    result_completions: HashMap<usize, bool>,
}

/// The (field, bound name) pairs a tag alternative destructures, sorted so
/// alternatives compare as sets. No parens and empty parens both bind nothing.
/// Nested patterns never reach this (they are rejected inside or-patterns).
fn binding_set(bindings: &Option<Vec<Binding>>) -> Vec<(&str, &str)> {
    let mut set: Vec<(&str, &str)> = bindings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|b| b.nested.is_none())
        .map(|b| (b.name.as_str(), b.alias.as_deref().unwrap_or(&b.name)))
        .collect();
    set.sort_unstable();
    set
}

/// Why two or-pattern alternatives do not bind the same set — the first
/// difference, named, so the message points at the binding to fix instead
/// of restating the rule: a name only one side binds, or a name the two
/// sides bind from different fields.
fn binding_mismatch(first: &TagPattern, other: &TagPattern) -> String {
    let a = binding_set(&first.bindings);
    let b = binding_set(&other.bindings);
    let bound = |set: &[(&str, &str)], name: &str| set.iter().any(|&(_, n)| n == name);
    for &(_, name) in &a {
        if !bound(&b, name) {
            return format!(
                "`{name}` is bound in `{}(...)` but not in `{}(...)`",
                first.tag, other.tag
            );
        }
    }
    for &(_, name) in &b {
        if !bound(&a, name) {
            return format!(
                "`{name}` is bound in `{}(...)` but not in `{}(...)`",
                other.tag, first.tag
            );
        }
    }
    // Same names on both sides, so some name is bound from different
    // fields (`A(x) | B(v: x)`).
    for &(field, name) in &a {
        if let Some(&(other_field, _)) = b.iter().find(|&&(_, n)| n == name)
            && field != other_field
        {
            return format!(
                "`{name}` is bound from field `{field}` in `{}(...)` but from field `{other_field}` in `{}(...)`",
                first.tag, other.tag
            );
        }
    }
    // The caller only asks when the sets differ, but stay total.
    "the alternatives bind different sets".to_string()
}

/// Collects every variable name the alternative binds, nested patterns
/// included, in source order.
fn leaf_bindings<'a>(alt: &'a TagPattern, out: &mut Vec<&'a str>) {
    for b in alt.bindings.as_deref().unwrap_or_default() {
        match &b.nested {
            Some(inner) => leaf_bindings(inner, out),
            None => out.push(b.alias.as_deref().unwrap_or(&b.name)),
        }
    }
}

/// Where a sub-program sits, syntactically.
#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Top,
    Stmt,
    Expr,
}

/// Where a region's statements ultimately *run* — the placement fact the
/// flow-based rules combine with each statement's own
/// [`crate::flow::in_function_body`] answer. An `if let` body and a
/// let-else `else` block are **inline**: their statements run where the
/// statement itself stands, so they inherit its place (upgraded to
/// [`Place::Function`] when the statement sits inside a function written
/// in its region). A match arm body, a `result` block's statements, and
/// every isolated value region reset to its owning place — match arms use
/// [`Place::ValueRegion`], while `result` bodies use [`Place::ResultRegion`].
/// An exit written there belongs to the construct value, never the user's
/// function.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    /// The module's top level (or an inline chain that bottoms out there).
    Module,
    /// Inside a user-written function (directly or through inline chains).
    Function,
    /// Inside an isolated tt value region.
    ValueRegion,
    /// Inside an isolated value region nested in a `result` block. The
    /// current function-targeted `try` would cross that region once Result
    /// scope becomes lexical, so the permanent crossing diagnostic reports it.
    ResultValueRegion,
    /// Inside a `result` block, whose generated region owns its returns.
    ResultRegion,
}

impl Place {
    /// The place of an inline sub-region (an `if let` body, a let-else
    /// `else` block) of a statement whose own region fact is
    /// `in_function`.
    fn inline(self, in_function: bool) -> Place {
        if in_function { Place::Function } else { self }
    }

    fn isolated(self) -> Place {
        if matches!(self, Place::ResultRegion | Place::ResultValueRegion) {
            Place::ResultValueRegion
        } else {
            Place::ValueRegion
        }
    }
}
