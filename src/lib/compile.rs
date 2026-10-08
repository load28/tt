//! Compilation, analysis, recovery, and diagnostic report APIs.

use super::*;

#[path = "compile/recovery.rs"]
mod recovery;
#[path = "compile/target_diagnostics.rs"]
mod target_diagnostics;

use recovery::{
    declare_recovered_variants, outermost_recoveries, recover_source, recoverable_constructs,
};
use target_diagnostics::{nonredundant_target_errors, target_errors};

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
/// assert!(opts.extern_variants.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// Filename reported in [`CompileError`]s (and their `Display` output).
    /// An existing file also locates the project for contextual type analysis.
    /// `None` renders as `<input>`.
    pub filename: Option<&'a str>,
    /// Whether the source surface is TypeScript or TSX.
    pub source_kind: SourceKind,
    /// Validate variant field types and the generated output with swc.
    /// Corresponds to the CLI's `--no-verify` escape hatch when `false`;
    /// disabling it lets syntactically bad field types flow into the output
    /// (where tsc will report them) and skips the emitted-code self-check.
    pub verify: bool,
    /// How relative `.tt`/`.ttx` import specifiers are rewritten in the output.
    pub rewrite_imports: ImportRewrite,
    /// Whether the project's TypeScript compiles JSX with `"jsx":
    /// "preserve"`. TypeScript names the JavaScript it emits for a `.tsx`
    /// file `.jsx` under `preserve` and `.js` under every other `jsx` value
    /// or none (`GetOutputExtension` in typescript-go's
    /// `internal/outputpaths`), so [`ImportRewrite::Js`] rewrites `./x.ttx`
    /// to `./x.jsx` only when this is set. The CLI reads it from the
    /// project's `tsconfig.json`. `false` by default, TypeScript's default.
    pub jsx_preserve: bool,
    /// Variant declarations imported from other modules, included in
    /// exhaustiveness checking (shadowed by local declarations; shadowing
    /// built-ins of the same name). The `ttc` CLI fills this from the
    /// file's direct relative `.tt`/`.ttx` imports.
    pub extern_variants: &'a [ExternVariant],
    /// Leave contextual storage typing and the two judgments below to a
    /// project TypeScript checker: match exhaustiveness, and which binding a
    /// mutation path is rooted at (`val`).
    ///
    /// ttc answers both on its own, from its variant declarations and a lexical
    /// scope model of its own, and those answers are what [`compile`]
    /// reports by default. Exhaustiveness is the *declared* type's answer,
    /// which is the language's rule on every surface: a case an earlier
    /// guard already removed is still demanded, and a variant from another
    /// module has to be collected ([`Options::extern_variants`]). A caller
    /// with a checker reports that answer too, and adds what the type at
    /// each `match` shows where the declarations cannot answer; `val`'s
    /// pairing is a scope model, so shadowing and redeclaration are ttc's
    /// reading rather than TypeScript's, and a caller with a checker pairs
    /// by symbol identity instead. `ttc --check-types` does exactly that
    /// ([`tag_matches`], [`literal_matches`], [`val_probes`]).
    ///
    /// Every other tt-level check runs either way: duplicate cases,
    /// misplaced wildcards, bad field types, `val`'s call-capability rule.
    pub defer_to_checker: bool,
    /// Per-module rewrites for compiler-provided support modules. Missing
    /// entries leave bare specifiers such as `@tt/runtime` untouched for a
    /// bundler plugin to resolve.
    pub std_imports: StdImports<'a>,
    /// The node binary the TypeScript client for contextual type analysis
    /// runs with. `None` runs `node` from `PATH`.
    pub node: Option<&'a std::path::Path>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            filename: None,
            source_kind: SourceKind::TypeScript,
            verify: true,
            rewrite_imports: ImportRewrite::default(),
            jsx_preserve: false,
            extern_variants: &[],
            defer_to_checker: false,
            std_imports: StdImports::default(),
            node: None,
        }
    }
}

