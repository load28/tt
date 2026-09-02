//! Mapping-aware emission metadata and lightweight source probes.

use super::*;

/// One chunk of emitted output copied verbatim from the source: `len` bytes
/// starting at byte `src` of the source appear at byte `out` of the output.
/// Produced by [`emit_mapped`]; chunks are non-overlapping in both
/// coordinate spaces. Compiler-written glue has no mapping.
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

/// A value explicitly returned from a `result` block, in both source and
/// emitted TypeScript coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultReturnTemp {
    /// Byte range of the returned value in the source.
    pub src: usize,
    /// End byte offset of the returned value in the source.
    pub src_end: usize,
    /// Byte offset of that value in the emitted TypeScript.
    pub out: usize,
    /// End byte offset of that value in the emitted TypeScript.
    pub out_end: usize,
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
    /// A `result` block's generated completion region.
    Result,
    /// A pipeline's apply helper (`$tt_ap`) or composition helper
    /// (`$tt_fl`).
    Pipe,
    /// A tt `variant`'s union type and constructor object.
    Variant,
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
    /// A second source range that explains a diagnostic on this glue —
    /// where the emitter alone knows the relationship. A pipeline's
    /// per-step anchor names the step that produced the rejected value
    /// here, so a reporter can label it ("the piped value comes from this
    /// step"). `None` when the construct has no such companion place.
    pub context: Option<(usize, usize)>,
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
    /// Explicit Result return values in source and emitted coordinates.
    pub(crate) result_return_temps: Vec<ResultReturnTemp>,
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
    let semantics = analysis::coverage_semantics(source, &program, &[]);
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
        source_kind,
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
        result_return_temps: flat.result_return_temps,
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
