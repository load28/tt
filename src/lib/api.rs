//! Source-kind, import, module-scan, and variant-symbol APIs.

use super::*;

/// How relative `.tt`/`.ttx` import specifiers are rewritten in emitted
/// TypeScript/TSX. Corresponds to the CLI's `--rewrite-imports` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportRewrite {
    /// `"./x.tt"` → `"./x.js"`, `"./x.ttx"` → `"./x.jsx"` — works under both `moduleResolution:
    /// nodenext` (Node ESM requires the extension) and `bundler` (tsc maps
    /// `.js` to `.ts`). The default.
    #[default]
    Js,
    /// `"./x.tt"` → `"./x.ts"`, `"./x.ttx"` → `"./x.tsx"` — points at the emitted file.
    /// Requires the consumer to enable TypeScript's
    /// `allowImportingTsExtensions` *and* `rewriteRelativeImportExtensions`
    /// (TypeScript 5.7+), which turn `.ts` specifiers into `.js` on emit.
    Ts,
    /// Leave `.tt`/`.ttx` specifiers untouched (byte-for-byte passthrough).
    Off,
}

/// The TypeScript surface accepted by one tt source file.
///
/// This is an explicit compiler input rather than a filename heuristic:
/// embedders often compile buffers without paths, while filesystem clients
/// map TypeScript-family extensions to these two kinds at their boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceKind {
    /// TypeScript without JSX (`.tt` → `.ts`).
    #[default]
    TypeScript,
    /// TypeScript with JSX (`.ttx` → `.tsx`).
    Tsx,
}

impl SourceKind {
    /// Whether this source kind admits JSX syntax.
    pub const fn is_tsx(self) -> bool {
        matches!(self, Self::Tsx)
    }

    /// Maps a tt or TypeScript-family source path to its language kind.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("tt" | "ts" | "mts" | "cts") => Some(Self::TypeScript),
            Some("ttx" | "tsx") => Some(Self::Tsx),
            _ => None,
        }
    }

    /// Maps only compiler-owned tt source extensions to their language kind.
    pub fn from_tt_path(path: &std::path::Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("tt") => Some(Self::TypeScript),
            Some("ttx") => Some(Self::Tsx),
            _ => None,
        }
    }

    /// Extension of the TypeScript source emitted for this kind.
    pub const fn output_extension(self) -> &'static str {
        match self {
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
        }
    }
}

/// A variant declaration from another module, made available to [`compile`]'s
/// pattern semantics via [`Options::extern_variants`].
///
/// Collected by build tools (the `ttc` CLI does this for direct relative
/// `.tt`/`.ttx` imports) with [`exported_variants`] over the imported file's source,
/// filtered through the importing file's clause ([`tt_imports`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternVariant {
    /// The variant's name in the *importing* file's scope (import aliases
    /// applied; `ns.Name` for a namespace import). A local declaration of
    /// the same name shadows it; it shadows a built-in of the same name.
    pub name: String,
    /// The variant's case tags.
    pub tags: Vec<String>,
    /// Where the declaration came from, quoted in error messages —
    /// typically the import specifier as written (e.g. `./token.tt`).
    /// [`exported_variants`] leaves it `None`; the collector fills it in.
    pub from: Option<String>,
}

/// One static relative `.tt`/`.ttx` import (or re-export) of a source file, in
/// source order — the file's outgoing module-graph edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtImport {
    /// The specifier as written, without quotes (e.g. `./token.tt`).
    pub specifier: String,
    /// What the statement brings into local scope.
    pub names: TtImportNames,
}

/// The bindings an [`TtImport`] brings into local scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtImportNames {
    /// `import * as ns from ...` — every export, namespace-qualified.
    Namespace(String),
    /// `import { a, b as c, type d } from ...` — (exported name, alias).
    Named(Vec<(String, Option<String>)>),
    /// A side-effect import or a re-export — nothing enters local scope.
    None,
}

/// Extracts the exported tt variant declarations of a source file as tag-only
/// [`ExternVariant`] entries, without compiling it. Those tags support
/// exhaustiveness and case-name checking across a module boundary. Rich field
/// checking in the project engine uses the [`VariantSymbol`] declarations from
/// [`variant_symbols_with_kind`] instead. Non-exported variants and TypeScript
/// enums are not included. The returned entries have [`ExternVariant::from`]
/// set to `None`.
///
/// ```
/// let decls = ttc::exported_variants(
///     "export variant Token { Num(value: number), Eof }\nvariant Private { A() }\n",
/// );
/// assert_eq!(decls.len(), 1);
/// assert_eq!(decls[0].name, "Token");
/// assert_eq!(decls[0].tags, ["Num", "Eof"]);
/// ```
pub fn exported_variants(source: &str) -> Vec<ExternVariant> {
    exported_variants_with_kind(source, SourceKind::TypeScript)
}