/// Compile tt source text to TypeScript or TSX source text.
///
/// Only tt constructs (`variant` declarations, `match` expressions, `try` and
/// let-else statements) and relative `.tt`/`.ttx` import specifiers (per
/// [`Options::rewrite_imports`]) are rewritten; everything else — including
/// all plain TypeScript `enum` forms — passes through byte for byte. A
/// candidate construct that does not fully parse as tt syntax is passed
/// through untouched rather than reported as an error.
/// The output has no generated banner comment (that is added by the CLI).
/// A pipeline that needs contextual application imports its helper from
/// `@tt/runtime`; project builders materialize that module once, while a
/// single-file adapter can replace its specifier through [`Options::std_imports`].
///
/// # Errors
///
/// Returns a [`CompileError`] with a 1-based position in `source` for every
/// tt-level rule violation: duplicate variant cases, invalid field types,
/// duplicate or misplaced `match` arms, and non-exhaustive matches over variants
/// declared in this source. With [`Options::verify`] enabled, a final
/// self-check that the generated output parses as TypeScript can also fail
/// (reported without a position). Run `ttc help errors` for guidance.
///
/// ```
/// use ttc::{compile, Options};
///
/// let source = "variant E { A(x: number), B }\nconst v = match (E.A(1)) { A(x) => x };";
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
    let semantics = analysis::coverage_semantics(source, &program, options.extern_variants);
    let core = core_ir::lower_semantic(&semantics, source, &tokens);
    let mut errors = tt_errors(source, &program, &tokens, options, &semantics);
    if errors
        .iter()
        .any(|error| error.code == DiagnosticCode::ResultNoSuccessValue)
    {
        if let Err(failure) =
            codegen::lowering_plan(&semantics, &core, source, options.source_kind, &tokens)
        {
            errors.push(verify::in_source(source, &failure));
        }
        suppress_discarded_result_fallthrough(&mut errors);
    }
    errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
    if let Some(first) = errors.into_iter().next() {
        return Err(
            diagnostics::Diagnostic::from_tt(first).to_compile_error(source, options.filename)
        );
    }
    let mut plan =
        match codegen::lowering_plan(&semantics, &core, source, options.source_kind, &tokens) {
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
    if let Some(first) = target_errors(&plan).into_iter().next() {
        return Err(
            diagnostics::Diagnostic::from_tt(first).to_compile_error(source, options.filename)
        );
    }
    let automatic_semicolons = crate::lexer::automatic_semicolons(&tokens);
    let comments = crate::lexer::comments(source, &tokens);
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        codegen::EmitSource {
            text: source,
            kind: options.source_kind,
            automatic_semicolons: &automatic_semicolons,
            comments: &comments,
        },
        &plan,
        options.rewrite_imports.extensions(options.jsx_preserve),
        options.std_imports,
    );
    if options.verify
        && let Err(failure) = verify::verify_emit(
            &flat.code,
            options.source_kind,
            &automatic_semicolons,
            &flat.mappings,
        )
    {
        // The self-check reads the *generated* module, but the user only
        // has the `.tt` file open. A position in a file no one wrote is
        // not a position, so it is carried back through the mappings to
        // the source — and where the failure fell in a construct's glue,
        // that construct is named. (Without this the error arrives with no
        // position at all and an editor pins it to line 1.)
        let failure = verify::at_source(
            &parser::unclaimed_candidates(&program),
            &flat.mappings,
            &flat.anchors,
            &flat.code,
            &failure,
        );
        return Err(
            diagnostics::Diagnostic::from_tt(failure).to_compile_error(source, options.filename)
        );
    }
    let emit = MappedEmit {
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
        restatements: flat.restatements,
        generated_names: flat.generated_names,
        declared_names: flat.declared_names,
        shared_bindings: flat.shared_bindings,
        destructured_lists: flat.destructured_lists,
        relocated_operands: flat.relocated_operands,
        inserted: flat.inserted,
        single_line_breaks: flat.single_line_breaks,
        completion_scopes: std::mem::take(&mut plan.completion_scopes),
        support_imports: flat.support_imports,
        commonjs: flat.commonjs,
    };
    if options.defer_to_checker {
        return Ok(emit);
    }
    // A missing toolchain is answered inside the pass, which is the only
    // place that can tell it from a project it could not read; what
    // reaches here is a failure either way.
    crate::typescript::contextual::standalone(emit, source, options).map_err(|failure| {
        CompileError {
            message: failure.to_string(),
            filename: options.filename.map(str::to_owned),
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
        }
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
        source,
        options.source_kind,
        program,
        options.verify,
        options.defer_to_checker,
        semantics,
        tokens,
    );
    if !options.defer_to_checker {
        errors.extend(val::check_all(
            source,
            options.source_kind,
            tokens,
            &parser::val_modifiers(program),
            &parser::pipeline_shapes(program),
            &parser::arm_scopes(program),
        ));
    }
    // One order for every producer: where the reader's eye goes, top to
    // bottom. Stable, so equal positions keep their category order.
    errors.sort_by_key(|e| e.offset.unwrap_or(usize::MAX));
    errors
}

