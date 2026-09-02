//! Compilation, analysis, recovery, and diagnostic report APIs.

use super::*;

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
    /// Variant declarations imported from other modules, included in
    /// exhaustiveness checking (shadowed by local declarations; shadowing
    /// built-ins of the same name). The `ttc` CLI fills this from the
    /// file's direct relative `.tt`/`.ttx` imports.
    pub extern_variants: &'a [ExternVariant],
    /// Leave the two judgments a TypeScript checker makes better to a
    /// TypeScript checker: match exhaustiveness, and which binding a
    /// mutation path is rooted at (`val`).
    ///
    /// ttc answers both on its own, from its variant declarations and a lexical
    /// scope model of its own, and those answers are what [`compile`]
    /// reports by default. Both are approximations of TypeScript's:
    /// exhaustiveness is the *declared* type's answer, so a case an earlier
    /// guard already removed is still demanded and a variant from another
    /// module has to be collected ([`Options::extern_variants`]); `val`'s
    /// pairing is a scope model, so shadowing and redeclaration are ttc's
    /// reading rather than TypeScript's. A caller with a checker asks it
    /// instead — the narrowed type at each `match`, and symbol identity for
    /// each binding — and reports what it says. `ttc --check-types` does
    /// exactly that ([`tag_matches`], [`literal_matches`], [`val_probes`]).
    ///
    /// Every other tt-level check runs either way: duplicate cases,
    /// misplaced wildcards, bad field types, `val`'s call-capability rule.
    pub defer_to_checker: bool,
    /// Per-module rewrites for compiler-provided support modules. Missing
    /// entries leave bare specifiers such as `@tt/runtime` untouched for a
    /// bundler plugin to resolve.
    pub std_imports: StdImports<'a>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            filename: None,
            source_kind: SourceKind::TypeScript,
            verify: true,
            rewrite_imports: ImportRewrite::default(),
            extern_variants: &[],
            defer_to_checker: false,
            std_imports: StdImports::default(),
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
    let core = core_ir::lower_semantic(&semantics, source);
    let mut errors = tt_errors(source, &program, &tokens, options, &semantics);
    if errors
        .iter()
        .any(|error| error.code == DiagnosticCode::ResultNoSuccessValue)
    {
        if let Err(failure) = codegen::lowering_plan(&semantics, &core, source, options.source_kind)
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
    if let Some(first) = target_errors(&plan).into_iter().next() {
        return Err(
            diagnostics::Diagnostic::from_tt(first).to_compile_error(source, options.filename)
        );
    }
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        source,
        options.source_kind,
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
    Ok(MappedEmit {
        code: flat.code,
        mappings: flat.mappings,
        scrutinee_temps: flat.scrutinee_temps,
        payload_temps: flat.payload_temps,
        anchors: flat.anchors,
        result_return_temps: flat.result_return_temps,
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
        program,
        options.verify,
        options.defer_to_checker,
        semantics,
    );
    if !options.defer_to_checker {
        errors.extend(val::check_all(source, tokens));
    }
    // One order for every producer: where the reader's eye goes, top to
    // bottom. Stable, so equal positions keep their category order.
    errors.sort_by_key(|e| e.offset.unwrap_or(usize::MAX));
    errors
}

fn try_target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    plan.unsupported_expression_propagations()
        .into_iter()
        .map(|failure| {
            let (message, help) = try_placement_message(failure.owner, failure.reason);
            TtError::span(
                failure.source.start,
                failure.source.end,
                message.to_string(),
            )
            .code(DiagnosticCode::TryPlacement)
            .help(help)
        })
        .collect()
}

fn match_target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    plan.unsupported_matches()
        .into_iter()
        .map(|failure| {
            let (message, help) = match_placement_message(failure.owner, failure.reason);
            TtError::span(
                failure.source.start,
                failure.source.end,
                message.to_string(),
            )
            .code(DiagnosticCode::MatchPlacement)
            .help(help)
        })
        .collect()
}

fn match_placement_message(
    owner: program_syntax::EvaluationOwner,
    reason: evaluation_ir::ExpressionBoundaryReason,
) -> (&'static str, &'static str) {
    use evaluation_ir::ExpressionBoundaryReason as Reason;
    use program_syntax::EvaluationOwner;
    let help =
        "move the match into a function-body statement that can own its generated control flow";
    match (owner, reason) {
        (EvaluationOwner::ParameterInitializer, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in a parameter initializer — this TypeScript boundary has no statement position",
            help,
        ),
        (EvaluationOwner::ClassInitializer, Reason::OwnerTakesNoStatements) => (
            "`match` cannot be used in a class field initializer — this TypeScript boundary has no statement position",
            help,
        ),
        (_, Reason::RepeatedInOwner) => (
            "`match` cannot be lowered from this repeated loop position without changing how often it evaluates",
            help,
        ),
        (_, Reason::ConditionalInOwner | Reason::ConditionalOperationNotStructurable) => (
            "`match` cannot be lowered from this conditional expression position without evaluating a skipped branch",
            help,
        ),
        (_, Reason::ReferenceNotPreservable) => (
            "`match` cannot be lowered from this reference position while preserving its receiver and `this`",
            help,
        ),
        (_, Reason::CaptureOverlapsValue) => (
            "`match` cannot be lowered from this expression because its ordered source captures overlap",
            help,
        ),
        (_, Reason::ValueHasNoStatementForm | Reason::OwnerTakesNoStatements) => (
            "`match` cannot be lowered in this TypeScript host because it has no sound statement region",
            help,
        ),
    }
}

