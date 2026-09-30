//! Mapping-aware emission metadata and lightweight source probes.

use super::*;

/// One chunk of emitted output copied verbatim from the source: `len` bytes
/// starting at byte `src` of the source appear at byte `out` of the output.
/// Produced by [`emit_mapped`]; chunks never overlap in the output. Text
/// the compiler passes through is copied once, so its chunks never overlap
/// in the source either; a tt construct's own text that its lowering writes
/// more than once (a variant field's type, in the union and in the
/// constructor) is copied, and mapped, at each place. Compiler-written glue
/// has no mapping.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeclaredName {
    pub src: usize,
    pub src_end: usize,
    pub out: usize,
    pub out_end: usize,
}

/// Glue the emitter wrote at one point of the source rather than for a
/// construct: the prelude of helpers and imports. Relative to the source
/// text around it, everything in `out..out_end` stands at `src`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InsertedGlue {
    pub src: usize,
    pub out: usize,
    pub out_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DestructuredList {
    pub src: usize,
    pub src_end: usize,
    pub out: usize,
    pub out_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedBinding {
    pub out: usize,
    pub out_end: usize,
    pub occurrences: Vec<BindingOccurrence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BindingOccurrence {
    pub src: usize,
    pub src_end: usize,
    pub shorthand: bool,
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
    /// step"). A `match`'s case label names the pattern it was written for,
    /// and a diagnostic on the label is shown there. `None` when the
    /// construct has no such companion place.
    pub context: Option<(usize, usize)>,
    /// What kind of construct wrote it.
    pub kind: AnchorKind,
}

impl EmitAnchor {
    pub(crate) fn display(&self) -> (usize, usize) {
        match (self.kind, self.context) {
            (AnchorKind::Match, Some(pattern)) => pattern,
            _ => (self.src, self.src_end),
        }
    }
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
    /// Byte offsets after generated value declaration identifiers.
    pub(crate) contextual_slots: Vec<usize>,
    /// Those of [`MappedEmit::contextual_slots`] whose storage holds the
    /// index of the arm a dispatch selected, not a value of the source.
    pub(crate) selector_slots: Vec<usize>,
    pub(crate) operand_slots: Vec<usize>,
    pub(crate) asserted_slots: Vec<(usize, usize)>,
    pub(crate) generated_names: std::collections::HashSet<String>,
    pub(crate) declared_names: Vec<DeclaredName>,
    pub(crate) shared_bindings: Vec<SharedBinding>,
    pub(crate) destructured_lists: Vec<DestructuredList>,
    /// Glue written at a source point, ordered by output offset.
    pub(crate) inserted: Vec<InsertedGlue>,
    pub(crate) single_line_breaks: Vec<usize>,
    /// What TypeScript's completion rules say at each construct's place.
    pub(crate) completion_scopes: Vec<crate::program_syntax::CompletionScope>,
    /// The compiler support modules the emitted code imports, in
    /// [`StdModule::ALL`](crate::StdModule::ALL) order: the standard-library
    /// modules the source imports, and the pipeline runtime when the
    /// emission calls one of its helpers through an import. A build writes
    /// exactly these modules for the outputs it writes.
    pub support_imports: Vec<crate::StdModule>,
    /// Whether the module is written with CommonJS syntax, so the support
    /// modules it imports are the [`StdImports::commonjs`](crate::StdImports) ones.
    pub commonjs: bool,
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
    /// glue — and `source`'s own tokens, so every token a chunk copies maps
    /// to its own line and column. `code` is not searched for anything but
    /// line breaks.
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
    let (program, tokens) = parser::lex_and_parse_with_kind(source, source_kind);
    emit_mapped_parsed(
        source,
        &Options {
            source_kind,
            rewrite_imports: ImportRewrite::Off,
            ..Options::default()
        },
        &program,
        &tokens,
    )
}

/// [`emit_mapped`] over a parse the caller already has, under `options`'
/// surface kind, imported variants, and import handling.
pub(crate) fn emit_mapped_parsed(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[crate::lexer::Token],
) -> MappedEmit {
    let source_kind = options.source_kind;
    let typescript_tokens = crate::lexer::TypeScriptTokens::of(source, source_kind, tokens);
    let semantics = analysis::coverage_semantics(source, program, options.extern_variants);
    let core = core_ir::lower_semantic(&semantics, source, typescript_tokens.tokens());
    // A buffer mid-edit is routinely not TypeScript yet, and this entry
    // point is infallible by contract: with no owner model there are no
    // host rewrites to plan, so every tt value the plan cannot own emits as
    // a recovery placeholder anchored to its construct — the same values
    // the plan refuses by placement. Reporting stays [`compile`]'s job.
    let plan = codegen::lowering_plan(&semantics, &core, source, source_kind, tokens)
        .unwrap_or_else(|_| {
            crate::evaluation_ir::LoweringPlan::without_owner_model(source, source_kind)
        });
    let automatic_semicolons = crate::lexer::automatic_semicolons(tokens);
    let comments = crate::lexer::comments(source, tokens);
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        codegen::EmitSource {
            text: source,
            kind: source_kind,
            automatic_semicolons: &automatic_semicolons,
            comments: &comments,
        },
        &plan,
        options.rewrite_imports,
        options.std_imports,
    );
    MappedEmit {
        code: flat.code,
        mappings: flat.mappings,
        scrutinee_temps: flat.scrutinee_temps,
        payload_temps: flat.payload_temps,
        anchors: flat.anchors,
        result_return_temps: flat.result_return_temps,
        contextual_slots: flat.contextual_slots,
        selector_slots: flat.selector_slots,
        operand_slots: flat.operand_slots,
        asserted_slots: flat.asserted_slots,
        generated_names: flat.generated_names,
        declared_names: flat.declared_names,
        shared_bindings: flat.shared_bindings,
        destructured_lists: flat.destructured_lists,
        inserted: flat.inserted,
        single_line_breaks: flat.single_line_breaks,
        completion_scopes: plan.completion_scopes.clone(),
        support_imports: flat.support_imports,
        commonjs: flat.commonjs,
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
///
/// A call is read from the TypeScript syntax tree, so the method may be
/// named by a string-literal key and the callee may be parenthesized. A
/// computed key that is not a literal names no method and is not collected:
///
/// ```
/// let probes = ttc::val_probes("val const d = mk();\nd[\"push\"](1);\n(d.pop)();\nd[k](2);\n");
/// let methods: Vec<&str> = probes
///     .mutations
///     .iter()
///     .map(|m| m.method.as_ref().unwrap().0.as_str())
///     .collect();
/// assert_eq!(methods, ["push", "pop"]);
/// ```
pub fn val_probes(source: &str) -> ValProbes {
    val_probes_with_kind(source, SourceKind::TypeScript)
}

/// [`val_probes`] under an explicit TypeScript surface kind.
pub fn val_probes_with_kind(source: &str, source_kind: SourceKind) -> ValProbes {
    let probes = val_syntax_probes(source, source_kind);
    if probes.bindings.is_empty() {
        return probes;
    }
    with_method_calls(
        probes,
        &emit_mapped_with_kind(source, source_kind),
        source_kind,
    )
}

/// [`val_probes_with_kind`] over an emission of `source` the caller already
/// has.
pub(crate) fn val_probes_with_emit(
    source: &str,
    source_kind: SourceKind,
    program: &ast::Program,
    tokens: &[crate::lexer::Token],
    emit: &MappedEmit,
) -> ValProbes {
    with_method_calls(
        val::probes(source, source_kind, tokens, &parser::val_modifiers(program)),
        emit,
        source_kind,
    )
}

fn val_syntax_probes(source: &str, source_kind: SourceKind) -> ValProbes {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, source_kind);
    val::probes(
        source,
        source_kind,
        &tokens,
        &parser::val_modifiers(&program),
    )
}

fn with_method_calls(
    mut probes: ValProbes,
    emit: &MappedEmit,
    source_kind: SourceKind,
) -> ValProbes {
    if probes.bindings.is_empty() {
        return probes;
    }
    probes
        .mutations
        .extend(val::method_calls(emit, source_kind));
    probes.mutations.sort_by_key(|mutation| mutation.root);
    probes
}

/// Converts a byte offset into `source` to a 1-based `(line, column)` —
/// the same mapping [`CompileError`] positions use: ECMA-262's line
/// terminators (LF, CR, CR LF, U+2028, U+2029), as `tsc` counts lines, and
/// the column counted in code points. Offsets past the end clamp to the
/// last position. [`lines::LineMap`] measures a text once for many
/// conversions, and under the editor protocol's line breaks.
///
/// ```
/// let source = "export {};\rconst b = 1;\r";
/// assert_eq!(ttc::line_col(source, source.find('b').unwrap()), (2, 7));
/// ```
pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    lines::line_col(source, offset)
}

/// A byte offset into `source` as a UTF-16 code-unit offset — the offset an
/// editor protocol addresses a buffer with.
///
/// ```
/// let source = "🎉ab";
/// assert_eq!(ttc::utf16_offset(source, source.find('a').unwrap()), 2);
/// ```
pub fn utf16_offset(source: &str, offset: usize) -> usize {
    crate::typescript::mapper::to_utf16(source, offset)
}

/// [`utf16_offset`] for many offsets into one measured source.
pub struct Utf16Offsets<'a> {
    source: &'a str,
    signature: usize,
    multibyte: Vec<(usize, usize)>,
    total: usize,
}

impl<'a> Utf16Offsets<'a> {
    /// Measures `source` once.
    pub fn new(source: &'a str) -> Self {
        let signature = error::signature_len(source);
        let source = error::decoded(source);
        let mut multibyte = Vec::new();
        let mut surplus = 0;
        for (byte, ch) in source.char_indices() {
            if !ch.is_ascii() {
                surplus += ch.len_utf8() - ch.len_utf16();
                multibyte.push((byte + ch.len_utf8(), surplus));
            }
        }
        Self {
            source,
            signature,
            multibyte,
            total: source.len() - surplus,
        }
    }

    /// The answer [`utf16_offset`] gives for `offset`.
    pub fn offset(&self, offset: usize) -> usize {
        let byte = offset.saturating_sub(self.signature);
        if !self.source.is_char_boundary(byte) {
            return self.total;
        }
        let before = self.multibyte.partition_point(|&(end, _)| end <= byte);
        byte - before
            .checked_sub(1)
            .map_or(0, |last| self.multibyte[last].1)
    }
}