/// [`exported_variants`] under an explicit TypeScript surface kind.
pub fn exported_variants_with_kind(source: &str, source_kind: SourceKind) -> Vec<ExternVariant> {
    let program = parser::parse_with_kind(source, source_kind);
    program
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ast::Segment::Variant(decl) if decl.exported => Some(ExternVariant {
                name: decl.name.clone(),
                tags: decl.cases.iter().map(|case| case.tag.clone()).collect(),
                from: None,
            }),
            _ => None,
        })
        .collect()
}

/// Lists a source file's static relative `.tt`/`.ttx` imports and re-exports, in
/// source order — the edges a build tool follows to collect declarations
/// with [`exported_variants`].
///
/// ```
/// let imports = ttc::tt_imports("import { Token as T } from \"./token.tt\";\n");
/// assert_eq!(imports[0].specifier, "./token.tt");
/// assert_eq!(
///     imports[0].names,
///     ttc::TtImportNames::Named(vec![("Token".into(), Some("T".into()))]),
/// );
/// ```
pub fn tt_imports(source: &str) -> Vec<TtImport> {
    scan_module(source).imports
}

/// [`tt_imports`] under an explicit TypeScript surface kind.
pub fn tt_imports_with_kind(source: &str, source_kind: SourceKind) -> Vec<TtImport> {
    scan_module_with_kind(source, source_kind).imports
}

/// Whether a source file imports any standard-library module.
///
/// Build tools use this to decide whether the module has to be written out
/// (the `ttc` CLI does it automatically) and where the importing file
/// should point — see [`Options::std_imports`].
///
/// ```
/// assert!(ttc::imports_std("import * as Option from \"@tt/std/option\";\n"));
/// assert!(!ttc::imports_std("import * as Option from \"./tt/option.js\";\n"));
/// ```
pub fn imports_std(source: &str) -> bool {
    scan_module(source).imports_std
}

/// A source file's module-level facts, gathered in a **single** parse:
/// its static relative `.tt`/`.ttx` imports ([`tt_imports`]) and whether it
/// imports the standard library ([`imports_std`]).
///
/// A build tool walking a whole project needs both of a file, and parsing
/// is the compiler's most expensive non-tsc phase — asking the two
/// single-fact helpers back to back parses the file twice. The `ttc` CLI
/// scans every input through this.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleScan {
    /// The file's static relative `.tt`/`.ttx` imports and re-exports, in source
    /// order — see [`tt_imports`].
    pub imports: Vec<TtImport>,
    /// Whether the file imports [`STD_SPECIFIER`] — see [`imports_std`].
    pub imports_std: bool,
    /// Whether this file contains a claimed pipeline or `flow` expression.
    pub uses_pipeline: bool,
}

/// Scans a source file's module-level facts in one parse — see
/// [`ModuleScan`].
///
/// ```
/// let scan = ttc::scan_module(
///     "import type { TOption } from \"@tt/std\";\nimport { T } from \"./t.tt\";\n",
/// );
/// assert!(scan.imports_std);
/// assert_eq!(scan.imports[0].specifier, "./t.tt");
/// ```
pub fn scan_module(source: &str) -> ModuleScan {
    scan_module_with_kind(source, SourceKind::TypeScript)
}

/// [`scan_module`] under an explicit TypeScript surface kind.
pub fn scan_module_with_kind(source: &str, source_kind: SourceKind) -> ModuleScan {
    let program = parser::parse_with_kind(source, source_kind);
    let mut scan = ModuleScan {
        uses_pipeline: program_uses_pipeline(&program),
        ..ModuleScan::default()
    };
    for segment in &program.segments {
        let ast::Segment::TtImport(decl) = segment else {
            continue;
        };
        match decl.kind {
            // The standard library is not a project module — nothing to
            // resolve or collect declarations from.
            ast::TtSpecifier::Std(_) => scan.imports_std = true,
            ast::TtSpecifier::Relative(_) => scan.imports.push(TtImport {
                specifier: source[decl.spec.start + 1..decl.spec.end - 1].to_string(),
                names: match &decl.names {
                    ast::TtImportNames::Namespace(ns) => TtImportNames::Namespace(ns.clone()),
                    ast::TtImportNames::Named(entries) => TtImportNames::Named(entries.clone()),
                    ast::TtImportNames::None => TtImportNames::None,
                },
            }),
        }
    }
    scan
}

