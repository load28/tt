//! Host-grammar answers used by the lossless tt parser.
//!
//! Files with ambiguous `match` positions use byte-preserving projections of
//! each recursive parser region. Confirmed tt nodes become category-safe
//! placeholders, while an ambiguous candidate remains source text only after an
//! expression-safe probe is rejected at that position. The projection is
//! retried, and ownership is accepted only when the converged SWC AST contains a
//! host declaration node for the original identifier span.
//!
//! Parameter-shaped `val` modifiers use the same projections: the host AST
//! says whether the binding after an erased `val` is a formal parameter.

use std::collections::HashSet;

use swc_common::{Span as SwcSpan, Spanned};
use swc_ecma_ast::{
    ArrowExpr, CatchClause, ClassMethod, FnDecl, FnExpr, GetterProp, MethodProp, Module, Param,
    PrivateMethod, PropName, SetterProp, TsFnParam, TsParamProp,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::ast::{Program, RecoveryKind, Segment, Span, TemplateChunk};

pub(super) fn owned_match_names_in_mixed(
    src: &str,
    source_kind: crate::SourceKind,
    program: &Program,
) -> Vec<Span> {
    let mut owned = Vec::new();
    super::parse::visit_programs(program, &mut |region| {
        probe_region(
            src,
            source_kind,
            region,
            region_wrappers(program, region),
            &mut owned,
        );
    });
    owned.sort_by_key(|span| (span.start, span.end));
    owned.dedup();
    owned
}

/// The keyword offsets, sorted, of the parameter-shaped `val` candidates
/// whose binding the host grammar does not read as a formal parameter.
///
/// Each region with candidates is projected in its tt reading (every lifted
/// construct masked, every `val` modifier erased) and parsed by the host. A
/// candidate stays a modifier when some projection places a formal
/// parameter — of a function, arrow, method, accessor, constructor, `catch`
/// clause, or TypeScript signature — exactly at its binding. Erasing `val`
/// before a binding that valid TypeScript already owns (`f(val [0])`, `c ?
/// (val [0]) : w => w`) leaves an array literal or tuple type in the same
/// grammatical position, never a parameter, so TypeScript keeps it. A region
/// the host cannot parse proves nothing, and its candidates stay
/// identifiers.
pub(super) fn rejected_val_candidates(
    src: &str,
    source_kind: crate::SourceKind,
    program: &Program,
) -> Vec<usize> {
    let mut candidates = Vec::new();
    let mut parameters = HashSet::new();
    super::parse::visit_programs(program, &mut |region| {
        if region.host_val_candidates().is_empty() {
            return;
        }
        candidates.extend(region.host_val_candidates().iter().copied());
        parameters.extend(parameter_starts(
            src,
            source_kind,
            region,
            region_wrappers(program, region),
        ));
    });
    let mut rejected: Vec<usize> = candidates
        .into_iter()
        .filter(|candidate| !parameters.contains(&candidate.end))
        .map(|candidate| candidate.start)
        .collect();
    rejected.sort_unstable();
    rejected.dedup();
    rejected
}

fn region_wrappers(root: &Program, region: &Program) -> &'static [Wrapper] {
    if std::ptr::eq(region, root) {
        &[Wrapper::Module]
    } else if region.expression_root {
        &[
            Wrapper::Expression,
            Wrapper::AsyncExpression,
            Wrapper::GeneratorExpression,
            Wrapper::AsyncGeneratorExpression,
        ]
    } else {
        &[
            Wrapper::Statements,
            Wrapper::AsyncStatements,
            Wrapper::GeneratorStatements,
            Wrapper::AsyncGeneratorStatements,
        ]
    }
}

fn parameter_starts(
    src: &str,
    source_kind: crate::SourceKind,
    program: &Program,
    wrappers: &[Wrapper],
) -> Vec<usize> {
    if program.span.start >= program.span.end || program.span.end > src.len() {
        return Vec::new();
    }
    let mut masks = Vec::new();
    let mut match_candidates = Vec::new();
    collect_region_facts(program, &mut masks, &mut match_candidates);
    let projection = projected_region(src, program.span, &masks, &match_candidates, &[]);
    for recovering in [false, true] {
        for &wrapper in wrappers {
            if let Ok(parsed) = parse_wrapped_with(
                &projection,
                source_kind,
                program.span.start,
                wrapper,
                recovering,
            ) {
                let mut collector = ParameterCollector {
                    frame: parsed.frame,
                    starts: Vec::new(),
                };
                parsed.module.visit_with(&mut collector);
                return collector.starts;
            }
        }
    }
    Vec::new()
}

