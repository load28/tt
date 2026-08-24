//! tt — a tiny preprocessor language that compiles to TypeScript and TSX.
//!
//! Every valid TypeScript file is a valid `.tt` file, and every valid TSX file
//! is a valid `.ttx` file. Both compile to themselves byte for byte; the
//! compiler only rewrites the constructs tt adds —
//! Rust-style `enum` declarations (plain TypeScript enums pass through
//! untouched), `match` expressions (literal, tuple and nested patterns
//! included), `try` statements (Rust-`?`-style error propagation over
//! `Result`), let-else and `if let` statements, and the pipeline operator
//! `|>` — plus the `val` binding modifier, which is erased, and relative
//! `.tt`/`.ttx` import specifiers, which are rewritten to a consumable form (see
//! [`ImportRewrite`]). tt-level errors — duplicate cases, non-exhaustive
//! matches, bad field types, misplaced `try`, mutation through a `val`
//! binding — are ttc compile errors with exact positions; the emitted
//! output is plain TypeScript.
//!
//! The core public API is [`compile`] plus its [`Options`] (with
//! [`ImportRewrite`]) and error type [`CompileError`] — code, or the first
//! error. The multi-diagnostic forms are [`analyze`] (every tt-level
//! [`Diagnostic`], in source order) and [`compile_report`] (the same, plus
//! the emission when one is possible); the tree-shakeable standard library
//! modules are exposed through [`StdModule`] and the `STD_*_SOURCE`
//! constants. The `ttc` binary in this crate is a thin CLI over it.
//!
//! # Example
//!
//! ```
//! use ttc::{compile, Options};
//!
//! let source = r#"
//! export enum Shape {
//!   Circle(radius: number),
//!   Point,
//! }
//!
//! const area = match (Shape.Circle(2)) {
//!   Circle(radius) => Math.PI * radius * radius,
//!   Point => 0,
//! };
//! "#;
//!
//! let ts = compile(source, &Options::default())?;
//! assert!(ts.contains(r#"{ kind: "Circle"; radius: number }"#));
//! assert!(ts.contains("switch ($tt_m.kind)"));
//! # Ok::<(), ttc::CompileError>(())
//! ```
//!
//! # Documentation
//!
//! - `README.md` / `README.ko.md` — installation, language overview, and
//!   contributor setup.
//! - `ttc help <topic>` — the language and workflow guide embedded in the CLI.
//! - `docs/design/` — architecture and design decisions.

mod analysis;
mod ast;
mod codegen;
mod core_ir;
mod diagnostics;
pub mod engine;
mod error;
mod evaluation_ir;
pub mod flow;
pub mod hir;
mod ice;
mod lexer;
mod parser;
mod probe;
mod program_syntax;
pub mod resolve;
mod scanner;
mod sema;
mod sidecar;
pub mod source_map;
mod stdlib;
pub(crate) mod typescript;
mod val;
mod verify;

pub use analysis::{
    AnalyzedArm, BodyBinding, Coverage, CoveredEnum, MatchAnalysis, MatchConstructor, MatchSubject,
    NameKind, Origin, PatternAnalyses, PatternBinding, PatternSite, PayloadField, SiteKind,
    UnresolvedName, pattern_analyses,
};
pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticOwner, Severity};
pub use error::CompileError;
pub use probe::{
    Literal, LiteralMatch, PayloadProbe, TagMatch, literal_matches, literal_matches_with_kind,
    payload_probes, payload_probes_with_kind, tag_matches, tag_matches_with_kind,
};
pub use sidecar::{Sidecar, build_sidecar};
pub use stdlib::{
    STD_OPTION_SOURCE, STD_RESULT_SOURCE, STD_SPECIFIER, STD_TYPES_SOURCE, StdImports, StdModule,
};
pub use val::{Mutation, ValBinding, ValFn, ValParam, ValPass, ValProbes, is_builtin_mutator_name};

use error::TtError;

/// How relative `.tt`/`.ttx` import specifiers are rewritten in emitted
/// TypeScript/TSX. Applies to static `import` declarations and
/// `export ... from` re-exports whose specifier is a relative path ending
/// in `.tt` or `.ttx`; every other specifier — and dynamic `import(...)` — passes
/// through untouched. Corresponds to the CLI's `--rewrite-imports` flag.
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

/// An enum declaration from another module, made available to [`compile`]'s
/// exhaustiveness checking via [`Options::extern_enums`].
///
/// Collected by build tools (the `ttc` CLI does this for direct relative
/// `.tt`/`.ttx` imports) with [`exported_enums`] over the imported file's source,
/// filtered through the importing file's clause ([`tt_imports`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternEnum {
    /// The enum's name in the *importing* file's scope (import aliases
    /// applied; `ns.Name` for a namespace import). A local declaration of
    /// the same name shadows it; it shadows a built-in of the same name.
    pub name: String,
    /// The enum's case tags.
    pub tags: Vec<String>,
    /// Where the declaration came from, quoted in error messages —
    /// typically the import specifier as written (e.g. `./token.tt`).
    /// [`exported_enums`] leaves it `None`; the collector fills it in.
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