fn program_uses_pipeline(program: &ast::Program) -> bool {
    program.segments.iter().any(|segment| match segment {
        ast::Segment::Pipe(_) => true,
        ast::Segment::Match(expr) => {
            program_uses_pipeline(&expr.scrutinee)
                || expr.arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|guard| program_uses_pipeline(&guard.expr))
                        || program_uses_pipeline(&arm.body)
                })
        }
        ast::Segment::TupleMatch(expr) => {
            expr.scrutinees
                .iter()
                .any(|(_, scrutinee)| program_uses_pipeline(scrutinee))
                || expr.arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|guard| program_uses_pipeline(&guard.expr))
                        || program_uses_pipeline(&arm.body)
                })
        }
        ast::Segment::Try(stmt) => program_uses_pipeline(&stmt.expr),
        ast::Segment::TryExpr(expr) => program_uses_pipeline(&expr.expr),
        ast::Segment::LetElse(stmt) => {
            program_uses_pipeline(&stmt.expr) || program_uses_pipeline(&stmt.else_body)
        }
        ast::Segment::IfLet(stmt) => if_let_uses_pipeline(stmt),
        ast::Segment::Template(template) => template.chunks.iter().any(|chunk| match chunk {
            ast::TemplateChunk::Raw(_) => false,
            ast::TemplateChunk::Interp(body) => program_uses_pipeline(body),
        }),
        ast::Segment::ResultBlock(block) => {
            block.items.iter().any(|item| {
                let ast::ResultItem::Stmts(body) = item;
                program_uses_pipeline(body)
            }) || block.value.as_ref().is_some_and(program_uses_pipeline)
        }
        ast::Segment::Verbatim(_)
        | ast::Segment::Variant(_)
        | ast::Segment::TtImport(_)
        | ast::Segment::ValModifier(_) => false,
    })
}

fn if_let_uses_pipeline(stmt: &ast::IfLetStmt) -> bool {
    program_uses_pipeline(&stmt.expr)
        || program_uses_pipeline(&stmt.body)
        || stmt.else_part.as_ref().is_some_and(|part| match part {
            ast::IfLetElse::Block(body) => program_uses_pipeline(body),
            ast::IfLetElse::IfLet(next) => if_let_uses_pipeline(next),
        })
}

/// A tt variant declaration with source positions — the symbol-interface
/// counterpart of [`ExternVariant`], produced by [`variant_symbols`] and emitted
/// as JSON by `ttc --symbols` for language tooling (go-to-definition,
/// completion, hover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantSymbol {
    /// The variant's declared name.
    pub name: String,
    /// Byte offset of the name in the source (see [`line_col`]).
    pub offset: usize,
    /// Whether the declaration has an `export` modifier.
    pub exported: bool,
    /// The verbatim `<...>` generic parameter list, or `""`.
    pub generics: String,
    /// The variant's cases, in declaration order.
    pub cases: Vec<VariantCaseSymbol>,
}

/// One case of an [`VariantSymbol`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantCaseSymbol {
    /// The case tag.
    pub tag: String,
    /// Byte offset of the tag in the source.
    pub offset: usize,
    /// `None` for a unit case without parens; `Some` (possibly empty) for
    /// a case with a field list.
    pub fields: Option<Vec<VariantFieldSymbol>>,
}

/// One field of a payload-carrying case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantFieldSymbol {
    /// The field name.
    pub name: String,
    /// Byte offset of the name in the source (see [`line_col`]).
    pub offset: usize,
    /// Whether the field is optional (`name?: T`).
    pub optional: bool,
    /// The verbatim type annotation text.
    pub ty: String,
}

/// Extracts every tt variant declaration of a source file with positions —
/// exported or not, flagged by [`VariantSymbol::exported`]. Plain TypeScript
/// enums are not tt variants and are not included.
///
/// ```
/// let syms = ttc::variant_symbols("export variant Token { Num(value: number), Eof }\n");
/// assert_eq!(syms[0].name, "Token");
/// assert_eq!(ttc::line_col(
///     "export variant Token { Num(value: number), Eof }\n", syms[0].offset), (1, 16));
/// assert_eq!(syms[0].cases[1].tag, "Eof");
/// assert_eq!(syms[0].cases[1].fields, None);
/// ```
pub fn variant_symbols(source: &str) -> Vec<VariantSymbol> {
    variant_symbols_with_kind(source, SourceKind::TypeScript)
}

/// [`variant_symbols`] under an explicit TypeScript surface kind.
pub fn variant_symbols_with_kind(source: &str, source_kind: SourceKind) -> Vec<VariantSymbol> {
    let program = parser::parse_with_kind(source, source_kind);
    program
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ast::Segment::Variant(decl) => Some(VariantSymbol {
                name: decl.name.clone(),
                offset: decl.name_off,
                exported: decl.exported,
                generics: decl.generics.clone(),
                cases: decl
                    .cases
                    .iter()
                    .map(|c| VariantCaseSymbol {
                        tag: c.tag.clone(),
                        offset: c.tag_off,
                        fields: c.fields.as_ref().map(|fields| {
                            fields
                                .iter()
                                .map(|f| VariantFieldSymbol {
                                    name: f.name.clone(),
                                    offset: f.name_off,
                                    optional: f.optional,
                                    ty: f.ty.clone(),
                                })
                                .collect()
                        }),
                    })
                    .collect(),
            }),
            _ => None,
        })
        .collect()
}