#[derive(Clone, Copy)]
enum Wrapper {
    Module,
    Expression,
    AsyncExpression,
    GeneratorExpression,
    AsyncGeneratorExpression,
    Statements,
    AsyncStatements,
    GeneratorStatements,
    AsyncGeneratorStatements,
}

#[derive(Clone, Copy)]
enum Placeholder {
    Expression,
    ProbeExpression,
    Statement,
    OperandHead,
    Type,
    Erase,
}

#[derive(Clone, Copy)]
struct Mask {
    span: Span,
    placeholder: Placeholder,
}

fn probe_region(
    src: &str,
    source_kind: crate::SourceKind,
    program: &Program,
    wrappers: &[Wrapper],
    owned: &mut Vec<Span>,
) {
    if program.span.start >= program.span.end || program.span.end > src.len() {
        return;
    }

    let mut masks = Vec::new();
    let mut candidates = Vec::new();
    collect_region_facts(program, &mut masks, &mut candidates);
    candidates.sort_by_key(|span| (span.start, span.end));
    candidates.dedup();
    if candidates.is_empty() {
        return;
    }

    for &wrapper in wrappers {
        let mut restored = Vec::new();
        for _ in 0..=candidates.len() {
            let projection = projected_region(src, program.span, &masks, &candidates, &restored);
            let parsed = match parse_wrapped(&projection, source_kind, program.span.start, wrapper)
            {
                Ok(parsed) => parsed,
                Err(error) => {
                    let Some(next) = candidate_at_error(&candidates, &restored, error) else {
                        break;
                    };
                    restored.push(next);
                    continue;
                }
            };
            // Recoverable diagnostics outside an unresolved candidate describe
            // context omitted by this recursive projection, not ownership. A
            // candidate is restored only when its own bytes caused an error;
            // the final name still has to be owned by a host AST declaration.
            let next = parsed
                .errors
                .iter()
                .filter_map(|error| candidate_at_error(&candidates, &restored, *error))
                .min_by_key(|candidate| candidate.end.saturating_sub(candidate.start));
            if let Some(next) = next {
                restored.push(next);
                continue;
            }
            let mut collector = MatchNameCollector {
                frame: parsed.frame,
                spans: Vec::new(),
            };
            parsed.module.visit_with(&mut collector);
            owned.extend(collector.spans);
            return;
        }
    }
}

fn candidate_at_error(candidates: &[Span], restored: &[Span], error: usize) -> Option<Span> {
    candidates
        .iter()
        .filter(|candidate| !restored.contains(candidate))
        .filter(|candidate| candidate.start <= error && error <= candidate.end)
        .min_by_key(|candidate| candidate.end.saturating_sub(candidate.start))
        .copied()
}

fn collect_region_facts(program: &Program, masks: &mut Vec<Mask>, candidates: &mut Vec<Span>) {
    crate::stack::grow(|| collect_region_facts_grown(program, masks, candidates));
}

fn collect_region_facts_grown(
    program: &Program,
    masks: &mut Vec<Mask>,
    candidates: &mut Vec<Span>,
) {
    candidates.extend(program.host_match_candidates().iter().copied());
    let region_candidates: HashSet<(usize, usize)> = program
        .host_match_candidates()
        .iter()
        .map(|span| (span.start, span.end))
        .collect();
    for segment in &program.segments {
        match segment {
            Segment::Verbatim(_) | Segment::TtImport(_) => {}
            Segment::Variant(decl) => masks.push(Mask {
                span: decl.span,
                placeholder: Placeholder::Statement,
            }),
            Segment::Match(expr) => {
                let span = Span {
                    start: expr.keyword_off,
                    end: expr.body_close + 1,
                };
                if !region_candidates.contains(&(span.start, span.end)) {
                    masks.push(Mask {
                        span,
                        placeholder: Placeholder::Expression,
                    });
                }
            }
            Segment::TupleMatch(expr) => {
                let span = Span {
                    start: expr.keyword_off,
                    end: expr.body_close + 1,
                };
                if !region_candidates.contains(&(span.start, span.end)) {
                    masks.push(Mask {
                        span,
                        placeholder: Placeholder::Expression,
                    });
                }
            }
            Segment::Try(stmt) => masks.push(Mask {
                span: stmt.owner_span,
                placeholder: Placeholder::Statement,
            }),
            Segment::TryExpr(expr) => masks.push(Mask {
                span: expr.span,
                placeholder: Placeholder::Expression,
            }),
            Segment::LetElse(stmt) => masks.push(Mask {
                span: stmt.owner_span,
                placeholder: Placeholder::Statement,
            }),
            Segment::IfLet(stmt) => masks.push(Mask {
                span: stmt.owner_span,
                placeholder: Placeholder::Statement,
            }),
            Segment::ValModifier(modifier) => masks.push(Mask {
                span: modifier.span,
                placeholder: Placeholder::Erase,
            }),
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let TemplateChunk::Interp(interp) = chunk {
                        collect_region_facts(interp, masks, candidates);
                    }
                }
            }
            Segment::Pipe(pipe) => masks.push(Mask {
                span: Span {
                    start: pipe.head_span.start,
                    end: pipe
                        .steps
                        .last()
                        .map_or(pipe.head_span.end, |step| step.span.end),
                },
                placeholder: Placeholder::Expression,
            }),
            Segment::ResultBlock(block) => masks.push(Mask {
                span: block.span,
                placeholder: Placeholder::Expression,
            }),
        }
    }

    for recovery in &program.recoveries {
        let placeholder = match recovery.kind {
            RecoveryKind::Expression | RecoveryKind::MatchArms(_) => Placeholder::Expression,
            RecoveryKind::ListElement => Placeholder::Statement,
            RecoveryKind::Statement | RecoveryKind::VariantDecl { .. } => Placeholder::Statement,
            RecoveryKind::Type => Placeholder::Type,
            RecoveryKind::OperandHead => Placeholder::OperandHead,
        };
        if !region_candidates.contains(&(recovery.span.start, recovery.span.end)) {
            masks.push(Mask {
                span: recovery.span,
                placeholder,
            });
        }
    }
}

