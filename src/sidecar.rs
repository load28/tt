//! Editor sidecars: `x.tt.d.ts` + `x.tt.d.ts.map`.
//!
//! A `.ts` file that imports `"./x.tt"` gets `TS2307` from tsserver, which
//! does not know the extension. TypeScript's own escape hatch is in that
//! error text — "or its corresponding type declarations" — so placing a
//! declaration file next to the module resolves it.
//!
//! The declaration *body* needs type inference, which is tsc's job (run it
//! with `--emitDeclarationOnly` over ttc's output). What tsc cannot know is
//! where each declaration lives in the original `.tt`; this module supplies
//! that as a declaration map whose `sources` points at the `.tt` file, which
//! is what sends "go to definition" to the original instead of the `.d.ts`.

use std::collections::HashMap;

use swc_ecma_ast::{Decl, Ident, ModuleDecl, ModuleItem, ObjectPatProp, Pat, Stmt, TsModuleName};
use swc_ecma_visit::{Visit, VisitWith};

use crate::host_input::HostInput;
use crate::lines::LineMap;

/// The two files that make up a module's editor sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    /// Contents of `<name>.d.ts` — the declarations plus a
    /// `sourceMappingURL` comment.
    pub declarations: String,
    /// Contents of `<name>.d.ts.map` — a source map v3 document whose
    /// `sources` is the original `.tt` file.
    pub map: String,
}

/// Builds the sidecar for one module.
///
/// `source` is the original `.tt` text, `declarations` is what tsc emitted
/// for ttc's output of that module, and `tt_path` is the path to the `.tt`
/// file **relative to where the sidecar will be written** — `"notice.tt"`
/// when the two sit together, `"../src/notice.tt"` when declarations live
/// in their own tree (which TypeScript merges back with `rootDirs`). Written
/// as a relative URL, it becomes the map's `sources`, and its file name
/// becomes the stem of the written files (`notice.tt.d.ts`) and, as a URL,
/// of the `sourceMappingURL` comment.
///
/// Every exported declaration that can be located in the source gets a
/// mapping segment at the column where its name starts, and a line that has
/// one also gets a segment at column 0. The name's segment is the one that
/// matters — "go to definition" asks about the name's position, and without
/// a segment there the editor stops at the `.d.ts`. Both sides are read as
/// syntax: the names are the declaration file's exported declarations, and
/// each is located at the module-level declaration of that name in the
/// source.
pub fn build_sidecar(source: &str, declarations: &str, tt_path: &str) -> Sidecar {
    let tt_file_name = tt_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(tt_path);
    let source_kind =
        crate::SourceKind::from_path(std::path::Path::new(tt_file_name)).unwrap_or_default();
    let source_lines = LineMap::ecma(source);
    let located = module_declarations(source, source_kind);

    // The declarations are rewritten line by line, and every line the map
    // counts is one ECMA-262 line of what is written: a line ending becomes
    // LF, and a U+2028 or U+2029 — a line terminator that is also text, as
    // inside a string literal — stays itself.
    let declaration_lines = LineMap::ecma(declarations);
    let mut by_line: HashMap<usize, Vec<Hit>> = HashMap::new();
    for (name, byte) in exported_declarations(declarations) {
        let Some(&at) = located.get(&name) else {
            continue;
        };
        let (line, generated_column) = declaration_lines.utf16_position(byte);
        let (source_line, source_column) = source_lines.utf16_position(at);
        by_line.entry(line).or_default().push(Hit {
            generated_column,
            line: source_line,
            column: source_column,
        });
    }
    let mut body = String::new();
    let mut hits: Vec<Vec<Hit>> = Vec::new();
    for index in 0..declaration_lines.len() {
        let line = declaration_lines.line_text(index).unwrap_or_default();
        if line.trim_start().starts_with("//# sourceMappingURL=") {
            continue;
        }
        if !hits.is_empty() {
            body.push_str(match declaration_lines.line_break(index - 1) {
                Some(separator @ ("\u{2028}" | "\u{2029}")) => separator,
                _ => "\n",
            });
        }
        body.push_str(line);
        let mut line_hits = by_line.remove(&index).unwrap_or_default();
        line_hits.sort_by_key(|hit| hit.generated_column);
        hits.push(line_hits);
    }

    let map_name = format!("{tt_file_name}.d.ts.map");
    // The banner goes where a compiled module's does: first, or below a
    // shebang, which TypeScript keeps first in the declarations it emits.
    // Its lines map to nothing.
    let mut declarations = format!(
        "{}\n//# sourceMappingURL={}\n",
        body.trim_end(),
        crate::source_map::url_path([map_name.as_str()])
    );
    let banner = crate::banner::write_banner(
        &mut declarations,
        &format!("// @generated from {tt_file_name} by ttc --sidecar — do not edit.\n"),
    );
    hits.splice(
        banner.at_line..banner.at_line,
        std::iter::repeat_with(Vec::new).take(banner.lines),
    );
    let mappings = encode_mappings(&hits);
    let map = format!(
        "{{\"version\":3,\"file\":{},\"sourceRoot\":\"\",\"sources\":[{}],\"names\":[],\"mappings\":\"{}\"}}\n",
        json_string(&format!("{tt_file_name}.d.ts")),
        json_string(&crate::source_map::url_path(tt_path.split('/'))),
        mappings,
    );

    Sidecar { declarations, map }
}