/// Extracts the exported tt enum declarations (name + case tags) of a
/// source file, without compiling it — the declaration-table half of
/// project-wide exhaustiveness checking. Non-exported enums and plain
/// TypeScript enums are not included. The returned entries have
/// [`ExternEnum::from`] set to `None`.
///
/// ```
/// let decls = ttc::exported_enums(
///     "export enum Token { Num(value: number), Eof }\nenum Private { A() }\n",
/// );
/// assert_eq!(decls.len(), 1);
/// assert_eq!(decls[0].name, "Token");
/// assert_eq!(decls[0].tags, ["Num", "Eof"]);
/// ```
pub fn exported_enums(source: &str) -> Vec<ExternEnum> {
    exported_enums_with_kind(source, SourceKind::TypeScript)
}

/// [`exported_enums`] under an explicit TypeScript surface kind.
pub fn exported_enums_with_kind(source: &str, source_kind: SourceKind) -> Vec<ExternEnum> {
    let program = parser::parse_with_kind(source, source_kind);
    program
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ast::Segment::Enum(decl) if decl.exported => Some(ExternEnum {
                name: decl.name.clone(),
                tags: decl.cases.iter().map(|c| c.tag.clone()).collect(),
                from: None,
            }),
            _ => None,
        })
        .collect()
}

/// Lists a source file's static relative `.tt`/`.ttx` imports and re-exports, in
/// source order — the edges a build tool follows to collect declarations
/// with [`exported_enums`].
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
    let mut scan = ModuleScan::default();
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

/// A tt enum declaration with source positions — the symbol-interface
/// counterpart of [`ExternEnum`], produced by [`enum_symbols`] and emitted
/// as JSON by `ttc --symbols` for language tooling (go-to-definition,
/// completion, hover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumSymbol {
    /// The enum's declared name.
    pub name: String,
    /// Byte offset of the name in the source (see [`line_col`]).
    pub offset: usize,
    /// Whether the declaration has an `export` modifier.
    pub exported: bool,
    /// The verbatim `<...>` generic parameter list, or `""`.
    pub generics: String,
    /// The enum's cases, in declaration order.
    pub cases: Vec<CaseSymbol>,
}

/// One case of an [`EnumSymbol`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseSymbol {
    /// The case tag.
    pub tag: String,
    /// Byte offset of the tag in the source.
    pub offset: usize,
    /// `None` for a unit case without parens; `Some` (possibly empty) for
    /// a case with a field list.
    pub fields: Option<Vec<FieldSymbol>>,
}

/// One field of a payload-carrying case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSymbol {
    /// The field name.
    pub name: String,
    /// Byte offset of the name in the source (see [`line_col`]).
    pub offset: usize,
    /// Whether the field is optional (`name?: T`).
    pub optional: bool,
    /// The verbatim type annotation text.
    pub ty: String,
}

/// Extracts every tt enum declaration of a source file with positions —
/// exported or not, flagged by [`EnumSymbol::exported`]. Plain TypeScript
/// enums are not tt enums and are not included.
///
/// ```
/// let syms = ttc::enum_symbols("export enum Token { Num(value: number), Eof }\n");
/// assert_eq!(syms[0].name, "Token");
/// assert_eq!(ttc::line_col(
///     "export enum Token { Num(value: number), Eof }\n", syms[0].offset), (1, 13));
/// assert_eq!(syms[0].cases[1].tag, "Eof");
/// assert_eq!(syms[0].cases[1].fields, None);
/// ```
pub fn enum_symbols(source: &str) -> Vec<EnumSymbol> {
    enum_symbols_with_kind(source, SourceKind::TypeScript)
}