fn projected_region(
    src: &str,
    region: Span,
    masks: &[Mask],
    candidates: &[Span],
    restored: &[Span],
) -> String {
    let mut bytes = src.as_bytes()[region.start..region.end].to_vec();
    for mask in masks {
        overwrite(&mut bytes, region.start, *mask);
    }
    for span in candidates {
        if restored.contains(span) {
            continue;
        }
        overwrite(
            &mut bytes,
            region.start,
            Mask {
                span: *span,
                placeholder: Placeholder::ProbeExpression,
            },
        );
    }
    String::from_utf8(bytes).expect("parser spans preserve UTF-8 boundaries")
}

fn overwrite(bytes: &mut [u8], base: usize, mask: Mask) {
    let start = mask.span.start.saturating_sub(base).min(bytes.len());
    let end = mask.span.end.saturating_sub(base).min(bytes.len());
    if start >= end {
        return;
    }
    bytes[start..end].fill(b' ');
    match mask.placeholder {
        Placeholder::Expression => bytes[start] = b'0',
        Placeholder::ProbeExpression => {
            let replacement = b"void 0";
            let count = replacement.len().min(end - start);
            bytes[start..start + count].copy_from_slice(&replacement[..count]);
        }
        Placeholder::Statement => bytes[start] = b';',
        Placeholder::OperandHead => {
            let replacement = b"void";
            let count = replacement.len().min(end - start);
            bytes[start..start + count].copy_from_slice(&replacement[..count]);
        }
        Placeholder::Type => {
            let replacement = b"any";
            let count = replacement.len().min(end - start);
            bytes[start..start + count].copy_from_slice(&replacement[..count]);
        }
        Placeholder::Erase => {}
    }
}

fn parse_wrapped(
    source: &str,
    source_kind: crate::SourceKind,
    source_offset: usize,
    wrapper: Wrapper,
) -> Result<HostParse, usize> {
    parse_wrapped_with(source, source_kind, source_offset, wrapper, false)
}

/// [`parse_wrapped`], optionally with the host parser's editor recovery: a
/// syntax error elsewhere in the region then leaves every complete
/// production readable, as TypeScript's parser does.
fn parse_wrapped_with(
    source: &str,
    source_kind: crate::SourceKind,
    source_offset: usize,
    wrapper: Wrapper,
    recovering: bool,
) -> Result<HostParse, usize> {
    crate::work::tick("host parses");
    let (prefix, suffix) = match wrapper {
        Wrapper::Module => ("", ""),
        Wrapper::Expression => ("const __tt_host_probe = (", ");"),
        Wrapper::AsyncExpression => ("async function __tt_host_probe() { return (", ");}"),
        Wrapper::GeneratorExpression => ("function* __tt_host_probe() { return (", ");}"),
        Wrapper::AsyncGeneratorExpression => {
            ("async function* __tt_host_probe() { return (", ");}")
        }
        Wrapper::Statements => ("function __tt_host_probe() {", "}"),
        Wrapper::AsyncStatements => ("async function __tt_host_probe() {", "}"),
        Wrapper::GeneratorStatements => ("function* __tt_host_probe() {", "}"),
        Wrapper::AsyncGeneratorStatements => ("async function* __tt_host_probe() {", "}"),
    };
    let mut projection = String::with_capacity(prefix.len() + source.len() + suffix.len());
    projection.push_str(prefix);
    projection.push_str(source);
    projection.push_str(suffix);
    parse_owned(
        &projection,
        source_kind,
        prefix.len(),
        source_offset,
        source.len(),
        recovering,
    )
}