fn recovered_target_errors(
    failure: &codegen::LoweringFailure,
    semantics: &analysis::SemanticFile,
    core: &core_ir::CoreFile,
    source: &str,
    tokens: &[lexer::Token],
    options: &Options,
    existing: &[TtError],
) -> Vec<TtError> {
    if !matches!(
        failure,
        codegen::LoweringFailure::SourceNotTypeScript { .. }
    ) {
        return Vec::new();
    }
    match codegen::lowering_plan_with(
        semantics,
        core,
        source,
        options.source_kind,
        tokens,
        crate::program_syntax::SyntaxMode::Diagnostic,
    ) {
        Ok(plan) => nonredundant_target_errors(&plan, existing),
        Err(_) => Vec::new(),
    }
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
/// let source = "variant E { A(x: number), B }\n\
///     const a = match (E.A(1)) { A(x) => x };\n\
///     const b = match (E.B) { B => 0 };\n";
/// let diagnostics = ttc::analyze(source, &ttc::Options::default());
/// assert_eq!(diagnostics.len(), 2);
/// assert!(diagnostics.iter().all(|d| d.code == ttc::DiagnosticCode::MatchNotExhaustive));
/// ```
pub fn analyze(source: &str, options: &Options) -> Vec<Diagnostic> {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    let semantics = analysis::coverage_semantics(source, &program, options.extern_variants);
    let core = core_ir::lower_semantic(&semantics, source, &tokens);
    let mut errors = tt_errors(source, &program, &tokens, options, &semantics);
    if !errors.iter().any(|error| error.code.blocks_projection()) {
        match codegen::lowering_plan(&semantics, &core, source, options.source_kind, &tokens) {
            Ok(plan) => errors.extend(nonredundant_target_errors(&plan, &errors)),
            Err(failure) => {
                errors.push(verify::in_source(source, &failure));
                errors.extend(recovered_target_errors(
                    &failure, &semantics, &core, source, &tokens, options, &errors,
                ));
            }
        }
    }
    suppress_discarded_result_fallthrough(&mut errors);
    errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
    errors
        .into_iter()
        .map(diagnostics::Diagnostic::from_tt)
        .collect()
}

/// A discarded Result makes its value-use error primary. Reporting the
/// nested block's fallthrough as well would ask for a return from an
/// expression the user must first stop discarding.
fn suppress_discarded_result_fallthrough(errors: &mut Vec<TtError>) {
    let discarded: Vec<_> = errors
        .iter()
        .filter(|error| error.code == DiagnosticCode::ResultValueDiscarded)
        .filter_map(|error| error.offset.zip(error.end))
        .collect();
    errors.retain(|error| {
        error.code != DiagnosticCode::ResultNoSuccessValue
            || !error.offset.zip(error.end).is_some_and(|(start, end)| {
                discarded
                    .iter()
                    .any(|(outer_start, outer_end)| *outer_start <= start && end <= *outer_end)
            })
    });
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
pub struct ProjectionReport {
    /// Editor projection, including valid siblings of malformed syntax nodes.
    pub emit: Option<MappedEmit>,
    /// This projection retains malformed host syntax for the editor only.
    /// It must never authorize build or declaration output.
    pub editor_only: bool,
    /// Original-source diagnostics, independent of recovery substitutions.
    pub diagnostics: Vec<Diagnostic>,
    /// Source byte ranges occupied by parser recovery nodes.
    pub recovered: Vec<(usize, usize)>,
    /// Host productions whose missing syntax was materialized. Their
    /// original syntax diagnostic remains authoritative; type checking continues.
    pub syntax_repairs: Vec<(usize, usize)>,
    /// Those of `recovered` that stand for a declaration (a malformed
    /// variant, whose name the projection declares with the error type) or
    /// sit in an exported statement, whose declared type the placeholder
    /// would decide.
    pub recovered_declarations: Vec<(usize, usize)>,
    /// The emission `emit` withholds from the typed program when the only
    /// thing wrong with the (recovered) file is its TypeScript: it does not
    /// parse, or its lowering plan could not be built over it. It is
    /// lowered without that plan, and every byte of it is TypeScript the
    /// user wrote, glue a claimed construct lowered to, or a recovery
    /// placeholder in `recovered` — no tt text is left as written — so
    /// what a TypeScript reader says about it is what it says about the
    /// user's code. `None` when `emit` is present, or when a construct
    /// would stay the tt text the user wrote.
    pub withheld: Option<MappedEmit>,
}

/// The emission of a file that has no verified one, when no tt text in it
/// is left as written: no diagnostic leaves its construct unlowered
/// ([`DiagnosticCode::leaves_tt_text`]) and the parser rolled back no tt
/// candidate into passthrough text.
fn withheld_emit(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[lexer::Token],
    diagnostics: &[Diagnostic],
) -> Option<MappedEmit> {
    if diagnostics.iter().any(|d| d.code.leaves_tt_text())
        || !parser::unclaimed_candidates(program).is_empty()
    {
        return None;
    }
    Some(emit_mapped_parsed(source, options, program, tokens))
}

/// Builds a valid TypeScript projection in the presence of parser-owned
/// error nodes. Replacements are byte-length preserving, so every mapping
/// outside the recovered node remains in the original source coordinate
/// space.
pub fn compile_projection_report(source: &str, options: &Options) -> ProjectionReport {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    compile_projection_report_parsed(source, options, &program, &tokens)
}

pub(crate) fn compile_projection_report_parsed(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[lexer::Token],
) -> ProjectionReport {
    let mut ordinary = compile_report_parsed(source, options, program, tokens);
    if ordinary.emit.is_some() {
        return ProjectionReport {
            emit: ordinary.emit,
            editor_only: false,
            diagnostics: ordinary.diagnostics,
            recovered: Vec::new(),
            syntax_repairs: Vec::new(),
            recovered_declarations: Vec::new(),
            withheld: None,
        };
    }

    if let Some(EditorHostEmit {
        emit,
        repaired: recovered,
        diagnostics: added,
    }) = editor_host_emit(source, options, program, tokens, &ordinary.diagnostics)
    {
        ordinary.diagnostics.extend(added);
        return ProjectionReport {
            emit: Some(emit),
            editor_only: true,
            diagnostics: ordinary.diagnostics,
            recovered: Vec::new(),
            syntax_repairs: recovered,
            recovered_declarations: Vec::new(),
            withheld: None,
        };
    }

    let mut nodes = parser::projection_recoveries(program);
    if ordinary
        .diagnostics
        .iter()
        .any(|d| d.code.restates_typescript_syntax())
    {
        let semantic = analysis::coverage_semantics(source, program, options.extern_variants);
        let core = core_ir::lower_semantic(&semantic, source, tokens);
        if let Ok(lost) = crate::program_syntax::lost_editor_values(
            &semantic,
            &core,
            source,
            options.source_kind,
            tokens,
        ) {
            nodes.extend(lost.into_iter().map(|span| ast::RecoveryNode {
                span: ast::Span {
                    start: span.start,
                    end: span.end,
                },
                kind: ast::RecoveryKind::Expression,
            }));
        }
    }
    nodes.extend(recoverable_constructs(&ordinary.diagnostics));
    let claimed_results = parser::claimed_result_blocks(program);
    // A construct the plan rejects is reported when planning reaches it, so
    // a file with several can show the next one only once the first is
    // recovered. Each round recovers at least one construct more, until the
    // projection emits or no recoverable construct is left.
    loop {
        nodes.sort_by_key(|node| (node.span.start, std::cmp::Reverse(node.span.end)));
        let selected = outermost_recoveries(nodes);
        if selected.is_empty() {
            return ProjectionReport {
                emit: None,
                editor_only: true,
                withheld: withheld_emit(source, options, program, tokens, &ordinary.diagnostics),
                diagnostics: ordinary.diagnostics,
                recovered: Vec::new(),
                syntax_repairs: Vec::new(),
                recovered_declarations: Vec::new(),
            };
        }
        let recovered_source = recover_source(source, &selected);
        let (recovered_program, recovered_tokens) =
            parser::lex_and_parse_with_kind(&recovered_source, options.source_kind);
        // A `result` block is claimed by the direct `try` it holds; a
        // recovery that replaced that `try` leaves the block's text as
        // written, which TypeScript would read as code. The block it
        // unclaimed is the construct to recover.
        let still_claimed = parser::claimed_result_blocks(&recovered_program);
        let unclaimed: Vec<_> = claimed_results
            .iter()
            .filter(|span| !still_claimed.contains(span))
            .filter(|span| {
                !selected
                    .iter()
                    .any(|outer| outer.span.start <= span.start && span.end <= outer.span.end)
            })
            .map(|&span| ast::RecoveryNode {
                span,
                kind: ast::RecoveryKind::Expression,
            })
            .collect();
        if !unclaimed.is_empty() {
            nodes = selected;
            nodes.extend(unclaimed);
            continue;
        }
        let mut recovered_report = compile_report_parsed(
            &recovered_source,
            options,
            &recovered_program,
            &recovered_tokens,
        );
        let mut editor_only = false;
        let mut host_recovered = Vec::new();
        if recovered_report.emit.is_none() {
            if let Some(EditorHostEmit {
                emit,
                repaired: recovered,
                diagnostics: added,
            }) = editor_host_emit(
                &recovered_source,
                options,
                &recovered_program,
                &recovered_tokens,
                &recovered_report.diagnostics,
            ) {
                recovered_report.emit = Some(emit);
                host_recovered = recovered;
                ordinary.diagnostics.extend(added);
            }
            editor_only = recovered_report.emit.is_some();
        }
        let further: Vec<_> = if recovered_report.emit.is_some() {
            Vec::new()
        } else {
            recoverable_constructs(&recovered_report.diagnostics)
                .into_iter()
                .filter(|node| {
                    !selected.iter().any(|outer| {
                        outer.span.start <= node.span.start && node.span.end <= outer.span.end
                    })
                })
                .collect()
        };
        if further.is_empty() {
            let withheld = match recovered_report.emit {
                Some(_) => None,
                None => withheld_emit(
                    &recovered_source,
                    options,
                    &recovered_program,
                    &recovered_tokens,
                    &recovered_report.diagnostics,
                ),
            };
            return ProjectionReport {
                editor_only,
                syntax_repairs: host_recovered,
                emit: recovered_report
                    .emit
                    .map(|emit| declare_recovered_variants(emit, &selected)),
                withheld: withheld.map(|emit| declare_recovered_variants(emit, &selected)),
                diagnostics: ordinary.diagnostics,
                recovered_declarations: {
                    let tokens =
                        crate::lexer::lex_with_kind(source, 0, source.len(), options.source_kind);
                    selected
                        .iter()
                        .filter(|node| {
                            matches!(node.kind, ast::RecoveryKind::VariantDecl { .. })
                                || in_exported_statement(source, &tokens, node.span.start)
                        })
                        .map(|node| (node.span.start, node.span.end))
                        .collect()
                },
                recovered: selected
                    .into_iter()
                    .map(|node| (node.span.start, node.span.end))
                    .collect(),
            };
        }
        nodes = selected;
        nodes.extend(further);
    }
}

struct EditorHostEmit {
    emit: MappedEmit,
    repaired: Vec<(usize, usize)>,
    diagnostics: Vec<Diagnostic>,
}

/// Lower the parser's current editor tree. Missing syntax is materialized
/// before lowering so an unfinished production cannot capture the following
/// generated prelude. Insertions never acquire authored-source mappings.
fn editor_host_emit(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[lexer::Token],
    diagnostics: &[Diagnostic],
) -> Option<EditorHostEmit> {
    if !diagnostics
        .iter()
        .any(|d| d.code.restates_typescript_syntax())
    {
        return None;
    }
    if diagnostics.iter().any(|d| d.code.leaves_tt_text())
        || !parser::unclaimed_candidates(program).is_empty()
    {
        return None;
    }
    let semantic = analysis::coverage_semantics(source, program, options.extern_variants);
    let core = core_ir::lower_semantic(&semantic, source, tokens);
    // No host placement is needed for pass-through text. Let TypeScript
    // diagnose that text verbatim rather than repairing syntax unnecessarily.
    if !core.requires_host_lowering() && !core.imports_std() {
        return report_parsed(
            source,
            options,
            program,
            tokens,
            false,
            crate::program_syntax::SyntaxMode::Editor,
        )
        .emit
        .map(|emit| EditorHostEmit {
            emit,
            repaired: Vec::new(),
            diagnostics: Vec::new(),
        });
    }
    if !crate::program_syntax::lost_editor_values(
        &semantic,
        &core,
        source,
        options.source_kind,
        tokens,
    )
    .ok()?
    .is_empty()
    {
        return None;
    }
    let insertions = crate::program_syntax::editor_insertions(
        &semantic,
        &core,
        source,
        options.source_kind,
        tokens,
    )
    .ok()?;
    if insertions.is_empty() {
        return report_parsed(
            source,
            options,
            program,
            tokens,
            false,
            crate::program_syntax::SyntaxMode::Editor,
        )
        .emit
        .map(|emit| EditorHostEmit {
            emit,
            repaired: Vec::new(),
            diagnostics: Vec::new(),
        });
    }
    let recovered = insertions
        .iter()
        .filter(|insertion| !insertion.terminates)
        .map(|insertion| (insertion.owner.start, insertion.owner.end))
        .collect();
    let mut added = Vec::new();
    for insertion in insertions.iter().filter(|insertion| !insertion.terminates) {
        if diagnostics.iter().chain(&added).any(|diagnostic| {
            diagnostic.code.restates_typescript_syntax()
                && diagnostic
                    .start
                    .is_some_and(|at| insertion.owner.start < at && at <= insertion.at)
        }) {
            continue;
        }
        added.push(Diagnostic {
            code: DiagnosticCode::SourceNotTypeScript,
            severity: Severity::Error,
            message: format!("expected {}", insertion.expected),
            start: Some(insertion.at),
            end: Some(insertion.at),
            owner: None,
            suggestions: Vec::new(),
            labels: Vec::new(),
        });
    }
    let repaired = crate::recovery::EditorSource::new(
        source,
        insertions
            .into_iter()
            .map(|insertion| (insertion.at, insertion.text))
            .collect(),
    );
    let (program, tokens) = parser::lex_and_parse_with_kind(&repaired.text, options.source_kind);
    report_parsed(
        &repaired.text,
        options,
        &program,
        &tokens,
        false,
        crate::program_syntax::SyntaxMode::Editor,
    )
    .emit
    .map(|emit| EditorHostEmit {
        emit: repaired.restore(emit),
        repaired: recovered,
        diagnostics: added,
    })
}

fn verified_emit(
    emit: MappedEmit,
    program: &ast::Program,
    automatic_semicolons: &[crate::lexer::AutomaticSemicolon],
    options: &Options,
    errors: &mut Vec<TtError>,
) -> Option<MappedEmit> {
    if !options.verify {
        return Some(emit);
    }
    let Err(failure) = verify::verify_emit(
        &emit.code,
        options.source_kind,
        automatic_semicolons,
        &emit.mappings,
    ) else {
        return Some(emit);
    };
    // A failed self-check *with tt errors already reported* is the
    // effect, not a second cause — the emitted text reflects the
    // invalid construct those errors name (e.g. a module-level `try`'s
    // `return`), and the backstop's "or a ttc bug" wording would
    // mislead. Report the causes and withhold the emit; the check
    // reappears on its own once they are fixed.
    if errors.is_empty() {
        errors.push(verify::at_source(
            &parser::unclaimed_candidates(program),
            &emit.mappings,
            &emit.anchors,
            &emit.code,
            &failure,
        ));
    }
    None
}

/// Compiles `source` and reports everything — the multi-diagnostic,
/// still-emitting form of [`compile_mapped`]. See [`CompileReport`].
pub fn compile_report(source: &str, options: &Options) -> CompileReport {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    compile_report_parsed(source, options, &program, &tokens)
}

/// [`compile_report`] for a caller that discards the emission, such as
/// `ttc --check`: the same diagnostics and output self-check, over the
/// emission before contextual refinement. That refinement only annotates
/// generated storage in the output a TypeScript project reads, so this
/// never reaches TypeScript.
pub fn check_report(source: &str, options: &Options) -> CompileReport {
    let (program, tokens) = parser::lex_and_parse_with_kind(source, options.source_kind);
    report_parsed(
        source,
        options,
        &program,
        &tokens,
        false,
        crate::program_syntax::SyntaxMode::Strict,
    )
}

pub(crate) fn compile_report_parsed(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[lexer::Token],
) -> CompileReport {
    report_parsed(
        source,
        options,
        program,
        tokens,
        true,
        crate::program_syntax::SyntaxMode::Strict,
    )
}

fn report_parsed(
    source: &str,
    options: &Options,
    program: &ast::Program,
    tokens: &[lexer::Token],
    refine: bool,
    mode: crate::program_syntax::SyntaxMode,
) -> CompileReport {
    let semantics = analysis::coverage_semantics(source, program, options.extern_variants);
    let core = core_ir::lower_semantic(&semantics, source, tokens);
    let mut errors = tt_errors(source, program, tokens, options, &semantics);
    if errors.iter().any(|e| e.code.blocks_projection()) {
        return CompileReport {
            emit: None,
            diagnostics: errors
                .into_iter()
                .map(diagnostics::Diagnostic::from_tt)
                .collect(),
        };
    }
    let mut plan = match codegen::lowering_plan_with(
        &semantics,
        &core,
        source,
        options.source_kind,
        tokens,
        mode,
    ) {
        Ok(plan) => plan,
        // Same class as a projection-blocking tt diagnostic: the file has
        // no emittable form, and the cause is reported with everything
        // else already found.
        Err(failure) => {
            errors.push(verify::in_source(source, &failure));
            errors.extend(recovered_target_errors(
                &failure, &semantics, &core, source, tokens, options, &errors,
            ));
            errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
            return CompileReport {
                emit: None,
                diagnostics: errors
                    .into_iter()
                    .map(diagnostics::Diagnostic::from_tt)
                    .collect(),
            };
        }
    };
    let target_errors = nonredundant_target_errors(&plan, &errors);
    if !target_errors.is_empty() {
        errors.extend(target_errors);
        errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
        return CompileReport {
            emit: None,
            diagnostics: errors
                .into_iter()
                .map(diagnostics::Diagnostic::from_tt)
                .collect(),
        };
    }
    let automatic_semicolons = crate::lexer::automatic_semicolons(tokens);
    let comments = crate::lexer::comments(source, tokens);
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        codegen::EmitSource {
            text: source,
            kind: options.source_kind,
            automatic_semicolons: &automatic_semicolons,
            comments: &comments,
        },
        &plan,
        options.rewrite_imports.extensions(options.jsx_preserve),
        options.std_imports,
    );
    let lowered = MappedEmit {
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
        restatements: flat.restatements,
        generated_names: flat.generated_names,
        declared_names: flat.declared_names,
        shared_bindings: flat.shared_bindings,
        destructured_lists: flat.destructured_lists,
        relocated_operands: flat.relocated_operands,
        inserted: flat.inserted,
        single_line_breaks: flat.single_line_breaks,
        completion_scopes: std::mem::take(&mut plan.completion_scopes),
        support_imports: flat.support_imports,
        commonjs: flat.commonjs,
    };
    let mut emit = if mode == crate::program_syntax::SyntaxMode::Editor {
        Some(lowered)
    } else {
        verified_emit(
            lowered,
            program,
            &automatic_semicolons,
            options,
            &mut errors,
        )
    };
    if refine
        && !options.defer_to_checker
        && let Some(lowered) = emit.take()
    {
        let annotated = !lowered.contextual_slots.is_empty();
        match crate::typescript::contextual::standalone(lowered, source, options) {
            Ok(typed) if annotated => {
                emit = verified_emit(typed, program, &automatic_semicolons, options, &mut errors);
            }
            Ok(typed) => emit = Some(typed),
            Err(failure) => errors.push(TtError::positionless(failure.to_string())),
        }
    }
    CompileReport {
        emit,
        diagnostics: errors
            .into_iter()
            .map(diagnostics::Diagnostic::from_tt)
            .collect(),
    }
}

fn in_exported_statement(source: &str, tokens: &[crate::lexer::Token], at: usize) -> bool {
    let mut depth = 0usize;
    let mut statement = None;
    for token in tokens.iter().take_while(|token| token.span.start < at) {
        if depth == 0 && token.facts.statement_start() {
            statement = Some(token);
        }
        if token.opens_bracket() {
            depth += 1;
        } else if token.closes_bracket() {
            depth = depth.saturating_sub(1);
        }
    }
    statement.is_some_and(|token| &source[token.span.start..token.span.end] == "export")
}