/// [`enum_symbols`] under an explicit TypeScript surface kind.
pub fn enum_symbols_with_kind(source: &str, source_kind: SourceKind) -> Vec<EnumSymbol> {
    let program = parser::parse_with_kind(source, source_kind);
    program
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ast::Segment::Enum(decl) => Some(EnumSymbol {
                name: decl.name.clone(),
                offset: decl.name_off,
                exported: decl.exported,
                generics: decl.generics.clone(),
                cases: decl
                    .cases
                    .iter()
                    .map(|c| CaseSymbol {
                        tag: c.tag.clone(),
                        offset: c.tag_off,
                        fields: c.fields.as_ref().map(|fields| {
                            fields
                                .iter()
                                .map(|f| FieldSymbol {
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

/// One chunk of emitted output copied verbatim from the source: `len` bytes
/// starting at byte `src` of the source appear at byte `out` of the output.
/// Produced by [`emit_mapped`]; chunks are non-overlapping in both
/// coordinate spaces. Compiler-written glue (region scaffolding,
/// destructurings, enum emissions) has no mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitMapping {
    /// Byte offset of the chunk in the source.
    pub src: usize,
    /// Byte offset of the chunk in the emitted output.
    pub out: usize,
    /// Length of the chunk in bytes (identical in both spaces).
    pub len: usize,
}

/// Where a `match` put its scrutinee.
///
/// Every `match` evaluates its scrutinee once, into a temporary the emitted
/// switch discriminates on. That temporary is the only place a type checker
/// can be *asked* about the scrutinee's type: asking at the scrutinee's own
/// text answers about the text — for `match (getShape())` that is the type
/// of `getShape`, a function, not the `Shape` the match is over. So the
/// emitter records where it wrote the name, and typed exhaustiveness
/// ([`tag_matches`], [`literal_matches`]) asks there.
///
/// ```
/// let source = "const v = match (f()) { Circle(r) => r };\n";
/// let emit = ttc::emit_mapped(source);
/// let temp = emit.scrutinee_temps[0];
/// assert_eq!(&source[temp.src..temp.src + 5], "match");
/// assert!(emit.code[temp.out..].starts_with("$tt_m = f()"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrutineeTemp {
    /// Byte offset of the `match` keyword in the source — the same offset
    /// [`probe::TagMatch::offset`] and [`probe::LiteralMatch::offset`] carry,
    /// so a probe and its temporary are joined by it.
    pub src: usize,
    /// Byte offset of the temporary's name in the emitted output.
    pub out: usize,
}

/// Which tt construct a stretch of compiler-written glue belongs to.
///
/// The kind is half of what turns a TypeScript diagnostic on that glue into
/// a tt one — the other half is the error code (see
/// `docs/design/rust-parity-analysis.md` §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorKind {
    /// A `match` expression's switch or if-chain.
    Match,
    /// A `try` statement's test, early return and binding.
    Try,
    /// A let-else statement's test and destructuring.
    LetElse,
    /// An `if let` statement's test and destructuring.
    IfLet,
    /// One `<-` binding of a `result` block.
    ResultBind,
    /// A pipeline's apply helper (`$tt_ap`) or composition helper
    /// (`$tt_fl`).
    Pipe,
    /// A tt `enum`'s union type and constructor object.
    Enum,
}

/// A stretch of emitted output that ttc wrote itself, and the construct it
/// wrote it for.
///
/// [`EmitMapping`] answers "which source bytes are these output bytes?" and
/// exists only where the answer is *these exact bytes*. Glue has no such
/// answer — it is text no one wrote — but it always has an **origin**, and
/// that is what an anchor records. It is deliberately one-way and for
/// diagnostics only: navigation and rename must never resolve into glue
/// (an edit there would corrupt the program), while a diagnostic there is
/// worth reporting at the construct that produced it.
///
/// Anchors nest, and are ordered so that an inner one comes before the
/// outer one that contains it — a consumer takes the first match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitAnchor {
    /// Byte offset in the emitted output where the construct's glue starts.
    pub out: usize,
    /// Byte offset just past its end.
    pub end: usize,
    /// Byte offset in the source of the construct's keyword — where a
    /// diagnostic about this glue belongs.
    pub src: usize,
    /// Byte offset just past the construct's own source text — the end of
    /// what a diagnostic about this glue should underline. The range it
    /// closes (`src..src_end`) is the construct as the user wrote it, not
    /// the whole statement: for `try` it is `try <expr>`, for a `match` the
    /// keyword and its scrutinee.
    pub src_end: usize,
    /// Byte offset just past the complete source construct that owns this
    /// lowering. This can be wider than the primary display span: a match
    /// diagnostic underlines only `match (subject)`, while an error in any
    /// arm still owns consequences produced by that match's glue.
    pub owner_end: usize,
    /// What kind of construct wrote it.
    pub kind: AnchorKind,
}

/// Where a nested pattern's **receiver** landed in the emitted output.
///
/// `Ok(value: Some(v))` lowers to a condition chain whose second link
/// reads `$tt_m.value.kind === "Some"`. That `$tt_m.value` is the only
/// place a type checker can be asked what the *payload* admits — ttc knows
/// the field's declared type text, but a text is not a type, and a type
/// parameter or a hand-written union names no declaration ttc holds. The
/// emitter records where it wrote the receiver, and the typed
/// exhaustiveness pass asks there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadTemp {
    /// Byte offset of the nested pattern's tag in the source — the
    /// occurrence this receiver was written for.
    pub src: usize,
    /// Byte offset of the receiver expression in the emitted output.
    pub out: usize,
}

/// The result of [`emit_mapped`]: the emitted TypeScript and the
/// source↔output mappings of every verbatim-copied chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedEmit {
    /// The emitted TypeScript.
    pub code: String,
    /// Source↔output mappings, ordered by output offset.
    pub mappings: Vec<EmitMapping>,
    /// Where each `match` bound its scrutinee, ordered by output offset.
    pub scrutinee_temps: Vec<ScrutineeTemp>,
    /// Where each nested pattern's receiver was written, ordered by output
    /// offset.
    pub payload_temps: Vec<PayloadTemp>,
    /// The glue each construct wrote, innermost first — the origin of a
    /// diagnostic that lands where no mapping reaches.
    pub anchors: Vec<EmitAnchor>,
}

impl MappedEmit {
    /// The construct that wrote the glue at output byte `out`, innermost
    /// first. `None` when the byte is not in any construct's glue.
    pub fn anchor_at(&self, out: usize) -> Option<&EmitAnchor> {
        self.anchors.iter().find(|a| a.out <= out && out < a.end)
    }

    /// The Source Map v3 for this emission, over the `source` it was
    /// compiled from.
    ///
    /// The map is built from [`MappedEmit::mappings`] and
    /// [`MappedEmit::anchors`] — the emission's own record of which output
    /// bytes are copied source and which construct wrote each stretch of
    /// glue. `code` is not searched for anything but line breaks.
    ///
    /// ```
    /// use ttc::{compile_mapped, source_map::SourceMapRequest, Options};
    ///
    /// let emit = compile_mapped("const n = 1;\n", &Options::default()).unwrap();
    /// let map = emit.source_map(
    ///     "const n = 1;\n",
    ///     &SourceMapRequest {
    ///         source: "a.tt",
    ///         ..SourceMapRequest::default()
    ///     },
    /// );
    /// assert!(map.to_json().contains("\"sources\":[\"a.tt\"]"));
    /// ```
    #[must_use]
    pub fn source_map(
        &self,
        source: &str,
        request: &source_map::SourceMapRequest<'_>,
    ) -> source_map::SourceMap {
        source_map::build(source, &self.code, &self.mappings, &self.anchors, request)
    }
}

/// Emits `source` for language tooling: structural parse + code emission
/// only, with source↔output byte mappings for every chunk copied verbatim
/// (passthrough segments, match scrutinees and arm bodies, `try`/let-else/
/// `if let` expressions, pipeline steps, template chunks).
///
/// Unlike [`compile`] this is **infallible**: semantic checks and output
/// verification are skipped, so a buffer mid-edit (with, say, a
/// non-exhaustive match) still emits — diagnostics remain [`compile`]/`ttc
/// --check`'s job. Relative `.tt`/`.ttx` import specifiers and `@tt/std` entries
/// are left untouched ([`ImportRewrite::Off`] semantics): the consumer — an
/// editor serving the output as a virtual TypeScript document — resolves
/// them itself. Corresponds to the CLI's `--emit-map`.
///
/// ```
/// let m = ttc::emit_mapped("const n: number = 1;\n");
/// assert_eq!(m.code, "const n: number = 1;\n");
/// assert_eq!(m.mappings, [ttc::EmitMapping { src: 0, out: 0, len: 21 }]);
/// ```
pub fn emit_mapped(source: &str) -> MappedEmit {
    emit_mapped_with_kind(source, SourceKind::TypeScript)
}

/// [`emit_mapped`] under an explicit TypeScript surface kind.
pub fn emit_mapped_with_kind(source: &str, source_kind: SourceKind) -> MappedEmit {
    let program = parser::parse_with_kind(source, source_kind);
    let semantics = analysis::coverage_semantics(&program, &[]);
    let core = core_ir::lower_semantic(&semantics, source);
    // A buffer mid-edit is routinely not TypeScript yet, and this entry
    // point is infallible by contract: with no owner model there are no
    // host rewrites to plan, so the emit degrades to the same shape a file
    // needing no host lowering gets. Reporting stays [`compile`]'s job.
    let plan = codegen::lowering_plan(&semantics, &core, source, source_kind).unwrap_or_default();
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        source,
        &plan,
        ImportRewrite::Off,
        StdImports::default(),
    );
    MappedEmit {
        code: flat.code,
        mappings: flat.mappings,
        scrutinee_temps: flat.scrutinee_temps,
        payload_temps: flat.payload_temps,
        anchors: flat.anchors,
    }
}

/// Collects a file's `val` bindings and its mutations, **unpaired** — the
/// delegated form of `val`'s analysis.
///
/// [`compile`] pairs the two itself, with a lexical scope model of its own.
/// A caller that has a TypeScript checker does better: the binding a
/// mutation belongs to is the one whose declaration shares its *symbol*, and
/// symbol identity is not an approximation of scope — it is scope, as
/// TypeScript resolved it. `ttc --check-types` pairs them that way; run
/// `ttc help val` for the user-facing behavior.
///
/// ```
/// let probes = ttc::val_probes("val const xs = [];\nxs.push(1);\nys.push(2);\n");
/// assert_eq!(probes.bindings.len(), 1);
/// assert_eq!(probes.bindings[0].name, "xs");
/// // Both calls are collected: which one is rooted at the `val` binding is
/// // not decided here.
/// assert_eq!(probes.mutations.len(), 2);
/// assert_eq!(probes.mutations[1].name, "ys");
/// ```
///
/// Method calls are collected whatever the method is called — whether one
/// mutates is the verdict's half (the checker's built-in answer plus
/// [`is_builtin_mutator_name`]), not collection's:
///
/// ```
/// let probes = ttc::val_probes("val const d = mk();\nd.at(0);\n");
/// assert_eq!(probes.mutations.len(), 1);
/// assert_eq!(probes.mutations[0].method.as_ref().unwrap().0, "at");
/// ```
pub fn val_probes(source: &str) -> ValProbes {
    val_probes_with_kind(source, SourceKind::TypeScript)
}

/// [`val_probes`] under an explicit TypeScript surface kind.
pub fn val_probes_with_kind(source: &str, source_kind: SourceKind) -> ValProbes {
    let tokens = lexer::lex_with_kind(source, 0, source.len(), source_kind);
    val::probes(source, &tokens)
}

/// Converts a byte offset into `source` to a 1-based `(line, column)` —
/// the same mapping [`CompileError`] positions use (column counted in
/// UTF-8 code points). Offsets past the end clamp to the last position.
pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    error::line_col(source, offset)
}

/// Compilation options for [`compile`].
///
/// The default is no filename, TypeScript source, verification enabled, `.tt` import
/// specifiers rewritten to `.js`, and no imported declarations:
///
/// ```
/// let opts = ttc::Options::default();
/// assert_eq!(opts.filename, None);
/// assert!(opts.verify);
/// assert_eq!(opts.rewrite_imports, ttc::ImportRewrite::Js);
/// assert!(opts.extern_enums.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// Filename reported in [`CompileError`]s (and their `Display` output).
    /// `None` renders as `<input>`.
    pub filename: Option<&'a str>,
    /// Whether the source surface is TypeScript or TSX.
    pub source_kind: SourceKind,
    /// Validate enum field types and the generated output with swc.
    /// Corresponds to the CLI's `--no-verify` escape hatch when `false`;
    /// disabling it lets syntactically bad field types flow into the output
    /// (where tsc will report them) and skips the emitted-code self-check.
    pub verify: bool,
    /// How relative `.tt`/`.ttx` import specifiers are rewritten in the output.
    pub rewrite_imports: ImportRewrite,
    /// Enum declarations imported from other modules, included in
    /// exhaustiveness checking (shadowed by local declarations; shadowing
    /// built-ins of the same name). The `ttc` CLI fills this from the
    /// file's direct relative `.tt`/`.ttx` imports.
    pub extern_enums: &'a [ExternEnum],
    /// Leave the two judgments a TypeScript checker makes better to a
    /// TypeScript checker: match exhaustiveness, and which binding a
    /// mutation path is rooted at (`val`).
    ///
    /// ttc answers both on its own, from its enum declarations and a lexical
    /// scope model of its own, and those answers are what [`compile`]
    /// reports by default. Both are approximations of TypeScript's:
    /// exhaustiveness is the *declared* type's answer, so a case an earlier
    /// guard already removed is still demanded and an enum from another
    /// module has to be collected ([`Options::extern_enums`]); `val`'s
    /// pairing is a scope model, so shadowing and redeclaration are ttc's
    /// reading rather than TypeScript's. A caller with a checker asks it
    /// instead — the narrowed type at each `match`, and symbol identity for
    /// each binding — and reports what it says. `ttc --check-types` does
    /// exactly that ([`tag_matches`], [`literal_matches`], [`val_probes`]).
    ///
    /// Every other tt-level check runs either way: duplicate cases,
    /// misplaced wildcards, bad field types, `val`'s call-capability rule.
    pub defer_to_checker: bool,
    /// Per-module rewrites for the standard-library package. Missing entries
    /// leave their bare specifiers untouched for a bundler plugin to resolve.
    pub std_imports: StdImports<'a>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            filename: None,
            source_kind: SourceKind::TypeScript,
            verify: true,
            rewrite_imports: ImportRewrite::default(),
            extern_enums: &[],
            defer_to_checker: false,
            std_imports: StdImports::default(),
        }
    }
}

