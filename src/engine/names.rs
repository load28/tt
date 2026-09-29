//! tt's own names — the semantic surface the checker cannot be asked about.
//!
//! Three of tt's name spaces exist only in `.tt` source: a **variant name**,
//! a **case tag**, and a **payload field name**. Inside the declaration and
//! in a pattern, none survives lowering in a form TypeScript can be pointed
//! at — a variant declaration is synthesized text with no mapping back, a
//! tag becomes a string literal, a field a destructuring key. So the answers
//! TypeScript gives for every other identifier (hover, go-to-definition) are
//! simply absent there, and tt has to give them itself.
//!
//! This module is that answer, and it follows the same layering the rest of
//! the engine does:
//!
//! - It is **parse-only**: source in, symbol out. No toolchain, no project,
//!   no service session — so it answers in a buffer mid-edit and in a
//!   workspace with no TypeScript installed, exactly like semantic tokens.
//! - It answers **only where the checker cannot be asked**. A use site like
//!   `Shape.Circle(1)` or a type annotation `const s: Shape` lowers to
//!   ordinary TypeScript that the checker knows better than tt does, so
//!   this module says nothing about those positions and lets the service
//!   answer. (The editor's previous implementation claimed them, which is
//!   how a local variable that happened to share a case's name came to
//!   hover as a variant case.)
//!
//! Where the declarations come from is the analysis' table, so an imported
//! variant is found under the name the import gave it, and a local declaration
//! shadows it — one resolution rule, not a second one written here.

use std::path::{Path, PathBuf};

use crate::analysis::{DeclaredVariant, NameKind, Origin};
use crate::{MatchConstructor, PayloadField, VariantCaseSymbol, VariantSymbol};

use super::documents::Texts;
use super::language::{Location, Position, Range};

/// What a tt name names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtSymbolKind {
    /// A variant, at its declaration.
    Variant,
    /// One case of a variant.
    Case,
    /// One payload field of a case.
    Field,
}

/// A tt name the editor asked about, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtSymbol {
    /// What the name names.
    pub kind: TtSymbolKind,
    /// The identifier's range in the `.tt` source — what an editor
    /// highlights.
    pub range: Range,
    /// The name as written.
    pub name: String,
    /// The variant it belongs to (itself, for a variant).
    pub variant_name: String,
    /// The declaration rendered in tt syntax — the hover's first line.
    pub signature: String,
    /// One sentence about what it is and where it came from.
    pub detail: String,
    /// Where it is declared, when that is a place the editor can open.
    pub definition: Option<Location>,
    /// Whether the identifier also declares a local binding — a shorthand
    /// payload pattern (`Circle(radius)`), whose rename is the binding's.
    pub binds: bool,
}

/// The tt name at `position`, or `None` when the position is not on one.
///
/// `path` is the file the source belongs to; it is used to resolve
/// relative `.tt` imports and to name the file a definition lives in.
///
/// This is the stand-alone question: an imported file is read as saved. A
/// session asks [`super::Workspace::tt_symbol_at`], which reads its open
/// documents.
pub fn tt_symbol_at(path: &Path, source: &str, position: Position) -> Option<TtSymbol> {
    symbol_at(path, source, position, Texts::Disk)
}

pub(super) fn symbol_at(
    path: &Path,
    source: &str,
    position: Position,
    texts: Texts<'_>,
) -> Option<TtSymbol> {
    let offset = super::language::source_byte(source, position);
    let locals = crate::variant_symbols_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let analyses = super::language::analyses_for(path, source, texts);

    // 1. Inside a variant declaration: the name, a case tag, a field name.
    //    These bytes have no counterpart in the output at all.
    if let Some(found) = declaration_at(path, source, &locals, offset) {
        return Some(found);
    }

    // 2. In a pattern: a case tag or a payload field name, resolved by the
    //    analysis against the same declaration table sema uses.
    let resolved = analyses
        .resolved
        .iter()
        .find(|r| r.start <= offset && offset <= r.end)?;
    let declared = analyses
        .declarations
        .iter()
        .find(|d| d.name == resolved.variant_name)?;
    let binds = resolved.kind == NameKind::Field
        && analyses
            .binding_at(resolved.start)
            .is_some_and(|binding| binding.start == resolved.start && binding.end == resolved.end);
    let (kind, signature, detail, definition) = match resolved.kind {
        NameKind::Case => {
            let constructor = declared
                .constructors
                .iter()
                .find(|c| c.tag == resolved.name)?;
            (
                TtSymbolKind::Case,
                case_signature(&declared.name, constructor),
                case_detail(declared, constructor),
                case_definition(path, source, &locals, declared, &resolved.name, texts),
            )
        }
        NameKind::Field => {
            let tag = resolved.tag.as_deref()?;
            let constructor = declared.constructors.iter().find(|c| c.tag == tag)?;
            let field = constructor
                .fields
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|f| f.name == resolved.name)?;
            (
                TtSymbolKind::Field,
                field_signature(field),
                format!(
                    "payload field of `{}.{tag}` — the pattern binds it by name",
                    declared.name
                ),
                field_definition(path, source, &locals, declared, tag, &resolved.name, texts),
            )
        }
    };
    Some(TtSymbol {
        kind,
        range: super::language::span_range(source, resolved.start, resolved.end),
        name: resolved.name.clone(),
        variant_name: declared.name.clone(),
        signature,
        detail,
        definition,
        binds,
    })
}