/// `declarations` — tsc's declarations for ttc's output — with every
/// relative module specifier that ttc rewrote from a `.tt`/`.ttx` import
/// spelled the way the source spells it.
///
/// ttc's output names a tt module by the file it compiles to
/// ([`crate::ImportRewrite`]), and tsc's declarations keep that name. A
/// sidecar is read where the source is — beside the `.tt` files and their
/// sidecars, or in a tree that mirrors them — so it has to name them as the
/// source does, as the sidecars `ttc --types` writes do. A specifier is
/// restored when a rewrite produces it from a `.tt`/`.ttx` specifier that
/// `names_source` confirms names a tt source, relative to the source file;
/// every other byte is kept. A text that does not parse is returned as it is.
pub fn source_specifiers(declarations: &str, names_source: impl Fn(&str) -> bool) -> String {
    let input = HostInput::new(declarations);
    let Ok(module) = input.declaration_parser().parse_module() else {
        return declarations.to_owned();
    };
    let mut specifiers = ModuleSpecifiers::default();
    module.visit_with(&mut specifiers);
    let mut restored = String::with_capacity(declarations.len());
    let mut copied = 0;
    let mut spans: Vec<_> = specifiers
        .0
        .iter()
        .map(|span| (input.byte(span.lo) + 1, input.byte(span.hi) - 1))
        .collect();
    spans.sort_unstable();
    spans.dedup();
    for (start, end) in spans {
        let Some(specifier) = declarations.get(start..end) else {
            continue;
        };
        let Some(source) = [crate::ImportRewrite::Js, crate::ImportRewrite::Ts]
            .into_iter()
            .flat_map(|rewrite| rewrite.source_candidates(specifier))
            .find(|source| names_source(source))
        else {
            continue;
        };
        restored.push_str(&declarations[copied..start]);
        restored.push_str(&source);
        copied = end;
    }
    restored.push_str(&declarations[copied..]);
    restored
}

/// The string literals of a module that name another module.
#[derive(Default)]
struct ModuleSpecifiers(Vec<swc_common::Span>);

impl Visit for ModuleSpecifiers {
    fn visit_import_decl(&mut self, node: &swc_ecma_ast::ImportDecl) {
        self.0.push(node.src.span);
    }

    fn visit_export_all(&mut self, node: &swc_ecma_ast::ExportAll) {
        self.0.push(node.src.span);
    }

    fn visit_named_export(&mut self, node: &swc_ecma_ast::NamedExport) {
        self.0.extend(node.src.as_ref().map(|src| src.span));
    }

    fn visit_ts_import_type(&mut self, node: &swc_ecma_ast::TsImportType) {
        self.0.push(node.arg.span);
        node.visit_children_with(self);
    }

    fn visit_ts_external_module_ref(&mut self, node: &swc_ecma_ast::TsExternalModuleRef) {
        self.0.push(node.expr.span);
    }

    fn visit_ts_module_decl(&mut self, node: &swc_ecma_ast::TsModuleDecl) {
        if let TsModuleName::Str(name) = &node.id {
            self.0.push(name.span);
        }
        node.visit_children_with(self);
    }
}

struct Hit {
    generated_column: usize,
    line: usize,
    column: usize,
}

/// The names a declaration file exports by declaring them, each with the
/// byte offset of its identifier. A text that does not parse declares
/// nothing.
fn exported_declarations(declarations: &str) -> Vec<(String, usize)> {
    let input = HostInput::new(declarations);
    let Ok(module) = input.declaration_parser().parse_module() else {
        return Vec::new();
    };
    module
        .body
        .iter()
        .filter_map(|item| match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => Some(&export.decl),
            _ => None,
        })
        .flat_map(declared_identifiers)
        .map(|ident| (ident.sym.to_string(), input.byte(ident.span.lo)))
        .collect()
}