/// Compile tt source text to TypeScript or TSX source text.
///
/// Only tt constructs (`enum` declarations, `match` expressions, `try` and
/// let-else statements) and relative `.tt`/`.ttx` import specifiers (per
/// [`Options::rewrite_imports`]) are rewritten; everything else — including
/// all plain TypeScript `enum` forms — passes through byte for byte. A
/// candidate construct that does not fully parse as tt syntax is passed
/// through untouched rather than reported as an error.
/// The output has no generated banner comment (that is added by the CLI).
///
/// # Errors
///
/// Returns a [`CompileError`] with a 1-based position in `source` for every
/// tt-level rule violation: duplicate enum cases, invalid field types,
/// duplicate or misplaced `match` arms, and non-exhaustive matches over enums
/// declared in this source. With [`Options::verify`] enabled, a final
/// self-check that the generated output parses as TypeScript can also fail
/// (reported without a position). Run `ttc help errors` for guidance.
///
/// ```
/// use ttc::{compile, Options};
///
/// let source = "enum E { A(x: number), B }\nconst v = match (E.A(1)) { A(x) => x };";
/// let options = Options { filename: Some("demo.tt"), ..Options::default() };
/// let err = compile(source, &options).unwrap_err();
/// assert_eq!((err.line, err.col), (2, 11));
/// assert!(err.message.contains(r#"not exhaustive: missing "B""#));
/// assert!(err.to_string().starts_with("demo.tt:2:11: "));
/// ```
pub fn compile(source: &str, options: &Options) -> Result<String, CompileError> {
    compile_mapped(source, options).map(|emit| emit.code)
}