fn target_errors(plan: &evaluation_ir::LoweringPlan) -> Vec<TtError> {
    let mut errors = try_target_errors(plan);
    errors.extend(match_target_errors(plan));
    errors.sort_by_key(|error| error.offset.unwrap_or(usize::MAX));
    errors
}

fn nonredundant_target_errors(
    plan: &evaluation_ir::LoweringPlan,
    existing: &[TtError],
) -> Vec<TtError> {
    target_errors(plan)
        .into_iter()
        .filter(|target| {
            !(target.code == DiagnosticCode::TryPlacement
                && target.offset.is_some()
                && existing.iter().any(|prior| {
                    prior.code == DiagnosticCode::TryCrossesValueRegion
                        && prior.offset == target.offset
                        && prior.end == target.end
                }))
        })
        .collect()
}

fn try_placement_message(
    owner: program_syntax::EvaluationOwner,
    reason: evaluation_ir::ExpressionBoundaryReason,
) -> (&'static str, &'static str) {
    use evaluation_ir::ExpressionBoundaryReason as Reason;
    use program_syntax::EvaluationOwner;

    let help = "move the propagation into the nearest function-body statement with \
                `const value = try <expression>;`";
    match (owner, reason) {
        (EvaluationOwner::StaticBlock, _) => (
            "`try` cannot be used in a class static block — it has no enclosing function \
             failure edge for its `Err` propagation",
            help,
        ),
        (_, Reason::RepeatedInOwner) => (
            "`try` cannot be used in a repeated loop position — propagating its `Err` \
             across this TypeScript control-flow boundary would run once per iteration",
            help,
        ),
        (EvaluationOwner::ParameterInitializer, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in a parameter initializer — this TypeScript control-flow \
             boundary has no statement position for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::ClassInitializer, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in a class field initializer — this TypeScript control-flow \
             boundary has no statement position for its `Err` propagation",
            help,
        ),
        (EvaluationOwner::Constructor, _) => (
            "`try` cannot be used in a constructor — its `Err` propagation requires an \
             ordinary function return",
            help,
        ),
        (EvaluationOwner::Generator, _) => (
            "`try` cannot be used in a generator — its `Err` propagation requires an \
             ordinary function return",
            help,
        ),
        (_, Reason::ConditionalInOwner) => (
            "`try` cannot be used in a conditionally evaluated expression position — \
             propagating its `Err` across this TypeScript control-flow boundary would \
             evaluate it when the surrounding expression skips it",
            help,
        ),
        (_, Reason::ConditionalOperationNotStructurable) => (
            "`try` cannot be used in this conditional operation — its TypeScript control-flow \
             boundary cannot be rebuilt without changing evaluation order",
            help,
        ),
        (_, Reason::CaptureOverlapsValue) => (
            "`try` cannot be used in this expression context — its TypeScript control-flow \
             boundary requires overlapping source captures",
            help,
        ),
        (_, Reason::ReferenceNotPreservable) => (
            "`try` cannot be used in this expression context — its TypeScript control-flow \
             boundary requires preserving a reference that statements cannot represent",
            help,
        ),
        (_, Reason::ValueHasNoStatementForm) | (_, Reason::OwnerTakesNoStatements) => (
            "`try` cannot be used in this expression context — propagating its `Err` \
             would require moving an evaluation across its TypeScript control-flow boundary",
            help,
        ),
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
    let core = core_ir::lower_semantic(&semantics, source);
    let mut errors = tt_errors(source, &program, &tokens, options, &semantics);
    if !errors.iter().any(|error| error.code.blocks_projection()) {
        match codegen::lowering_plan(&semantics, &core, source, options.source_kind) {
            Ok(plan) => errors.extend(nonredundant_target_errors(&plan, &errors)),
            Err(failure) => errors.push(verify::in_source(source, &failure)),
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
            DiagnosticCode::TryPlacement
            | DiagnosticCode::TryCrossesValueRegion
            | DiagnosticCode::MatchPlacement => nodes.push(ast::RecoveryNode {
                span: ast::Span { start, end },
                kind: ast::RecoveryKind::Expression,
            }),
            DiagnosticCode::VariantInvalidFieldType => nodes.push(ast::RecoveryNode {
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
            ast::RecoveryKind::VariantDecl { name, exported } => {
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
    // Every overwrite replaces a whole node's byte range with ASCII, and
    // a node's range is a char boundary on both ends, so what is left is
    // still the UTF-8 it started as.
    let recovered_source =
        String::from_utf8(recovered).expect("recovery replaces whole nodes with ASCII");
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
    let semantics = analysis::coverage_semantics(source, &program, options.extern_variants);
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
    let flat = codegen::emit_with_map(
        &semantics,
        &core,
        source,
        options.source_kind,
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
        result_return_temps: flat.result_return_temps,
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
                &parser::unclaimed_candidates(&program),
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