/// Where the source's module-level declarations declare their names, as
/// byte offsets of the source — the first in source order where a name is
/// declared more than once (overloads, a variant's type and constructor).
///
/// The source is tt, so it is read through the TypeScript ttc emits for it:
/// the emitted module's declarations are parsed, and each name is placed
/// where the emission took it from — the source bytes the chunk holding it
/// was copied from, or the tt name ttc declared it for.
fn module_declarations(source: &str, source_kind: crate::SourceKind) -> HashMap<String, usize> {
    let emit = crate::emit_mapped_with_kind(source, source_kind);
    let input = HostInput::new(&emit.code);
    let mut located: HashMap<String, usize> = HashMap::new();
    let Ok(module) = input.parser(source_kind).parse_module() else {
        return located;
    };
    let declarations = module.body.iter().filter_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(declaration)) => Some(declaration),
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => Some(&export.decl),
        _ => None,
    });
    for ident in declarations.flat_map(declared_identifiers) {
        let Some(at) = source_byte(&emit, input.byte(ident.span.lo)) else {
            continue;
        };
        located
            .entry(ident.sym.to_string())
            .and_modify(|first| *first = (*first).min(at))
            .or_insert(at);
    }
    located
}

/// The source byte an emitted byte was written for.
fn source_byte(emit: &crate::MappedEmit, out: usize) -> Option<usize> {
    emit.mappings
        .iter()
        .find(|chunk| chunk.out <= out && out < chunk.out + chunk.len)
        .map(|chunk| chunk.src + (out - chunk.out))
        .or_else(|| {
            emit.declared_names
                .iter()
                .find(|name| name.out <= out && out < name.out_end)
                .map(|name| name.src + (out - name.out).min(name.src_end - name.src))
        })
}

/// The identifiers a declaration binds.
fn declared_identifiers(declaration: &Decl) -> Vec<&Ident> {
    let mut out = Vec::new();
    match declaration {
        Decl::Class(class) => out.push(&class.ident),
        Decl::Fn(function) => out.push(&function.ident),
        Decl::Var(var) => {
            for declarator in &var.decls {
                pattern_identifiers(&declarator.name, &mut out);
            }
        }
        Decl::Using(using) => {
            for declarator in &using.decls {
                pattern_identifiers(&declarator.name, &mut out);
            }
        }
        Decl::TsInterface(interface) => out.push(&interface.id),
        Decl::TsTypeAlias(alias) => out.push(&alias.id),
        Decl::TsEnum(declaration) => out.push(&declaration.id),
        Decl::TsModule(declaration) => {
            if let TsModuleName::Ident(ident) = &declaration.id {
                out.push(ident);
            }
        }
    }
    out
}

fn pattern_identifiers<'a>(pattern: &'a Pat, out: &mut Vec<&'a Ident>) {
    crate::stack::grow(|| pattern_identifiers_grown(pattern, out));
}

fn pattern_identifiers_grown<'a>(pattern: &'a Pat, out: &mut Vec<&'a Ident>) {
    match pattern {
        Pat::Ident(binding) => out.push(&binding.id),
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                pattern_identifiers(element, out);
            }
        }
        Pat::Rest(rest) => pattern_identifiers(&rest.arg, out),
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(pair) => pattern_identifiers(&pair.value, out),
                    ObjectPatProp::Assign(assign) => out.push(&assign.key.id),
                    ObjectPatProp::Rest(rest) => pattern_identifiers(&rest.arg, out),
                }
            }
        }
        Pat::Assign(assign) => pattern_identifiers(&assign.left, out),
        Pat::Expr(_) | Pat::Invalid(_) => {}
    }
}

/// Encodes the located declarations of each generated line into a source
/// map v3 `mappings` string: a segment at column 0 for the line's first
/// declaration, then one at each declaration's name.
fn encode_mappings(hits: &[Vec<Hit>]) -> String {
    let mut previous_line: i64 = 0;
    let mut previous_column: i64 = 0;
    let mut lines: Vec<String> = Vec::with_capacity(hits.len());

    for line_hits in hits {
        let Some(first) = line_hits.first() else {
            lines.push(String::new());
            continue;
        };
        let start = (first.generated_column > 0).then_some(Hit {
            generated_column: 0,
            line: first.line,
            column: first.column,
        });
        let mut previous_generated: i64 = 0;
        let mut segments: Vec<String> = Vec::new();
        for hit in start.iter().chain(line_hits) {
            let mut segment = String::new();
            vlq(
                hit.generated_column as i64 - previous_generated,
                &mut segment,
            );
            vlq(0, &mut segment);
            vlq(hit.line as i64 - previous_line, &mut segment);
            vlq(hit.column as i64 - previous_column, &mut segment);
            previous_generated = hit.generated_column as i64;
            previous_line = hit.line as i64;
            previous_column = hit.column as i64;
            segments.push(segment);
        }
        lines.push(segments.join(","));
    }

    lines.join(";")
}

/// Base64 VLQ, the source map encoding for a signed delta.
fn vlq(value: i64, out: &mut String) {
    const DIGITS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = if value < 0 {
        ((-value) << 1) | 1
    } else {
        value << 1
    };
    loop {
        let mut digit = (bits & 31) as usize;
        bits >>= 5;
        if bits > 0 {
            digit |= 32;
        }
        out.push(DIGITS[digit] as char);
        if bits == 0 {
            break;
        }
    }
}

/// Minimal JSON string literal (the map only ever carries file names).
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