/// [`compile`], also returning the source↔output byte mappings of every
/// chunk copied verbatim from the source — the same mappings
/// [`emit_mapped`] produces, but from a fully checked compilation.
///
/// Callers that report a tsc diagnostic over the emitted TypeScript use
/// these to name the position in the `.tt` source instead of one in a file
/// that was never written (`ttc --types`).
///
/// ```
/// use ttc::{compile_mapped, Options};
///
/// let emit = compile_mapped("const n = 1;\n", &Options::default()).unwrap();
/// assert_eq!(emit.code, "const n = 1;\n");
/// assert_eq!(emit.mappings, [ttc::EmitMapping { src: 0, out: 0, len: 13 }]);
/// ```
///
/// # Errors
///
/// Identical to [`compile`].
pub fn compile_mapped(source: &str, options: &Options) -> Result<MappedEmit, CompileError> {
    // The swc-style pipeline: structural parse (infallible; anything that is
    // not fully tt syntax stays a verbatim byte range) → semantic checks
    // (every tt-level error, including exhaustiveness — never delegated to
    // tsc; `val`'s binding analysis reads the token stream the parse
    // already produced) → code emission (infallible).
    //
    // The checks accumulate everything ([`analyze`] is the API that returns
    // it all); this entry point keeps its historical contract — code, or
    // the first error in source order — and skips emission when the checks
    // already failed.
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    let semantics = analysis::coverage_semantics(&program, options.extern_enums);
    let core = core_ir::lower_semantic(&semantics, source);
    if let Some(first) = tt_errors(source, &program, &tokens, options, &semantics)
        .into_iter()
        .next()
    {
        return Err(
            diagnostics::Diagnostic::from_tt(first).to_compile_error(source, options.filename)
        );
    }
    let plan = match codegen::lowering_plan(&semantics, &core, source, options.source_kind) {
        Ok(plan) => plan,
        // The file's own TypeScript does not parse, so no owner model
        // exists to lower against. Reported where the source says it, not
        // as a panic out of emission.
        Err(failure) => {
            return Err(
                diagnostics::Diagnostic::from_tt(verify::in_source(source, &failure))
                    .to_compile_error(source, options.filename),
            );
        }
    };
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        source,
        &plan,
        options.rewrite_imports,
        options.std_imports,
    );
    if options.verify
        && let Err(failure) = verify::verify_output(&flat.code, options.source_kind)
    {
        // The self-check reads the *generated* module, but the user only
        // has the `.tt` file open. A position in a file no one wrote is
        // not a position, so it is carried back through the mappings to
        // the source — and where the failure fell in a construct's glue,
        // that construct is named. (Without this the error arrives with no
        // position at all and an editor pins it to line 1.)
        let failure =
            verify::at_source(source, &flat.mappings, &flat.anchors, &flat.code, &failure);
        return Err(
            diagnostics::Diagnostic::from_tt(failure).to_compile_error(source, options.filename)
        );
    }
    Ok(MappedEmit {
        code: flat.code,
        mappings: flat.mappings,
        scrutinee_temps: flat.scrutinee_temps,
        payload_temps: flat.payload_temps,
        anchors: flat.anchors,
    })
}

