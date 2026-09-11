//! TypeScript-owned identifier positions used by the lossless tt parser.
//!
//! Files with ambiguous `match` positions use byte-preserving projections of
//! each recursive parser region. Confirmed tt nodes become category-safe
//! placeholders, while an ambiguous candidate remains source text only after an
//! expression-safe probe is rejected at that position. The projection is
//! retried, and ownership is accepted only when the converged SWC AST contains a
//! host declaration node for the original identifier span.

use std::collections::HashSet;

use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Span as SwcSpan, Spanned};
use swc_ecma_ast::{
    ClassMethod, FnDecl, FnExpr, GetterProp, MethodProp, PrivateMethod, PropName, SetterProp,
};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser as SwcParser, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use crate::ast::{Program, RecoveryKind, Segment, Span, TemplateChunk};

pub(super) fn owned_match_names_in_mixed(
    src: &str,
    source_kind: crate::SourceKind,
    program: &Program,
) -> HashSet<usize> {
    let mut owned = HashSet::new();
    super::parse::visit_programs(program, &mut |region| {
        let wrappers: &[Wrapper] = if std::ptr::eq(region, program) {
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
        };
        probe_region(src, source_kind, region, wrappers, &mut owned);
    });
    owned
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
    owned: &mut HashSet<usize>,
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
            owned.extend(parsed.names);
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
    candidates.extend(program.host_match_candidates.iter().copied());
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
                if !program.host_match_candidates.contains(&span) {
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
                if !program.host_match_candidates.contains(&span) {
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
            Segment::ValModifier(span) => masks.push(Mask {
                span: *span,
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
            RecoveryKind::Expression => Placeholder::Expression,
            RecoveryKind::Statement | RecoveryKind::VariantDecl { .. } => Placeholder::Statement,
            RecoveryKind::Type => Placeholder::Type,
        };
        if !program.host_match_candidates.contains(&recovery.span) {
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
    )
}

fn parse_owned(
    src: &str,
    source_kind: crate::SourceKind,
    prefix_len: usize,
    source_offset: usize,
    source_len: usize,
) -> Result<HostParse, usize> {
    let source_map: Lrc<SourceMap> = Default::default();
    let file = source_map.new_source_file(Lrc::new(FileName::Anon), src.to_owned());
    let source_start = file.start_pos.0;
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: source_kind.is_tsx(),
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = SwcParser::new_from(lexer);
    let module = parser.parse_module().map_err(|error| {
        source_error_offset(error.span(), source_start, prefix_len, source_offset)
    })?;
    let errors = parser
        .take_errors()
        .into_iter()
        .map(|error| source_error_offset(error.span(), source_start, prefix_len, source_offset))
        .collect();

    let mut collector = MatchNameCollector {
        source_start,
        prefix_len,
        source_offset,
        source_end: source_offset + source_len,
        offsets: HashSet::new(),
    };
    module.visit_with(&mut collector);
    Ok(HostParse {
        names: collector.offsets,
        errors,
    })
}

struct HostParse {
    names: HashSet<usize>,
    errors: Vec<usize>,
}

fn source_error_offset(
    span: SwcSpan,
    source_start: u32,
    prefix_len: usize,
    source_offset: usize,
) -> usize {
    let projected = usize::try_from(span.lo.0.saturating_sub(source_start)).unwrap_or(0);
    source_offset + projected.saturating_sub(prefix_len)
}

struct MatchNameCollector {
    source_start: u32,
    prefix_len: usize,
    source_offset: usize,
    source_end: usize,
    offsets: HashSet<usize>,
}

impl MatchNameCollector {
    fn insert(&mut self, name: &str, span: SwcSpan) {
        if name != "match" {
            return;
        }
        let Ok(projected) = usize::try_from(span.lo.0.saturating_sub(self.source_start)) else {
            return;
        };
        let Some(relative) = projected.checked_sub(self.prefix_len) else {
            return;
        };
        let offset = self.source_offset + relative;
        if offset < self.source_end {
            self.offsets.insert(offset);
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
