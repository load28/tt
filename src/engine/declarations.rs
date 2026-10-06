//! The declaration surface an editor consumes — the compiler's own answer.
//!
//! This is what retires the editor's second, regex-based implementation of
//! tt semantics (`docs/design/rust-parity-analysis.md` GAP-3): the list of
//! variants visible in a file — local, imported (aliases applied), built-in —
//! comes from [`crate::resolve`], under exactly the shadowing the compiler
//! resolves with, together with everything a completion or an outline
//! needs (case signatures, payload fields, spans for local declarations).
//! The file's `match` sites ride along for the one editing question the
//! declarations cannot answer: where a missing arm is inserted.

use std::path::Path;

use super::documents::Texts;
use crate::hir;
use crate::resolve::{self, DeclOrigin, DefKind};

/// Everything the declaration surface answers for one file.
#[derive(Debug)]
pub struct TtDeclarations {
    /// The variants visible in the file, in resolution order (local
    /// declarations in source order, then imports, then built-ins) —
    /// shadowed names appear once, as the declaration that wins.
    pub variants: Vec<TtVariantDecl>,
    /// The file's `match` sites (nested ones included), in source order.
    pub matches: Vec<TtMatchSite>,
}

/// One visible variant.
#[derive(Debug)]
pub struct TtVariantDecl {
    /// The name this variant is known by in the file's scope.
    pub name: String,
    /// The verbatim `<...>` generic parameter list, or `""`.
    pub generics: String,
    /// Where it comes from.
    pub origin: TtVariantOrigin,
    /// The cases, in declaration order.
    pub cases: Vec<TtCaseDecl>,
}

/// Where a visible variant is declared.
#[derive(Debug, PartialEq, Eq)]
pub enum TtVariantOrigin {
    /// Declared in this file. `name_span` is the declared name; `span` is
    /// the whole declaration — the outline range.
    Local {
        /// Byte span of the declared name.
        name_span: (usize, usize),
        /// Byte span of the whole declaration, from its first modifier
        /// (`export`, `declare`) or `variant` through the closing `}`.
        span: (usize, usize),
    },
    /// Imported; the specifier as written, when recorded.
    Imported {
        /// e.g. `./token.tt`.
        specifier: Option<String>,
    },
    /// A built-in (`Option`, `Result`).
    Builtin,
}

/// One case of a visible variant.
#[derive(Debug)]
pub struct TtCaseDecl {
    /// The tag.
    pub tag: String,
    /// Byte span of the tag, for a local declaration.
    pub name_span: Option<(usize, usize)>,
    /// Byte span of the whole case, tag through the payload's closing `)`,
    /// for a local declaration.
    pub span: Option<(usize, usize)>,
    /// `true` when the case is declared without parens — the constructor
    /// is a plain value, not a call.
    pub unit: bool,
    /// The payload fields (empty for a unit case).
    pub fields: Vec<TtFieldDecl>,
}

/// One payload field of a case.
#[derive(Debug)]
pub struct TtFieldDecl {
    /// The field name.
    pub name: String,
    /// Whether it is optional (`name?: T`).
    pub optional: bool,
    /// The verbatim declared type text.
    pub ty: String,
}