/// Every tt-level violation of `source`, in source order — the semantic
/// passes over an already-built parse. What [`analyze`] and
/// [`compile_report`] share.
fn tt_errors(
    source: &str,
    program: &ast::Program,
    tokens: &[lexer::Token],
    options: &Options,
    semantics: &analysis::SemanticFile,
) -> Vec<TtError> {
    let mut errors = sema::check_all(
        program,
        options.verify,
        options.defer_to_checker,
        &semantics.patterns,
    );
    if !options.defer_to_checker {
        errors.extend(val::check_all(source, tokens));
    }
    // One order for every producer: where the reader's eye goes, top to
    // bottom. Stable, so equal positions keep their category order.
    errors.sort_by_key(|e| e.offset.unwrap_or(usize::MAX));
    errors
}

/// Checks `source` and returns **every** tt-level diagnostic, in source
/// order — nothing is emitted and nothing stops at the first violation.
///
/// This is the multi-diagnostic form of [`compile`]'s error half: the CLI's
/// `--check`, the `--server`, and the engine all report from it, so one
/// broken match no longer hides the file's other problems (TASK-117).
/// Positions are byte offsets ([`Diagnostic::to_compile_error`] converts to
/// the CLI's line/column form). The output self-check needs an emission and
/// is [`compile_report`]'s half.
///
/// ```
/// let source = "enum E { A(x: number), B }\n\
///     const a = match (E.A(1)) { A(x) => x };\n\
///     const b = match (E.B) { B => 0 };\n";
/// let diagnostics = ttc::analyze(source, &ttc::Options::default());
/// assert_eq!(diagnostics.len(), 2);
/// assert!(diagnostics.iter().all(|d| d.code == ttc::DiagnosticCode::MatchNotExhaustive));
/// ```
pub fn analyze(source: &str, options: &Options) -> Vec<Diagnostic> {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    let semantics = analysis::coverage_semantics(&program, options.extern_enums);
    tt_errors(source, &program, &tokens, options, &semantics)
        .into_iter()
        .map(diagnostics::Diagnostic::from_tt)
        .collect()
}