/// Every name in `source`'s patterns that resolves to the tt declaration at
/// `target` — the case tags and payload fields TypeScript never sees, since
/// they lower to string literals and destructuring keys. A name resolves
/// here by the same rule it does for [`tt_symbol_at`], so a pattern is a
/// reference exactly when go-to-definition from it lands on `target`.
pub(super) fn tt_pattern_references(
    path: &Path,
    source: &str,
    target: &Location,
    texts: Texts<'_>,
) -> Vec<Range> {
    let locals = crate::variant_symbols_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let analyses = super::language::analyses_for(path, source, texts);
    let mut definitions = std::collections::HashMap::new();
    let mut out = Vec::new();
    for resolved in &analyses.resolved {
        let key = (
            resolved.kind == NameKind::Case,
            resolved.variant_name.clone(),
            resolved.tag.clone(),
            resolved.name.clone(),
        );
        let definition = definitions
            .entry(key)
            .or_insert_with(|| {
                let declared = analyses
                    .declarations
                    .iter()
                    .find(|d| d.name == resolved.variant_name)?;
                match resolved.kind {
                    NameKind::Case => {
                        case_definition(path, source, &locals, declared, &resolved.name, texts)
                    }
                    NameKind::Field => field_definition(
                        path,
                        source,
                        &locals,
                        declared,
                        resolved.tag.as_deref()?,
                        &resolved.name,
                        texts,
                    ),
                }
            })
            .clone();
        if definition.is_some_and(|definition| same_location(&definition, target)) {
            out.push(super::language::span_range(
                source,
                resolved.start,
                resolved.end,
            ));
        }
    }
    out
}

/// Whether two locations name the same range of the same file, however
/// each path was spelled.
pub(crate) fn same_location(a: &Location, b: &Location) -> bool {
    let file =
        |path: &Path| super::normalize_document_path(path).unwrap_or_else(|_| path.to_path_buf());
    a.range == b.range && file(&a.path) == file(&b.path)
}

/// The symbol at `offset` when it sits inside one of this file's own variant/// The symbol at `offset` when it sits inside one of this file's own variant
/// declarations — the one region that lowers to text with no mapping at
/// all, so nothing else can answer for it.
fn declaration_at(
    path: &Path,
    source: &str,
    locals: &[VariantSymbol],
    offset: usize,
) -> Option<TtSymbol> {
    for declaration in locals {
        if offset >= declaration.offset && offset <= declaration.offset + declaration.name.len() {
            let here = Location {
                path: path.to_path_buf(),
                range: super::language::span_range(
                    source,
                    declaration.offset,
                    declaration.offset + declaration.name.len(),
                ),
            };
            return Some(TtSymbol {
                kind: TtSymbolKind::Variant,
                range: here.range,
                name: declaration.name.clone(),
                variant_name: declaration.name.clone(),
                signature: variant_signature(declaration),
                detail: "tt variant — compiles to a `kind`-tagged union type and a constructor \
                         object of the same name"
                    .to_string(),
                definition: Some(here),
                binds: false,
            });
        }
        for case in &declaration.cases {
            if offset >= case.offset && offset <= case.offset + case.tag.len() {
                return Some(symbol_of_local_case(path, source, declaration, case));
            }
            for field in case.fields.as_deref().unwrap_or_default() {
                if offset >= field.offset && offset <= field.offset + field.name.len() {
                    let range = super::language::span_range(
                        source,
                        field.offset,
                        field.offset + field.name.len(),
                    );
                    return Some(TtSymbol {
                        kind: TtSymbolKind::Field,
                        range,
                        name: field.name.clone(),
                        variant_name: declaration.name.clone(),
                        signature: format!(
                            "{}{}: {}",
                            field.name,
                            if field.optional { "?" } else { "" },
                            field.ty
                        ),
                        detail: format!(
                            "payload field of `{}.{}` — a pattern binds it by name",
                            declaration.name, case.tag
                        ),
                        definition: Some(Location {
                            path: path.to_path_buf(),
                            range,
                        }),
                        binds: false,
                    });
                }
            }
        }
    }
    None
}