fn parse_owned(
    src: &str,
    source_kind: crate::SourceKind,
    prefix_len: usize,
    source_offset: usize,
    source_len: usize,
    recovering: bool,
) -> Result<HostParse, usize> {
    // Recovery consumes text the strict parser would reject; the lexical
    // protections that keep the host parser safe on such text come first.
    if recovering && let Some((span, _)) = crate::lexer::host_lexical_error(src, source_kind) {
        return Err(span.start.saturating_sub(prefix_len) + source_offset);
    }
    let input = crate::host_input::HostInput::new(src);
    let frame = SourceFrame {
        origin: input.origin(),
        prefix_len,
        source_offset,
        source_end: source_offset + source_len,
    };
    let mut parser = if recovering {
        input.editor_parser(source_kind)
    } else {
        input.parser(source_kind)
    };
    let module = parser
        .parse_module()
        .map_err(|error| frame.error_offset(error.span()))?;
    let errors = parser
        .take_errors()
        .into_iter()
        .map(|error| frame.error_offset(error.span()))
        .collect();
    Ok(HostParse {
        module,
        frame,
        errors,
    })
}

struct HostParse {
    module: Module,
    frame: SourceFrame,
    errors: Vec<usize>,
}

/// Maps positions in a wrapped projection back to source bytes.
#[derive(Clone, Copy)]
struct SourceFrame {
    origin: crate::host_input::HostOrigin,
    prefix_len: usize,
    source_offset: usize,
    source_end: usize,
}

impl SourceFrame {
    fn error_offset(self, span: SwcSpan) -> usize {
        self.source_offset + self.origin.byte(span.lo).saturating_sub(self.prefix_len)
    }

    fn source_span(self, span: SwcSpan) -> Option<Span> {
        let start = self.origin.byte(span.lo).checked_sub(self.prefix_len)?;
        let end = self.origin.byte(span.hi).checked_sub(self.prefix_len)?;
        let span = Span {
            start: self.source_offset + start,
            end: self.source_offset + end,
        };
        (span.start < span.end && span.end <= self.source_end).then_some(span)
    }
}

/// Collects the source offset where each formal parameter's binding starts.
struct ParameterCollector {
    frame: SourceFrame,
    starts: Vec<usize>,
}

impl ParameterCollector {
    fn insert(&mut self, span: SwcSpan) {
        if let Some(span) = self.frame.source_span(span) {
            self.starts.push(span.start);
        }
    }
}

impl Visit for ParameterCollector {
    fn visit_param(&mut self, node: &Param) {
        self.insert(node.pat.span());
        node.visit_children_with(self);
    }

    fn visit_ts_param_prop(&mut self, node: &TsParamProp) {
        self.insert(node.param.span());
        node.visit_children_with(self);
    }

    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        for param in &node.params {
            self.insert(param.span());
        }
        node.visit_children_with(self);
    }

    fn visit_catch_clause(&mut self, node: &CatchClause) {
        if let Some(param) = &node.param {
            self.insert(param.span());
        }
        node.visit_children_with(self);
    }

    fn visit_ts_fn_param(&mut self, node: &TsFnParam) {
        self.insert(node.span());
        node.visit_children_with(self);
    }
}

struct MatchNameCollector {
    frame: SourceFrame,
    spans: Vec<Span>,
}

impl MatchNameCollector {
    fn insert(&mut self, name: &str, span: SwcSpan) {
        if name != "match" {
            return;
        }
        if let Some(span) = self.frame.source_span(span) {
            self.spans.push(span);
        }
    }

    fn insert_prop_name(&mut self, name: &PropName) {
        if let PropName::Ident(name) = name {
            self.insert(name.sym.as_ref(), name.span);
        }
    }
}

impl Visit for MatchNameCollector {
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        self.insert(node.ident.sym.as_ref(), node.ident.span);
        node.visit_children_with(self);
    }

    fn visit_fn_expr(&mut self, node: &FnExpr) {
        if let Some(name) = &node.ident {
            self.insert(name.sym.as_ref(), name.span);
        }
        node.visit_children_with(self);
    }

    fn visit_class_method(&mut self, node: &ClassMethod) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_private_method(&mut self, node: &PrivateMethod) {
        self.insert(node.key.name.as_ref(), node.key.span);
        node.visit_children_with(self);
    }

    fn visit_method_prop(&mut self, node: &MethodProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_getter_prop(&mut self, node: &GetterProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_setter_prop(&mut self, node: &SetterProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }
}