/// A full compilation's answer: everything found, and the emission when one
/// was possible.
///
/// Unlike [`compile`], recoverable tt errors do not withhold the emission:
/// codegen is infallible, so a file with a duplicate arm still lowers to
/// plain TypeScript — which is what lets a typed pass run and report its
/// diagnostics *alongside* the tt ones instead of losing them
/// ([`DiagnosticCode::blocks_projection`], TASK-117 symptom 3). `emit` is
/// `None` only when a diagnostic blocks projection: text the parser could
/// not claim, a bad field type, or a failed output self-check.
#[derive(Debug, Clone)]
pub struct CompileReport {
    /// The emitted TypeScript with its mappings, unless a diagnostic made
    /// emission impossible.
    pub emit: Option<MappedEmit>,
    /// Every tt-level diagnostic, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// The project engine's recovering form of [`CompileReport`]. Recovery is
/// intentionally not part of normal compilation: only the typed projection
/// may substitute parser-owned error nodes so later independent code remains
/// checkable.
pub(crate) struct ProjectionReport {
    pub emit: Option<MappedEmit>,
    pub diagnostics: Vec<Diagnostic>,
    pub recovered: Vec<(usize, usize)>,
}

fn overwrite_recovery(source: &mut [u8], start: usize, end: usize, replacement: &str) {
    let start = start.min(source.len());
    let end = end.min(source.len()).max(start);
    source[start..end].fill(b' ');
    let bytes = replacement.as_bytes();
    let count = bytes.len().min(end - start);
    source[start..start + count].copy_from_slice(&bytes[..count]);
}

/// Builds a valid TypeScript projection in the presence of parser-owned
/// error nodes. Replacements are byte-length preserving, so every mapping
/// outside the recovered node remains in the original source coordinate
/// space.
pub(crate) fn compile_projection_report(source: &str, options: &Options) -> ProjectionReport {
    let ordinary = compile_report(source, options);
    if ordinary.emit.is_some() {
        return ProjectionReport {
            emit: ordinary.emit,
            diagnostics: ordinary.diagnostics,
            recovered: Vec::new(),
        };
    }

    let program = parser::parse_with_kind(source, options.source_kind);
    let mut nodes = parser::projection_recoveries(&program);
    for diagnostic in &ordinary.diagnostics {
        let (Some(start), Some(end)) = (diagnostic.start, diagnostic.end) else {
            continue;
        };
        match diagnostic.code {
            DiagnosticCode::TryPlacement => nodes.push(ast::RecoveryNode {
                span: ast::Span { start, end },
                kind: ast::RecoveryKind::Expression,
            }),
            DiagnosticCode::EnumInvalidFieldType => nodes.push(ast::RecoveryNode {
                span: ast::Span { start, end },
                kind: ast::RecoveryKind::Type,
            }),
            _ => {}
        }
    }
    nodes.sort_by_key(|node| (node.span.start, std::cmp::Reverse(node.span.end)));

    // Keep the outer error node when parser recovery found nested symptoms
    // inside it. This is the same synchronization rule as an error AST node:
    // one placeholder owns one malformed construct.
    let mut selected: Vec<ast::RecoveryNode> = Vec::new();
    for node in nodes {
        if selected
            .last()
            .is_some_and(|outer| node.span.end <= outer.span.end)
        {
            continue;
        }
        selected.push(node);
    }
    if selected.is_empty() {
        return ProjectionReport {
            emit: None,
            diagnostics: ordinary.diagnostics,
            recovered: Vec::new(),
        };
    }

    let mut recovered = source.as_bytes().to_vec();
    for node in &selected {
        let replacement = match &node.kind {
            ast::RecoveryKind::Expression => {
                let replacement =
                    if node.span.end.saturating_sub(node.span.start) >= "undefined as any".len() {
                        "undefined as any"
                    } else {
                        "0"
                    };
                overwrite_recovery(&mut recovered, node.span.start, node.span.end, replacement);
                continue;
            }
            ast::RecoveryKind::Statement => ";",
            ast::RecoveryKind::Type => "any",
            ast::RecoveryKind::EnumDecl { name, exported } => {
                let declaration = if *exported {
                    format!("export class {name} {{}}")
                } else {
                    format!("class {name} {{}}")
                };
                let replacement =
                    if declaration.len() <= node.span.end.saturating_sub(node.span.start) {
                        declaration.as_str()
                    } else {
                        ";"
                    };
                overwrite_recovery(&mut recovered, node.span.start, node.span.end, replacement);
                continue;
            }
        };
        overwrite_recovery(&mut recovered, node.span.start, node.span.end, replacement);
    }
    let recovered_source = String::from_utf8(recovered).expect("recovery preserves UTF-8 source");
    let recovered_report = compile_report(&recovered_source, options);
    ProjectionReport {
        emit: recovered_report.emit,
        diagnostics: ordinary.diagnostics,
        recovered: selected
            .into_iter()
            .map(|node| (node.span.start, node.span.end))
            .collect(),
    }
}

/// Compiles `source` and reports everything — the multi-diagnostic,
/// still-emitting form of [`compile_mapped`]. See [`CompileReport`].
pub fn compile_report(source: &str, options: &Options) -> CompileReport {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    let semantics = analysis::coverage_semantics(&program, options.extern_enums);
    let core = core_ir::lower_semantic(&semantics, source);
    let mut errors = tt_errors(source, &program, &tokens, options, &semantics);
    if errors.iter().any(|e| e.code.blocks_projection()) {
        return CompileReport {
            emit: None,
            diagnostics: errors
                .into_iter()
                .map(diagnostics::Diagnostic::from_tt)
                .collect(),
        };
    }
    let plan = match codegen::lowering_plan(&semantics, &core, source, options.source_kind) {
        Ok(plan) => plan,
        // Same class as a projection-blocking tt diagnostic: the file has
        // no emittable form, and the cause is reported with everything
        // else already found.
        Err(failure) => {
            errors.push(verify::in_source(source, &failure));
            return CompileReport {
                emit: None,
                diagnostics: errors
                    .into_iter()
                    .map(diagnostics::Diagnostic::from_tt)
                    .collect(),
            };
        }
    };
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        source,
        &plan,
        options.rewrite_imports,
        options.std_imports,
    );
    let mut emit = Some(MappedEmit {
        code: flat.code,
        mappings: flat.mappings,
        scrutinee_temps: flat.scrutinee_temps,
        payload_temps: flat.payload_temps,
        anchors: flat.anchors,
    });
    if options.verify
        && let Some(flat) = &emit
        && let Err(failure) = verify::verify_output(&flat.code, options.source_kind)
    {
        // A failed self-check *with tt errors already reported* is the
        // effect, not a second cause — the emitted text reflects the
        // invalid construct those errors name (e.g. a module-level `try`'s
        // `return`), and the backstop's "or a ttc bug" wording would
        // mislead. Report the causes and withhold the emit; the check
        // reappears on its own once they are fixed.
        if errors.is_empty() {
            errors.push(verify::at_source(
                source,
                &flat.mappings,
                &flat.anchors,
                &flat.code,
                &failure,
            ));
        }
        emit = None;
    }
    CompileReport {
        emit,
        diagnostics: errors
            .into_iter()
            .map(diagnostics::Diagnostic::from_tt)
            .collect(),
    }
}