fn symbol_of_local_case(
    path: &Path,
    source: &str,
    declaration: &VariantSymbol,
    case: &VariantCaseSymbol,
) -> TtSymbol {
    let range = super::language::span_range(source, case.offset, case.offset + case.tag.len());
    let constructor = MatchConstructor {
        tag: case.tag.clone(),
        fields: case.fields.as_ref().map(|fields| {
            fields
                .iter()
                .map(|f| PayloadField {
                    name: f.name.clone(),
                    optional: f.optional,
                    ty: f.ty.clone(),
                })
                .collect()
        }),
    };
    TtSymbol {
        kind: TtSymbolKind::Case,
        range,
        name: case.tag.clone(),
        variant_name: declaration.name.clone(),
        signature: case_signature(&declaration.name, &constructor),
        detail: unit_or_payload(&constructor),
        definition: Some(Location {
            path: path.to_path_buf(),
            range,
        }),
        binds: false,
    }
}

/// Where a case is declared: this file when the variant is local, the imported
/// file when the analysis found it there, and nowhere for a built-in (whose
/// declaration is the compiler's own).
fn case_definition(
    path: &Path,
    source: &str,
    locals: &[VariantSymbol],
    declared: &DeclaredVariant,
    tag: &str,
    texts: Texts<'_>,
) -> Option<Location> {
    if let Some(local) = locals.iter().find(|d| d.name == declared.name) {
        let case = local.cases.iter().find(|c| c.tag == tag)?;
        return Some(Location {
            path: path.to_path_buf(),
            range: super::language::span_range(source, case.offset, case.offset + case.tag.len()),
        });
    }
    let (target, text, imported) = imported_declaration(path, source, declared, texts)?;
    let case = imported.cases.iter().find(|c| c.tag == tag)?;
    Some(Location {
        path: target,
        range: super::language::span_range(&text, case.offset, case.offset + case.tag.len()),
    })
}

fn field_definition(
    path: &Path,
    source: &str,
    locals: &[VariantSymbol],
    declared: &DeclaredVariant,
    tag: &str,
    name: &str,
    texts: Texts<'_>,
) -> Option<Location> {
    let find = |declaration: &VariantSymbol| -> Option<(usize, usize)> {
        let case = declaration.cases.iter().find(|c| c.tag == tag)?;
        let field = case.fields.as_deref()?.iter().find(|f| f.name == name)?;
        Some((field.offset, field.offset + field.name.len()))
    };
    if let Some(local) = locals.iter().find(|d| d.name == declared.name) {
        let (start, end) = find(local)?;
        return Some(Location {
            path: path.to_path_buf(),
            range: super::language::span_range(source, start, end),
        });
    }
    let (target, text, imported) = imported_declaration(path, source, declared, texts)?;
    let (start, end) = find(&imported)?;
    Some(Location {
        path: target,
        range: super::language::span_range(&text, start, end),
    })
}

/// The file an imported variant was declared in, its text, and the declaration
/// — found the way the analysis found it, by walking this file's relative
/// `.tt` imports.
fn imported_declaration(
    path: &Path,
    source: &str,
    declared: &DeclaredVariant,
    texts: Texts<'_>,
) -> Option<(PathBuf, String, VariantSymbol)> {
    let Origin::Imported { .. } = declared.origin else {
        return None;
    };
    let read = std::cell::RefCell::new(std::collections::HashMap::new());
    let imports = crate::tt_imports_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let (target, found) = super::language::imported_variants(path, &imports, &|target| {
        let text = texts.read(target)?;
        let exported = crate::exported_variant_symbols_with_kind(
            &text,
            crate::SourceKind::from_path(target).unwrap_or_default(),
        );
        read.borrow_mut().insert(target.to_path_buf(), text);
        Some(exported)
    })
    .into_iter()
    .find(|(_, symbol)| symbol.name == declared.name)?;
    let text = read.borrow_mut().remove(&target)?;
    Some((target, text, found))
}