/// One `match` site: where its keyword is and where an arm is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtMatchSite {
    /// Byte offset of the `match` keyword.
    pub keyword: usize,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`.
    pub body_close: usize,
}

/// The declarations visible in `source`, at `path` (which is what resolves
/// its relative `.tt` imports; an editor passes the buffer's text for the
/// file itself). This is the stand-alone question: an imported file is read
/// as saved. A session asks [`super::Workspace::tt_declarations`], which
/// reads its open documents.
pub fn tt_declarations(path: &Path, source: &str) -> TtDeclarations {
    declarations(path, source, Texts::Disk)
}

pub(super) fn declarations(path: &Path, source: &str, texts: Texts<'_>) -> TtDeclarations {
    let program = crate::parser::parse_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let externs: Vec<resolve::ExternDecl> =
        super::language::externs_of(path, source, &|target| texts.read(target))
            .iter()
            .map(Into::into)
            .collect();
    let mut hir = hir::lower_program(hir::FileId(0), source, &program);
    let resolution = resolve::resolve_file(&mut hir, &externs);

    let mut variants = Vec::new();
    for (id, def) in resolution.defs.iter() {
        let DefKind::Variant(data) = &def.kind else {
            continue;
        };
        // Only the winner of each name: a shadowed declaration is not
        // visible.
        if resolution.type_ns.get(&def.name) != Some(&id) {
            continue;
        }
        let cases: Vec<TtCaseDecl> = data
            .variants
            .iter()
            .map(|variant| TtCaseDecl {
                tag: variant.name.clone(),
                name_span: variant
                    .node
                    .and_then(|node| hir.source_map.node_span(node))
                    .map(|s| (s.start, s.end)),
                span: variant
                    .node
                    .and_then(|node| hir.source_map.node_extent(node))
                    .map(|s| (s.start, s.end)),
                unit: variant.fields.is_none(),
                fields: variant
                    .fields
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|field| TtFieldDecl {
                        name: field.name.clone(),
                        optional: field.optional,
                        ty: field.ty_text.clone(),
                    })
                    .collect(),
            })
            .collect();
        let origin = match &data.origin {
            DeclOrigin::Local(node) => {
                let name_span = hir
                    .source_map
                    .node_span(*node)
                    .map(|s| (s.start, s.end))
                    .unwrap_or((0, 0));
                let span = hir
                    .source_map
                    .node_extent(*node)
                    .map_or(name_span, |s| (s.start, s.end));
                TtVariantOrigin::Local { name_span, span }
            }
            DeclOrigin::Imported { from } => TtVariantOrigin::Imported {
                specifier: from.clone(),
            },
            DeclOrigin::Builtin => TtVariantOrigin::Builtin,
        };
        variants.push(TtVariantDecl {
            name: def.name.clone(),
            generics: data.generics.clone(),
            origin,
            cases,
        });
    }

    let mut sites = Vec::new();
    collect_matches(&program, &mut sites);
    let mut matches: Vec<TtMatchSite> = sites.into_iter().map(|(site, _)| site).collect();
    matches.sort_by_key(|m| m.keyword);

    TtDeclarations { variants, matches }
}

pub(super) fn scrutinee_at(
    source: &str,
    source_kind: crate::SourceKind,
    offset: usize,
    position: Option<usize>,
) -> Option<(usize, usize)> {
    let program = crate::parser::parse_with_kind(source, source_kind);
    let mut sites = Vec::new();
    collect_matches(&program, &mut sites);
    let (_, spans) = sites
        .into_iter()
        .filter(|(site, _)| site.body_open < offset && offset <= site.body_close)
        .max_by_key(|(site, _)| site.body_open)?;
    let span = match position {
        Some(position) if spans.len() > 1 => *spans.get(position)?,
        None if spans.len() == 1 => spans[0],
        _ => return None,
    };
    let text = &source[span.start..span.end];
    let start = span.start
        + (text.len()
            - text
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len());
    let end = span.end
        - (text.len()
            - text
                .trim_end_matches(|c: char| c.is_ascii_whitespace())
                .len());
    (start < end).then_some((start, end))
}

/// Every `match` of a program, nested positions included.
fn collect_matches(
    program: &crate::ast::Program,
    out: &mut Vec<(TtMatchSite, Vec<crate::ast::Span>)>,
) {
    crate::stack::grow(|| collect_matches_grown(program, out));
}

fn collect_matches_grown(
    program: &crate::ast::Program,
    out: &mut Vec<(TtMatchSite, Vec<crate::ast::Span>)>,
) {
    use crate::ast::{IfLetElse, ResultItem, Segment, TemplateChunk};
    for segment in &program.segments {
        match segment {
            Segment::Verbatim(_) | Segment::TtImport(_) | Segment::ValModifier(_) => {}
            Segment::Variant(_) => {}
            Segment::Match(expr) => {
                out.push((
                    TtMatchSite {
                        keyword: expr.keyword_off,
                        body_open: expr.body_open,
                        body_close: expr.body_close,
                    },
                    vec![expr.scrutinee_span],
                ));
                collect_matches(&expr.scrutinee, out);
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        collect_matches(&guard.expr, out);
                    }
                    collect_matches(&arm.body, out);
                }
            }
            Segment::TupleMatch(expr) => {
                out.push((
                    TtMatchSite {
                        keyword: expr.keyword_off,
                        body_open: expr.body_open,
                        body_close: expr.body_close,
                    },
                    expr.scrutinees.iter().map(|(span, _)| *span).collect(),
                ));
                for (_, scrutinee) in &expr.scrutinees {
                    collect_matches(scrutinee, out);
                }
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        collect_matches(&guard.expr, out);
                    }
                    collect_matches(&arm.body, out);
                }
            }
            Segment::Try(stmt) => collect_matches(&stmt.expr, out),
            Segment::TryExpr(expr) => collect_matches(&expr.expr, out),
            Segment::LetElse(stmt) => {
                collect_matches(&stmt.expr, out);
                collect_matches(&stmt.else_body, out);
            }
            Segment::IfLet(stmt) => {
                let mut current = Some(stmt);
                while let Some(stmt) = current {
                    collect_matches(&stmt.expr, out);
                    collect_matches(&stmt.body, out);
                    match &stmt.else_part {
                        Some(IfLetElse::Block(block)) => {
                            collect_matches(block, out);
                            current = None;
                        }
                        Some(IfLetElse::IfLet(inner)) => current = Some(inner),
                        None => current = None,
                    }
                }
            }
            Segment::Pipe(pipe) => {
                if let Some(head) = &pipe.head {
                    collect_matches(head, out);
                }
                for step in &pipe.steps {
                    collect_matches(&step.body, out);
                }
            }
            Segment::ResultBlock(block) => {
                for item in &block.items {
                    let ResultItem::Stmts(stmts) = item;
                    collect_matches(stmts, out);
                }
                if let Some(value) = &block.value {
                    collect_matches(value, out);
                }
            }
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let TemplateChunk::Interp(interp) = chunk {
                        collect_matches(interp, out);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarations(source: &str) -> TtDeclarations {
        tt_declarations(Path::new("/nowhere/a.tt"), source)
    }

    #[test]
    fn locals_come_with_spans_imports_and_builtins_without() {
        let src = "export variant Shape { Circle(radius: number), Point }\n";
        let decls = declarations(src);
        let names: Vec<&str> = decls.variants.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Shape", "Option", "Result"]);
        let shape = &decls.variants[0];
        let TtVariantOrigin::Local { name_span, span } = &shape.origin else {
            panic!("Shape is local");
        };
        assert_eq!(&src[name_span.0..name_span.1], "Shape");
        assert_eq!(&src[span.0..span.1], src.trim_end(), "outline range");
        assert!(!shape.cases[0].unit);
        assert_eq!(shape.cases[0].fields[0].ty, "number");
        assert!(shape.cases[1].unit);
        // The built-ins carry their declared type parameters.
        let option = decls.variants.iter().find(|e| e.name == "Option").unwrap();
        assert_eq!(option.generics, "<T>");
        assert_eq!(option.origin, TtVariantOrigin::Builtin);
    }

    #[test]
    fn outline_ranges_enclose_the_whole_declaration_and_each_case() {
        for (src, declaration) in [
            (
                "/** doc */ export declare variant P { R(v: number) }\n",
                "export declare variant P { R(v: number) }",
            ),
            (
                "declare variant P {\n  R(v: number),\n  Q,\n}\n",
                "declare variant P {\n  R(v: number),\n  Q,\n}",
            ),
            (
                "const n = 1;\nvariant P<T> { R(v: T, w?: string), Q }\n",
                "variant P<T> { R(v: T, w?: string), Q }",
            ),
        ] {
            let decls = declarations(src);
            let TtVariantOrigin::Local { name_span, span } = &decls.variants[0].origin else {
                panic!("P is local in {src:?}");
            };
            assert_eq!(&src[span.0..span.1], declaration, "{src:?}");
            assert!(span.0 <= name_span.0 && name_span.1 <= span.1, "{src:?}");
            for case in &decls.variants[0].cases {
                let (name, whole) = (case.name_span.unwrap(), case.span.unwrap());
                assert_eq!(&src[name.0..name.1], case.tag, "{src:?}");
                assert!(whole.0 <= name.0 && name.1 <= whole.1, "{src:?}");
                assert!(span.0 <= whole.0 && whole.1 <= span.1, "{src:?}");
                let text = &src[whole.0..whole.1];
                if case.unit {
                    assert_eq!(text, case.tag, "{src:?}");
                } else {
                    assert!(text.starts_with(&format!("{}(", case.tag)), "{src:?}");
                    assert!(text.ends_with(')'), "{src:?}");
                }
            }
        }
    }

    #[test]
    fn a_local_declaration_shadows_the_builtin_once() {
        let src = "variant Option { Nothing, Just(v: number) }\n";
        let decls = declarations(src);
        let options: Vec<&TtVariantDecl> = decls
            .variants
            .iter()
            .filter(|e| e.name == "Option")
            .collect();
        assert_eq!(options.len(), 1, "the shadowed built-in is not listed");
        assert!(matches!(options[0].origin, TtVariantOrigin::Local { .. }));
        assert_eq!(options[0].cases[0].tag, "Nothing");
    }

    #[test]
    fn matches_carry_their_arm_insertion_point() {
        let src = "variant E { A(x: number), B }\n\
            const v = match (e) { A(x) => match (inner) { B => 0, _ => 1 }, B => 2 };\n";
        let decls = declarations(src);
        assert_eq!(decls.matches.len(), 2);
        for site in &decls.matches {
            assert_eq!(&src[site.keyword..site.keyword + 5], "match");
            assert_eq!(&src[site.body_close..site.body_close + 1], "}");
        }
        assert!(decls.matches[0].keyword < decls.matches[1].keyword);
    }
}