/// `variant Shape { Circle(radius: number), Point }` — the declaration as the
/// user would write it, on one line.
fn variant_signature(declaration: &VariantSymbol) -> String {
    let cases: Vec<String> = declaration
        .cases
        .iter()
        .map(|case| match &case.fields {
            None => case.tag.clone(),
            Some(fields) => format!(
                "{}({})",
                case.tag,
                fields
                    .iter()
                    .map(|f| format!("{}{}: {}", f.name, if f.optional { "?" } else { "" }, f.ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
        .collect();
    format!(
        "variant {}{} {{ {} }}",
        declaration.name,
        declaration.generics,
        cases.join(", ")
    )
}

pub(super) fn case_signature(variant_name: &str, constructor: &MatchConstructor) -> String {
    match &constructor.fields {
        None => format!("{variant_name}.{}", constructor.tag),
        Some(fields) => format!(
            "{variant_name}.{}({})",
            constructor.tag,
            fields
                .iter()
                .map(field_signature)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(super) fn field_signature(field: &PayloadField) -> String {
    format!(
        "{}{}: {}",
        field.name,
        if field.optional { "?" } else { "" },
        field.ty
    )
}

fn case_detail(declared: &DeclaredVariant, constructor: &MatchConstructor) -> String {
    let origin = match &declared.origin {
        Origin::Local => format!("`variant {}`", declared.name),
        Origin::Builtin => format!("built-in `variant {}`", declared.name),
        Origin::Imported { from: Some(from) } => {
            format!("`variant {}` (imported from \"{from}\")", declared.name)
        }
        Origin::Imported { from: None } => format!("imported `variant {}`", declared.name),
    };
    format!("{} of {origin}", unit_or_payload(constructor))
}

fn unit_or_payload(constructor: &MatchConstructor) -> String {
    match &constructor.fields {
        None => "unit case — compiles to a singleton value".to_string(),
        Some(_) => "payload case — compiles to a constructor function".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(source: &str, needle: &str, delta: usize) -> Position {
        let offset = source.find(needle).expect("needle") + delta;
        let (line, character) = crate::lines::LineMap::lsp(source).utf16_position(offset);
        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    fn symbol(source: &str, needle: &str, delta: usize) -> TtSymbol {
        tt_symbol_at(Path::new("/p/a.tt"), source, at(source, needle, delta))
            .unwrap_or_else(|| panic!("no tt symbol at {needle:?}+{delta}"))
    }

    const SRC: &str = "variant Shape { Circle(radius: number), Point }\n\
                       const a = match (s) { Circle(radius) => radius, Point => 0 };\n\
                       if let Circle(radius: r) = s { use(r); }\n";

    #[test]
    fn a_variant_declaration_answers_for_its_own_names() {
        let e = symbol(SRC, "variant Shape", 8);
        assert_eq!(e.kind, TtSymbolKind::Variant);
        assert_eq!(
            e.signature,
            "variant Shape { Circle(radius: number), Point }"
        );

        let case = symbol(SRC, "Circle(radius: number)", 0);
        assert_eq!(case.kind, TtSymbolKind::Case);
        assert_eq!(case.signature, "Shape.Circle(radius: number)");

        let field = symbol(SRC, "radius: number", 0);
        assert_eq!(field.kind, TtSymbolKind::Field);
        assert_eq!(field.signature, "radius: number");
    }

    #[test]
    fn a_pattern_tag_resolves_to_its_case() {
        let case = symbol(SRC, "Circle(radius) =>", 0);
        assert_eq!(case.kind, TtSymbolKind::Case);
        assert_eq!(case.signature, "Shape.Circle(radius: number)");
        assert!(case.detail.contains("payload case"), "{}", case.detail);
        // ...and points at the declaration.
        let definition = case.definition.expect("declared in this file");
        assert_eq!(definition.range.start.line, 0);
    }

    #[test]
    fn protocol_positions_count_the_protocols_line_breaks() {
        const DECLARATION: &str = "variant Shape { Circle(radius: number), Point }";
        const USE: &str = "const a = match (s) { Circle(radius) => radius, Point => 0 };";
        for (separator, position) in [
            ("\r", (1, 22)),
            ("\r\n", (1, 22)),
            ("\u{2028}", (0, DECLARATION.len() + 1 + 22)),
            ("\u{2029}\r", (1, 22)),
        ] {
            let source = format!("\u{feff}{DECLARATION}{separator}{USE}{separator}");
            let position = Position {
                line: position.0 as u32,
                character: position.1 as u32,
            };
            let case = tt_symbol_at(Path::new("/p/a.tt"), &source, position)
                .unwrap_or_else(|| panic!("no symbol in {source:?}"));
            assert_eq!(case.signature, "Shape.Circle(radius: number)");
            let definition = case.definition.expect("declared in this file");
            assert_eq!(
                (
                    definition.range.start.line,
                    definition.range.start.character
                ),
                (0, 16),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_if_let_pattern_answers_too() {
        // The construct with no answer at all before this module.
        let case = symbol(SRC, "Circle(radius: r)", 0);
        assert_eq!(case.signature, "Shape.Circle(radius: number)");
        let field = symbol(SRC, "radius: r)", 0);
        assert_eq!(field.kind, TtSymbolKind::Field);
        assert_eq!(field.signature, "radius: number");
    }

    #[test]
    fn a_shorthand_payload_binding_is_a_field_that_also_binds() {
        let src = "variant Shape { Circle(radius: number), Rect(width: number, height: number) }\n\
                   const a = match (s) { Circle(radius) => radius, Rect(width: w, height) => w };\n\
                   let Rect(width, height: h) = s else { throw 0; };\n\
                   if let Circle(radius: r) = s { use(r); }\n";
        for (needle, delta) in [
            ("radius) =>", 0),
            ("height) =>", 0),
            ("width, height: h", 0),
        ] {
            let shorthand = symbol(src, needle, delta);
            assert_eq!(shorthand.kind, TtSymbolKind::Field, "{needle}");
            assert!(shorthand.binds, "{needle}");
            assert!(shorthand.definition.is_some(), "{needle}");
        }
        for (needle, delta) in [
            ("width: w", 0),
            ("height: h", 0),
            ("radius: r", 0),
            ("radius: number", 0),
            ("width: number", 0),
            ("Circle(radius) =>", 0),
            ("Shape {", 0),
        ] {
            assert!(!symbol(src, needle, delta).binds, "{needle}");
        }
    }

    #[test]
    fn a_builtin_case_is_named_as_one() {
        let src = "const n = match (o) { Some(value) => value, None => 0 };\n";
        let case = symbol(src, "Some(value)", 0);
        assert_eq!(case.variant_name, "Option");
        assert!(case.detail.contains("built-in"), "{}", case.detail);
        // The built-ins have no declaration to open.
        assert_eq!(case.definition, None);
    }

    #[test]
    fn an_imported_case_is_found_under_the_name_the_buffer_imports_it_by() {
        let dir = crate::test_workspace::Workspace::new("imported-definition");
        let shapes = "export variant Shape { Circle(r: number), Point }\n";
        std::fs::write(dir.join("shapes.tt"), shapes).unwrap();
        let user = dir.join("user.tt");
        std::fs::write(&user, "export const saved = 1;\n").unwrap();
        let declared = dir.join("shapes.tt").canonicalize().unwrap();
        let case = shapes.find("Circle").unwrap();
        let field = shapes.find("r:").unwrap();
        let arms = "match (s) { Circle(r) => r, Point => 0 }";
        for header in [
            "import { Shape as S } from \"./shapes.tt\";\ndeclare const s: S;\n",
            "import { Gone } from \"./gone.tt\";\nimport { Shape } from \"./shapes.tt\";\n\
             declare const s: Shape;\n",
            "import * as ns from \"./shapes.tt\";\ndeclare const s: ns.Shape;\n",
        ] {
            let source = format!("{header}export const f = {arms};\n");
            let tag = tt_symbol_at(&user, &source, at(&source, "Circle(r)", 0))
                .unwrap_or_else(|| panic!("no case in {source}"));
            let definition = tag.definition.unwrap_or_else(|| panic!("{source}"));
            assert_eq!(definition.path, declared, "{source}");
            assert_eq!(
                definition.range,
                super::super::language::span_range(shapes, case, case + 6),
                "{source}"
            );
            let binding = tt_symbol_at(&user, &source, at(&source, "Circle(r)", 7))
                .unwrap_or_else(|| panic!("no field in {source}"));
            assert_eq!(
                binding.definition.map(|location| location.range),
                Some(super::super::language::span_range(shapes, field, field + 1)),
                "{source}"
            );
        }
    }

    #[test]
    fn ordinary_identifiers_are_left_to_the_checker() {
        // A binding, a value use of the constructor, a type annotation: all
        // of them lower to TypeScript the service knows better.
        let src = "variant Shape { Circle(radius: number), Point }\n\
                   const s: Shape = Shape.Circle(1);\n\
                   const Point = 2;\n";
        assert!(tt_symbol_at(Path::new("/p/a.tt"), src, at(src, "s: Shape", 0)).is_none());
        assert!(tt_symbol_at(Path::new("/p/a.tt"), src, at(src, "Shape.Circle(1)", 0)).is_none());
        assert!(tt_symbol_at(Path::new("/p/a.tt"), src, at(src, "const Point = 2", 6)).is_none());
    }
}
